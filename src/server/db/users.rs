//! Users, their per-case capability assignments, and admin mutations.

use crate::server::db::{audit, ids, pool};
use crate::types::{AccountRole, CaseAssignment, CaseCapability, Page, User};
use std::collections::{BTreeMap, HashMap};

#[derive(sqlx::FromRow)]
struct UserRow {
    id: String,
    first_name: String,
    last_name: String,
    email: String,
    phone: String,
    home_address: String,
    role: String,
}

impl UserRow {
    fn into_user(self, assigned_cases: Vec<CaseAssignment>, audit_log: Vec<crate::types::ChangeLogEntry>) -> User {
        User {
            id: self.id,
            first_name: self.first_name,
            last_name: self.last_name,
            email: self.email,
            phone: self.phone,
            home_address: self.home_address,
            // Passwords are never surfaced through the domain type anymore; the
            // hash stays in the DB. Keep the field for API compatibility.
            password: String::new(),
            role: AccountRole::from_slug(&self.role).unwrap_or(AccountRole::Client),
            assigned_cases,
            audit_log,
        }
    }
}

/// Group flat `(case_id, capability)` rows into per-case [`CaseAssignment`]s,
/// ordered by case id. Shared by every code path that turns a user's
/// `case_assignments` rows into the domain shape.
fn group_assignments(rows: Vec<(String, String)>) -> Vec<CaseAssignment> {
    let mut grouped: BTreeMap<String, Vec<CaseCapability>> = BTreeMap::new();
    for (case_id, cap) in rows {
        if let Some(c) = CaseCapability::from_slug(&cap) {
            grouped.entry(case_id).or_default().push(c);
        }
    }
    grouped
        .into_iter()
        .map(|(case_id, capabilities)| CaseAssignment {
            case_id,
            capabilities,
        })
        .collect()
}

async fn load_assignments(user_id: &str) -> Result<Vec<CaseAssignment>, sqlx::Error> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT case_id, capability FROM case_assignments WHERE user_id = $1 ORDER BY case_id",
    )
    .bind(user_id)
    .fetch_all(pool())
    .await?;
    Ok(group_assignments(rows))
}

async fn hydrate(row: UserRow) -> Result<User, sqlx::Error> {
    let assignments = load_assignments(&row.id).await?;
    let audit_log = audit::for_entity(pool(), audit::Entity::User, &row.id).await?;
    Ok(row.into_user(assignments, audit_log))
}

/// Batched capability assignments for many users, grouped by user id then case id
/// (mirrors [`load_assignments`] but for a whole page in one query).
async fn load_assignments_many(
    user_ids: &[String],
) -> Result<HashMap<String, Vec<CaseAssignment>>, sqlx::Error> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT user_id, case_id, capability FROM case_assignments
         WHERE user_id = ANY($1) ORDER BY user_id, case_id",
    )
    .bind(user_ids)
    .fetch_all(pool())
    .await?;

    let mut grouped: HashMap<String, BTreeMap<String, Vec<CaseCapability>>> = HashMap::new();
    for (user_id, case_id, cap) in rows {
        if let Some(c) = CaseCapability::from_slug(&cap) {
            grouped
                .entry(user_id)
                .or_default()
                .entry(case_id)
                .or_default()
                .push(c);
        }
    }
    Ok(grouped
        .into_iter()
        .map(|(user_id, cases)| {
            (
                user_id,
                cases
                    .into_iter()
                    .map(|(case_id, capabilities)| CaseAssignment {
                        case_id,
                        capabilities,
                    })
                    .collect(),
            )
        })
        .collect())
}

/// Fully hydrate a page of users with a fixed, small number of queries instead
/// of one-query-per-sub-resource-per-user (an N+1 pattern). The two independent
/// batch loads run concurrently.
async fn hydrate_many(rows: Vec<UserRow>) -> Result<Vec<User>, sqlx::Error> {
    if rows.is_empty() {
        return Ok(Vec::new());
    }
    let ids: Vec<String> = rows.iter().map(|r| r.id.clone()).collect();

    let (mut assignments, mut audit_log) = tokio::try_join!(
        load_assignments_many(&ids),
        audit::for_entities(pool(), audit::Entity::User, &ids),
    )?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let a = assignments.remove(&row.id).unwrap_or_default();
            let log = audit_log.remove(&row.id).unwrap_or_default();
            row.into_user(a, log)
        })
        .collect())
}

/// One page of users (ordered by id), each fully hydrated with assignments and
/// audit log, plus the total number of users matching the search.
///
/// This replaces the old "load every user" behaviour: it fetches — and runs the
/// per-user hydration queries for — only the requested window, so it stays cheap
/// no matter how large the table grows. `search`, when non-blank, matches a
/// user's id, first/last name, full name ("first last"), or email
/// case-insensitively. `limit` is clamped
/// to a sane range so a caller can never request an unbounded scan.
pub async fn page(offset: i64, limit: i64, search: &str) -> Result<Page<User>, sqlx::Error> {
    let limit = limit.clamp(1, 100);
    let offset = offset.max(0);

    // Build an escaped `%term%` pattern, or `None` for "no filter". Escaping the
    // LIKE metacharacters means user input is matched literally.
    let term = search.trim();
    let pattern = if term.is_empty() {
        None
    } else {
        Some(format!(
            "%{}%",
            term.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
        ))
    };

    // The same predicate drives the count and the page fetch. A NULL pattern
    // (no search term) matches every row.
    const FILTER: &str = "WHERE $1::text IS NULL
           OR id ILIKE $1
           OR first_name ILIKE $1
           OR last_name ILIKE $1
           OR (first_name || ' ' || last_name) ILIKE $1
           OR email ILIKE $1";

    let count_sql = format!("SELECT count(*) FROM users {FILTER}");
    let page_sql = format!(
        "SELECT id, first_name, last_name, email, phone, home_address, role
         FROM users {FILTER} ORDER BY id LIMIT $2 OFFSET $3"
    );

    let total_fut = sqlx::query_scalar::<_, i64>(&count_sql)
        .bind(&pattern)
        .fetch_one(pool());

    let rows_fut = sqlx::query_as::<_, UserRow>(&page_sql)
        .bind(&pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool());

    // The count and the page fetch are independent, so run them concurrently —
    // one fewer sequential round-trip against a networked database.
    let (total, rows) = tokio::try_join!(total_fut, rows_fut)?;

    let items = hydrate_many(rows).await?;
    Ok(Page { items, total })
}

/// A single user by id.
pub async fn get(id: &str) -> Result<Option<User>, sqlx::Error> {
    let row = sqlx::query_as::<_, UserRow>(
        "SELECT id, first_name, last_name, email, phone, home_address, role FROM users WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool())
    .await?;
    match row {
        Some(r) => Ok(Some(hydrate(r).await?)),
        None => Ok(None),
    }
}

/// A single user with their per-case capability assignments, but **without** the
/// audit log. Loads the user row and the assignments concurrently.
///
/// This is the lighter counterpart to [`get`]: authorization only needs
/// identity, role, and capabilities — never the user's own change history (which
/// the client never renders for the signed-in user), so we skip that query.
pub async fn get_with_capabilities(id: &str) -> Result<Option<User>, sqlx::Error> {
    let row_fut = sqlx::query_as::<_, UserRow>(
        "SELECT id, first_name, last_name, email, phone, home_address, role FROM users WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool());

    let (row, assignments) = tokio::try_join!(row_fut, load_assignments(id))?;
    Ok(row.map(|r| r.into_user(assignments, Vec::new())))
}

/// Resolve a signed-in user (with capabilities, no audit log) from a session
/// token hash. Scoped to the *one* session matching the token — the two queries
/// (the session's user, and that user's assignments) are independent, so they
/// run concurrently for a single effective round-trip without a fan-out JOIN
/// that would ship the user's row once per assignment.
///
/// Returns `None` when there is no live (unexpired) session for the token.
pub async fn resolve_by_session_token(token_hash: &str) -> Result<Option<User>, sqlx::Error> {
    // The session's user (exactly one row, or none if the token is unknown/expired).
    let user_fut = sqlx::query_as::<_, UserRow>(
        "SELECT u.id, u.first_name, u.last_name, u.email, u.phone, u.home_address, u.role
         FROM sessions s
         JOIN users u ON u.id = s.user_id
         WHERE s.token_hash = $1 AND s.expires_at > now()",
    )
    .bind(token_hash)
    .fetch_optional(pool());

    // That same user's capability assignments (each a small `(case_id, capability)`
    // row), scoped through the identical session predicate.
    let assignments_fut = sqlx::query_as::<_, (String, String)>(
        "SELECT case_id, capability FROM case_assignments
         WHERE user_id = (SELECT user_id FROM sessions
                          WHERE token_hash = $1 AND expires_at > now())
         ORDER BY case_id",
    )
    .bind(token_hash)
    .fetch_all(pool());

    let (row, assignment_rows) = tokio::try_join!(user_fut, assignments_fut)?;
    Ok(row.map(|r| r.into_user(group_assignments(assignment_rows), Vec::new())))
}

/// Build a lightweight directory [`User`] (names + role only, no PII/assignments
/// /audit) from a `(id, first_name, last_name, role)` row.
fn directory_user((id, first_name, last_name, role): (String, String, String, String)) -> User {
    User {
        id,
        first_name,
        last_name,
        email: String::new(),
        phone: String::new(),
        home_address: String::new(),
        password: String::new(),
        role: AccountRole::from_slug(&role).unwrap_or(AccountRole::Client),
        assigned_cases: Vec::new(),
        audit_log: Vec::new(),
    }
}

/// Name-resolution directory entries for the given user ids: id, names, and role
/// only. Deliberately omits contact details, assignments, and audit history (so
/// no PII leaves the database), and skips the per-user hydration queries that
/// [`page`] runs. Used to build the non-admin bootstrap directory efficiently.
pub async fn directory(ids: &[String]) -> Result<Vec<User>, sqlx::Error> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT id, first_name, last_name, role FROM users WHERE id = ANY($1) ORDER BY id",
    )
    .bind(ids)
    .fetch_all(pool())
    .await?;
    Ok(rows.into_iter().map(directory_user).collect())
}

/// Lightweight directory of *every* user (id, names, and role only — same shape
/// as [`directory`]). One flat query with no per-user hydration, so it stays
/// cheap even with thousands of rows. This is what the admin bootstrap needs for
/// name resolution and owner pickers; the admin management screen loads full
/// user detail a page at a time via [`page`].
pub async fn directory_all() -> Result<Vec<User>, sqlx::Error> {
    let rows: Vec<(String, String, String, String)> =
        sqlx::query_as("SELECT id, first_name, last_name, role FROM users ORDER BY id")
            .fetch_all(pool())
            .await?;
    Ok(rows.into_iter().map(directory_user).collect())
}

/// Distinct ids of users assigned (via `case_assignments`) to any of the given
/// cases. Used to build the non-admin bootstrap directory without loading every
/// user in the system.
pub async fn ids_assigned_to_cases(case_ids: &[String]) -> Result<Vec<String>, sqlx::Error> {
    if case_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT user_id FROM case_assignments WHERE case_id = ANY($1)",
    )
    .bind(case_ids)
    .fetch_all(pool())
    .await?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// Whether a user with this email (case-insensitive) already exists.
pub async fn email_exists(email: &str) -> Result<bool, sqlx::Error> {
    let exists: Option<i32> =
        sqlx::query_scalar("SELECT 1 FROM users WHERE lower(email) = lower($1) LIMIT 1")
            .bind(email)
            .fetch_optional(pool())
            .await?;
    Ok(exists.is_some())
}

/// Resolve a login attempt: fetch the auth-ready [`User`] (id, role, name,
/// capabilities — no audit log) *and* the stored password hash for the account
/// with this email (case-insensitive). The user row and the assignments load
/// concurrently for a single effective round-trip. Returns `None` when no such
/// account exists.
pub async fn authenticate(email: &str) -> Result<Option<(User, String)>, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct AuthRow {
        id: String,
        first_name: String,
        last_name: String,
        email: String,
        phone: String,
        home_address: String,
        role: String,
        password_hash: String,
    }

    // The account row (with the password hash), or none if the email is unknown.
    let user_fut = sqlx::query_as::<_, AuthRow>(
        "SELECT id, first_name, last_name, email, phone, home_address, role, password_hash
         FROM users WHERE lower(email) = lower($1)",
    )
    .bind(email)
    .fetch_optional(pool());

    // That account's capability assignments, scoped through the same email predicate.
    let assignments_fut = sqlx::query_as::<_, (String, String)>(
        "SELECT case_id, capability FROM case_assignments
         WHERE user_id = (SELECT id FROM users WHERE lower(email) = lower($1))
         ORDER BY case_id",
    )
    .bind(email)
    .fetch_all(pool());

    let (row, assignment_rows) = tokio::try_join!(user_fut, assignments_fut)?;
    let Some(row) = row else {
        return Ok(None);
    };

    let user = UserRow {
        id: row.id,
        first_name: row.first_name,
        last_name: row.last_name,
        email: row.email,
        phone: row.phone,
        home_address: row.home_address,
        role: row.role,
    }
    .into_user(group_assignments(assignment_rows), Vec::new());
    Ok(Some((user, row.password_hash)))
}

/// Insert a user with a pre-computed password hash. Used by registration and by
/// the initial seed. The caller supplies the id (or use [`ids::next`]).
#[allow(clippy::too_many_arguments)]
pub async fn insert(
    id: &str,
    first_name: &str,
    last_name: &str,
    email: &str,
    phone: &str,
    home_address: &str,
    password_hash: &str,
    role: AccountRole,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO users (id, first_name, last_name, email, phone, home_address, password_hash, role)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(id)
    .bind(first_name)
    .bind(last_name)
    .bind(email)
    .bind(phone)
    .bind(home_address)
    .bind(password_hash)
    .bind(role.slug())
    .execute(pool())
    .await?;
    Ok(())
}

/// Allocate a fresh user id (`u-<n>`).
pub async fn next_id() -> Result<String, sqlx::Error> {
    ids::next(pool(), "u").await
}

/// Change a user's global role, recording an audit entry when it changes.
pub async fn set_role(user_id: &str, role: AccountRole, actor: &str) -> Result<(), sqlx::Error> {
    let current: Option<String> = sqlx::query_scalar("SELECT role FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool())
        .await?;
    let Some(current) = current else {
        return Ok(());
    };
    if current == role.slug() {
        return Ok(());
    }
    sqlx::query("UPDATE users SET role = $1 WHERE id = $2")
        .bind(role.slug())
        .bind(user_id)
        .execute(pool())
        .await?;
    audit::record(
        pool(),
        audit::Entity::User,
        user_id,
        actor,
        "role",
        &current,
        role.slug(),
    )
    .await
}

/// Replace a user's capability set on a case (adds the assignment if missing).
pub async fn assign_capabilities(
    user_id: &str,
    case_id: &str,
    capabilities: &[CaseCapability],
    actor: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = pool().begin().await?;
    sqlx::query("DELETE FROM case_assignments WHERE user_id = $1 AND case_id = $2")
        .bind(user_id)
        .bind(case_id)
        .execute(&mut *tx)
        .await?;
    for cap in capabilities {
        sqlx::query(
            "INSERT INTO case_assignments (user_id, case_id, capability) VALUES ($1, $2, $3)
             ON CONFLICT DO NOTHING",
        )
        .bind(user_id)
        .bind(case_id)
        .bind(cap.slug())
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    let summary = capabilities
        .iter()
        .map(|c| c.slug())
        .collect::<Vec<_>>()
        .join(", ");
    audit::record(
        pool(),
        audit::Entity::User,
        user_id,
        actor,
        &format!("case:{case_id}"),
        "",
        &summary,
    )
    .await
}

/// Toggle a single capability for a user on a case.
pub async fn toggle_capability(
    user_id: &str,
    case_id: &str,
    cap: CaseCapability,
    enabled: bool,
    actor: &str,
) -> Result<(), sqlx::Error> {
    if enabled {
        sqlx::query(
            "INSERT INTO case_assignments (user_id, case_id, capability) VALUES ($1, $2, $3)
             ON CONFLICT DO NOTHING",
        )
        .bind(user_id)
        .bind(case_id)
        .bind(cap.slug())
        .execute(pool())
        .await?;
    } else {
        sqlx::query(
            "DELETE FROM case_assignments WHERE user_id = $1 AND case_id = $2 AND capability = $3",
        )
        .bind(user_id)
        .bind(case_id)
        .bind(cap.slug())
        .execute(pool())
        .await?;
    }
    let (old, new) = if enabled { ("", cap.slug()) } else { (cap.slug(), "") };
    audit::record(
        pool(),
        audit::Entity::User,
        user_id,
        actor,
        &format!("case:{case_id}:{}", cap.slug()),
        old,
        new,
    )
    .await
}

/// Remove a user's assignment to a case entirely.
pub async fn unassign(user_id: &str, case_id: &str, actor: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM case_assignments WHERE user_id = $1 AND case_id = $2")
        .bind(user_id)
        .bind(case_id)
        .execute(pool())
        .await?;
    audit::record(
        pool(),
        audit::Entity::User,
        user_id,
        actor,
        &format!("case:{case_id}"),
        "assigned",
        "removed",
    )
    .await
}

/// Count of users (used to decide whether to seed).
pub async fn count() -> Result<i64, sqlx::Error> {
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(pool())
        .await?;
    Ok(n)
}
