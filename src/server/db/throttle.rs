//! Generic per-identifier throttling for auth endpoints (SSR only).
//!
//! Shared by any auth action that can be hammered — the password step of
//! [`crate::server_fns::auth::login`], password-reset email requests. Each
//! counter is namespaced by an [`Action`] so, for example, spamming reset
//! requests cannot lock out login and vice versa.
//!
//! Failures accumulate per (action, normalized identifier); once
//! [`MAX_ATTEMPTS`] pile up the identifier is locked for [`LOCKOUT_MINUTES`].
//! The lock auto-expires — the first attempt afterwards starts a fresh window —
//! so a legitimate user is only briefly delayed. A caller with a notion of
//! success (like login) can [`clear`] the counter outright.

use crate::server::db::pool;

/// Consecutive failures tolerated before the identifier is temporarily locked.
pub const MAX_ATTEMPTS: i32 = 5;

/// How long the identifier stays locked once [`MAX_ATTEMPTS`] is reached.
pub const LOCKOUT_MINUTES: i32 = 15;

/// A throttled auth action. Namespaces the counter so unrelated flows keeping
/// separate budgets and cannot lock one another out.
#[derive(Clone, Copy)]
pub enum Action {
    /// The password step of login.
    Login,
    /// Requesting a password-reset email.
    PasswordReset,
    /// Submitting the sign-up form (which emails a verification code).
    Register,
    /// Asking for a fresh copy of the sign-up verification code.
    ResendCode,
}

impl Action {
    /// Stable scope string persisted as the row's `scope`.
    fn scope(self) -> &'static str {
        match self {
            Action::Login => "login",
            Action::PasswordReset => "password_reset",
            Action::Register => "register",
            Action::ResendCode => "resend_code",
        }
    }
}

/// Normalize an identifier (typically an email) into the throttle key (trimmed +
/// lowercased) so callers cannot bypass the counter by varying case or padding.
fn key(identifier: &str) -> String {
    identifier.trim().to_ascii_lowercase()
}

/// Convert the remaining lock seconds into a friendly, rounded-up minute count
/// (never less than 1) for user-facing "try again in N minutes" messages.
pub fn minutes_remaining(seconds: i64) -> i64 {
    ((seconds + 59) / 60).max(1)
}

/// Seconds remaining on an active lock for `identifier` under `action`, or
/// `None` when it is not currently locked. Callers should reject the attempt up
/// front (before doing expensive or side-effecting work) whenever this is `Some`.
pub async fn seconds_locked(action: Action, identifier: &str) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT CEIL(EXTRACT(EPOCH FROM (locked_until - now())))::bigint
         FROM auth_throttle
         WHERE scope = $1 AND identifier = $2
           AND locked_until IS NOT NULL AND locked_until > now()",
    )
    .bind(action.scope())
    .bind(key(identifier))
    .fetch_optional(pool())
    .await
}

/// Record a failed/abusive attempt for `identifier` under `action`, locking it
/// once [`MAX_ATTEMPTS`] consecutive failures accumulate. The first failure
/// *after* an expired lock starts a fresh window, so lockouts do not compound.
pub async fn record_failure(action: Action, identifier: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO auth_throttle (scope, identifier, fail_count, last_failed_at, locked_until)
         VALUES ($1, $2, 1, now(), NULL)
         ON CONFLICT (scope, identifier) DO UPDATE SET
             fail_count = CASE
                 WHEN auth_throttle.locked_until IS NOT NULL
                      AND auth_throttle.locked_until <= now()
                 THEN 1
                 ELSE auth_throttle.fail_count + 1
             END,
             locked_until = CASE
                 WHEN auth_throttle.locked_until IS NOT NULL
                      AND auth_throttle.locked_until <= now()
                 THEN NULL
                 WHEN auth_throttle.fail_count + 1 >= $3
                 THEN now() + make_interval(mins => $4)
                 ELSE auth_throttle.locked_until
             END,
             last_failed_at = now()",
    )
    .bind(action.scope())
    .bind(key(identifier))
    .bind(MAX_ATTEMPTS)
    .bind(LOCKOUT_MINUTES)
    .execute(pool())
    .await?;
    Ok(())
}

/// Clear the failure record for `identifier` under `action` (e.g. after a
/// successful sign-in).
pub async fn clear(action: Action, identifier: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM auth_throttle WHERE scope = $1 AND identifier = $2")
        .bind(action.scope())
        .bind(key(identifier))
        .execute(pool())
        .await?;
    Ok(())
}
