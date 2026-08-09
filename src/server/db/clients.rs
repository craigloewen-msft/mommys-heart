//! Persistence for the client record built on top of a user (SSR only).
//!
//! The counterpart to [`crate::server::db::volunteers`]: `users` holds identity
//! and role, this holds what is true because someone is a client. Deliberately
//! thin for now. Rows are written by registration and
//! [`crate::server::db::users::set_role_in`].

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
