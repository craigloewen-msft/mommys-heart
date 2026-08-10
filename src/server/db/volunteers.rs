//! Persistence for the volunteer record built on top of a user (SSR only): the
//! agreement they accepted, the details they gave with it, and the decision on
//! their application.
//!
//! The role itself is set by [`crate::server::db::users::set_role_in`], which
//! keeps this table in step with it.
//!
//! # The Social Security Number
//!
//! [`SELECT_COLUMNS`] deliberately does *not* include the `ssn` column. Every
//! ordinary read reports only `ssn <> ''` as a boolean, so no query on the
//! normal path has the digits in hand and none can leak them. The single raw
//! read is [`reveal_ssn`], which audits the disclosure in the same transaction
//! that fetches it.

use std::fmt;

use crate::helpers::volunteer_details::{VolunteerDetails, VolunteerDetailsView};
use crate::server::db::{audit, pool, users};
use crate::server_fns::users::AccountRole;
use crate::server_fns::volunteers::{Volunteer, VolunteerApplication, VolunteerStatus};

/// How timestamps are rendered for display. These are shown, never compared.
const STAMP: &str = "%Y-%m-%d %H:%M";

/// What can go wrong deciding an application. The `Display` text is what the
/// admin reads, so a lost race explains itself instead of leaking a db error.
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
    skills_focus: String,
    date_of_birth: Option<String>,
    /// Whether a number is on file. Never the number: see the module note.
    has_ssn: bool,
    phone: String,
    emergency_first_name: String,
    emergency_last_name: String,
    emergency_relationship: String,
    emergency_phone: String,
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
            details: VolunteerDetailsView {
                skills_focus: row.skills_focus,
                date_of_birth: row.date_of_birth.unwrap_or_default(),
                has_ssn: row.has_ssn,
                phone: row.phone,
                emergency_first_name: row.emergency_first_name,
                emergency_last_name: row.emergency_last_name,
                emergency_relationship: row.emergency_relationship,
                emergency_phone: row.emergency_phone,
            },
            agreed_at: stamp(row.agreed_at),
            decided_by_name: row.decided_by_name,
            decision_note: row.decision_note,
            decided_at: row.decided_at.map(stamp).unwrap_or_default(),
        }
    }
}

/// The columns every ordinary read selects. `ssn` is absent by design and
/// reduced to a boolean; see the module note.
const SELECT_COLUMNS: &str =
    "status, agreement_version, agreed_at, decided_by_name, decision_note, decided_at,
     skills_focus, to_char(date_of_birth, 'YYYY-MM-DD') AS date_of_birth,
     (ssn <> '') AS has_ssn, phone, emergency_first_name, emergency_last_name,
     emergency_relationship, emergency_phone";

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

/// Record an acceptance of the volunteer agreement together with the details
/// submitted alongside it.
///
/// `already_a_volunteer` says what it means: `false` files a `pending`
/// application, `true` records the agreement against an existing volunteer, who
/// has nothing left to approve. Upserts either way.
///
/// The phone number is written through to `users.phone` in the same transaction,
/// so the account and the accepted application cannot disagree about how to reach
/// this person.
pub async fn apply(
    user_id: &str,
    agreement_version: &str,
    details: &VolunteerDetails,
    already_a_volunteer: bool,
    actor: &str,
) -> Result<(), sqlx::Error> {
    let status = if already_a_volunteer {
        VolunteerStatus::Approved.slug()
    } else {
        VolunteerStatus::Pending.slug()
    };

    let mut tx = pool().begin().await?;

    // An existing volunteer keeps their approval; a fresh application clears any
    // previous decision so a re-applicant starts from a clean pending state.
    sqlx::query(
        "INSERT INTO volunteers (
             user_id, status, agreement_version, skills_focus, date_of_birth, ssn,
             phone, emergency_first_name, emergency_last_name, emergency_relationship,
             emergency_phone
         )
         VALUES ($1, $2, $3, $4, NULLIF($5, '')::date, $6, $7, $8, $9, $10, $11)
         ON CONFLICT (user_id) DO UPDATE SET
             status = EXCLUDED.status,
             agreement_version = EXCLUDED.agreement_version,
             agreed_at = now(),
             skills_focus = EXCLUDED.skills_focus,
             date_of_birth = EXCLUDED.date_of_birth,
             -- A blank submission keeps whatever is already on file, matching
             -- the edit form: the browser is never sent the number, so it
             -- cannot echo it back and an empty box must not erase it.
             ssn = CASE WHEN EXCLUDED.ssn = '' THEN volunteers.ssn ELSE EXCLUDED.ssn END,
             phone = EXCLUDED.phone,
             emergency_first_name = EXCLUDED.emergency_first_name,
             emergency_last_name = EXCLUDED.emergency_last_name,
             emergency_relationship = EXCLUDED.emergency_relationship,
             emergency_phone = EXCLUDED.emergency_phone,
             decided_by = CASE WHEN EXCLUDED.status = 'pending' THEN NULL ELSE volunteers.decided_by END,
             decided_by_name = CASE WHEN EXCLUDED.status = 'pending' THEN '' ELSE volunteers.decided_by_name END,
             decision_note = CASE WHEN EXCLUDED.status = 'pending' THEN '' ELSE volunteers.decision_note END,
             decided_at = CASE WHEN EXCLUDED.status = 'pending' THEN NULL ELSE volunteers.decided_at END",
    )
    .bind(user_id)
    .bind(status)
    .bind(agreement_version)
    .bind(&details.skills_focus)
    .bind(&details.date_of_birth)
    .bind(&details.ssn)
    .bind(&details.phone)
    .bind(&details.emergency_first_name)
    .bind(&details.emergency_last_name)
    .bind(&details.emergency_relationship)
    .bind(&details.emergency_phone)
    .execute(&mut *tx)
    .await?;

    sync_user_phone(&mut tx, user_id, &details.phone, actor).await?;

    tx.commit().await?;
    Ok(())
}

/// Update one volunteer's own details, auditing each field that changed.
///
/// A blank `details.ssn` leaves the stored number alone; `remove_ssn` is the
/// only way to clear it. The audit entry for the SSN records presence only,
/// never a value, so reading the log can never disclose it.
pub async fn save_details(
    user_id: &str,
    details: &VolunteerDetails,
    remove_ssn: bool,
    actor: &str,
) -> Result<(), sqlx::Error> {
    type DetailsRow = (
        String,
        Option<String>,
        bool,
        String,
        String,
        String,
        String,
        String,
    );

    let mut tx = pool().begin().await?;

    let current = sqlx::query_as::<_, DetailsRow>(
        "SELECT skills_focus, to_char(date_of_birth, 'YYYY-MM-DD'), (ssn <> ''), phone,
                emergency_first_name, emergency_last_name, emergency_relationship, emergency_phone
         FROM volunteers WHERE user_id = $1 FOR UPDATE",
    )
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((
        skills_focus,
        date_of_birth,
        had_ssn,
        phone,
        emergency_first_name,
        emergency_last_name,
        emergency_relationship,
        emergency_phone,
    )) = current
    else {
        return Ok(());
    };

    sqlx::query(
        "UPDATE volunteers SET
             skills_focus = $2,
             date_of_birth = NULLIF($3, '')::date,
             ssn = CASE WHEN $4 THEN '' WHEN $5 = '' THEN ssn ELSE $5 END,
             phone = $6,
             emergency_first_name = $7,
             emergency_last_name = $8,
             emergency_relationship = $9,
             emergency_phone = $10
         WHERE user_id = $1",
    )
    .bind(user_id)
    .bind(&details.skills_focus)
    .bind(&details.date_of_birth)
    .bind(remove_ssn)
    .bind(&details.ssn)
    .bind(&details.phone)
    .bind(&details.emergency_first_name)
    .bind(&details.emergency_last_name)
    .bind(&details.emergency_relationship)
    .bind(&details.emergency_phone)
    .execute(&mut *tx)
    .await?;

    let changes = [
        ("volunteer skills", skills_focus, &details.skills_focus),
        (
            "volunteer date of birth",
            date_of_birth.unwrap_or_default(),
            &details.date_of_birth,
        ),
        ("volunteer phone", phone, &details.phone),
        (
            "volunteer emergency contact first name",
            emergency_first_name,
            &details.emergency_first_name,
        ),
        (
            "volunteer emergency contact last name",
            emergency_last_name,
            &details.emergency_last_name,
        ),
        (
            "volunteer emergency contact relationship",
            emergency_relationship,
            &details.emergency_relationship,
        ),
        (
            "volunteer emergency contact phone",
            emergency_phone,
            &details.emergency_phone,
        ),
    ];
    for (field, old, new) in changes {
        if &old != new {
            audit::record_in_transaction(
                &mut tx,
                audit::Entity::User,
                user_id,
                actor,
                field,
                &old,
                new,
            )
            .await?;
        }
    }

    // Presence only. The number never appears in the log.
    let has_ssn = if remove_ssn {
        false
    } else {
        had_ssn || !details.ssn.is_empty()
    };
    if has_ssn != had_ssn {
        audit::record_in_transaction(
            &mut tx,
            audit::Entity::User,
            user_id,
            actor,
            "volunteer SSN",
            if had_ssn { "on file" } else { "" },
            if has_ssn { "on file" } else { "removed" },
        )
        .await?;
    }

    sync_user_phone(&mut tx, user_id, &details.phone, actor).await?;

    tx.commit().await?;
    Ok(())
}

/// Copy the volunteer's phone onto their user record when it differs, auditing
/// the change so it reads the same as an edit made from the profile form.
async fn sync_user_phone(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: &str,
    phone: &str,
    actor: &str,
) -> Result<(), sqlx::Error> {
    let current: Option<String> = sqlx::query_scalar("SELECT phone FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await?;
    let Some(current) = current else {
        return Ok(());
    };
    if current == phone {
        return Ok(());
    }
    sqlx::query("UPDATE users SET phone = $1 WHERE id = $2")
        .bind(phone)
        .bind(user_id)
        .execute(&mut **tx)
        .await?;
    audit::record_in_transaction(
        tx,
        audit::Entity::User,
        user_id,
        actor,
        "phone",
        &current,
        phone,
    )
    .await
}

/// One volunteer's Social Security Number, auditing the disclosure first.
///
/// The audit entry and the read share a transaction, so the number cannot be
/// returned without the record of who asked for it being committed alongside.
/// This is the only query in the codebase that selects the `ssn` column, and
/// [`crate::server_fns::volunteers::reveal_volunteer_ssn`] is its only caller.
pub async fn reveal_ssn(user_id: &str, actor: &str) -> Result<String, sqlx::Error> {
    let mut tx = pool().begin().await?;
    let ssn: Option<String> = sqlx::query_scalar("SELECT ssn FROM volunteers WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;
    let ssn = ssn.unwrap_or_default();
    if !ssn.is_empty() {
        audit::record_in_transaction(
            &mut tx,
            audit::Entity::User,
            user_id,
            actor,
            "volunteer SSN",
            "on file",
            "revealed",
        )
        .await?;
    }
    tx.commit().await?;
    Ok(ssn)
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
        skills_focus: String,
        agreed_at: chrono::DateTime<chrono::Utc>,
    }

    let rows = sqlx::query_as::<_, PendingRow>(
        "SELECT v.user_id, u.first_name, u.last_name, u.email, v.skills_focus, v.agreed_at
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
            skills_focus: row.skills_focus,
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

/// Approve or decline a pending application. Approving delegates to
/// [`users::set_role_in`], which grants the role and audits it atomically.
/// Deciding one that is no longer pending errors rather than overwriting.
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
        users::set_role_in(&mut tx, user_id, AccountRole::Volunteer, actor_name).await?;
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
