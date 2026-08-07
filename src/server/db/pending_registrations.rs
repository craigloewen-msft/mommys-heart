//! Email-verification challenges for self-service registration (SSR only).
//!
//! A challenge is created when the sign-up form is submitted but *before* any
//! `users` row exists: it holds the not-yet-created account's details plus the
//! hash of the emailed 6-digit code. The raw challenge token is returned to the
//! client (in the short-lived `register` cookie); only its SHA-256 hash is
//! stored, alongside the hash of the code. Verification is single-use and
//! attempt-limited, mirroring [`super::mfa`] — the difference is that a correct
//! code yields the pending account's details (so the caller can materialize the
//! real user) rather than an existing user id.

use crate::server::db::pool;

/// How long a pending registration stays valid.
const CHALLENGE_TTL_MINUTES: i64 = 10;

/// Maximum number of wrong-code attempts before the challenge is burned.
pub const MAX_ATTEMPTS: i32 = 5;

/// The not-yet-created account carried by a pending registration.
#[derive(Clone, Debug)]
pub struct PendingAccount {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub password_hash: String,
}

/// A staged case signup carried by the verification challenge.
#[derive(Clone, Debug)]
pub struct PendingCaseSignup {
    pub case_id: String,
    pub case_name: String,
    pub intake_json: String,
    /// The version of the Terms and Conditions the client accepted before
    /// filling in the case form. Recorded against the user and case once the
    /// email code verifies.
    pub terms_version: String,
}

/// Everything needed to materialize a verified registration.
#[derive(Clone, Debug)]
pub struct PendingRegistration {
    pub account: PendingAccount,
    pub case_signup: Option<PendingCaseSignup>,
}

#[derive(sqlx::FromRow)]
struct PendingRow {
    first_name: String,
    last_name: String,
    email: String,
    password_hash: String,
    code_hash: String,
    attempts: i32,
    is_live: bool,
    create_case: bool,
    case_id: String,
    case_name: String,
    intake_json: String,
    terms_version: String,
}

impl PendingAccount {
    /// The registrant's full display name (used as the email greeting).
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
            .trim()
            .to_string()
    }
}

/// Hash an opaque token or code for storage/lookup (SHA-256, hex).
fn hash(raw: &str) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(raw.as_bytes()))
}

/// Persist a new pending registration for `challenge_token`. `code` is the
/// plaintext 6-digit code emailed to the user; `account` carries the details the
/// user row will be created from once the code is verified. Any prior pending
/// registration for the same token is replaced (used on resend, which reuses the
/// challenge but rotates the code and resets the attempt counter/expiry).
pub async fn create(
    challenge_token: &str,
    account: &PendingAccount,
    code: &str,
) -> Result<(), sqlx::Error> {
    let expires_at = chrono::Utc::now() + chrono::Duration::minutes(CHALLENGE_TTL_MINUTES);
    sqlx::query(
        "INSERT INTO pending_registrations
             (challenge_hash, first_name, last_name, email, password_hash, code_hash, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT (challenge_hash) DO UPDATE SET
             first_name = EXCLUDED.first_name,
             last_name = EXCLUDED.last_name,
             email = EXCLUDED.email,
             password_hash = EXCLUDED.password_hash,
             code_hash = EXCLUDED.code_hash,
             attempts = 0,
             created_at = now(),
             expires_at = EXCLUDED.expires_at",
    )
    .bind(hash(challenge_token))
    .bind(&account.first_name)
    .bind(&account.last_name)
    .bind(&account.email)
    .bind(&account.password_hash)
    .bind(hash(code))
    .bind(expires_at)
    .execute(pool())
    .await?;
    Ok(())
}

/// Persist a pending registration together with the case it will create and the
/// terms version the client accepted on the way in.
pub async fn create_case_signup(
    challenge_token: &str,
    account: &PendingAccount,
    signup: &PendingCaseSignup,
    code: &str,
) -> Result<(), sqlx::Error> {
    let expires_at = chrono::Utc::now() + chrono::Duration::minutes(CHALLENGE_TTL_MINUTES);
    sqlx::query(
        "INSERT INTO pending_registrations
             (challenge_hash, first_name, last_name, email, password_hash, code_hash, expires_at,
              create_case, case_id, case_name, intake_json, terms_version)
         VALUES ($1, $2, $3, $4, $5, $6, $7, true, $8, $9, $10, $11)",
    )
    .bind(hash(challenge_token))
    .bind(&account.first_name)
    .bind(&account.last_name)
    .bind(&account.email)
    .bind(&account.password_hash)
    .bind(hash(code))
    .bind(expires_at)
    .bind(&signup.case_id)
    .bind(&signup.case_name)
    .bind(&signup.intake_json)
    .bind(&signup.terms_version)
    .execute(pool())
    .await?;
    Ok(())
}

/// The pending account for the live (unexpired) challenge identified by
/// `challenge_token`, or `None`. Used when resending a code to look up the
/// in-progress registration without consuming it.
pub async fn account_for(challenge_token: &str) -> Result<Option<PendingAccount>, sqlx::Error> {
    let row: Option<(String, String, String, String)> = sqlx::query_as(
        "SELECT first_name, last_name, email, password_hash FROM pending_registrations
         WHERE challenge_hash = $1 AND expires_at > now()",
    )
    .bind(hash(challenge_token))
    .fetch_optional(pool())
    .await?;
    Ok(row.map(
        |(first_name, last_name, email, password_hash)| PendingAccount {
            first_name,
            last_name,
            email,
            password_hash,
        },
    ))
}

/// The outcome of verifying a submitted code against a pending registration.
pub enum Verify {
    /// The code matched; the challenge was consumed in the caller's transaction.
    Ok(PendingRegistration),
    /// The code did not match (and the challenge is still usable, unless the
    /// attempt cap was just reached — in which case it has been burned).
    WrongCode,
    /// No live challenge exists for this token (missing, expired, or already
    /// consumed / burned by too many attempts).
    Expired,
}

/// Verify `code` against the pending registration identified by
/// `challenge_token`. On a correct code the row is deleted and the pending
/// account returned. On a wrong code the attempt counter is incremented and,
/// once it reaches [`MAX_ATTEMPTS`], the challenge is deleted so it can no longer
/// be guessed.
pub async fn verify_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    challenge_token: &str,
    code: &str,
) -> Result<Verify, sqlx::Error> {
    let challenge_hash = hash(challenge_token);

    let row = sqlx::query_as::<_, PendingRow>(
        "SELECT first_name, last_name, email, password_hash, code_hash, attempts,
                expires_at > now() AS is_live, create_case, case_id, case_name, intake_json,
                terms_version
         FROM pending_registrations
         WHERE challenge_hash = $1
         FOR UPDATE",
    )
    .bind(&challenge_hash)
    .fetch_optional(&mut **tx)
    .await?;

    let Some(row) = row else {
        return Ok(Verify::Expired);
    };

    if !row.is_live {
        sqlx::query("DELETE FROM pending_registrations WHERE challenge_hash = $1")
            .bind(&challenge_hash)
            .execute(&mut **tx)
            .await?;
        return Ok(Verify::Expired);
    }

    if row.code_hash == hash(code) {
        sqlx::query("DELETE FROM pending_registrations WHERE challenge_hash = $1")
            .bind(&challenge_hash)
            .execute(&mut **tx)
            .await?;
        let case_signup = row.create_case.then_some(PendingCaseSignup {
            case_id: row.case_id,
            case_name: row.case_name,
            intake_json: row.intake_json,
            terms_version: row.terms_version,
        });
        return Ok(Verify::Ok(PendingRegistration {
            account: PendingAccount {
                first_name: row.first_name,
                last_name: row.last_name,
                email: row.email,
                password_hash: row.password_hash,
            },
            case_signup,
        }));
    }

    if row.attempts + 1 >= MAX_ATTEMPTS {
        sqlx::query("DELETE FROM pending_registrations WHERE challenge_hash = $1")
            .bind(&challenge_hash)
            .execute(&mut **tx)
            .await?;
        return Ok(Verify::Expired);
    }

    sqlx::query(
        "UPDATE pending_registrations SET attempts = attempts + 1 WHERE challenge_hash = $1",
    )
    .bind(&challenge_hash)
    .execute(&mut **tx)
    .await?;
    Ok(Verify::WrongCode)
}

/// Delete a pending registration (best-effort cleanup, e.g. on abandon).
pub async fn delete(challenge_token: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM pending_registrations WHERE challenge_hash = $1")
        .bind(hash(challenge_token))
        .execute(pool())
        .await?;
    Ok(())
}

/// Delete every expired pending registration. Nothing outside the row needs
/// cleaning up any more: an abandoned signup now leaves no staged file behind,
/// only the row itself.
pub async fn delete_expired() -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM pending_registrations WHERE expires_at <= now()")
        .execute(pool())
        .await?;
    Ok(())
}
