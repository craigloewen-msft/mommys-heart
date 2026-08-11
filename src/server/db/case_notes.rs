//! Structured Case Notes persistence (SSR only).
//!
//! This repository owns note lifecycle transactions, draft isolation, immutable
//! final records/addenda, and the restricted note audit table.

use serde_json::json;

use crate::server::db::{ids, now_stamp, pool};
use crate::server_fns::case_notes::{
    AddendumCategory, CaseNoteAddendum, CaseNoteAddendumInput, CaseNoteAudience,
    CaseNoteAuditAction, CaseNoteAuditEntry, CaseNoteDetail, CaseNoteDraftInput,
    CaseNoteInteractionType, CaseNoteListFilters, CaseNoteListItem, CaseNoteState,
    CompletionOutcome, ContactCategory, ContactDirection, InformationSource, ServiceArea,
    UrgencyLevel, ValidatedCaseNoteFinalization,
};
use crate::server_fns::cases::{CaseNote as LegacyCaseNote, LegacyCaseNoteAddendum};
use crate::server_fns::pagination::Page;
use crate::server_fns::users::{AccountRole, User};

#[derive(sqlx::FromRow)]
struct CaseNoteListRow {
    id: String,
    case_id: String,
    state: String,
    audience: String,
    author_user_id: String,
    author: String,
    author_role_snapshot: String,
    created_at: String,
    updated_at: String,
    finalized_at: String,
    discarded_at: String,
    activity_date: String,
    total_minutes: Option<i32>,
    primary_interaction: String,
    urgency: String,
    addendum_count: i64,
}

impl CaseNoteListRow {
    fn into_item(self) -> CaseNoteListItem {
        CaseNoteListItem {
            id: self.id,
            case_id: self.case_id,
            state: CaseNoteState::from_slug(&self.state).unwrap_or(CaseNoteState::Legacy),
            audience: CaseNoteAudience::from_slug(&self.audience)
                .unwrap_or(CaseNoteAudience::SharedLegacy),
            author_user_id: self.author_user_id,
            author: self.author,
            author_role_snapshot: AccountRole::from_slug(&self.author_role_snapshot),
            created_at: self.created_at,
            updated_at: self.updated_at,
            finalized_at: self.finalized_at,
            discarded_at: self.discarded_at,
            activity_date: self.activity_date,
            total_minutes: self.total_minutes,
            primary_interaction: CaseNoteInteractionType::from_slug(&self.primary_interaction),
            urgency: UrgencyLevel::from_slug(&self.urgency),
            addendum_count: self.addendum_count,
        }
    }
}

#[derive(sqlx::FromRow)]
struct CaseNoteDetailRow {
    id: String,
    case_id: String,
    state: String,
    audience: String,
    author_user_id: String,
    author: String,
    author_role_snapshot: String,
    created_at: String,
    updated_at: String,
    finalized_at: String,
    discarded_at: String,
    signature_name: String,
    signature_signed_at: String,
    body: String,
    activity_date: String,
    start_time: String,
    end_time: String,
    total_minutes: Option<i32>,
    location: String,
    delayed_entry_reason: String,
    primary_interaction: String,
    contact_category: String,
    contact_direction: String,
    completion_outcome: String,
    participant_summary: String,
    service_areas: Vec<String>,
    purpose: String,
    client_reported_info: String,
    verified_observed_info: String,
    information_sources: Vec<String>,
    actions_taken: String,
    outcome_response: String,
    progress_barriers: String,
    urgency: String,
    urgency_details: String,
    next_steps: String,
    next_steps_not_applicable: bool,
    narrative: String,
}

impl CaseNoteDetailRow {
    fn into_detail(self, addenda: Vec<CaseNoteAddendum>) -> CaseNoteDetail {
        CaseNoteDetail {
            id: self.id,
            case_id: self.case_id,
            state: CaseNoteState::from_slug(&self.state).unwrap_or(CaseNoteState::Legacy),
            audience: CaseNoteAudience::from_slug(&self.audience)
                .unwrap_or(CaseNoteAudience::SharedLegacy),
            author_user_id: self.author_user_id,
            author: self.author,
            author_role_snapshot: AccountRole::from_slug(&self.author_role_snapshot),
            created_at: self.created_at,
            updated_at: self.updated_at,
            finalized_at: self.finalized_at,
            discarded_at: self.discarded_at,
            signature_name: self.signature_name,
            signature_signed_at: self.signature_signed_at,
            total_minutes: self.total_minutes,
            draft: CaseNoteDraftInput {
                activity_date: self.activity_date,
                start_time: self.start_time,
                end_time: self.end_time,
                location: self.location,
                delayed_entry_reason: self.delayed_entry_reason,
                primary_interaction: CaseNoteInteractionType::from_slug(&self.primary_interaction),
                contact_category: ContactCategory::from_slug(&self.contact_category),
                contact_direction: ContactDirection::from_slug(&self.contact_direction),
                completion_outcome: CompletionOutcome::from_slug(&self.completion_outcome),
                participant_summary: self.participant_summary,
                service_areas: self
                    .service_areas
                    .iter()
                    .filter_map(|s| ServiceArea::from_slug(s))
                    .collect(),
                purpose: self.purpose,
                client_reported_info: self.client_reported_info,
                verified_observed_info: self.verified_observed_info,
                information_sources: self
                    .information_sources
                    .iter()
                    .filter_map(|s| InformationSource::from_slug(s))
                    .collect(),
                actions_taken: self.actions_taken,
                outcome_response: self.outcome_response,
                progress_barriers: self.progress_barriers,
                urgency: UrgencyLevel::from_slug(&self.urgency),
                urgency_details: self.urgency_details,
                next_steps: self.next_steps,
                next_steps_not_applicable: self.next_steps_not_applicable,
                narrative: self.narrative,
            },
            legacy_body: self.body,
            content_revealed: true,
            addenda,
        }
    }
}

#[derive(sqlx::FromRow)]
struct AddendumRow {
    id: String,
    note_id: String,
    case_id: String,
    audience: String,
    author_user_id: String,
    author: String,
    author_role_snapshot: String,
    reason: String,
    information: String,
    affected_categories: Vec<String>,
    follow_up: String,
    signature_name: String,
    signed_at: String,
}

impl AddendumRow {
    fn into_addendum(self) -> CaseNoteAddendum {
        CaseNoteAddendum {
            id: self.id,
            note_id: self.note_id,
            case_id: self.case_id,
            audience: CaseNoteAudience::from_slug(&self.audience)
                .unwrap_or(CaseNoteAudience::SharedLegacy),
            author_user_id: self.author_user_id,
            author: self.author,
            author_role_snapshot: AccountRole::from_slug(&self.author_role_snapshot)
                .unwrap_or(AccountRole::Volunteer),
            reason: self.reason,
            information: self.information,
            affected_categories: self
                .affected_categories
                .iter()
                .filter_map(|s| AddendumCategory::from_slug(s))
                .collect(),
            follow_up: self.follow_up,
            signature_name: self.signature_name,
            signed_at: self.signed_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct AuditRow {
    id: String,
    note_id: String,
    case_id: String,
    addendum_id: String,
    actor_user_id: String,
    actor: String,
    actor_role_snapshot: String,
    action: String,
    at: String,
    metadata: serde_json::Value,
}

impl AuditRow {
    fn into_entry(self) -> CaseNoteAuditEntry {
        CaseNoteAuditEntry {
            id: self.id,
            note_id: self.note_id,
            case_id: self.case_id,
            addendum_id: self.addendum_id,
            actor_user_id: self.actor_user_id,
            actor: self.actor,
            actor_role_snapshot: AccountRole::from_slug(&self.actor_role_snapshot)
                .unwrap_or(AccountRole::Volunteer),
            action: CaseNoteAuditAction::from_slug(&self.action)
                .unwrap_or(CaseNoteAuditAction::SaveDraft),
            at: self.at,
            metadata: self.metadata.to_string(),
        }
    }
}

#[derive(sqlx::FromRow)]
struct LockedNoteRow {
    case_id: String,
    state: String,
    author_user_id: String,
    audience: String,
}

const DETAIL_SELECT: &str = "SELECT id, case_id, state, audience, author_user_id, author,
    author_role_snapshot, created_at, updated_at, finalized_at, discarded_at,
    signature_name, signature_signed_at, body,
    COALESCE(activity_date::text, '') AS activity_date,
    COALESCE(to_char(start_time, 'HH24:MI'), '') AS start_time,
    COALESCE(to_char(end_time, 'HH24:MI'), '') AS end_time,
    total_minutes, location, delayed_entry_reason, primary_interaction,
    contact_category, contact_direction, completion_outcome, participant_summary,
    service_areas, purpose, client_reported_info, verified_observed_info,
    information_sources, actions_taken, outcome_response, progress_barriers,
    urgency, urgency_details,
    next_steps, next_steps_not_applicable, narrative
 FROM case_notes";

/// The case a note belongs to, if it exists. Used before capability checks so the
/// capability target always comes from storage rather than the caller.
pub async fn case_id(note_id: &str) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT case_id FROM case_notes WHERE id = $1")
        .bind(note_id)
        .fetch_optional(pool())
        .await
}

/// The migrated legacy notes for the old case detail model. This intentionally
/// exposes no structured-note metadata and no new staff-only notes.
pub async fn legacy_notes_for_case(case_id: &str) -> Result<Vec<LegacyCaseNote>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT id, author, body, created_at
         FROM case_notes
         WHERE case_id = $1 AND state = 'legacy' AND audience = 'shared_legacy'
         ORDER BY seq DESC
         LIMIT 10",
    )
    .bind(case_id)
    .fetch_all(pool())
    .await?;

    let mut notes = Vec::with_capacity(rows.len());
    for (id, author, body, created_at) in rows.into_iter().rev() {
        let addenda = sqlx::query_as::<_, (String, String, String, String, String)>(
            "SELECT author, reason, information, follow_up, signed_at
             FROM case_note_addenda
             WHERE note_id = $1 AND audience = 'shared_legacy'
             ORDER BY seq ASC",
        )
        .bind(&id)
        .fetch_all(pool())
        .await?
        .into_iter()
        .map(
            |(author, reason, information, follow_up, signed_at)| LegacyCaseNoteAddendum {
                author,
                reason,
                information,
                follow_up,
                signed_at,
            },
        )
        .collect();
        notes.push(LegacyCaseNote {
            id,
            author,
            body,
            created_at,
            addenda,
        });
    }
    Ok(notes)
}

/// Server-side filtered, paginated note listing for one selected case.
pub async fn page(
    case_id: &str,
    filters: &CaseNoteListFilters,
    offset: i64,
    limit: i64,
    viewer: &User,
) -> Result<Page<CaseNoteListItem>, sqlx::Error> {
    let limit = limit.clamp(1, 100);
    let offset = offset.max(0);
    let is_admin = viewer.role.has_operations_admin_permissions();
    let interaction = filters
        .primary_interaction
        .map(|v| v.slug())
        .unwrap_or_default();
    let state = filters.state.map(|v| v.slug()).unwrap_or_default();
    let urgency = filters.urgency.map(|v| v.slug()).unwrap_or_default();

    const WHERE: &str = "n.case_id = $1
        AND (n.state IN ('finalized', 'legacy')
             OR n.author_user_id = $2
             OR ($3 AND n.state IN ('draft', 'discarded')))
        AND ($4 = '' OR COALESCE(n.activity_date, NULLIF(left(n.created_at, 10), '')::date) >= NULLIF($4, '')::date)
        AND ($5 = '' OR COALESCE(n.activity_date, NULLIF(left(n.created_at, 10), '')::date) <= NULLIF($5, '')::date)
        AND ($6 = '' OR n.author ILIKE '%' || $6 || '%')
        AND ($7 = '' OR n.primary_interaction = $7)
        AND ($8 = '' OR n.state = $8)
        AND ($9 = '' OR n.urgency = $9)
        AND ($10 = '' OR n.author ILIKE '%' || $10 || '%'
             OR to_tsvector('simple', concat_ws(' ', n.body, n.narrative, n.purpose,
                    n.client_reported_info, n.verified_observed_info, n.actions_taken,
                    n.outcome_response, n.progress_barriers, n.urgency_details, n.next_steps,
                    n.participant_summary, n.location)) @@ plainto_tsquery('simple', $10))";

    let count_sql = format!("SELECT count(*) FROM case_notes n WHERE {WHERE}");
    let total: i64 = sqlx::query_scalar(&count_sql)
        .bind(case_id)
        .bind(&viewer.id)
        .bind(is_admin)
        .bind(&filters.start_date)
        .bind(&filters.end_date)
        .bind(&filters.author)
        .bind(interaction)
        .bind(state)
        .bind(urgency)
        .bind(&filters.keyword)
        .fetch_one(pool())
        .await?;

    let rows_sql = format!(
        "SELECT n.id, n.case_id, n.state, n.audience, n.author_user_id, n.author,
                n.author_role_snapshot, n.created_at, n.updated_at, n.finalized_at,
                n.discarded_at,
                COALESCE(n.activity_date::text, left(n.created_at, 10), '') AS activity_date,
                n.total_minutes, n.primary_interaction, n.urgency,
                (SELECT count(*) FROM case_note_addenda a WHERE a.note_id = n.id) AS addendum_count
         FROM case_notes n
         WHERE {WHERE}
         ORDER BY COALESCE(n.activity_date, NULLIF(left(n.created_at, 10), '')::date) DESC NULLS LAST,
                  n.seq DESC
         LIMIT $11 OFFSET $12"
    );
    let rows = sqlx::query_as::<_, CaseNoteListRow>(&rows_sql)
        .bind(case_id)
        .bind(&viewer.id)
        .bind(is_admin)
        .bind(&filters.start_date)
        .bind(&filters.end_date)
        .bind(&filters.author)
        .bind(interaction)
        .bind(state)
        .bind(urgency)
        .bind(&filters.keyword)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool())
        .await?;

    Ok(Page {
        total,
        items: rows.into_iter().map(CaseNoteListRow::into_item).collect(),
    })
}

/// A note detail if it is visible without administrative draft inspection.
pub async fn get_visible(
    note_id: &str,
    viewer: &User,
    include_admin_drafts: bool,
) -> Result<Option<CaseNoteDetail>, sqlx::Error> {
    let Some(detail) = detail(note_id).await? else {
        return Ok(None);
    };
    let own = detail.author_user_id == viewer.id;
    let admin_shell = include_admin_drafts
        && viewer.role.has_operations_admin_permissions()
        && !own
        && detail.state == CaseNoteState::Draft;
    let visible = matches!(
        detail.state,
        CaseNoteState::Finalized | CaseNoteState::Legacy
    ) || own
        || admin_shell;
    if !visible {
        return Ok(None);
    }
    if admin_shell {
        return Ok(Some(CaseNoteDetail {
            draft: CaseNoteDraftInput::default(),
            legacy_body: String::new(),
            addenda: Vec::new(),
            content_revealed: false,
            ..detail
        }));
    }
    Ok(Some(detail))
}

pub async fn create_draft(
    case_id: &str,
    author: &User,
    draft: &CaseNoteDraftInput,
) -> Result<CaseNoteDetail, sqlx::Error> {
    let mut tx = pool().begin().await?;
    lock_writable_case(&mut tx, case_id).await?;
    let note_id = ids::next(&mut *tx, "n").await?;
    let now = now_stamp();
    write_insert(&mut tx, &note_id, case_id, author, draft, &now).await?;
    record_audit(
        &mut tx,
        &note_id,
        case_id,
        "",
        author,
        CaseNoteAuditAction::CreateDraft,
        json!({ "state": CaseNoteState::Draft.slug() }).to_string(),
    )
    .await?;
    tx.commit().await?;
    detail(&note_id)
        .await?
        .ok_or_else(|| domain_error("Created note was not found."))
}

// REQ-CN-001..011: direct finalization is one official, audited transaction.
pub async fn create_finalized(
    case_id: &str,
    author: &User,
    finalization: &ValidatedCaseNoteFinalization,
) -> Result<CaseNoteDetail, sqlx::Error> {
    let mut tx = pool().begin().await?;
    lock_writable_case(&mut tx, case_id).await?;
    let note_id = ids::next(&mut *tx, "n").await?;
    let now = now_stamp();
    write_insert(
        &mut tx,
        &note_id,
        case_id,
        author,
        &finalization.input,
        &now,
    )
    .await?;
    finalize_in(&mut tx, &note_id, case_id, author, finalization, &now).await?;
    tx.commit().await?;
    detail(&note_id)
        .await?
        .ok_or_else(|| domain_error("Finalized note was not found."))
}

pub async fn save_draft(
    note_id: &str,
    author: &User,
    draft: &CaseNoteDraftInput,
) -> Result<CaseNoteDetail, sqlx::Error> {
    let mut tx = pool().begin().await?;
    let locked = lock_note(&mut tx, note_id).await?;
    require_author_draft(&locked, author)?;
    lock_writable_case(&mut tx, &locked.case_id).await?;
    let now = now_stamp();
    write_update(&mut tx, note_id, draft, None, &now).await?;
    record_audit(
        &mut tx,
        note_id,
        &locked.case_id,
        "",
        author,
        CaseNoteAuditAction::SaveDraft,
        json!({ "state": CaseNoteState::Draft.slug() }).to_string(),
    )
    .await?;
    tx.commit().await?;
    detail(note_id)
        .await?
        .ok_or_else(|| domain_error("Saved note was not found."))
}

pub async fn finalize_draft(
    note_id: &str,
    author: &User,
    finalization: &ValidatedCaseNoteFinalization,
) -> Result<CaseNoteDetail, sqlx::Error> {
    let mut tx = pool().begin().await?;
    let locked = lock_note(&mut tx, note_id).await?;
    require_author_draft(&locked, author)?;
    lock_writable_case(&mut tx, &locked.case_id).await?;
    let now = now_stamp();
    finalize_in(
        &mut tx,
        note_id,
        &locked.case_id,
        author,
        finalization,
        &now,
    )
    .await?;
    tx.commit().await?;
    detail(note_id)
        .await?
        .ok_or_else(|| domain_error("Finalized note was not found."))
}

pub async fn discard_draft(note_id: &str, author: &User) -> Result<(), sqlx::Error> {
    let mut tx = pool().begin().await?;
    let locked = lock_note(&mut tx, note_id).await?;
    require_author_draft(&locked, author)?;
    lock_writable_case(&mut tx, &locked.case_id).await?;
    let now = now_stamp();
    sqlx::query(
        "UPDATE case_notes
         SET state = 'discarded', updated_at = $2, updated_at_utc = now(),
             discarded_at = $2, discarded_at_utc = now(), body = '',
             activity_date = NULL, start_time = NULL, end_time = NULL,
             total_minutes = NULL, location = '', delayed_entry_reason = '',
             primary_interaction = '', contact_category = '', contact_direction = '',
             completion_outcome = '', participant_summary = '', service_areas = '{}',
             purpose = '', client_reported_info = '', verified_observed_info = '',
             information_sources = '{}', actions_taken = '', outcome_response = '',
             progress_barriers = '', urgency = '', urgency_details = '', next_steps = '',
             next_steps_not_applicable = false, narrative = ''
         WHERE id = $1",
    )
    .bind(note_id)
    .bind(&now)
    .execute(&mut *tx)
    .await?;
    record_audit(
        &mut tx,
        note_id,
        &locked.case_id,
        "",
        author,
        CaseNoteAuditAction::DiscardDraft,
        json!({ "state": CaseNoteState::Discarded.slug() }).to_string(),
    )
    .await?;
    tx.commit().await
}

pub async fn admin_inspect_draft(
    note_id: &str,
    admin: &User,
) -> Result<CaseNoteDetail, sqlx::Error> {
    let mut tx = pool().begin().await?;
    let locked = lock_note(&mut tx, note_id).await?;
    if CaseNoteState::from_slug(&locked.state) != Some(CaseNoteState::Draft) {
        return Err(domain_error(
            "Only drafts can be administratively inspected.",
        ));
    }
    if !admin.role.has_operations_admin_permissions() {
        return Err(domain_error("Operations-admin permissions required."));
    }
    record_audit(
        &mut tx,
        note_id,
        &locked.case_id,
        "",
        admin,
        CaseNoteAuditAction::AdminInspectDraft,
        json!({ "state": CaseNoteState::Draft.slug() }).to_string(),
    )
    .await?;
    tx.commit().await?;
    detail(note_id)
        .await?
        .ok_or_else(|| domain_error("Inspected note was not found."))
}

pub async fn add_addendum(
    note_id: &str,
    author: &User,
    input: &CaseNoteAddendumInput,
) -> Result<CaseNoteAddendum, sqlx::Error> {
    let mut tx = pool().begin().await?;
    let locked = lock_note(&mut tx, note_id).await?;
    let state = CaseNoteState::from_slug(&locked.state).unwrap_or(CaseNoteState::Draft);
    lock_writable_case(&mut tx, &locked.case_id).await?;
    if !matches!(state, CaseNoteState::Finalized | CaseNoteState::Legacy) {
        return Err(domain_error(
            "Addenda can be attached only to finalized or legacy notes.",
        ));
    }
    let id = ids::next(&mut *tx, "na").await?;
    let signed_at = now_stamp();
    let affected = addendum_category_slugs(&input.affected_categories);
    sqlx::query(
        "INSERT INTO case_note_addenda
            (id, note_id, case_id, audience, author_user_id, author,
             author_role_snapshot, reason, information, affected_categories,
             follow_up, signature_name, signed_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
    )
    .bind(&id)
    .bind(note_id)
    .bind(&locked.case_id)
    .bind(&locked.audience)
    .bind(&author.id)
    .bind(author.full_name())
    .bind(author.role.slug())
    .bind(&input.reason)
    .bind(&input.information)
    .bind(&affected)
    .bind(&input.follow_up)
    .bind(&input.signature_name)
    .bind(&signed_at)
    .execute(&mut *tx)
    .await?;
    record_audit(
        &mut tx,
        note_id,
        &locked.case_id,
        &id,
        author,
        CaseNoteAuditAction::AddAddendum,
        json!({ "state": state.slug(), "addendum_id": id }).to_string(),
    )
    .await?;
    tx.commit().await?;
    addendum(&id)
        .await?
        .ok_or_else(|| domain_error("Created addendum was not found."))
}

pub async fn audit_page(
    case_id: &str,
    note_id: &str,
    offset: i64,
    limit: i64,
) -> Result<Page<CaseNoteAuditEntry>, sqlx::Error> {
    let limit = limit.clamp(1, 100);
    let offset = offset.max(0);
    let total: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM case_note_audit_log
         WHERE case_id = $1 AND ($2 = '' OR note_id = $2)",
    )
    .bind(case_id)
    .bind(note_id)
    .fetch_one(pool())
    .await?;
    let rows = sqlx::query_as::<_, AuditRow>(
        "SELECT id, note_id, case_id, addendum_id, actor_user_id, actor,
                actor_role_snapshot, action, at, metadata
         FROM case_note_audit_log
         WHERE case_id = $1 AND ($2 = '' OR note_id = $2)
         ORDER BY seq DESC
         LIMIT $3 OFFSET $4",
    )
    .bind(case_id)
    .bind(note_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool())
    .await?;
    Ok(Page {
        total,
        items: rows.into_iter().map(AuditRow::into_entry).collect(),
    })
}

async fn detail(note_id: &str) -> Result<Option<CaseNoteDetail>, sqlx::Error> {
    let row = sqlx::query_as::<_, CaseNoteDetailRow>(&format!("{DETAIL_SELECT} WHERE id = $1"))
        .bind(note_id)
        .fetch_optional(pool())
        .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let addenda = addenda_for_note(note_id).await?;
    Ok(Some(row.into_detail(addenda)))
}

async fn addenda_for_note(note_id: &str) -> Result<Vec<CaseNoteAddendum>, sqlx::Error> {
    let rows = sqlx::query_as::<_, AddendumRow>(
        "SELECT id, note_id, case_id, audience, author_user_id, author,
                author_role_snapshot, reason, information, affected_categories,
                follow_up, signature_name, signed_at
         FROM case_note_addenda
         WHERE note_id = $1
         ORDER BY seq ASC",
    )
    .bind(note_id)
    .fetch_all(pool())
    .await?;
    Ok(rows.into_iter().map(AddendumRow::into_addendum).collect())
}

async fn addendum(id: &str) -> Result<Option<CaseNoteAddendum>, sqlx::Error> {
    let row = sqlx::query_as::<_, AddendumRow>(
        "SELECT id, note_id, case_id, audience, author_user_id, author,
                author_role_snapshot, reason, information, affected_categories,
                follow_up, signature_name, signed_at
         FROM case_note_addenda
         WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool())
    .await?;
    Ok(row.map(AddendumRow::into_addendum))
}

async fn lock_writable_case(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    case_id: &str,
) -> Result<(), sqlx::Error> {
    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM cases WHERE id = $1 FOR UPDATE")
            .bind(case_id)
            .fetch_optional(&mut **tx)
            .await?;
    match status.and_then(|value| crate::server_fns::cases::CaseStatus::from_slug(&value)) {
        Some(status) if status.accepts_changes() => Ok(()),
        Some(_) => Err(domain_error("This case no longer accepts changes.")),
        None => Err(domain_error("Case not found.")),
    }
}

async fn lock_note(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    note_id: &str,
) -> Result<LockedNoteRow, sqlx::Error> {
    sqlx::query_as::<_, LockedNoteRow>(
        "SELECT case_id, state, author_user_id, audience
         FROM case_notes
         WHERE id = $1
         FOR UPDATE",
    )
    .bind(note_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| domain_error("Note not found."))
}

fn require_author_draft(locked: &LockedNoteRow, author: &User) -> Result<(), sqlx::Error> {
    if CaseNoteState::from_slug(&locked.state) != Some(CaseNoteState::Draft) {
        return Err(domain_error("Only drafts can be changed."));
    }
    if locked.author_user_id != author.id {
        return Err(domain_error("Only the draft author may change this note."));
    }
    Ok(())
}

async fn finalize_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    note_id: &str,
    case_id: &str,
    author: &User,
    finalization: &ValidatedCaseNoteFinalization,
    now: &str,
) -> Result<(), sqlx::Error> {
    write_update(
        tx,
        note_id,
        &finalization.input,
        Some(finalization.total_minutes),
        now,
    )
    .await?;
    sqlx::query(
        "UPDATE case_notes
         SET state = 'finalized', finalized_at = $2, finalized_at_utc = now(),
             signature_name = $3, accuracy_confirmed = $4,
             signature_signed_at = $2, signature_signed_at_utc = now(),
             updated_at = $2, updated_at_utc = now()
         WHERE id = $1",
    )
    .bind(note_id)
    .bind(now)
    .bind(&finalization.signature_name)
    .bind(finalization.accuracy_confirmed)
    .execute(&mut **tx)
    .await?;
    record_audit(
        tx,
        note_id,
        case_id,
        "",
        author,
        CaseNoteAuditAction::FinalizeNote,
        json!({ "state": CaseNoteState::Finalized.slug() }).to_string(),
    )
    .await
}

async fn write_insert(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    note_id: &str,
    case_id: &str,
    author: &User,
    draft: &CaseNoteDraftInput,
    now: &str,
) -> Result<(), sqlx::Error> {
    let total_minutes = draft.calculated_total_minutes();
    let service_areas = service_area_slugs(&draft.service_areas);
    let information_sources = information_source_slugs(&draft.information_sources);
    sqlx::query(
        "INSERT INTO case_notes
            (id, case_id, author, body, created_at, state, audience, author_user_id,
             author_role_snapshot, updated_at, activity_date, start_time, end_time,
             total_minutes, location, delayed_entry_reason, primary_interaction,
             contact_category, contact_direction, completion_outcome, participant_summary,
             service_areas, purpose, client_reported_info, verified_observed_info,
             information_sources, actions_taken, outcome_response, progress_barriers, urgency,
             urgency_details, next_steps, next_steps_not_applicable, narrative)
         VALUES ($1, $2, $3, '', $4, 'draft', 'volunteer_only', $5, $6, $4,
             NULLIF($7, '')::date, NULLIF($8, '')::time, NULLIF($9, '')::time,
             $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21,
             $22, $23, $24, $25, $26, $27, $28, $29, $30)",
    )
    .bind(note_id)
    .bind(case_id)
    .bind(author.full_name())
    .bind(now)
    .bind(&author.id)
    .bind(author.role.slug())
    .bind(&draft.activity_date)
    .bind(&draft.start_time)
    .bind(&draft.end_time)
    .bind(total_minutes)
    .bind(&draft.location)
    .bind(&draft.delayed_entry_reason)
    .bind(
        draft
            .primary_interaction
            .map(|v| v.slug())
            .unwrap_or_default(),
    )
    .bind(draft.contact_category.map(|v| v.slug()).unwrap_or_default())
    .bind(
        draft
            .contact_direction
            .map(|v| v.slug())
            .unwrap_or_default(),
    )
    .bind(
        draft
            .completion_outcome
            .map(|v| v.slug())
            .unwrap_or_default(),
    )
    .bind(&draft.participant_summary)
    .bind(&service_areas)
    .bind(&draft.purpose)
    .bind(&draft.client_reported_info)
    .bind(&draft.verified_observed_info)
    .bind(&information_sources)
    .bind(&draft.actions_taken)
    .bind(&draft.outcome_response)
    .bind(&draft.progress_barriers)
    .bind(draft.urgency.map(|v| v.slug()).unwrap_or_default())
    .bind(&draft.urgency_details)
    .bind(&draft.next_steps)
    .bind(draft.next_steps_not_applicable)
    .bind(&draft.narrative)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn write_update(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    note_id: &str,
    draft: &CaseNoteDraftInput,
    forced_total_minutes: Option<i32>,
    now: &str,
) -> Result<(), sqlx::Error> {
    let total_minutes = forced_total_minutes.or_else(|| draft.calculated_total_minutes());
    let service_areas = service_area_slugs(&draft.service_areas);
    let information_sources = information_source_slugs(&draft.information_sources);
    sqlx::query(
        "UPDATE case_notes
         SET updated_at = $2, updated_at_utc = now(),
             activity_date = NULLIF($3, '')::date,
             start_time = NULLIF($4, '')::time,
             end_time = NULLIF($5, '')::time,
             total_minutes = $6,
             location = $7,
             delayed_entry_reason = $8,
             primary_interaction = $9,
             contact_category = $10,
             contact_direction = $11,
             completion_outcome = $12,
             participant_summary = $13,
             service_areas = $14,
             purpose = $15,
             client_reported_info = $16,
             verified_observed_info = $17,
             information_sources = $18,
             actions_taken = $19,
             outcome_response = $20,
             progress_barriers = $21,
             urgency = $22,
             urgency_details = $23,
             next_steps = $24,
             next_steps_not_applicable = $25,
             narrative = $26
         WHERE id = $1",
    )
    .bind(note_id)
    .bind(now)
    .bind(&draft.activity_date)
    .bind(&draft.start_time)
    .bind(&draft.end_time)
    .bind(total_minutes)
    .bind(&draft.location)
    .bind(&draft.delayed_entry_reason)
    .bind(
        draft
            .primary_interaction
            .map(|v| v.slug())
            .unwrap_or_default(),
    )
    .bind(draft.contact_category.map(|v| v.slug()).unwrap_or_default())
    .bind(
        draft
            .contact_direction
            .map(|v| v.slug())
            .unwrap_or_default(),
    )
    .bind(
        draft
            .completion_outcome
            .map(|v| v.slug())
            .unwrap_or_default(),
    )
    .bind(&draft.participant_summary)
    .bind(&service_areas)
    .bind(&draft.purpose)
    .bind(&draft.client_reported_info)
    .bind(&draft.verified_observed_info)
    .bind(&information_sources)
    .bind(&draft.actions_taken)
    .bind(&draft.outcome_response)
    .bind(&draft.progress_barriers)
    .bind(draft.urgency.map(|v| v.slug()).unwrap_or_default())
    .bind(&draft.urgency_details)
    .bind(&draft.next_steps)
    .bind(draft.next_steps_not_applicable)
    .bind(&draft.narrative)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn record_audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    note_id: &str,
    case_id: &str,
    addendum_id: &str,
    actor: &User,
    action: CaseNoteAuditAction,
    metadata: String,
) -> Result<(), sqlx::Error> {
    let id = ids::next(&mut **tx, "cna").await?;
    sqlx::query(
        "INSERT INTO case_note_audit_log
            (id, note_id, case_id, addendum_id, actor_user_id, actor,
             actor_role_snapshot, action, at, metadata)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::jsonb)",
    )
    .bind(&id)
    .bind(note_id)
    .bind(case_id)
    .bind(addendum_id)
    .bind(&actor.id)
    .bind(actor.full_name())
    .bind(actor.role.slug())
    .bind(action.slug())
    .bind(now_stamp())
    .bind(metadata)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn service_area_slugs(values: &[ServiceArea]) -> Vec<String> {
    values.iter().map(|v| v.slug().to_string()).collect()
}

fn information_source_slugs(values: &[InformationSource]) -> Vec<String> {
    values.iter().map(|v| v.slug().to_string()).collect()
}

fn addendum_category_slugs(values: &[AddendumCategory]) -> Vec<String> {
    values.iter().map(|v| v.slug().to_string()).collect()
}

fn domain_error(message: &str) -> sqlx::Error {
    sqlx::Error::Protocol(message.to_string())
}
