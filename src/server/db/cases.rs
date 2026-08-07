//! Cases, sub properties of evidence and case_properties are their own files

use crate::server::db::{
    audit, capabilities, case_folders, case_properties, channels, evidence, ids, now_stamp, pool,
    terms_acceptances, users,
};
use crate::server_fns::capabilities::CaseCapability;
use crate::server_fns::case_properties::CaseProperty;
use crate::server_fns::cases::{Case, CaseNote, CaseReviewState, CaseStatus, CaseSummary};
use crate::server_fns::channels::ChannelKind;
use crate::server_fns::pagination::Page;
use crate::server_fns::users::AccountRole;

#[derive(sqlx::FromRow)]
struct CaseRow {
    id: String,
    name: String,
    status: String,
    review_state: String,
    review_reason: String,
    owner_id: String,
}

#[derive(sqlx::FromRow)]
struct NoteRow {
    id: String,
    author: String,
    body: String,
    created_at: String,
}

/// Flat row shape for the sparse [`CaseSummary`] projection (header fields plus
/// resolved owner name and message count). Shared by every summary query.
#[derive(sqlx::FromRow)]
struct SummaryRow {
    id: String,
    name: String,
    status: String,
    review_state: String,
    review_reason: String,
    owner_id: String,
    owner_first_name: String,
    owner_last_name: String,
    message_count: i64,
    last_activity: Option<String>,
}

impl SummaryRow {
    fn into_summary(
        self,
        capabilities: Vec<CaseCapability>,
        assigned_volunteers: Vec<String>,
        threshold: &str,
    ) -> CaseSummary {
        let inactive = self
            .last_activity
            .as_deref()
            .is_some_and(|ts| ts < threshold);
        CaseSummary {
            id: self.id,
            name: self.name,
            status: CaseStatus::from_slug(&self.status).unwrap_or(CaseStatus::Open),
            review_state: CaseReviewState::from_slug(&self.review_state)
                .unwrap_or(CaseReviewState::Accepted),
            review_reason: self.review_reason,
            assigned_volunteers,
            owner_id: self.owner_id,
            owner_first_name: self.owner_first_name,
            owner_last_name: self.owner_last_name,
            message_count: self.message_count.max(0) as usize,
            inactive,
            capabilities,
        }
    }
}

/// The `YYYY-MM-DD HH:MM` local-time cutoff (30 days ago) for the case-chat
/// inactivity badge, matching the format [`now_stamp`] writes for `sent_at`.
fn inactivity_threshold() -> String {
    (chrono::Local::now() - chrono::Duration::days(30))
        .format("%Y-%m-%d %H:%M")
        .to_string()
}

/// The `SELECT` list that projects a `cases` row (aliased `c`) into a
/// [`SummaryRow`]: header fields, the owner's first/last name, and the case's
/// message count. Callers append their own `WHERE`/`ORDER`/`LIMIT`.
///
/// `message_count_scope` is an SQL boolean spliced into the count subquery that
/// decides whether the volunteer-only channel's messages are included. Passing
/// `"true"` counts everything (site-admin-only lookups that never surface to a
/// client); a viewer-scoped query passes a role test so a client's case list
/// does not even leak *how many* private staff messages exist. It is always an
/// internal constant or a bind-parameter reference, never user input.
fn summary_select(message_count_scope: &str) -> String {
    format!(
        "SELECT c.id, c.name, c.status, c.review_state, c.review_reason, c.owner_id,
        u.first_name AS owner_first_name,
        u.last_name AS owner_last_name,
        (SELECT COUNT(*) FROM messages m
          JOIN case_channels ch ON ch.id = m.channel_id
          WHERE m.case_id = c.id
            AND (({message_count_scope}) OR ch.kind <> '{restricted}')) AS message_count,
        (SELECT MAX(m.sent_at) FROM messages m
          JOIN case_channels ch ON ch.id = m.channel_id
          WHERE m.case_id = c.id
            AND (({message_count_scope}) OR ch.kind <> '{restricted}')) AS last_activity
 FROM cases c
 LEFT JOIN users u ON u.id = c.owner_id",
        restricted = ChannelKind::VolunteerOnly.slug()
    )
}

/// Display names of the staff assigned to work each of `case_ids`, keyed by case
/// id. "Assigned to work it" means holding [`CaseCapability::EditCase`] while
/// *not* being a client account: the signup flow gives the client owner every
/// capability on their own case, so a plain capability check would report every
/// unstaffed intake as staffed by the person asking for help.
///
/// One batched query for a whole page. Cases with nobody assigned are simply
/// absent from the map, which is what makes them "unstaffed".
async fn assigned_by_case(
    case_ids: &[String],
) -> Result<std::collections::HashMap<String, Vec<String>>, sqlx::Error> {
    use std::collections::HashMap;

    if case_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<(String, String, String)> = sqlx::query_as(&format!(
        "SELECT a.case_id, u.first_name, u.last_name
           FROM case_assignments a
           JOIN users u ON u.id = a.user_id
          WHERE a.case_id = ANY($1)
            AND a.capability = '{edit}'
            AND u.role <> '{client}'
          ORDER BY a.case_id, u.first_name, u.last_name",
        edit = CaseCapability::EditCase.slug(),
        client = AccountRole::Client.slug(),
    ))
    .bind(case_ids)
    .fetch_all(pool())
    .await?;

    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for (case_id, first, last) in rows {
        let name = format!("{first} {last}").trim().to_string();
        if !name.is_empty() {
            map.entry(case_id).or_default().push(name);
        }
    }
    Ok(map)
}

/// The owner id of a case, if it exists (cheap authorization lookup).
pub async fn owner_id(case_id: &str) -> Result<Option<String>, sqlx::Error> {
    let owner: Option<String> = sqlx::query_scalar("SELECT owner_id FROM cases WHERE id = $1")
        .bind(case_id)
        .fetch_optional(pool())
        .await?;
    Ok(owner)
}

/// The display name of a case, if it exists (cheap lookup for notifications).
pub async fn name(case_id: &str) -> Result<Option<String>, sqlx::Error> {
    let name: Option<String> = sqlx::query_scalar("SELECT name FROM cases WHERE id = $1")
        .bind(case_id)
        .fetch_optional(pool())
        .await?;
    Ok(name)
}

/// One page of sparse cases visible to a user_id
pub async fn get_summaries_for_user(
    offset: i64,
    limit: i64,
    search: &str,
    user_id: &str,
) -> Result<Page<CaseSummary>, sqlx::Error> {
    let limit = limit.clamp(1, 100);
    let offset = offset.max(0);

    // Escaped `%term%` pattern (or `None` for "no filter"), so user input is
    // matched literally rather than as LIKE metacharacters.
    let term = search.trim();
    let pattern = if term.is_empty() {
        None
    } else {
        Some(format!(
            "%{}%",
            term.replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        ))
    };

    // `$1` is the (nullable) search pattern; the viewer id scopes the results to
    // cases they can actually open — i.e. where they hold the `view_case`
    // capability, matching what [`load_case`] enforces. Its bind position differs
    // between the COUNT query (`$2`) and the page query (`$4`, after LIMIT/OFFSET).
    // The capability slug is an internal constant (never user input), so
    // interpolating it into the SQL is safe.
    const SEARCH: &str = "($1::text IS NULL OR c.id ILIKE $1 OR c.name ILIKE $1)";
    let scope = format!(
        "EXISTS (SELECT 1 FROM case_assignments a \
         WHERE a.case_id = c.id AND a.user_id = $VIEWER AND a.capability = '{}')",
        CaseCapability::ViewCase.slug()
    );

    let count_sql = format!(
        "SELECT count(*) FROM cases c WHERE {SEARCH} AND {}",
        scope.replace("$VIEWER", "$2")
    );
    // The viewer's own account role decides whether the volunteer-only channel
    // counts toward the message total they see; clients are never told it exists.
    let page_sql = format!(
        "{}
         WHERE {SEARCH} AND {}
         ORDER BY c.id LIMIT $2 OFFSET $3",
        summary_select(&format!(
            "(SELECT v.role FROM users v WHERE v.id = $4) <> '{}'",
            AccountRole::Client.slug()
        )),
        scope.replace("$VIEWER", "$4")
    );

    // The count and the page fetch are independent, so run them concurrently —
    // one fewer sequential round-trip against a networked database.
    let count_fut = sqlx::query_scalar::<_, i64>(&count_sql)
        .bind(&pattern)
        .bind(user_id)
        .fetch_one(pool());
    let rows_fut = sqlx::query_as::<_, SummaryRow>(&page_sql)
        .bind(&pattern)
        .bind(limit)
        .bind(offset)
        .bind(user_id)
        .fetch_all(pool());
    let (total, rows) = tokio::try_join!(count_fut, rows_fut)?;

    // Resolve the requesting user's capabilities on this page of cases in one
    // batched query, then build each summary complete with its rights attached —
    // so the client can gate actions straight from the case data.
    let ids: Vec<String> = rows.iter().map(|r| r.id.clone()).collect();
    let mut capabilities_by_case = if ids.is_empty() {
        Default::default()
    } else {
        capabilities::get_multi_case(user_id, &ids).await?
    };
    let threshold = inactivity_threshold();
    let mut assigned = assigned_by_case(&ids).await?;
    let items = rows
        .into_iter()
        .map(|r| {
            let caps = capabilities_by_case.remove(&r.id).unwrap_or_default();
            let names = assigned.remove(&r.id).unwrap_or_default();
            r.into_summary(caps, names, &threshold)
        })
        .collect::<Vec<_>>();
    Ok(Page { items, total })
}

/// Up to `limit` lightweight cases whose id or name matches `search`
/// (case-insensitive, metacharacters escaped), ordered by id. Unscoped — used by
/// the admin capability tool to find any case to grant access to. Empty search
/// returns the first `limit` cases.
pub async fn search_lite(search: &str, limit: i64) -> Result<Vec<CaseSummary>, sqlx::Error> {
    let limit = limit.clamp(1, 50);
    let term = search.trim();
    let pattern = if term.is_empty() {
        None
    } else {
        Some(format!(
            "%{}%",
            term.replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        ))
    };
    let rows = sqlx::query_as::<_, SummaryRow>(&format!(
        "{}
         WHERE $1::text IS NULL OR c.id ILIKE $1 OR c.name ILIKE $1
         ORDER BY c.id LIMIT $2",
        summary_select("true")
    ))
    .bind(&pattern)
    .bind(limit)
    .fetch_all(pool())
    .await?;
    let threshold = inactivity_threshold();
    let ids: Vec<String> = rows.iter().map(|r| r.id.clone()).collect();
    let mut assigned = assigned_by_case(&ids).await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let names = assigned.remove(&r.id).unwrap_or_default();
            r.into_summary(Vec::new(), names, &threshold)
        })
        .collect())
}

/// The cases awaiting an accept/decline decision, oldest id first. Callers are
/// responsible for the operations-admin gate.
///
/// Unscoped by assignment on purpose: a case waiting for review has nobody
/// assigned to it yet, so scoping by assignment would hide every row this list
/// exists to show.
pub async fn pending_review_cases() -> Result<Vec<CaseSummary>, sqlx::Error> {
    let rows = sqlx::query_as::<_, SummaryRow>(&format!(
        "{} WHERE c.review_state = $1 ORDER BY c.id",
        // Admin-only listing, so count every message including the
        // volunteer-only channel, matching the other admin lookups.
        summary_select("true")
    ))
    .bind(CaseReviewState::PendingReview.slug())
    .fetch_all(pool())
    .await?;
    let threshold = inactivity_threshold();
    let ids: Vec<String> = rows.iter().map(|r| r.id.clone()).collect();
    let mut assigned = assigned_by_case(&ids).await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let names = assigned.remove(&r.id).unwrap_or_default();
            r.into_summary(Vec::new(), names, &threshold)
        })
        .collect())
}

/// How many cases are waiting for an admin decision.
pub async fn pending_review_count() -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT count(*) FROM cases WHERE review_state = $1")
        .bind(CaseReviewState::PendingReview.slug())
        .fetch_one(pool())
        .await
}

/// Lightweight [`CaseSummary`]s for a specific set of ids, ordered by id. Used to
/// resolve case names for display without loading every case. Empty input
/// returns an empty vec (no query).
pub async fn get_summaries_by_ids(ids: &[String]) -> Result<Vec<CaseSummary>, sqlx::Error> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query_as::<_, SummaryRow>(&format!(
        "{} WHERE c.id = ANY($1) ORDER BY c.id",
        summary_select("true")
    ))
    .bind(ids)
    .fetch_all(pool())
    .await?;
    let threshold = inactivity_threshold();
    let mut assigned = assigned_by_case(ids).await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let names = assigned.remove(&r.id).unwrap_or_default();
            r.into_summary(Vec::new(), names, &threshold)
        })
        .collect())
}

/// A single case by id, fully hydrated (properties, evidence, and its newest
/// notes) including the capabilities `user_id` holds on it, or `None` if no
/// such case exists.
pub async fn get(
    id: &str,
    user_id: &str,
    has_volunteer_access: bool,
) -> Result<Option<Case>, sqlx::Error> {
    let Some(row) = sqlx::query_as::<_, CaseRow>(
        "SELECT id, name, status, review_state, review_reason, owner_id FROM cases WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool())
    .await?
    else {
        return Ok(None);
    };

    // Newest 10 notes: fetched newest-first (so only the 10 most recent are
    // kept), then reversed to oldest-first so they render chronologically with
    // the most recent last.
    let note_rows = sqlx::query_as::<_, NoteRow>(
        "SELECT id, author, body, created_at FROM case_notes
         WHERE case_id = $1 ORDER BY seq DESC LIMIT 10",
    )
    .bind(id)
    .fetch_all(pool())
    .await?;
    let notes = note_rows
        .into_iter()
        .rev()
        .map(|r| CaseNote {
            id: r.id,
            author: r.author,
            body: r.body,
            created_at: r.created_at,
        })
        .collect();

    let evidence = evidence::get_case_evidence(id, has_volunteer_access).await?;
    let folders = case_folders::list(id, has_volunteer_access).await?;
    let properties = case_properties::get_case_properties(id, has_volunteer_access).await?;
    let terms_accepted = terms_acceptances::for_case(id).await?.map(|acceptance| {
        format!(
            "{} (version {})",
            acceptance
                .accepted_at
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M"),
            acceptance.terms_version
        )
    });
    let assigned_volunteers = assigned_by_case(&[id.to_string()])
        .await?
        .remove(id)
        .unwrap_or_default();

    Ok(Some(Case {
        id: row.id,
        name: row.name,
        status: CaseStatus::from_slug(&row.status).unwrap_or(CaseStatus::Open),
        review_state: CaseReviewState::from_slug(&row.review_state)
            .unwrap_or(CaseReviewState::Accepted),
        review_reason: row.review_reason,
        assigned_volunteers,
        owner_id: row.owner_id,
        notes,
        evidence,
        folders,
        properties,
        message_count: 0,
        capabilities: capabilities::get_single_case(user_id, id).await?,
        terms_accepted,
    }))
}

/// Create a case owned by `owner_id` with an initial status, cleaned property
/// set, and an optional first note. Grants the owner full capabilities. Returns
/// the new case id.
pub async fn create(
    owner_id: &str,
    owner_name: &str,
    name: &str,
    status: CaseStatus,
    review_state: CaseReviewState,
    initial_properties: Vec<CaseProperty>,
    first_note: Option<String>,
) -> Result<String, sqlx::Error> {
    let id = ids::next(pool(), "c").await?;

    let mut tx = pool().begin().await?;
    sqlx::query(
        "INSERT INTO cases (id, name, status, review_state, owner_id) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(&id)
    .bind(name)
    .bind(status.slug())
    .bind(review_state.slug())
    .bind(owner_id)
    .execute(&mut *tx)
    .await?;

    // Every case starts with its permanent volunteer-only back-channel and a
    // "General" channel, created in the same transaction so a case can never
    // exist without somewhere to chat.
    channels::create_defaults(&mut *tx, &id).await?;

    // Likewise its standing folder tree, which everything filed on the case
    // hangs from.
    case_folders::create_for_new_case(&mut tx, &id).await?;

    // The fields every case starts with come first, so the standing paperwork
    // sits above whatever the creator typed in.
    case_properties::add_for_new_case(&mut tx, &id, case_properties::clean(initial_properties))
        .await?;

    if let Some(body) = first_note {
        let body = body.trim().to_string();
        if !body.is_empty() {
            let note_id = ids::next(&mut *tx, "n").await?;
            sqlx::query(
                "INSERT INTO case_notes (id, case_id, author, body, created_at)
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(&note_id)
            .bind(&id)
            .bind(owner_name)
            .bind(&body)
            .bind(now_stamp())
            .execute(&mut *tx)
            .await?;
        }
    }
    users::assign_capabilities_in(&mut tx, owner_id, &id, &CaseCapability::ALL, owner_name).await?;
    tx.commit().await?;
    Ok(id)
}

/// Create the case a verified public signup asked for, inside the same
/// transaction that creates the account.
pub async fn create_from_signup_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    case_id: &str,
    owner_id: &str,
    owner_name: &str,
    name: &str,
    initial_properties: Vec<CaseProperty>,
    terms_version: &str,
) -> Result<(), sqlx::Error> {
    // A case that arrives through public signup has had no staff involvement at
    // all, so it starts life awaiting a decision rather than quietly counting as
    // accepted work.
    sqlx::query(
        "INSERT INTO cases (id, name, status, review_state, owner_id) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(case_id)
    .bind(name)
    .bind(CaseStatus::Open.slug())
    .bind(CaseReviewState::PendingReview.slug())
    .bind(owner_id)
    .execute(&mut **tx)
    .await?;
    channels::create_defaults(&mut **tx, case_id).await?;
    case_folders::create_for_new_case(tx, case_id).await?;
    case_properties::add_for_new_case(tx, case_id, case_properties::clean(initial_properties))
        .await?;
    terms_acceptances::insert_in(tx, owner_id, Some(case_id), terms_version).await?;
    users::assign_capabilities_in(tx, owner_id, case_id, &CaseCapability::ALL, owner_name).await?;
    Ok(())
}

/// Update a single `cases` column and record an audit entry — but only when the
/// case exists and the value actually changes. `column` and `field` are always
/// internal string constants (never caller/user input), so interpolating the
/// column name into the SQL is safe. `old_display`/`new_display` are the
/// human-readable values written to the audit log, which may differ from the
/// stored value (e.g. owner ids stored, owner names audited).
async fn update_field(
    case_id: &str,
    column: &str,
    new_stored: &str,
    current_stored: &str,
    actor: &str,
    field: &str,
    old_display: &str,
    new_display: &str,
) -> Result<(), sqlx::Error> {
    if current_stored == new_stored {
        return Ok(());
    }
    sqlx::query(&format!("UPDATE cases SET {column} = $1 WHERE id = $2"))
        .bind(new_stored)
        .bind(case_id)
        .execute(pool())
        .await?;
    audit::record(
        pool(),
        audit::Entity::Case,
        case_id,
        actor,
        field,
        old_display,
        new_display,
    )
    .await
}

/// Current stored value of a single `cases` column, or `None` if the case is
/// missing. `column` is always an internal constant, never user input.
async fn current_field(case_id: &str, column: &str) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar(&format!("SELECT {column} FROM cases WHERE id = $1"))
        .bind(case_id)
        .fetch_optional(pool())
        .await
}

/// Update a case's status, auditing the change.
pub async fn set_status(case_id: &str, status: CaseStatus, actor: &str) -> Result<(), sqlx::Error> {
    let Some(current) = current_field(case_id, "status").await? else {
        return Ok(());
    };
    update_field(
        case_id,
        "status",
        status.slug(),
        &current,
        actor,
        "status",
        &current,
        status.slug(),
    )
    .await
}

/// Record an accept/decline decision on a case, auditing the change. Returns
/// whether anything actually changed, so callers can skip notifying people about
/// a decision that was already in place.
///
/// The reason, decider, and timestamp are written in the same statement as the
/// state: a decline without its reason on record is the failure mode this whole
/// feature exists to prevent.
pub async fn set_review_state(
    case_id: &str,
    state: CaseReviewState,
    reason: &str,
    actor: &str,
) -> Result<bool, sqlx::Error> {
    let Some(current) = current_field(case_id, "review_state").await? else {
        return Ok(false);
    };
    if current == state.slug() {
        return Ok(false);
    }
    sqlx::query(
        "UPDATE cases
            SET review_state = $1, review_reason = $2, reviewed_by = $3, reviewed_at = $4
          WHERE id = $5",
    )
    .bind(state.slug())
    .bind(reason)
    .bind(actor)
    .bind(now_stamp())
    .bind(case_id)
    .execute(pool())
    .await?;
    audit::record(
        pool(),
        audit::Entity::Case,
        case_id,
        actor,
        "review state",
        &current,
        state.slug(),
    )
    .await?;
    Ok(true)
}

/// Rename a case, auditing the change.
pub async fn set_name(case_id: &str, name: &str, actor: &str) -> Result<(), sqlx::Error> {
    let Some(current) = current_field(case_id, "name").await? else {
        return Ok(());
    };
    update_field(
        case_id, "name", name, &current, actor, "name", &current, name,
    )
    .await
}

/// Change a case's owner. `owner_id` must reference an existing user. Audits
/// using display names for readability.
pub async fn set_owner(case_id: &str, owner_id: &str, actor: &str) -> Result<(), sqlx::Error> {
    let Some(current) = current_field(case_id, "owner_id").await? else {
        return Ok(());
    };
    if current == owner_id {
        return Ok(());
    }
    let old_display = user_name(&current).await?;
    let new_display = user_name(owner_id).await?;
    update_field(
        case_id,
        "owner_id",
        owner_id,
        &current,
        actor,
        "owner",
        &old_display,
        &new_display,
    )
    .await
}

/// Display name for a user id, falling back to the id.
async fn user_name(user_id: &str) -> Result<String, sqlx::Error> {
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT first_name, last_name FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(pool())
            .await?;
    Ok(row
        .map(|(f, l)| format!("{f} {l}").trim().to_string())
        .unwrap_or_else(|| user_id.to_string()))
}

/// Append a note to a case.
pub async fn add_note(case_id: &str, author: &str, body: &str) -> Result<(), sqlx::Error> {
    let id = ids::next(pool(), "n").await?;
    sqlx::query(
        "INSERT INTO case_notes (id, case_id, author, body, created_at) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(&id)
    .bind(case_id)
    .bind(author)
    .bind(body)
    .bind(now_stamp())
    .execute(pool())
    .await?;
    Ok(())
}
