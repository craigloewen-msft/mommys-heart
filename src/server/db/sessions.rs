//! Opaque session tokens. The raw token is returned to the client (as a cookie);
//! only its SHA-256 hash is stored, so a database leak cannot be replayed.

use crate::server::db::pool;
use rand::RngCore;
use sha2::{Digest, Sha256};

/// How long a session stays valid.
const SESSION_TTL_DAYS: i64 = 30;

/// Hash a raw token for storage/lookup.
pub(crate) fn hash_token(raw: &str) -> String {
    let digest = Sha256::digest(raw.as_bytes());
    hex::encode(digest)
}

/// Mint a new session for `user_id`, returning the raw token to hand to the client.
pub async fn create(user_id: &str) -> Result<String, sqlx::Error> {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let raw = hex::encode(bytes);
    let token_hash = hash_token(&raw);
    let expires_at = chrono::Utc::now() + chrono::Duration::days(SESSION_TTL_DAYS);

    sqlx::query("INSERT INTO sessions (token_hash, user_id, expires_at) VALUES ($1, $2, $3)")
        .bind(&token_hash)
        .bind(user_id)
        .bind(expires_at)
        .execute(pool())
        .await?;
    Ok(raw)
}

/// Invalidate a session (logout).
pub async fn delete(raw: &str) -> Result<(), sqlx::Error> {
    let token_hash = hash_token(raw);
    sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
        .bind(&token_hash)
        .execute(pool())
        .await?;
    Ok(())
}
