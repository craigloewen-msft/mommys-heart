//! Self-service password-reset tokens (SSR only).
//!
//! A reset token is delivered as a link (`APP_URL/reset-password?token=…`). The
//! raw token travels in the URL; only its SHA-256 hash is stored. Tokens are
//! single-use (`used` flips true on consumption) and short-lived, so a link
//! cannot be replayed even before it expires.

use crate::server::db::pool;

/// How long a reset link stays valid.
const RESET_TTL_MINUTES: i64 = 60;

/// Hash an opaque token for storage/lookup (SHA-256, hex).
fn hash(raw: &str) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(raw.as_bytes()))
}

/// Create a reset token for `user_id`, valid for one hour. Returns `Ok(())`.
pub async fn create(user_id: &str, token: &str) -> Result<(), sqlx::Error> {
    let expires_at = chrono::Utc::now() + chrono::Duration::minutes(RESET_TTL_MINUTES);
    sqlx::query(
        "INSERT INTO password_reset_tokens (token_hash, user_id, expires_at)
         VALUES ($1, $2, $3)",
    )
    .bind(hash(token))
    .bind(user_id)
    .bind(expires_at)
    .execute(pool())
    .await?;
    Ok(())
}

/// Atomically consume a reset token: if it is live and unused, mark it used and
/// return its `user_id`. Returns `None` when the token is unknown, expired, or
/// already used. The `UPDATE … RETURNING` guarantees a token can be redeemed at
/// most once even under concurrent requests.
pub async fn consume(token: &str) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar(
        "UPDATE password_reset_tokens
         SET used = true
         WHERE token_hash = $1 AND used = false AND expires_at > now()
         RETURNING user_id",
    )
    .bind(hash(token))
    .fetch_optional(pool())
    .await
}
