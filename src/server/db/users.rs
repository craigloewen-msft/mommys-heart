//! Users, their per-case capability assignments, and admin mutations.

use crate::server::db::{audit, ids, pool};
use crate::server_fns::pagination::Page;
use crate::server_fns::capabilities::{CaseAssignment, CaseCapability};
use crate::server_fns::users::{AccountRole, User};
use std::collections::BTreeMap;

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
    fn into_user(self, assigned_cases: Vec<CaseAssignment>) -> User {
        User {
            id: self.id,
            first_name: self.first_name,
            last_name: self.last_name,
            email: self.email,
            phone: self.phone,
            home_address: self.home_address,
            role: AccountRole::from_slug(&self.role).unwrap_or(AccountRole::Client),
            assigned_cases,
        }
    }
}

/// Count of users (used to decide whether to seed).
pub async fn count() -> Result<i64, sqlx::Error> {
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(pool())
        .await?;
    Ok(n)
}

/// Allocate a fresh user id (`u-<n>`).
pub async fn next_id() -> Result<String, sqlx::Error> {
    ids::next(pool(), "u").await
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

/// Insert a user with a pre-computed password hash. Used by registration and by
/// the initial seed. The caller supplies the id (or use [`next_id`]).
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

/// A single user's summary (id + display name) by id, or `None` if no such user
/// exists. The name falls back to the id when unset.
pub async fn summary(
    id: &str,
) -> Result<Option<crate::server_fns::users::UserSummary>, sqlx::Error> {
    use crate::server_fns::users::UserSummary;

    let row: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT id,
                NULLIF(TRIM(COALESCE(first_name,'') || ' ' || COALESCE(last_name,'')), '') AS name
         FROM users
         WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool())
    .await?;

    Ok(row.map(|(id, name)| UserSummary {
        name: name.unwrap_or_else(|| id.clone()),
        id,
    }))
}

/// Case-insensitive search over users, returning up to `limit` id + display name
/// summaries. Empty search returns the first `limit` users. Metacharacters in
/// the search term are escaped so user input is matched literally.
pub async fn search_user_summaries(
    query: &str,
    limit: i64,
) -> Result<Vec<crate::server_fns::users::UserSummary>, sqlx::Error> {
    use crate::server_fns::users::UserSummary;

    let limit = limit.clamp(1, 50);

    let term = query.trim();
    let pattern = if term.is_empty() {
        None
    } else {
        Some(format!(
            "%{}%",
            term.replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        ))
    };

    let rows: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT id,
                NULLIF(TRIM(COALESCE(first_name,'') || ' ' || COALESCE(last_name,'')), '') AS name
         FROM users
         WHERE $1::text IS NULL
            OR id ILIKE $1
            OR first_name ILIKE $1
            OR last_name ILIKE $1
            OR (first_name || ' ' || last_name) ILIKE $1
            OR email ILIKE $1
         ORDER BY name NULLS LAST, id
         LIMIT $2",
    )
    .bind(&pattern)
    .bind(limit)
    .fetch_all(pool())
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, name)| UserSummary {
            name: name.unwrap_or_else(|| id.clone()),
            id,
        })
        .collect())
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
    let (old, new) = if enabled {
        ("", cap.slug())
    } else {
        (cap.slug(), "")
    };
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

/// A single user by id, with their per-case capability assignments.
pub async fn get(id: &str) -> Result<Option<User>, sqlx::Error> {
    let row = sqlx::query_as::<_, UserRow>(
        "SELECT id, first_name, last_name, email, phone, home_address, role FROM users WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool())
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };

    let assignment_rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT case_id, capability FROM case_assignments WHERE user_id = $1 ORDER BY case_id",
    )
    .bind(id)
    .fetch_all(pool())
    .await?;

    let mut grouped: BTreeMap<String, Vec<CaseCapability>> = BTreeMap::new();
    for (case_id, cap) in assignment_rows {
        if let Some(c) = CaseCapability::from_slug(&cap) {
            grouped.entry(case_id).or_default().push(c);
        }
    }
    let assignments = grouped
        .into_iter()
        .map(|(case_id, capabilities)| CaseAssignment {
            case_id,
            capabilities,
        })
        .collect();

    Ok(Some(row.into_user(assignments)))
}

/// Resolve a signed-in user (with capabilities) from a session token hash.
/// Scoped to the *one* session matching the token — the two queries (the
/// session's user, and that user's assignments) are independent, so they run
/// concurrently for a single effective round-trip without a fan-out JOIN.
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
    let Some(row) = row else {
        return Ok(None);
    };

    let mut grouped: BTreeMap<String, Vec<CaseCapability>> = BTreeMap::new();
    for (case_id, cap) in assignment_rows {
        if let Some(c) = CaseCapability::from_slug(&cap) {
            grouped.entry(case_id).or_default().push(c);
        }
    }
    let assignments = grouped
        .into_iter()
        .map(|(case_id, capabilities)| CaseAssignment {
            case_id,
            capabilities,
        })
        .collect();

    Ok(Some(row.into_user(assignments)))
}

/// Resolve a login attempt: fetch the auth-ready [`User`] (id, role, name,
/// capabilities) *and* the stored password hash for the account with this email
/// (case-insensitive). The user row and the assignments load concurrently for a
/// single effective round-trip. Returns `None` when no such account exists.
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

    let mut grouped: BTreeMap<String, Vec<CaseCapability>> = BTreeMap::new();
    for (case_id, cap) in assignment_rows {
        if let Some(c) = CaseCapability::from_slug(&cap) {
            grouped.entry(case_id).or_default().push(c);
        }
    }
    let assignments = grouped
        .into_iter()
        .map(|(case_id, capabilities)| CaseAssignment {
            case_id,
            capabilities,
        })
        .collect();

    let password_hash = row.password_hash.clone();
    let user = UserRow {
        id: row.id,
        first_name: row.first_name,
        last_name: row.last_name,
        email: row.email,
        phone: row.phone,
        home_address: row.home_address,
        role: row.role,
    }
    .into_user(assignments);
    Ok(Some((user, password_hash)))
}

/// One page of users (ordered by id), each with their assignments, plus the
/// total number of users matching the search. `search`, when non-blank, matches
/// a user's id, first/last name, full name ("first last"), or email
/// case-insensitively. `limit` is clamped so a caller can never request an
/// unbounded scan.
pub async fn page(offset: i64, limit: i64, search: &str) -> Result<Page<User>, sqlx::Error> {
    let limit = limit.clamp(1, 100);
    let offset = offset.max(0);

    let term = search.trim();
    let pattern = if term.is_empty() {
        None
    } else {
        Some(format!(
            "%{}%",
            term.replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
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
    let (total, rows) = tokio::try_join!(total_fut, rows_fut)?;

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let assignment_rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT case_id, capability FROM case_assignments WHERE user_id = $1 ORDER BY case_id",
        )
        .bind(&row.id)
        .fetch_all(pool())
        .await?;

        let mut grouped: BTreeMap<String, Vec<CaseCapability>> = BTreeMap::new();
        for (case_id, cap) in assignment_rows {
            if let Some(c) = CaseCapability::from_slug(&cap) {
                grouped.entry(case_id).or_default().push(c);
            }
        }
        let assignments = grouped
            .into_iter()
            .map(|(case_id, capabilities)| CaseAssignment {
                case_id,
                capabilities,
            })
            .collect();

        items.push(row.into_user(assignments));
    }
    Ok(Page { items, total })
}
