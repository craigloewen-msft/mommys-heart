//! Trusted devices for "remember this device" (SSR only).
//!
//! When a user opts to trust a browser during MFA, an opaque token is set as a
//! long-lived cookie and its SHA-256 hash is stored here against their user id.
//! A subsequent login that presents a still-live token belonging to the *same*
//! authenticating user skips the OTP step. Tokens are single-user-scoped so a
//! leaked cookie cannot unlock a different account.

use crate::server::db::pool;

/// How long a trusted-device grant lasts.
pub const TRUST_TTL_DAYS: i64 = 30;

/// Hash an opaque token for storage/lookup (SHA-256, hex).
fn hash(raw: &str) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(raw.as_bytes()))
}

/// Record `token` as a trusted device for `user_id`, valid for
/// [`TRUST_TTL_DAYS`]. Returns `Ok(())` on success.
pub async fn create(user_id: &str, token: &str) -> Result<(), sqlx::Error> {
    let expires_at = chrono::Utc::now() + chrono::Duration::days(TRUST_TTL_DAYS);
    sqlx::query(
        "INSERT INTO trusted_devices (token_hash, user_id, expires_at) VALUES ($1, $2, $3)
         ON CONFLICT (token_hash) DO UPDATE SET
             user_id = EXCLUDED.user_id,
             created_at = now(),
             expires_at = EXCLUDED.expires_at",
    )
    .bind(hash(token))
    .bind(user_id)
    .bind(expires_at)
    .execute(pool())
    .await?;
    Ok(())
}

/// Whether `token` is a live trusted device *for this specific user*. A missing,
/// expired, or other-user token returns `false`.
pub async fn is_trusted_for_user(user_id: &str, token: &str) -> Result<bool, sqlx::Error> {
    let exists: Option<i32> = sqlx::query_scalar(
        "SELECT 1 FROM trusted_devices
         WHERE token_hash = $1 AND user_id = $2 AND expires_at > now()
         LIMIT 1",
    )
    .bind(hash(token))
    .bind(user_id)
    .fetch_optional(pool())
    .await?;
    Ok(exists.is_some())
}

/// Forget a trusted device (best-effort cleanup).
pub async fn delete(token: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM trusted_devices WHERE token_hash = $1")
        .bind(hash(token))
        .execute(pool())
        .await?;
    Ok(())
}

/// Revoke every remembered browser inside a credential-change transaction.
pub async fn delete_all_for_user_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM trusted_devices WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
