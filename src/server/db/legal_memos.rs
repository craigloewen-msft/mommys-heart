use crate::helpers::visibility::Visibility;
use crate::server::db::{audit, ids, local_stamp, pool};
use crate::server_fns::legal_memos::{
    LegalMemo, LegalMemoContent, LegalMemoSummary, MemoAttorney, MemoReview, MemoStatus,
    MemoVersionEntry, MemoVersionSnapshot, ReviewCheck, ReviewMark, VersionEvent, DOCKET_KEYS,
};
use crate::server_fns::users::AccountRole;

type Tx<'a> = sqlx::Transaction<'a, sqlx::Postgres>;

struct Locked {
    status: MemoStatus,
    version: i32,
    title: String,
    content: String,
    review: MemoReview,
    author_id: String,
    attorney_id: String,
    attorney_name: String,
    revision: String,
}

pub const STALE_MEMO_ERROR: &str =
    "This memo was changed in another window. Reload to get the latest version.";

fn parse_content(raw: &str) -> Result<LegalMemoContent, sqlx::Error> {
    serde_json::from_str(raw)
        .map_err(|_| sqlx::Error::Protocol("This memo couldn't be read.".to_string()))
}

fn to_json<T: serde::Serialize>(value: &T) -> Result<String, sqlx::Error> {
    serde_json::to_string(value)
        .map_err(|_| sqlx::Error::Protocol("This memo couldn't be saved.".to_string()))
}

pub struct MemoHeader {
    pub case_id: String,
    pub author_id: String,
    pub attorney_id: String,
}

pub async fn header(memo_id: &str) -> Result<Option<MemoHeader>, sqlx::Error> {
    sqlx::query_as::<_, (String, Option<String>, Option<String>)>(
        "SELECT case_id, author_id, attorney_user_id FROM legal_memos WHERE id = $1",
    )
    .bind(memo_id)
    .fetch_optional(pool())
    .await
    .map(|row| {
        row.map(|(case_id, author_id, attorney_id)| MemoHeader {
            case_id,
            author_id: author_id.unwrap_or_default(),
            attorney_id: attorney_id.unwrap_or_default(),
        })
    })
}

#[derive(sqlx::FromRow)]
struct SummaryRow {
    id: String,
    title: String,
    status: String,
    version: i32,
    author_name: String,
    attorney_name: String,
    updated_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list(case_id: &str) -> Result<Vec<LegalMemoSummary>, sqlx::Error> {
    let rows = sqlx::query_as::<_, SummaryRow>(
        "SELECT id, title, status, version, author_name, attorney_name, updated_at
         FROM legal_memos WHERE case_id = $1 ORDER BY seq DESC",
    )
    .bind(case_id)
    .fetch_all(pool())
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| LegalMemoSummary {
            id: r.id,
            title: r.title,
            status: MemoStatus::from_slug(&r.status).unwrap_or(MemoStatus::Draft),
            version: r.version,
            author_name: r.author_name,
            attorney_name: r.attorney_name,
            updated_on: local_stamp(r.updated_at)[..10].to_string(),
        })
        .collect())
}

#[derive(sqlx::FromRow)]
struct MemoRow {
    id: String,
    case_id: String,
    case_name: String,
    docket: String,
    title: String,
    status: String,
    version: i32,
    attorney_user_id: Option<String>,
    attorney_name: String,
    author_id: Option<String>,
    author_name: String,
    content: String,
    review: String,
    approved_by_name: String,
    approved_at: Option<chrono::DateTime<chrono::Utc>>,
    updated_at: chrono::DateTime<chrono::Utc>,
    revision: String,
}

pub async fn get(memo_id: &str) -> Result<Option<LegalMemo>, sqlx::Error> {
    let row = sqlx::query_as::<_, MemoRow>(
        "SELECT m.id, m.case_id, c.name AS case_name,
                COALESCE((SELECT btrim(p.value) FROM case_properties p
                          WHERE p.case_id = c.id AND p.key = ANY($2) AND btrim(p.value) <> ''
                          ORDER BY array_position($2, p.key), p.ord LIMIT 1), '') AS docket,
                m.title, m.status, m.version,
                m.attorney_user_id, m.attorney_name, m.author_id, m.author_name,
                m.content::text AS content, m.review::text AS review,
                m.approved_by_name, m.approved_at, m.updated_at,
                m.updated_at::text AS revision
         FROM legal_memos m JOIN cases c ON c.id = m.case_id
         WHERE m.id = $1",
    )
    .bind(memo_id)
    .bind(&DOCKET_KEYS[..])
    .fetch_optional(pool())
    .await?;
    let Some(r) = row else {
        return Ok(None);
    };
    Ok(Some(LegalMemo {
        id: r.id,
        case_id: r.case_id,
        case_name: r.case_name,
        docket: r.docket,
        title: r.title,
        status: MemoStatus::from_slug(&r.status).unwrap_or(MemoStatus::Draft),
        version: r.version,
        attorney_id: r.attorney_user_id.unwrap_or_default(),
        attorney_name: r.attorney_name,
        author_id: r.author_id.unwrap_or_default(),
        author_name: r.author_name,
        content: parse_content(&r.content)?,
        review: serde_json::from_str(&r.review).unwrap_or_default(),
        approved_by_name: r.approved_by_name,
        approved_at: r.approved_at.map(local_stamp).unwrap_or_default(),
        updated_at: local_stamp(r.updated_at),
        revision: r.revision,
        can_edit: false,
        can_review: false,
        can_start_new: false,
        can_file: false,
    }))
}

#[derive(sqlx::FromRow)]
struct VersionRow {
    id: String,
    version: i32,
    event: String,
    title: String,
    content: String,
    attorney_name: String,
    actor_name: String,
    comment: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn versions(memo_id: &str) -> Result<Vec<MemoVersionEntry>, sqlx::Error> {
    let rows = sqlx::query_as::<_, VersionRow>(
        "SELECT id, version, event, title, '' AS content, attorney_name, actor_name, comment,
                created_at
         FROM legal_memo_versions WHERE memo_id = $1 ORDER BY seq DESC",
    )
    .bind(memo_id)
    .fetch_all(pool())
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| {
            Some(MemoVersionEntry {
                id: r.id,
                version: r.version,
                event: VersionEvent::from_slug(&r.event)?,
                actor_name: r.actor_name,
                comment: r.comment,
                at: local_stamp(r.created_at),
            })
        })
        .collect())
}

pub async fn version_snapshot(
    memo_id: &str,
    version_id: &str,
) -> Result<Option<MemoVersionSnapshot>, sqlx::Error> {
    let row = sqlx::query_as::<_, VersionRow>(
        "SELECT id, version, event, title, content::text AS content, attorney_name, actor_name,
                comment, created_at
         FROM legal_memo_versions WHERE memo_id = $1 AND id = $2",
    )
    .bind(memo_id)
    .bind(version_id)
    .fetch_optional(pool())
    .await?;
    let Some(r) = row else {
        return Ok(None);
    };
    let Some(event) = VersionEvent::from_slug(&r.event) else {
        return Ok(None);
    };
    Ok(Some(MemoVersionSnapshot {
        version: r.version,
        event,
        title: r.title,
        content: parse_content(&r.content)?,
        attorney_name: r.attorney_name,
        actor_name: r.actor_name,
        at: local_stamp(r.created_at),
    }))
}

pub async fn attorney_choices(case_id: &str) -> Result<Vec<MemoAttorney>, sqlx::Error> {
    sqlx::query_as::<_, (String, String)>(&format!(
        "SELECT u.id, btrim(u.first_name || ' ' || u.last_name) AS name
         FROM users u
         WHERE u.role IN {staff}
           AND (u.role = 'site_admin'
                OR EXISTS (SELECT 1 FROM case_assignments a
                           WHERE a.case_id = $1 AND a.user_id = u.id
                             AND a.capability = 'view_evidence'))
         ORDER BY name",
        staff = AccountRole::STAFF_ROLES_SQL
    ))
    .bind(case_id)
    .fetch_all(pool())
    .await
    .map(|rows| {
        rows.into_iter()
            .map(|(id, name)| MemoAttorney { id, name })
            .collect()
    })
}

pub async fn create(
    case_id: &str,
    title: &str,
    author_id: &str,
    author_name: &str,
) -> Result<String, sqlx::Error> {
    let mut tx = pool().begin().await?;
    audit::set_actor_in_transaction(&mut tx, author_id).await?;
    let id = ids::next(&mut *tx, "lm").await?;
    sqlx::query(
        "INSERT INTO legal_memos (id, case_id, title, author_id, author_name)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(&id)
    .bind(case_id)
    .bind(title)
    .bind(author_id)
    .bind(author_name)
    .execute(&mut *tx)
    .await?;
    record(&mut tx, case_id, &id, author_name, "", "created").await?;
    tx.commit().await?;
    Ok(id)
}

pub async fn save_draft(
    memo_id: &str,
    author_id: &str,
    revision: &str,
    title: &str,
    attorney: Option<&MemoAttorney>,
    content: &LegalMemoContent,
) -> Result<String, sqlx::Error> {
    let mut tx = pool().begin().await?;
    audit::set_actor_in_transaction(&mut tx, author_id).await?;
    let locked = lock(&mut tx, memo_id).await?;
    require_author(&locked, author_id)?;
    require_editable(&locked)?;
    require_revision(&locked, revision)?;
    let next = write_draft(&mut tx, memo_id, title, attorney, content).await?;
    tx.commit().await?;
    Ok(next)
}

#[allow(clippy::too_many_arguments)]
pub async fn send_for_review(
    memo_id: &str,
    case_id: &str,
    actor_id: &str,
    actor: &str,
    revision: &str,
    title: &str,
    attorney: &MemoAttorney,
    content: &LegalMemoContent,
) -> Result<(), sqlx::Error> {
    let mut tx = pool().begin().await?;
    audit::set_actor_in_transaction(&mut tx, actor_id).await?;
    let locked = lock(&mut tx, memo_id).await?;
    require_author(&locked, actor_id)?;
    require_editable(&locked)?;
    require_revision(&locked, revision)?;
    write_draft(&mut tx, memo_id, title, Some(attorney), content).await?;
    sqlx::query(
        "UPDATE legal_memos SET status = 'in_review', review = '{}'::jsonb, updated_at = now()
         WHERE id = $1",
    )
    .bind(memo_id)
    .execute(&mut *tx)
    .await?;
    let snapshot = to_json(content)?;
    add_version(
        &mut tx,
        memo_id,
        locked.version,
        VersionEvent::Sent,
        title,
        &snapshot,
        &attorney.name,
        actor_id,
        actor,
        "",
    )
    .await?;
    record(
        &mut tx,
        case_id,
        memo_id,
        actor,
        locked.status.label(),
        &format!("sent to {} for review (version {})", attorney.name, locked.version),
    )
    .await?;
    tx.commit().await
}

pub async fn request_revisions(
    memo_id: &str,
    case_id: &str,
    actor_id: &str,
    actor: &str,
    comment: &str,
    is_site_admin: bool,
) -> Result<(), sqlx::Error> {
    let mut tx = pool().begin().await?;
    audit::set_actor_in_transaction(&mut tx, actor_id).await?;
    let locked = lock(&mut tx, memo_id).await?;
    require_in_review(&locked)?;
    require_reviewer(&locked, actor_id, is_site_admin)?;
    add_version(
        &mut tx,
        memo_id,
        locked.version,
        VersionEvent::RevisionsRequested,
        &locked.title,
        &locked.content,
        &locked.attorney_name,
        actor_id,
        actor,
        comment,
    )
    .await?;
    sqlx::query(
        "UPDATE legal_memos
         SET status = 'revisions_requested', version = version + 1, updated_at = now()
         WHERE id = $1",
    )
    .bind(memo_id)
    .execute(&mut *tx)
    .await?;
    record(
        &mut tx,
        case_id,
        memo_id,
        actor,
        locked.status.label(),
        &format!("revisions requested on version {}", locked.version),
    )
    .await?;
    tx.commit().await
}

#[allow(clippy::too_many_arguments)]
pub async fn set_review_check(
    memo_id: &str,
    case_id: &str,
    check: ReviewCheck,
    checked: bool,
    actor_id: &str,
    actor: &str,
    is_site_admin: bool,
) -> Result<MemoReview, sqlx::Error> {
    let mut tx = pool().begin().await?;
    audit::set_actor_in_transaction(&mut tx, actor_id).await?;
    let mut locked = lock(&mut tx, memo_id).await?;
    require_in_review(&locked)?;
    require_reviewer(&locked, actor_id, is_site_admin)?;
    locked.review.marks.retain(|m| m.check != check);
    if checked {
        locked.review.marks.push(ReviewMark {
            check,
            by_name: actor.to_string(),
            at: crate::server::db::now_stamp(),
        });
    }
    sqlx::query("UPDATE legal_memos SET review = $2::jsonb, updated_at = now() WHERE id = $1")
        .bind(memo_id)
        .bind(to_json(&locked.review)?)
        .execute(&mut *tx)
        .await?;
    record(
        &mut tx,
        case_id,
        memo_id,
        actor,
        "",
        &format!(
            "{} \"{}\" on version {}",
            if checked { "checked" } else { "unchecked" },
            check.label(),
            locked.version
        ),
    )
    .await?;
    tx.commit().await?;
    Ok(locked.review)
}

pub async fn approve(
    memo_id: &str,
    case_id: &str,
    actor_id: &str,
    actor: &str,
    is_site_admin: bool,
) -> Result<i32, sqlx::Error> {
    let mut tx = pool().begin().await?;
    audit::set_actor_in_transaction(&mut tx, actor_id).await?;
    let locked = lock(&mut tx, memo_id).await?;
    require_in_review(&locked)?;
    require_reviewer(&locked, actor_id, is_site_admin)?;
    let missing: Vec<&str> = ReviewCheck::REQUIRED_FOR_APPROVAL
        .iter()
        .filter(|check| !locked.review.has(**check))
        .map(|check| check.label())
        .collect();
    if !missing.is_empty() {
        return Err(sqlx::Error::Protocol(format!(
            "Check these before approving: {}.",
            missing.join(", ")
        )));
    }
    add_version(
        &mut tx,
        memo_id,
        locked.version,
        VersionEvent::Approved,
        &locked.title,
        &locked.content,
        &locked.attorney_name,
        actor_id,
        actor,
        "",
    )
    .await?;
    sqlx::query(
        "UPDATE legal_memos
         SET status = 'approved', approved_by_name = $2, approved_at = now(), updated_at = now()
         WHERE id = $1",
    )
    .bind(memo_id)
    .bind(actor)
    .execute(&mut *tx)
    .await?;
    record(
        &mut tx,
        case_id,
        memo_id,
        actor,
        locked.status.label(),
        &format!("approved version {}", locked.version),
    )
    .await?;
    tx.commit().await?;
    Ok(locked.version)
}

pub async fn start_new_version(
    memo_id: &str,
    case_id: &str,
    actor_id: &str,
    actor: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = pool().begin().await?;
    audit::set_actor_in_transaction(&mut tx, actor_id).await?;
    let locked = lock(&mut tx, memo_id).await?;
    require_author(&locked, actor_id)?;
    if locked.status != MemoStatus::Approved {
        return Err(sqlx::Error::Protocol(
            "Only an approved memo starts a new version.".into(),
        ));
    }
    sqlx::query(
        "UPDATE legal_memos
         SET status = 'draft', version = version + 1, review = '{}'::jsonb,
             approved_by_name = '', approved_at = NULL, updated_at = now()
         WHERE id = $1",
    )
    .bind(memo_id)
    .execute(&mut *tx)
    .await?;
    record(
        &mut tx,
        case_id,
        memo_id,
        actor,
        locked.status.label(),
        &format!("started version {}", locked.version + 1),
    )
    .await?;
    tx.commit().await
}

async fn lock(tx: &mut Tx<'_>, memo_id: &str) -> Result<Locked, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct Row {
        status: String,
        version: i32,
        title: String,
        content: String,
        review: String,
        author_id: Option<String>,
        attorney_user_id: Option<String>,
        attorney_name: String,
        revision: String,
    }
    let row = sqlx::query_as::<_, Row>(
        "SELECT status, version, title, content::text AS content, review::text AS review,
                author_id, attorney_user_id, attorney_name, updated_at::text AS revision
         FROM legal_memos WHERE id = $1 FOR UPDATE",
    )
    .bind(memo_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| sqlx::Error::Protocol("Legal memo not found.".into()))?;
    Ok(Locked {
        status: MemoStatus::from_slug(&row.status).unwrap_or(MemoStatus::Draft),
        version: row.version,
        title: row.title,
        content: row.content,
        review: serde_json::from_str(&row.review).unwrap_or_default(),
        author_id: row.author_id.unwrap_or_default(),
        attorney_id: row.attorney_user_id.unwrap_or_default(),
        attorney_name: row.attorney_name,
        revision: row.revision,
    })
}

fn require_author(locked: &Locked, actor_id: &str) -> Result<(), sqlx::Error> {
    if locked.author_id == actor_id {
        Ok(())
    } else {
        Err(sqlx::Error::Protocol("Only the memo's author can change it.".into()))
    }
}

fn require_revision(locked: &Locked, revision: &str) -> Result<(), sqlx::Error> {
    if locked.revision == revision {
        Ok(())
    } else {
        Err(sqlx::Error::Protocol(STALE_MEMO_ERROR.into()))
    }
}

fn require_editable(locked: &Locked) -> Result<(), sqlx::Error> {
    match locked.status {
        MemoStatus::Draft | MemoStatus::RevisionsRequested => Ok(()),
        MemoStatus::InReview => Err(sqlx::Error::Protocol(
            "This memo is with the attorney for review and can't be edited right now.".into(),
        )),
        MemoStatus::Approved => Err(sqlx::Error::Protocol(
            "This memo is approved. Start a new version to change it.".into(),
        )),
    }
}

fn require_reviewer(locked: &Locked, actor_id: &str, is_site_admin: bool) -> Result<(), sqlx::Error> {
    if locked.author_id == actor_id {
        return Err(sqlx::Error::Protocol(
            "You wrote this memo, so someone else has to review it.".into(),
        ));
    }
    if is_site_admin || (!locked.attorney_id.is_empty() && locked.attorney_id == actor_id) {
        Ok(())
    } else {
        Err(sqlx::Error::Protocol(
            "Only the attorney this memo is addressed to, or a site admin, can review it.".into(),
        ))
    }
}

fn require_in_review(locked: &Locked) -> Result<(), sqlx::Error> {
    if locked.status == MemoStatus::InReview {
        Ok(())
    } else {
        Err(sqlx::Error::Protocol(
            "This memo isn't waiting for attorney review.".into(),
        ))
    }
}

async fn write_draft(
    tx: &mut Tx<'_>,
    memo_id: &str,
    title: &str,
    attorney: Option<&MemoAttorney>,
    content: &LegalMemoContent,
) -> Result<String, sqlx::Error> {
    sqlx::query_scalar(
        "UPDATE legal_memos
         SET title = $2, attorney_user_id = $3, attorney_name = $4,
             content = $5::jsonb, updated_at = clock_timestamp()
         WHERE id = $1
         RETURNING updated_at::text",
    )
    .bind(memo_id)
    .bind(title)
    .bind(attorney.map(|a| a.id.as_str()))
    .bind(attorney.map(|a| a.name.as_str()).unwrap_or_default())
    .bind(to_json(content)?)
    .fetch_one(&mut **tx)
    .await
}

#[allow(clippy::too_many_arguments)]
async fn add_version(
    tx: &mut Tx<'_>,
    memo_id: &str,
    version: i32,
    event: VersionEvent,
    title: &str,
    content_json: &str,
    attorney_name: &str,
    actor_id: &str,
    actor: &str,
    comment: &str,
) -> Result<(), sqlx::Error> {
    let id = ids::next(&mut **tx, "lv").await?;
    sqlx::query(
        "INSERT INTO legal_memo_versions
             (id, memo_id, version, event, title, content, attorney_name, actor_id, actor_name,
              comment)
         VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7, $8, $9, $10)",
    )
    .bind(&id)
    .bind(memo_id)
    .bind(version)
    .bind(event.slug())
    .bind(title)
    .bind(content_json)
    .bind(attorney_name)
    .bind(actor_id)
    .bind(actor)
    .bind(comment)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn record(
    tx: &mut Tx<'_>,
    case_id: &str,
    memo_id: &str,
    actor: &str,
    old: &str,
    new: &str,
) -> Result<(), sqlx::Error> {
    audit::record_in_transaction_with_visibility(
        tx,
        audit::Entity::Case,
        case_id,
        actor,
        &format!("legal memo {memo_id}"),
        old,
        new,
        Visibility::VolunteerOnly,
    )
    .await
}
