use crate::server::db::{local_stamp, pool};
use crate::server_fns::legal_memos::{
    LegalMemo, LegalMemoContent, LegalMemoSummary, MemoAttorney, MemoStatus, MemoVersionEntry,
    MemoVersionSnapshot, VersionEvent, DOCKET_KEYS,
};
use crate::server_fns::users::AccountRole;

fn parse_content(raw: &str) -> Result<LegalMemoContent, sqlx::Error> {
    serde_json::from_str(raw)
        .map_err(|_| sqlx::Error::Protocol("This memo couldn't be read.".to_string()))
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
