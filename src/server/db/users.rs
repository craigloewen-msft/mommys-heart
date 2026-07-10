//! Users, their per-case capability assignments, and admin mutations.

use crate::server::db::{audit, ids, pool};
use crate::types::{AccountRole, CaseAssignment, CaseCapability, User};
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

async fn load_assignments(user_id: &str) -> Result<Vec<CaseAssignment>, sqlx::Error> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT case_id, capability FROM case_assignments WHERE user_id = $1 ORDER BY case_id",
    )
    .bind(user_id)
    .fetch_all(pool())
    .await?;

    let mut grouped: BTreeMap<String, Vec<CaseCapability>> = BTreeMap::new();
    for (case_id, cap) in rows {
        if let Some(c) = CaseCapability::from_slug(&cap) {
            grouped.entry(case_id).or_default().push(c);
        }
    }
    Ok(grouped
        .into_iter()
        .map(|(case_id, capabilities)| CaseAssignment {
            case_id,
            capabilities,
        })
        .collect())
}

async fn hydrate(row: UserRow) -> Result<User, sqlx::Error> {
    let assignments = load_assignments(&row.id).await?;
    let audit_log = audit::for_entity(pool(), audit::Entity::User, &row.id).await?;
    Ok(row.into_user(assignments, audit_log))
}

/// Every user, ordered by id, fully hydrated with assignments and audit log.
pub async fn list() -> Result<Vec<User>, sqlx::Error> {
    let rows = sqlx::query_as::<_, UserRow>(
        "SELECT id, first_name, last_name, email, phone, home_address, role FROM users ORDER BY id",
    )
    .fetch_all(pool())
    .await?;
    let mut users = Vec::with_capacity(rows.len());
    for row in rows {
        users.push(hydrate(row).await?);
    }
    Ok(users)
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

/// Name-resolution directory entries for the given user ids: id, names, and role
/// only. Deliberately omits contact details, assignments, and audit history (so
/// no PII leaves the database), and skips the per-user hydration queries that
/// [`list`] runs. Used to build the non-admin bootstrap directory efficiently.
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
    Ok(rows
        .into_iter()
        .map(|(id, first_name, last_name, role)| User {
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
        })
        .collect())
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

/// The (id, password_hash) for a login attempt, matched case-insensitively.
pub async fn credentials(email: &str) -> Result<Option<(String, String)>, sqlx::Error> {
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT id, password_hash FROM users WHERE lower(email) = lower($1)")
            .bind(email)
            .fetch_optional(pool())
            .await?;
    Ok(row)
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
