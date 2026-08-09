//! Persistence for the volunteer record built on top of a user (SSR only): the
//! agreement they accepted, their application, and the decision on it.
//!
//! There is exactly one row per person and it doubles as the application, so a
//! decision updates it in place rather than filing a second record. Approving is
//! transactional with the role change on `users`, because a volunteer whose
//! application says "approved" but whose role never changed would be locked out
//! of the access the approval was supposed to grant.

use crate::server::db::{audit, pool};
use crate::server_fns::users::AccountRole;
use crate::server_fns::volunteers::{Volunteer, VolunteerApplication, VolunteerStatus};

/// How timestamps are rendered for display. These are shown, never compared.
const STAMP: &str = "%Y-%m-%d %H:%M";

#[derive(sqlx::FromRow)]
struct VolunteerRow {
    status: String,
    agreement_version: String,
    agreed_at: chrono::DateTime<chrono::Utc>,
    decided_by_name: String,
    decision_note: String,
    decided_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<VolunteerRow> for VolunteerApplication {
    fn from(row: VolunteerRow) -> Self {
        let stamp = |at: chrono::DateTime<chrono::Utc>| {
            at.with_timezone(&chrono::Local).format(STAMP).to_string()
        };
        Self {
            // An unparseable status would mean the CHECK constraint was bypassed;
            // treat it as pending so it surfaces in the admin queue for a human.
            status: VolunteerStatus::from_slug(&row.status).unwrap_or(VolunteerStatus::Pending),
            agreement_version: row.agreement_version,
            agreed_at: stamp(row.agreed_at),
            decided_by_name: row.decided_by_name,
            decision_note: row.decision_note,
            decided_at: row.decided_at.map(stamp).unwrap_or_default(),
        }
    }
}

const SELECT_COLUMNS: &str =
    "status, agreement_version, agreed_at, decided_by_name, decision_note, decided_at";

/// One person's volunteer record, or `None` if they have never applied.
pub async fn get(user_id: &str) -> Result<Option<VolunteerApplication>, sqlx::Error> {
    let row = sqlx::query_as::<_, VolunteerRow>(&format!(
        "SELECT {SELECT_COLUMNS} FROM volunteers WHERE user_id = $1"
    ))
    .bind(user_id)
    .fetch_optional(pool())
    .await?;
    Ok(row.map(Into::into))
}

/// Record an acceptance of the volunteer agreement as a pending application.
///
/// Upsert rather than insert: a previously declined applicant may accept again,
/// which returns their record to pending and clears the old decision.
pub async fn apply(user_id: &str, agreement_version: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO volunteers (user_id, status, agreement_version)
         VALUES ($1, 'pending', $2)
         ON CONFLICT (user_id) DO UPDATE SET
             status = 'pending',
             agreement_version = EXCLUDED.agreement_version,
             agreed_at = now(),
             decided_by = NULL,
             decided_by_name = '',
             decision_note = '',
             decided_at = NULL",
    )
    .bind(user_id)
    .bind(agreement_version)
    .execute(pool())
    .await?;
    Ok(())
}

/// Every application waiting on a decision, oldest first so the longest wait is
/// dealt with first.
pub async fn list_pending() -> Result<Vec<Volunteer>, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct PendingRow {
        user_id: String,
        #[sqlx(flatten)]
        application: VolunteerRow,
    }

    let rows = sqlx::query_as::<_, PendingRow>(&format!(
        "SELECT user_id, {SELECT_COLUMNS} FROM volunteers
         WHERE status = 'pending'
         ORDER BY agreed_at"
    ))
    .fetch_all(pool())
    .await?;

    let mut volunteers = Vec::with_capacity(rows.len());
    for row in rows {
        // Skip an application whose user vanished between the two reads rather
        // than failing the whole queue for one missing row.
        let Some(user) = crate::server::db::users::get(&row.user_id).await? else {
            continue;
        };
        volunteers.push(Volunteer {
            user,
            application: row.application.into(),
        });
    }
    Ok(volunteers)
}

/// How many applications are waiting on a decision.
pub async fn pending_count() -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT count(*) FROM volunteers WHERE status = 'pending'")
        .fetch_one(pool())
        .await
}

/// Approve or decline a pending application.
///
/// Approving grants the Volunteer role and marks the record approved in one
/// transaction, with the role change written to the audit log exactly as any
/// other role change is. Declining only records the decision. Deciding an
/// application that is no longer pending is an error rather than a silent
/// overwrite, so two admins acting at once cannot both "win".
pub async fn decide(
    user_id: &str,
    approve: bool,
    actor_id: &str,
    actor_name: &str,
    note: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = pool().begin().await?;

    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM volunteers WHERE user_id = $1 FOR UPDATE")
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?;
    match status.as_deref() {
        Some("pending") => {}
        Some(_) => return Err(sqlx::Error::Protocol("already_decided".into())),
        None => return Err(sqlx::Error::RowNotFound),
    }

    if approve {
        let current_role: Option<String> =
            sqlx::query_scalar("SELECT role FROM users WHERE id = $1 FOR UPDATE")
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;
        let current_role = current_role.ok_or(sqlx::Error::RowNotFound)?;
        // Don't demote an admin who happened to apply: they already have
        // volunteer privileges, so approving is about the agreement, not the role.
        if !AccountRole::from_slug(&current_role).is_some_and(AccountRole::has_volunteer_privileges)
        {
            sqlx::query("UPDATE users SET role = $1 WHERE id = $2")
                .bind(AccountRole::Volunteer.slug())
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
            audit::record_in_transaction(
                &mut tx,
                audit::Entity::User,
                user_id,
                actor_name,
                "role",
                &current_role,
                AccountRole::Volunteer.slug(),
            )
            .await?;
        }
    }

    sqlx::query(
        "UPDATE volunteers SET
             status = $2,
             decided_by = $3,
             decided_by_name = $4,
             decision_note = $5,
             decided_at = now()
         WHERE user_id = $1",
    )
    .bind(user_id)
    .bind(if approve { "approved" } else { "denied" })
    .bind(actor_id)
    .bind(actor_name)
    .bind(note)
    .execute(&mut *tx)
    .await?;

    tx.commit().await
}
