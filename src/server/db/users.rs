//! Users, their per-case capability assignments, and admin mutations.

use crate::server::db::{audit, clients, ids, pool};
use crate::server_fns::capabilities::{CaseAssignment, CaseCapability};
use crate::server_fns::pagination::Page;
use crate::server_fns::profile::ProfileEdit;
use crate::server_fns::users::{AccountRole, AgreementStatus, User, VolunteerListItem};
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

/// Allocate a fresh user id.
///
/// Unlike most records, a user id is addressable in a URL (`/profile/<id>`), so
/// it is drawn from the CSPRNG rather than the shared sequence: a sequential
/// `u-5` would let anyone walk the whole user table and read off how many
/// accounts exist and the order they signed up in.
pub fn next_id() -> String {
    ids::opaque("u")
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

/// Overwrite a user's password hash. Used by the self-service password-reset
/// flow after a valid reset token is consumed.
pub async fn set_password_hash(user_id: &str, password_hash: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
        .bind(password_hash)
        .bind(user_id)
        .execute(pool())
        .await?;
    Ok(())
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
    let mut tx = pool().begin().await?;
    insert_in(
        &mut tx,
        id,
        first_name,
        last_name,
        email,
        phone,
        home_address,
        password_hash,
        role,
    )
    .await?;
    tx.commit().await
}

/// Insert a user as part of a larger transaction.
#[allow(clippy::too_many_arguments)]
pub async fn insert_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
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
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// A single user's summary (id + display name) by id, or `None` if no such user
/// exists. The name falls back to the id when unset.
pub async fn summary(
    id: &str,
) -> Result<Option<crate::server_fns::users::UserSummary>, sqlx::Error> {
    use crate::server_fns::users::UserSummary;

    let row: Option<(String, Option<String>, Option<String>, String)> = sqlx::query_as(
        "SELECT id, first_name, last_name, role
         FROM users
         WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool())
    .await?;

    let Some((id, first_name, last_name, role)) = row else {
        return Ok(None);
    };
    let role = AccountRole::from_slug(&role).ok_or_else(|| {
        sqlx::Error::Decode(format!("invalid account role slug: {role:?}").into())
    })?;

    Ok(Some(UserSummary {
        first_name: first_name.unwrap_or_else(|| id.clone()),
        last_name: last_name.unwrap_or_else(|| id.clone()),
        id,
        role,
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

    let rows: Vec<(String, Option<String>, Option<String>, String)> = sqlx::query_as(
        "SELECT id, first_name, last_name, role
         FROM users
         WHERE $1::text IS NULL
            OR id ILIKE $1
            OR first_name ILIKE $1
            OR last_name ILIKE $1
            OR (first_name || ' ' || last_name) ILIKE $1
            OR email ILIKE $1
         ORDER BY first_name NULLS LAST, id
         LIMIT $2",
    )
    .bind(&pattern)
    .bind(limit)
    .fetch_all(pool())
    .await?;

    rows.into_iter()
        .map(|(id, first_name, last_name, role)| {
            let role = AccountRole::from_slug(&role).ok_or_else(|| {
                sqlx::Error::Decode(format!("invalid account role slug: {role:?}").into())
            })?;

            Ok(UserSummary {
                first_name: first_name.unwrap_or_else(|| id.clone()),
                last_name: last_name.unwrap_or_else(|| id.clone()),
                id,
                role,
            })
        })
        .collect()
}

/// Whether a user currently has any capability assignment on a case. Used to
/// distinguish a brand-new case assignment (worth an email) from an edit to an
/// existing one.
pub async fn is_assigned(user_id: &str, case_id: &str) -> Result<bool, sqlx::Error> {
    let exists: Option<i32> = sqlx::query_scalar(
        "SELECT 1 FROM case_assignments WHERE user_id = $1 AND case_id = $2 LIMIT 1",
    )
    .bind(user_id)
    .bind(case_id)
    .fetch_optional(pool())
    .await?;
    Ok(exists.is_some())
}

async fn lock_capability_target(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1 FROM users WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .fetch_optional(&mut **transaction)
        .await?;
    Ok(())
}

/// Change a user's global role, keeping the subtype tables in step.
///
/// **This is the only code that may change `users.role`.** A role is the
/// discriminator for the `volunteers` and `clients` subtype records, so changing
/// it and reconciling them has to be one atomic step — otherwise a user ends up
/// with a role whose subtype record disagrees, which is unrepresentable in the
/// model but was reachable before this existed. Call it from inside whatever
/// transaction the caller already has so the role change, the subtype rows, and
/// the audit entry all commit together.
///
/// Maintains:
/// - `role = 'volunteer'` ⟺ a `volunteers` row with `status = 'approved'`
/// - `role = 'client'` ⟹ a `clients` row
///
/// Losing the volunteer role *revokes* rather than deletes: the agreement they
/// accepted is a historical fact worth keeping, and a revoked user can accept it
/// again to re-apply. Gaining it by any route other than the application flow
/// (an admin setting the role directly, or approving a role request) records an
/// approved row with an empty `agreement_version` — they hold the role but never
/// signed anything, which is exactly what the admin list shows as "Outstanding".
///
/// No-ops when the role is unchanged, so callers need not check first.
pub async fn apply_role_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: &str,
    role: AccountRole,
    actor: &str,
) -> Result<(), sqlx::Error> {
    let current: Option<String> =
        sqlx::query_scalar("SELECT role FROM users WHERE id = $1 FOR UPDATE")
            .bind(user_id)
            .fetch_optional(&mut **tx)
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
        .execute(&mut **tx)
        .await?;

    // Reconcile the volunteer record with the new role.
    let was_volunteer = AccountRole::from_slug(&current) == Some(AccountRole::Volunteer);
    match role {
        AccountRole::Volunteer => {
            sqlx::query(
                "INSERT INTO volunteers (user_id, status, agreement_version, decided_by_name)
                 VALUES ($1, 'approved', '', $2)
                 ON CONFLICT (user_id) DO UPDATE SET
                     status = 'approved',
                     decided_by_name = EXCLUDED.decided_by_name,
                     decided_at = now()",
            )
            .bind(user_id)
            .bind(actor)
            .execute(&mut **tx)
            .await?;
        }
        _ if was_volunteer => {
            sqlx::query(
                "UPDATE volunteers SET
                     status = 'revoked',
                     decided_by_name = $2,
                     decided_at = now()
                 WHERE user_id = $1",
            )
            .bind(user_id)
            .bind(actor)
            .execute(&mut **tx)
            .await?;
        }
        _ => {}
    }

    if role == AccountRole::Client {
        clients::insert_in(tx, user_id).await?;
    }

    audit::record_in_transaction(
        tx,
        audit::Entity::User,
        user_id,
        actor,
        "role",
        &current,
        role.slug(),
    )
    .await
}

/// Change a user's global role in its own transaction. Thin wrapper over
/// [`apply_role_in`] for callers that are not already inside one.
pub async fn set_role(user_id: &str, role: AccountRole, actor: &str) -> Result<(), sqlx::Error> {
    let mut tx = pool().begin().await?;
    apply_role_in(&mut tx, user_id, role, actor).await?;
    tx.commit().await
}

/// Remove a user's assignment to a case entirely.
pub async fn unassign(user_id: &str, case_id: &str, actor: &str) -> Result<(), sqlx::Error> {
    let mut transaction = pool().begin().await?;
    lock_capability_target(&mut transaction, user_id).await?;
    sqlx::query("DELETE FROM case_assignments WHERE user_id = $1 AND case_id = $2")
        .bind(user_id)
        .bind(case_id)
        .execute(&mut *transaction)
        .await?;
    audit::record_in_transaction(
        &mut transaction,
        audit::Entity::User,
        user_id,
        actor,
        &format!("case:{case_id}"),
        "assigned",
        "removed",
    )
    .await?;
    transaction.commit().await
}

/// Toggle a single capability for a user on a case.
pub async fn toggle_capability(
    user_id: &str,
    case_id: &str,
    cap: CaseCapability,
    enabled: bool,
    actor: &str,
) -> Result<(), sqlx::Error> {
    let mut transaction = pool().begin().await?;
    lock_capability_target(&mut transaction, user_id).await?;
    if enabled {
        sqlx::query(
            "INSERT INTO case_assignments (user_id, case_id, capability) VALUES ($1, $2, $3)
             ON CONFLICT DO NOTHING",
        )
        .bind(user_id)
        .bind(case_id)
        .bind(cap.slug())
        .execute(&mut *transaction)
        .await?;
    } else {
        sqlx::query(
            "DELETE FROM case_assignments WHERE user_id = $1 AND case_id = $2 AND capability = $3",
        )
        .bind(user_id)
        .bind(case_id)
        .bind(cap.slug())
        .execute(&mut *transaction)
        .await?;
    }
    let (old, new) = if enabled {
        ("", cap.slug())
    } else {
        (cap.slug(), "")
    };
    audit::record_in_transaction(
        &mut transaction,
        audit::Entity::User,
        user_id,
        actor,
        &format!("case:{case_id}:{}", cap.slug()),
        old,
        new,
    )
    .await?;
    transaction.commit().await
}

/// Replace a user's capability set on a case (adds the assignment if missing).
pub async fn assign_capabilities(
    user_id: &str,
    case_id: &str,
    capabilities: &[CaseCapability],
    actor: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = pool().begin().await?;
    assign_capabilities_in(&mut tx, user_id, case_id, capabilities, actor).await?;
    tx.commit().await
}

/// Replace a user's case capabilities inside an existing transaction.
pub async fn assign_capabilities_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: &str,
    case_id: &str,
    capabilities: &[CaseCapability],
    actor: &str,
) -> Result<(), sqlx::Error> {
    lock_capability_target(tx, user_id).await?;
    sqlx::query("DELETE FROM case_assignments WHERE user_id = $1 AND case_id = $2")
        .bind(user_id)
        .bind(case_id)
        .execute(&mut **tx)
        .await?;
    for cap in capabilities {
        sqlx::query(
            "INSERT INTO case_assignments (user_id, case_id, capability) VALUES ($1, $2, $3)
             ON CONFLICT DO NOTHING",
        )
        .bind(user_id)
        .bind(case_id)
        .bind(cap.slug())
        .execute(&mut **tx)
        .await?;
    }

    let summary = capabilities
        .iter()
        .map(|c| c.slug())
        .collect::<Vec<_>>()
        .join(", ");
    audit::record_in_transaction(
        tx,
        audit::Entity::User,
        user_id,
        actor,
        &format!("case:{case_id}"),
        "",
        &summary,
    )
    .await?;
    Ok(())
}

/// Whether `viewer_id` and `other_id` work at least one case together — the
/// basis for profile visibility. A user counts as being on a case when they
/// hold the `view_case` capability on it or own it. Stops at the first match
/// rather than listing the cases, since only the yes/no answer is needed.
pub async fn shares_case(viewer_id: &str, other_id: &str) -> Result<bool, sqlx::Error> {
    // The capability slug is an internal constant (never user input), so
    // interpolating it into the SQL is safe.
    let on_case = format!(
        "(c.owner_id = $ID OR EXISTS (SELECT 1 FROM case_assignments a \
          WHERE a.case_id = c.id AND a.user_id = $ID AND a.capability = '{}'))",
        CaseCapability::ViewCase.slug()
    );
    let sql = format!(
        "SELECT 1 FROM cases c WHERE {} AND {} LIMIT 1",
        on_case.replace("$ID", "$1"),
        on_case.replace("$ID", "$2"),
    );

    let found: Option<i32> = sqlx::query_scalar(&sql)
        .bind(viewer_id)
        .bind(other_id)
        .fetch_optional(pool())
        .await?;
    Ok(found.is_some())
}

/// Overwrite a user's own profile details, recording one audit entry per field
/// that actually changed. No-ops for a user id that does not exist.
pub async fn update_profile(
    user_id: &str,
    edit: &ProfileEdit,
    actor: &str,
) -> Result<(), sqlx::Error> {
    let current: Option<(String, String, String, String)> = sqlx::query_as(
        "SELECT first_name, last_name, phone, home_address FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool())
    .await?;
    let Some((first_name, last_name, phone, home_address)) = current else {
        return Ok(());
    };

    sqlx::query(
        "UPDATE users
         SET first_name = $1, last_name = $2, phone = $3, home_address = $4
         WHERE id = $5",
    )
    .bind(&edit.first_name)
    .bind(&edit.last_name)
    .bind(&edit.phone)
    .bind(&edit.home_address)
    .bind(user_id)
    .execute(pool())
    .await?;

    let changes = [
        ("first_name", first_name, &edit.first_name),
        ("last_name", last_name, &edit.last_name),
        ("phone", phone, &edit.phone),
        ("home_address", home_address, &edit.home_address),
    ];
    for (field, old, new) in changes {
        if &old != new {
            audit::record(
                pool(),
                audit::Entity::User,
                user_id,
                actor,
                field,
                &old,
                new,
            )
            .await?;
        }
    }
    Ok(())
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
         FROM users {FILTER} ORDER BY last_name, first_name, id LIMIT $2 OFFSET $3"
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

/// One page of volunteer accounts for the admin "Volunteers" tab: the
/// `volunteer` role only, with the same optional search as [`page`]. Case
/// assignments are not fetched — the list does not show them.
///
/// Left-joins the `volunteers` record so the list can report whether each person
/// has actually accepted the volunteer agreement. A volunteer with no record, or
/// one backfilled by migration (empty version), predates the agreement.
pub async fn volunteers_page(
    offset: i64,
    limit: i64,
    search: &str,
) -> Result<Page<VolunteerListItem>, sqlx::Error> {
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

    // A NULL pattern (no search term) matches every volunteer.
    const FILTER: &str = "WHERE u.role = 'volunteer'
           AND ($1::text IS NULL
                OR u.first_name ILIKE $1
                OR u.last_name ILIKE $1
                OR (u.first_name || ' ' || u.last_name) ILIKE $1
                OR u.email ILIKE $1)";

    let count_sql = format!("SELECT count(*) FROM users u {FILTER}");
    let page_sql = format!(
        "SELECT u.id, u.first_name, u.last_name, u.email,
                COALESCE(v.agreement_version, '') AS agreement_version
         FROM users u
         LEFT JOIN volunteers v ON v.user_id = u.id
         {FILTER} ORDER BY u.last_name, u.first_name, u.id LIMIT $2 OFFSET $3"
    );

    let total_fut = sqlx::query_scalar::<_, i64>(&count_sql)
        .bind(&pattern)
        .fetch_one(pool());
    let rows_fut = sqlx::query_as::<_, (String, String, String, String, String)>(&page_sql)
        .bind(&pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool());
    let (total, rows) = tokio::try_join!(total_fut, rows_fut)?;

    let items = rows
        .into_iter()
        .map(
            |(id, first_name, last_name, email, agreement_version)| VolunteerListItem {
                id,
                first_name,
                last_name,
                email,
                agreement: if agreement_version.is_empty() {
                    AgreementStatus::Outstanding
                } else {
                    AgreementStatus::Completed
                },
            },
        )
        .collect();
    Ok(Page { items, total })
}
