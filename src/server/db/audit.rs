//! The unified append-only audit log shared by users and cases.
//!
//! As well as writing and paging the log, this module serves the admin activity
//! feed and its digest, which are reads over the same table — see the
//! "Admin activity feed" section below.

use crate::helpers::visibility::Visibility;
use crate::server::db::{ids, now_stamp, pool};
use crate::server_fns::admin_activity::{AdminActivityCategory, AdminActivityEvent};
use crate::server_fns::audit::ChangeLogEntry;
use crate::server_fns::pagination::Page;

/// How long audit entries are retained before the background task prunes them.
const RETENTION_YEARS: i64 = 10;

/// How often the background retention task runs.
const RETENTION_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30 * 24 * 60 * 60);

/// Which kind of entity an audit entry is attached to.
#[derive(Clone, Copy, Debug)]
pub enum Entity {
    User,
    Case,
    Contact,
    Organization,
    Grant,
    Funding,
}

impl Entity {
    fn as_str(self) -> &'static str {
        match self {
            Entity::User => "user",
            Entity::Case => "case",
            Entity::Contact => "contact",
            Entity::Organization => "organization",
            Entity::Grant => "grant",
            Entity::Funding => "funding",
        }
    }
}

#[derive(sqlx::FromRow)]
struct AuditRow {
    id: String,
    actor_user_id: Option<String>,
    actor: String,
    field: String,
    old_value: String,
    new_value: String,
    at: String,
}

impl From<AuditRow> for ChangeLogEntry {
    fn from(r: AuditRow) -> Self {
        ChangeLogEntry {
            id: r.id,
            actor_user_id: r.actor_user_id.unwrap_or_default(),
            actor: r.actor,
            field: r.field,
            old_value: r.old_value,
            new_value: r.new_value,
            at: r.at,
        }
    }
}

// REQ-AUD-001..004: callers choose whether restricted metadata is visible;
// substantive message and note content never belongs in this table.
pub async fn page(
    entity: Entity,
    entity_id: &str,
    start: &str,
    end: &str,
    offset: i64,
    limit: i64,
    include_restricted: bool,
    restrict_contact_history: bool,
) -> Result<Page<ChangeLogEntry>, sqlx::Error> {
    let pool = pool();

    let total: i64 = sqlx::query_scalar(
        "SELECT count(*)
         FROM audit_log
         WHERE entity_type = $1 AND entity_id = $2
           AND ($3 = '' OR left(at, 10) >= $3)
           AND ($4 = '' OR left(at, 10) <= $4)
           AND ($5 OR COALESCE(visibility, 'shared') <> $6)
           AND (NOT $7 OR field NOT IN ('case link', 'account link'))",
    )
    .bind(entity.as_str())
    .bind(entity_id)
    .bind(start)
    .bind(end)
    .bind(include_restricted)
    .bind(Visibility::VolunteerOnly.slug())
    .bind(restrict_contact_history)
    .fetch_one(pool)
    .await?;

    let rows = sqlx::query_as::<_, AuditRow>(
        "SELECT id, actor_user_id, actor, field, old_value, new_value, at
         FROM audit_log
         WHERE entity_type = $1 AND entity_id = $2
           AND ($3 = '' OR left(at, 10) >= $3)
           AND ($4 = '' OR left(at, 10) <= $4)
           AND ($5 OR COALESCE(visibility, 'shared') <> $6)
           AND (NOT $7 OR field NOT IN ('case link', 'account link'))
         ORDER BY seq DESC
         LIMIT $8 OFFSET $9",
    )
    .bind(entity.as_str())
    .bind(entity_id)
    .bind(start)
    .bind(end)
    .bind(include_restricted)
    .bind(Visibility::VolunteerOnly.slug())
    .bind(restrict_contact_history)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(Page {
        items: rows.into_iter().map(Into::into).collect(),
        total,
    })
}

/// Append a new audit entry, allocating its id. `actor` is the display name of
/// whoever made the change.
pub async fn record(
    pool: &sqlx::PgPool,
    entity: Entity,
    entity_id: &str,
    actor: &str,
    field: &str,
    old_value: &str,
    new_value: &str,
) -> Result<(), sqlx::Error> {
    record_with_visibility(
        pool,
        entity,
        entity_id,
        actor,
        field,
        old_value,
        new_value,
        Visibility::Shared,
    )
    .await
}

/// Append an audit entry with explicit audience visibility.
pub async fn record_with_visibility(
    pool: &sqlx::PgPool,
    entity: Entity,
    entity_id: &str,
    actor: &str,
    field: &str,
    old_value: &str,
    new_value: &str,
    visibility: Visibility,
) -> Result<(), sqlx::Error> {
    let mut connection = pool.acquire().await?;
    record_with_connection(
        &mut connection,
        entity,
        entity_id,
        actor,
        field,
        old_value,
        new_value,
        visibility,
    )
    .await
}

/// Bind the authenticated account to this transaction. CRM audit writes inherit
/// the stable id without duplicating it at every repository call.
pub async fn set_actor_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor_user_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT set_config('app.actor_user_id', $1, true)")
        .bind(actor_user_id)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

/// Append an audit entry as part of an existing transaction, so the audited
/// mutation and its audit record commit or roll back together.
pub async fn record_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    entity: Entity,
    entity_id: &str,
    actor: &str,
    field: &str,
    old_value: &str,
    new_value: &str,
) -> Result<(), sqlx::Error> {
    record_in_transaction_with_visibility(
        transaction,
        entity,
        entity_id,
        actor,
        field,
        old_value,
        new_value,
        Visibility::Shared,
    )
    .await
}

/// Append one audit entry per `(entity_id, old_value)` pair in a single
/// statement, all sharing the same field and new value.
///
/// A bulk edit touches many records at once; recording them one row at a time
/// would issue thousands of round trips inside a single transaction, holding
/// locks for the whole run.
pub async fn record_many_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    entity: Entity,
    entries: &[(String, String)],
    actor_user_id: &str,
    actor: &str,
    field: &str,
    new_value: &str,
) -> Result<(), sqlx::Error> {
    if entries.is_empty() {
        return Ok(());
    }
    let entity_ids: Vec<&str> = entries.iter().map(|(id, _)| id.as_str()).collect();
    let old_values: Vec<&str> = entries.iter().map(|(_, old)| old.as_str()).collect();
    sqlx::query(
        "INSERT INTO audit_log
            (id, entity_type, entity_id, actor_user_id, actor, field, old_value,
             new_value, at, visibility)
         SELECT 'cl-' || nextval('app_id_seq'), $1, entry.entity_id, $2, $3, $4,
                entry.old_value, $5, $6, $7
         FROM unnest($8::text[], $9::text[]) AS entry(entity_id, old_value)",
    )
    .bind(entity.as_str())
    .bind(actor_user_id)
    .bind(actor)
    .bind(field)
    .bind(new_value)
    .bind(now_stamp())
    .bind(Visibility::Shared.slug())
    .bind(&entity_ids)
    .bind(&old_values)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

/// Record a mutation with both stable actor identity and its historical display
/// snapshot. Legacy callers may continue using [`record_in_transaction`].
pub async fn record_in_transaction_by(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    entity: Entity,
    entity_id: &str,
    actor_user_id: &str,
    actor: &str,
    field: &str,
    old_value: &str,
    new_value: &str,
) -> Result<(), sqlx::Error> {
    record_with_connection_by(
        &mut **transaction,
        entity,
        entity_id,
        Some(actor_user_id),
        actor,
        field,
        old_value,
        new_value,
        Visibility::Shared,
    )
    .await
}

/// Stable-actor variant for restricted audit metadata.
pub async fn record_in_transaction_by_with_visibility(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    entity: Entity,
    entity_id: &str,
    actor_user_id: &str,
    actor: &str,
    field: &str,
    old_value: &str,
    new_value: &str,
    visibility: Visibility,
) -> Result<(), sqlx::Error> {
    record_with_connection_by(
        &mut **transaction,
        entity,
        entity_id,
        Some(actor_user_id),
        actor,
        field,
        old_value,
        new_value,
        visibility,
    )
    .await
}

/// Append an explicitly visible audit entry inside an existing transaction.
pub async fn record_in_transaction_with_visibility(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    entity: Entity,
    entity_id: &str,
    actor: &str,
    field: &str,
    old_value: &str,
    new_value: &str,
    visibility: Visibility,
) -> Result<(), sqlx::Error> {
    record_with_connection(
        &mut **transaction,
        entity,
        entity_id,
        actor,
        field,
        old_value,
        new_value,
        visibility,
    )
    .await
}

async fn record_with_connection(
    connection: &mut sqlx::PgConnection,
    entity: Entity,
    entity_id: &str,
    actor: &str,
    field: &str,
    old_value: &str,
    new_value: &str,
    visibility: Visibility,
) -> Result<(), sqlx::Error> {
    record_with_connection_by(
        connection, entity, entity_id, None, actor, field, old_value, new_value, visibility,
    )
    .await
}

async fn record_with_connection_by(
    connection: &mut sqlx::PgConnection,
    entity: Entity,
    entity_id: &str,
    actor_user_id: Option<&str>,
    actor: &str,
    field: &str,
    old_value: &str,
    new_value: &str,
    visibility: Visibility,
) -> Result<(), sqlx::Error> {
    let id = ids::next(&mut *connection, "cl").await?;
    sqlx::query(
        "INSERT INTO audit_log
            (id, entity_type, entity_id, actor_user_id, actor, field, old_value,
             new_value, at, visibility)
         VALUES ($1, $2, $3,
                 COALESCE($4, NULLIF(current_setting('app.actor_user_id', true), '')),
                 $5, $6, $7, $8, $9, $10)",
    )
    .bind(&id)
    .bind(entity.as_str())
    .bind(entity_id)
    .bind(actor_user_id)
    .bind(actor)
    .bind(field)
    .bind(old_value)
    .bind(new_value)
    .bind(now_stamp())
    .bind(visibility.slug())
    .execute(connection)
    .await?;
    Ok(())
}

/// Delete audit entries older than the retention window ([`RETENTION_YEARS`]),
/// returning the number of rows removed. Retention is computed against the
/// machine-readable `created_at` timestamp (not the display-only `at` text).
pub async fn purge_expired(pool: &sqlx::PgPool) -> Result<u64, sqlx::Error> {
    let result =
        sqlx::query("DELETE FROM audit_log WHERE created_at < now() - make_interval(years => $1)")
            .bind(RETENTION_YEARS as i32)
            .execute(pool)
            .await?;
    Ok(result.rows_affected())
}

/// Spawn a background task that prunes expired audit entries at startup and then
/// once every [`RETENTION_INTERVAL`]. Mirrors [`crate::server::rag::start_background_ingest`]:
/// failures are logged and never crash the server.
pub fn start_retention_task() {
    tokio::spawn(async {
        loop {
            match purge_expired(pool()).await {
                Ok(n) => tracing::info!("audit retention: deleted {n} expired entries"),
                Err(e) => tracing::warn!("audit retention failed: {e}"),
            }
            tokio::time::sleep(RETENTION_INTERVAL).await;
        }
    });
}

// ─── Admin activity feed ─────────────────────────────────────────────────────
//
// The feed administrators read under Admin → Activity, and the daily digest
// email, are both *views over this log* (plus the restricted
// `case_note_audit_log`, unioned in without any note content). They record
// nothing of their own: the only state is a one-row watermark saying how far the
// digest has mailed, so the digest never marks or mutates an append-only log.
//
// What these add over `page` above is *classification*: audit rows are
// field-level and include work administrators did not ask to be alerted about.
// `AdminActivityCategory::classify` decides what belongs in the feed; everything
// here is the SQL that feeds it.

/// How far the digest has got through each log.
#[derive(Clone, Copy, Debug)]
pub struct DigestWatermark {
    pub last_audit_seq: i64,
    pub last_note_seq: i64,
}

/// A row from either log, before classification. `entity`/`field` come from
/// `audit_log`; the note log maps onto the same shape.
#[derive(sqlx::FromRow)]
struct ActivityRow {
    id: String,
    seq: i64,
    entity: String,
    entity_id: String,
    actor: String,
    field: String,
    old_value: String,
    new_value: String,
    subject_name: Option<String>,
    at: String,
}

/// `audit_log` rows joined to their subject's current name. Restricted to the
/// entity types the feed classifies, so the scan never walks user/permission
/// history. `$1` is an exclusive lower bound on `seq` (0 for "from the start").
const AUDIT_SELECT: &str = "
    SELECT a.id, a.seq, a.entity_type AS entity, a.entity_id, a.actor, a.field,
           a.old_value, a.new_value, a.at,
           COALESCE(
               c.name,
               NULLIF(btrim(concat_ws(' ',
                   NULLIF(ct.preferred_name, ''), ct.first_name, ct.last_name)), ''),
               o.name
           ) AS subject_name
    FROM audit_log a
    LEFT JOIN cases c ON a.entity_type = 'case' AND c.id = a.entity_id
    LEFT JOIN contacts ct ON a.entity_type = 'contact' AND ct.id = a.entity_id
    LEFT JOIN organizations o ON a.entity_type = 'organization' AND o.id = a.entity_id
    WHERE a.entity_type IN ('case', 'contact', 'organization')
      AND a.seq > $1
";

/// Finalized notes and addenda from the restricted note log, projected onto the
/// same shape. Only these two actions are surfaced: drafts are private to their
/// author until finalized, and no note *content* is ever selected.
const NOTE_SELECT: &str = "
    SELECT n.id, n.seq, 'case' AS entity, n.case_id AS entity_id, n.actor,
           'case note' AS field, '' AS old_value, n.action AS new_value, n.at,
           c.name AS subject_name
    FROM case_note_audit_log n
    LEFT JOIN cases c ON c.id = n.case_id
    WHERE n.action IN ('finalize_note', 'add_addendum')
      AND n.seq > $1
";

impl ActivityRow {
    /// Classify and render, or `None` when this row is not feed material.
    fn into_event(self) -> Option<AdminActivityEvent> {
        let category = AdminActivityCategory::classify(&self.entity, &self.field)?;
        let subject = AdminActivityCategory::subject_for(&self.entity)?;
        Some(AdminActivityEvent {
            id: self.id,
            category,
            actor: self.actor,
            summary: summarize(&self.field, &self.old_value, &self.new_value),
            subject,
            subject_id: self.entity_id.clone(),
            // A deleted record leaves the join empty; its id is still meaningful.
            subject_name: self.subject_name.unwrap_or(self.entity_id),
            at: self.at,
        })
    }
}

/// Turn an audit row's field/old/new into the sentence the feed shows.
///
/// Audit rows are field-level (`status: Open -> Closed`); the feed reads as prose
/// (`changed the status from "Open" to "Closed"`). Only the shapes the feed
/// actually surfaces are special-cased; anything else falls back to a faithful
/// generic rendering rather than inventing wording.
fn summarize(field: &str, old_value: &str, new_value: &str) -> String {
    match (field, new_value) {
        ("case", "created") => "created the case".to_string(),
        ("case", value) => format!("{value} the case"),
        ("case note", "finalize_note") => "finalized a case note".to_string(),
        ("case note", _) => "added an addendum to a case note".to_string(),
        ("properties", _) => "updated the case information".to_string(),
        ("evidence", _) if old_value.is_empty() => format!("added the file \"{new_value}\""),
        ("evidence", _) => format!("changed the file \"{new_value}\""),
        ("folder", _) if old_value.is_empty() => format!("added the folder \"{new_value}\""),
        ("folder", _) => format!("removed the folder \"{old_value}\""),
        ("case contact", _) => format!("{new_value} on the case"),
        ("contact", value) => format!("{value} the contact"),
        ("organization", value) if old_value.is_empty() => format!("{value} the organization"),
        ("categories", _) => "changed the contact's categories".to_string(),
        ("account link", value) => format!("{value} the contact and an account"),
        // Field-level fallback: name the field and both values honestly.
        (field, _) if old_value.is_empty() => format!("set {field} to \"{new_value}\""),
        (field, _) => format!("changed {field} from \"{old_value}\" to \"{new_value}\""),
    }
}

/// One page of activity, newest first, optionally narrowed to one category.
///
/// Classification happens in Rust, not SQL, so the category rules live in one
/// place. That means the page is assembled by scanning recent rows and keeping
/// the ones that classify — the `seq` indexes make that walk cheap, and the
/// window is bounded by [`SCAN_LIMIT`].
pub async fn activity_page(
    category: Option<AdminActivityCategory>,
    offset: i64,
    limit: i64,
) -> Result<Page<AdminActivityEvent>, sqlx::Error> {
    /// How far back a single request will look. Bounds the work regardless of
    /// how much history exists; the feed is a recent-activity view, and the
    /// Change Log remains the exhaustive record.
    const SCAN_LIMIT: i64 = 2_000;

    let rows = sqlx::query_as::<_, ActivityRow>(&format!(
        "SELECT * FROM (
             ({AUDIT_SELECT} ORDER BY a.seq DESC LIMIT {SCAN_LIMIT})
             UNION ALL
             ({NOTE_SELECT} ORDER BY n.seq DESC LIMIT {SCAN_LIMIT})
         ) rows ORDER BY at DESC, seq DESC"
    ))
    .bind(0i64)
    .fetch_all(pool())
    .await?;

    let events: Vec<AdminActivityEvent> = rows
        .into_iter()
        .filter_map(ActivityRow::into_event)
        .filter(|event| category.is_none_or(|wanted| event.category == wanted))
        .collect();

    let total = events.len() as i64;
    Ok(Page {
        items: events
            .into_iter()
            .skip(offset.max(0) as usize)
            .take(limit.max(0) as usize)
            .collect(),
        total,
    })
}

/// The digest's current position in each log.
///
/// A watermark ahead of the log it tracks is clamped back to the log's end. That
/// only happens when a log has been truncated and its sequence restarted (which
/// `etc/dev.sh reset` does); without the clamp the digest would sit past every
/// row and silently report nothing forever.
pub async fn activity_watermark() -> Result<DigestWatermark, sqlx::Error> {
    let stored: Option<(i64, i64)> =
        sqlx::query_as("SELECT last_audit_seq, last_note_seq FROM admin_activity_digest_state")
            .fetch_optional(pool())
            .await?;
    let (last_audit_seq, last_note_seq) = stored.unwrap_or((0, 0));

    let (audit_max, note_max): (i64, i64) = sqlx::query_as(
        "SELECT COALESCE((SELECT max(seq) FROM audit_log), 0),
                COALESCE((SELECT max(seq) FROM case_note_audit_log), 0)",
    )
    .fetch_one(pool())
    .await?;

    Ok(DigestWatermark {
        last_audit_seq: last_audit_seq.min(audit_max),
        last_note_seq: last_note_seq.min(note_max),
    })
}

/// Everything worth reporting since `watermark`, oldest first, with the new
/// watermark to store once it has been sent. Returns at most `limit` events plus
/// a count of how many further ones were left for the next run.
pub async fn activity_since(
    watermark: DigestWatermark,
    limit: i64,
) -> Result<(Vec<AdminActivityEvent>, i64, DigestWatermark), sqlx::Error> {
    let audit_rows = sqlx::query_as::<_, ActivityRow>(&format!("{AUDIT_SELECT} ORDER BY a.seq"))
        .bind(watermark.last_audit_seq)
        .fetch_all(pool())
        .await?;
    let note_rows = sqlx::query_as::<_, ActivityRow>(&format!("{NOTE_SELECT} ORDER BY n.seq"))
        .bind(watermark.last_note_seq)
        .fetch_all(pool())
        .await?;

    // Advance past everything examined, including rows that did not classify —
    // otherwise unclassified rows would be rescanned forever.
    let next = DigestWatermark {
        last_audit_seq: audit_rows
            .iter()
            .map(|row| row.seq)
            .max()
            .unwrap_or(watermark.last_audit_seq),
        last_note_seq: note_rows
            .iter()
            .map(|row| row.seq)
            .max()
            .unwrap_or(watermark.last_note_seq),
    };

    let mut events: Vec<AdminActivityEvent> = audit_rows
        .into_iter()
        .chain(note_rows)
        .filter_map(ActivityRow::into_event)
        .collect();
    events.sort_by(|a, b| a.at.cmp(&b.at));

    let overflow = (events.len() as i64 - limit).max(0);
    events.truncate(limit.max(0) as usize);
    Ok((events, overflow, next))
}

/// Store the digest's new position after a successful send. Upserts, so a
/// missing state row (only possible if it were deleted out of band) heals.
pub async fn set_activity_watermark(next: DigestWatermark) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO admin_activity_digest_state
             (id, last_audit_seq, last_note_seq, last_sent_at)
         VALUES (true, $1, $2, now())
         ON CONFLICT (id) DO UPDATE SET
             last_audit_seq = EXCLUDED.last_audit_seq,
             last_note_seq  = EXCLUDED.last_note_seq,
             last_sent_at   = EXCLUDED.last_sent_at",
    )
    .bind(next.last_audit_seq)
    .bind(next.last_note_seq)
    .execute(pool())
    .await?;
    Ok(())
}
