//! Persistence for the client record built on top of a user (SSR only).
//!
//! The counterpart to [`crate::server::db::volunteers`]: `users` holds identity
//! and role, and this table holds what is true *because* someone is a client.
//! It is deliberately thin for now — it exists so client-only data has a home and
//! the two account kinds are structured the same way.

use crate::server::db::pool;

/// Record that a user is a client as part of a larger transaction, so the client
/// record and the account it extends commit together or not at all.
pub async fn insert_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO clients (user_id) VALUES ($1)
         ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(user_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Record that a user is a client. Idempotent, so callers need not check first.
pub async fn ensure(user_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO clients (user_id) VALUES ($1)
         ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(user_id)
    .execute(pool())
    .await?;
    Ok(())
}

/// Whether a user has a client record.
pub async fn exists(user_id: &str) -> Result<bool, sqlx::Error> {
    let found: Option<i32> = sqlx::query_scalar("SELECT 1 FROM clients WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(pool())
        .await?;
    Ok(found.is_some())
}
