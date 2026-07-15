//! Email-OTP multi-factor challenges (SSR only).
//!
//! A challenge is created right after a correct password but before a session
//! exists. The raw challenge token is returned to the client (in the short-lived
//! `mfa` cookie); only its SHA-256 hash is stored, alongside the hash of the
//! 6-digit code emailed to the user. Verification is single-use and
//! attempt-limited, mirroring the opaque-token approach in [`super::sessions`].

use crate::server::db::pool;

/// How long a pending challenge stays valid.
const CHALLENGE_TTL_MINUTES: i64 = 10;

/// Maximum number of wrong-code attempts before a challenge is burned.
pub const MAX_ATTEMPTS: i32 = 5;

/// Hash an opaque token or code for storage/lookup (SHA-256, hex).
fn hash(raw: &str) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(raw.as_bytes()))
}

/// Persist a new challenge for `user_id`. `challenge_token` is the opaque token
/// that will live in the client's `mfa` cookie; `code` is the plaintext 6-digit
/// code emailed to the user. Any prior pending challenge for the same token is
/// replaced. Returns `Ok(())` on success.
pub async fn create(user_id: &str, challenge_token: &str, code: &str) -> Result<(), sqlx::Error> {
    let expires_at = chrono::Utc::now() + chrono::Duration::minutes(CHALLENGE_TTL_MINUTES);
    sqlx::query(
        "INSERT INTO mfa_challenges (challenge_hash, user_id, code_hash, expires_at)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (challenge_hash) DO UPDATE SET
             user_id = EXCLUDED.user_id,
             code_hash = EXCLUDED.code_hash,
             attempts = 0,
             created_at = now(),
             expires_at = EXCLUDED.expires_at",
    )
    .bind(hash(challenge_token))
    .bind(user_id)
    .bind(hash(code))
    .bind(expires_at)
    .execute(pool())
    .await?;
    Ok(())
}

/// The `user_id` of the live (unexpired) challenge for `challenge_token`, or
/// `None`. Used when resending a code to look up who the pending challenge is
/// for without consuming it.
pub async fn user_for(challenge_token: &str) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT user_id FROM mfa_challenges
         WHERE challenge_hash = $1 AND expires_at > now()",
    )
    .bind(hash(challenge_token))
    .fetch_optional(pool())
    .await
}

/// The outcome of verifying a submitted code against a pending challenge.
pub enum Verify {
    /// The code matched; the challenge was consumed. Carries the user id.
    Ok(String),
    /// The code did not match (and the challenge is still usable, unless the
    /// attempt cap was just reached — in which case it has been burned).
    WrongCode,
    /// No live challenge exists for this token (missing, expired, or already
    /// consumed / burned by too many attempts).
    Expired,
}

/// Verify `code` against the challenge identified by `challenge_token`. On a
/// correct code the challenge row is deleted and the user id returned. On a
/// wrong code the attempt counter is incremented and, once it reaches
/// [`MAX_ATTEMPTS`], the challenge is deleted so it can no longer be guessed.
pub async fn verify(challenge_token: &str, code: &str) -> Result<Verify, sqlx::Error> {
    let challenge_hash = hash(challenge_token);

    let row: Option<(String, String, i32)> = sqlx::query_as(
        "SELECT user_id, code_hash, attempts FROM mfa_challenges
         WHERE challenge_hash = $1 AND expires_at > now()",
    )
    .bind(&challenge_hash)
    .fetch_optional(pool())
    .await?;

    let Some((user_id, code_hash, attempts)) = row else {
        return Ok(Verify::Expired);
    };

    if code_hash == hash(code) {
        sqlx::query("DELETE FROM mfa_challenges WHERE challenge_hash = $1")
            .bind(&challenge_hash)
            .execute(pool())
            .await?;
        return Ok(Verify::Ok(user_id));
    }

    if attempts + 1 >= MAX_ATTEMPTS {
        sqlx::query("DELETE FROM mfa_challenges WHERE challenge_hash = $1")
            .bind(&challenge_hash)
            .execute(pool())
            .await?;
        return Ok(Verify::Expired);
    }

    sqlx::query("UPDATE mfa_challenges SET attempts = attempts + 1 WHERE challenge_hash = $1")
        .bind(&challenge_hash)
        .execute(pool())
        .await?;
    Ok(Verify::WrongCode)
}

/// Delete a challenge (best-effort cleanup, e.g. on logout/abandon).
pub async fn delete(challenge_token: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM mfa_challenges WHERE challenge_hash = $1")
        .bind(hash(challenge_token))
        .execute(pool())
        .await?;
    Ok(())
}
