//! Persistence for the volunteer record built on top of a user (SSR only): the
//! agreement they accepted, their application, and the decision on it.
//!
//! There is exactly one row per person and it doubles as the application, so a
//! decision updates it in place rather than filing a second record.
//!
//! The role itself is *not* set here. Granting or removing volunteer access goes
//! through [`crate::server::db::users::apply_role_in`], the single function
//! allowed to change `users.role`, which keeps this table in step with it. That
//! is what makes the invariant hold no matter which route a role change takes:
//!
//!   `users.role = 'volunteer'`  <=>  a row here with `status = 'approved'`

use std::fmt;

use crate::server::db::{pool, users};
use crate::server_fns::users::AccountRole;
use crate::server_fns::volunteers::{Volunteer, VolunteerApplication, VolunteerStatus};

/// How timestamps are rendered for display. These are shown, never compared.
const STAMP: &str = "%Y-%m-%d %H:%M";

/// What can go wrong deciding an application. Mirrors
/// [`crate::server::db::admin_requests::Error`]: the `Display` text is what the
/// person on the other end reads, so a lost race explains itself rather than
/// surfacing a database error string.
#[derive(Debug)]
pub enum Error {
    Database(sqlx::Error),
    NotFound,
    AlreadyDecided,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(error) => write!(formatter, "{error}"),
            Self::NotFound => write!(formatter, "No volunteer application was found."),
            Self::AlreadyDecided => {
                write!(formatter, "This application has already been decided.")
            }
        }
    }
}

impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

#[derive(sqlx::FromRow)]
struct VolunteerRow {
    status: String,
    agreement_version: String,
    agreed_at: chrono::DateTime<chrono::Utc>,
    decided_by_name: String,
    decision_note: String,
    decided_at: Option<chrono::DateTime<chrono::Utc>>,
}

fn stamp(at: chrono::DateTime<chrono::Utc>) -> String {
    at.with_timezone(&chrono::Local).format(STAMP).to_string()
}

impl From<VolunteerRow> for VolunteerApplication {
    fn from(row: VolunteerRow) -> Self {
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

/// Record an acceptance of the volunteer agreement.
///
/// `already_a_volunteer` decides what the acceptance *means*:
///
/// - `false` — this is an application. The record goes to `pending` for an admin
///   to decide, clearing any earlier decision so a declined or revoked person can
///   apply afresh.
/// - `true` — the person already holds the role (an admin set it directly, or
///   they predate the agreement) and is signing the paperwork after the fact.
///   There is nothing to approve, so the record stays `approved` and only gains
///   the agreement version. Without this, an existing volunteer signing would
///   demote themselves into a review queue.
///
/// Upsert either way, so the caller need not know whether a record exists.
pub async fn apply(
    user_id: &str,
    agreement_version: &str,
    already_a_volunteer: bool,
) -> Result<(), sqlx::Error> {
    if already_a_volunteer {
        sqlx::query(
            "INSERT INTO volunteers (user_id, status, agreement_version)
             VALUES ($1, 'approved', $2)
             ON CONFLICT (user_id) DO UPDATE SET
                 status = 'approved',
                 agreement_version = EXCLUDED.agreement_version,
                 agreed_at = now()",
        )
        .bind(user_id)
        .bind(agreement_version)
        .execute(pool())
        .await?;
        return Ok(());
    }

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
/// dealt with first. One join: the queue shows a name and an email, so it does
/// not pay for the full user record and its case assignments.
pub async fn list_pending() -> Result<Vec<Volunteer>, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct PendingRow {
        user_id: String,
        first_name: String,
        last_name: String,
        email: String,
        agreed_at: chrono::DateTime<chrono::Utc>,
    }

    let rows = sqlx::query_as::<_, PendingRow>(
        "SELECT v.user_id, u.first_name, u.last_name, u.email, v.agreed_at
         FROM volunteers v
         JOIN users u ON u.id = v.user_id
         WHERE v.status = 'pending'
         ORDER BY v.agreed_at",
    )
    .fetch_all(pool())
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| Volunteer {
            id: row.user_id,
            first_name: row.first_name,
            last_name: row.last_name,
            email: row.email,
            agreed_at: stamp(row.agreed_at),
        })
        .collect())
}

/// How many applications are waiting on a decision.
pub async fn pending_count() -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT count(*) FROM volunteers WHERE status = 'pending'")
        .fetch_one(pool())
        .await
}

/// Approve or decline a pending application.
///
/// Approving delegates the role change to [`users::apply_role_in`], which grants
/// the Volunteer role, marks this record approved, and writes the audit entry as
/// one atomic step. Declining only records the decision. Deciding an application
/// that is no longer pending is an error rather than a silent overwrite, so two
/// admins acting at once cannot both "win".
pub async fn decide(
    user_id: &str,
    approve: bool,
    actor_id: &str,
    actor_name: &str,
    note: &str,
) -> Result<(), Error> {
    let mut tx = pool().begin().await?;

    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM volunteers WHERE user_id = $1 FOR UPDATE")
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?;
    match status.as_deref() {
        Some("pending") => {}
        Some(_) => return Err(Error::AlreadyDecided),
        None => return Err(Error::NotFound),
    }

    if approve {
        // Sets the role *and* flips this record to approved, together.
        users::apply_role_in(&mut tx, user_id, AccountRole::Volunteer, actor_name).await?;
    }

    // Record the decision itself. On approval the status is already 'approved';
    // this adds who decided and why, and is what marks a decline.
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
    .bind(if approve {
        VolunteerStatus::Approved.slug()
    } else {
        VolunteerStatus::Denied.slug()
    })
    .bind(actor_id)
    .bind(actor_name)
    .bind(note)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}
