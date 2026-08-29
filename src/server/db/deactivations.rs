//! Persistence for the deactivation record built on top of a user (SSR only):
//! why an account was retired, by whom, and the role to restore it to.
//!
//! The counterpart to [`crate::server::db::volunteers`] and
//! [`crate::server::db::clients`]: `users` holds identity and role, this holds
//! what is true *because* an account holds [`AccountRole::Deactivated`]. The
//! role itself is set by [`crate::server::db::users::set_role_in`], and the two
//! are kept in step by [`crate::server::db::users::set_deactivated`], the only
//! writer of this table.
//!
//! A reactivated account keeps its row, with `reactivated_at` set. Nothing is
//! deleted, so "this account was once retired" survives the restore.

use crate::server::db::pool;
use crate::server_fns::users::{AccountDeactivation, AccountRole};

/// How timestamps are rendered for display. These are shown, never compared.
const STAMP: &str = "%Y-%m-%d %H:%M";

fn stamp(at: chrono::DateTime<chrono::Utc>) -> String {
    at.with_timezone(&chrono::Local).format(STAMP).to_string()
}

#[derive(sqlx::FromRow)]
struct DeactivationRow {
    previous_role: String,
    reason: String,
    deactivated_by_name: String,
    deactivated_at: chrono::DateTime<chrono::Utc>,
}

impl DeactivationRow {
    fn into_domain(self) -> Result<AccountDeactivation, sqlx::Error> {
        // A previous role that no longer parses would mean the restore target is
        // gone, so fail loudly rather than silently reactivating as something else.
        let previous_role = AccountRole::from_slug(&self.previous_role).ok_or_else(|| {
            sqlx::Error::Decode(
                format!("invalid previous account role slug: {:?}", self.previous_role).into(),
            )
        })?;
        Ok(AccountDeactivation {
            previous_role,
            reason: self.reason,
            by: self.deactivated_by_name,
            at: stamp(self.deactivated_at),
        })
    }
}

/// The live deactivation for a user, or `None` if the account is not currently
/// deactivated. A restored account's historical row is deliberately not returned.
pub async fn get(user_id: &str) -> Result<Option<AccountDeactivation>, sqlx::Error> {
    let row = sqlx::query_as::<_, DeactivationRow>(
        "SELECT previous_role, reason, deactivated_by_name, deactivated_at
         FROM account_deactivations
         WHERE user_id = $1 AND reactivated_at IS NULL",
    )
    .bind(user_id)
    .fetch_optional(pool())
    .await?;
    row.map(DeactivationRow::into_domain).transpose()
}

/// Record a deactivation inside the caller's transaction, replacing any earlier
/// (restored) record for the same person so re-deactivating starts clean.
pub async fn record_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: &str,
    previous_role: AccountRole,
    reason: &str,
    actor_user_id: &str,
    actor: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO account_deactivations
             (user_id, previous_role, reason, deactivated_by, deactivated_by_name)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (user_id) DO UPDATE SET
             previous_role = EXCLUDED.previous_role,
             reason = EXCLUDED.reason,
             deactivated_by = EXCLUDED.deactivated_by,
             deactivated_by_name = EXCLUDED.deactivated_by_name,
             deactivated_at = now(),
             reactivated_by_name = '',
             reactivated_at = NULL",
    )
    .bind(user_id)
    .bind(previous_role.slug())
    .bind(reason)
    .bind(actor_user_id)
    .bind(actor)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Read the role to restore, locking the row so two concurrent reactivations
/// cannot both act on it. `None` when the account is not currently deactivated.
pub async fn lock_previous_role_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: &str,
) -> Result<Option<AccountRole>, sqlx::Error> {
    let slug: Option<String> = sqlx::query_scalar(
        "SELECT previous_role FROM account_deactivations
         WHERE user_id = $1 AND reactivated_at IS NULL
         FOR UPDATE",
    )
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(slug) = slug else {
        return Ok(None);
    };
    AccountRole::from_slug(&slug).map(Some).ok_or_else(|| {
        sqlx::Error::Decode(format!("invalid previous account role slug: {slug:?}").into())
    })
}

/// Mark a deactivation as reversed, keeping the row as history.
pub async fn record_reactivation_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: &str,
    actor: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE account_deactivations
            SET reactivated_at = now(), reactivated_by_name = $2
          WHERE user_id = $1 AND reactivated_at IS NULL",
    )
    .bind(user_id)
    .bind(actor)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
