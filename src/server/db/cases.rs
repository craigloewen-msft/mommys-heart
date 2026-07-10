//! Cases and their sub-resources: notes, evidence, properties, and audit log.

use crate::server::db::{audit, ids, now_stamp, pool, users};
use crate::types::{
    Case, CaseCapability, CaseNote, CaseProperty, CaseStatus, Evidence, Page,
};
use std::collections::HashMap;

#[derive(sqlx::FromRow)]
struct CaseRow {
    id: String,
    name: String,
    status: String,
    owner_id: String,
}

#[derive(sqlx::FromRow)]
struct NoteRow {
    id: String,
    author: String,
    body: String,
    created_at: String,
}

#[derive(sqlx::FromRow)]
struct EvidenceRow {
    id: String,
    name: String,
    case_id: String,
    uploaded_by: String,
    uploaded_at: String,
    description: String,
}

async fn load_notes(case_id: &str) -> Result<Vec<CaseNote>, sqlx::Error> {
    let rows = sqlx::query_as::<_, NoteRow>(
        "SELECT id, author, body, created_at FROM case_notes WHERE case_id = $1 ORDER BY seq ASC",
    )
    .bind(case_id)
    .fetch_all(pool())
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| CaseNote {
            id: r.id,
            author: r.author,
            body: r.body,
            created_at: r.created_at,
        })
        .collect())
}

async fn load_evidence(case_id: &str) -> Result<Vec<Evidence>, sqlx::Error> {
    let rows = sqlx::query_as::<_, EvidenceRow>(
        "SELECT id, name, case_id, uploaded_by, uploaded_at, description
         FROM evidence WHERE case_id = $1 ORDER BY seq ASC",
    )
    .bind(case_id)
    .fetch_all(pool())
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| Evidence {
            id: r.id,
            name: r.name,
            case_id: r.case_id,
            uploaded_by: r.uploaded_by,
            uploaded_at: r.uploaded_at,
            description: r.description,
        })
        .collect())
}

async fn load_properties(case_id: &str) -> Result<Vec<CaseProperty>, sqlx::Error> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT key, value FROM case_properties WHERE case_id = $1 ORDER BY ord ASC",
    )
    .bind(case_id)
    .fetch_all(pool())
    .await?;
    Ok(rows
        .into_iter()
        .map(|(key, value)| CaseProperty { key, value })
        .collect())
}

async fn hydrate(row: CaseRow) -> Result<Case, sqlx::Error> {
    let notes = load_notes(&row.id).await?;
    let evidence = load_evidence(&row.id).await?;
    let properties = load_properties(&row.id).await?;
    let audit_log = audit::for_entity(pool(), audit::Entity::Case, &row.id).await?;
    Ok(Case {
        id: row.id,
        name: row.name,
        status: CaseStatus::from_slug(&row.status).unwrap_or(CaseStatus::Open),
        owner_id: row.owner_id,
        notes,
        evidence,
        properties,
        audit_log,
        message_count: 0,
    })
}

/// Batched notes for many cases, grouped by case id (each list in `seq` order).
async fn load_notes_many(
    case_ids: &[String],
) -> Result<HashMap<String, Vec<CaseNote>>, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct Row {
        case_id: String,
        id: String,
        author: String,
        body: String,
        created_at: String,
    }
    let rows = sqlx::query_as::<_, Row>(
        "SELECT case_id, id, author, body, created_at
         FROM case_notes WHERE case_id = ANY($1) ORDER BY case_id, seq ASC",
    )
    .bind(case_ids)
    .fetch_all(pool())
    .await?;
    let mut map: HashMap<String, Vec<CaseNote>> = HashMap::new();
    for r in rows {
        map.entry(r.case_id).or_default().push(CaseNote {
            id: r.id,
            author: r.author,
            body: r.body,
            created_at: r.created_at,
        });
    }
    Ok(map)
}

/// Batched evidence for many cases, grouped by case id (each list in `seq` order).
async fn load_evidence_many(
    case_ids: &[String],
) -> Result<HashMap<String, Vec<Evidence>>, sqlx::Error> {
    let rows = sqlx::query_as::<_, EvidenceRow>(
        "SELECT id, name, case_id, uploaded_by, uploaded_at, description
         FROM evidence WHERE case_id = ANY($1) ORDER BY case_id, seq ASC",
    )
    .bind(case_ids)
    .fetch_all(pool())
    .await?;
    let mut map: HashMap<String, Vec<Evidence>> = HashMap::new();
    for r in rows {
        map.entry(r.case_id.clone()).or_default().push(Evidence {
            id: r.id,
            name: r.name,
            case_id: r.case_id,
            uploaded_by: r.uploaded_by,
            uploaded_at: r.uploaded_at,
            description: r.description,
        });
    }
    Ok(map)
}

/// Batched properties for many cases, grouped by case id (each list in `ord` order).
async fn load_properties_many(
    case_ids: &[String],
) -> Result<HashMap<String, Vec<CaseProperty>>, sqlx::Error> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT case_id, key, value FROM case_properties
         WHERE case_id = ANY($1) ORDER BY case_id, ord ASC",
    )
    .bind(case_ids)
    .fetch_all(pool())
    .await?;
    let mut map: HashMap<String, Vec<CaseProperty>> = HashMap::new();
    for (case_id, key, value) in rows {
        map.entry(case_id)
            .or_default()
            .push(CaseProperty { key, value });
    }
    Ok(map)
}

/// Fully hydrate a page of cases with a fixed, small number of queries instead
/// of one-query-per-sub-resource-per-case (an N+1 pattern). The four independent
/// batch loads run concurrently, so the whole page costs roughly one round-trip.
async fn hydrate_many(rows: Vec<CaseRow>) -> Result<Vec<Case>, sqlx::Error> {
    if rows.is_empty() {
        return Ok(Vec::new());
    }
    let ids: Vec<String> = rows.iter().map(|r| r.id.clone()).collect();

    let (mut notes, mut evidence, mut properties, mut audit_log) = tokio::try_join!(
        load_notes_many(&ids),
        load_evidence_many(&ids),
        load_properties_many(&ids),
        audit::for_entities(pool(), audit::Entity::Case, &ids),
    )?;

    Ok(rows
        .into_iter()
        .map(|row| Case {
            notes: notes.remove(&row.id).unwrap_or_default(),
            evidence: evidence.remove(&row.id).unwrap_or_default(),
            properties: properties.remove(&row.id).unwrap_or_default(),
            audit_log: audit_log.remove(&row.id).unwrap_or_default(),
            status: CaseStatus::from_slug(&row.status).unwrap_or(CaseStatus::Open),
            id: row.id,
            name: row.name,
            owner_id: row.owner_id,
            message_count: 0,
        })
        .collect())
}

/// A lightweight [`Case`] (id, name, status, owner only — no notes, evidence,
/// properties, or audit) from a `CaseRow`. Used for list/directory views that
/// never render the sub-resources.
fn summary(row: CaseRow) -> Case {
    Case {
        id: row.id,
        name: row.name,
        status: CaseStatus::from_slug(&row.status).unwrap_or(CaseStatus::Open),
        owner_id: row.owner_id,
        notes: Vec::new(),
        evidence: Vec::new(),
        properties: Vec::new(),
        audit_log: Vec::new(),
        message_count: 0,
    }
}

/// Cases visible to `viewer` (those they hold capabilities on) in lightweight
/// form (see [`summary`]). Bounded by the user's own assignments, so it stays
/// cheap no matter how many cases exist system-wide. This is what bootstrap
/// ships for name resolution and the case/chat pickers; the case screen loads
/// full, paginated detail on demand via [`page`].
pub async fn directory_for(viewer: &str) -> Result<Vec<Case>, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct DirRow {
        id: String,
        name: String,
        status: String,
        owner_id: String,
        message_count: i64,
    }

    let rows = sqlx::query_as::<_, DirRow>(
        "SELECT c.id, c.name, c.status, c.owner_id,
                (SELECT COUNT(*) FROM messages m WHERE m.case_id = c.id) AS message_count
         FROM cases c
         WHERE EXISTS (SELECT 1 FROM case_assignments a
                       WHERE a.case_id = c.id AND a.user_id = $1)
         ORDER BY c.id",
    )
    .bind(viewer)
    .fetch_all(pool())
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| Case {
            id: r.id,
            name: r.name,
            status: CaseStatus::from_slug(&r.status).unwrap_or(CaseStatus::Open),
            owner_id: r.owner_id,
            notes: Vec::new(),
            evidence: Vec::new(),
            properties: Vec::new(),
            audit_log: Vec::new(),
            message_count: r.message_count.max(0) as usize,
        })
        .collect())
}

/// Up to `limit` lightweight cases whose id or name matches `search`
/// (case-insensitive, metacharacters escaped), ordered by id. Unscoped — used by
/// the admin permission tool to find any case to grant access to. Empty search
/// returns the first `limit` cases.
pub async fn search_lite(search: &str, limit: i64) -> Result<Vec<Case>, sqlx::Error> {
    let limit = limit.clamp(1, 50);
    let term = search.trim();
    let pattern = if term.is_empty() {
        None
    } else {
        Some(format!(
            "%{}%",
            term.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
        ))
    };
    let rows = sqlx::query_as::<_, CaseRow>(
        "SELECT id, name, status, owner_id FROM cases
         WHERE $1::text IS NULL OR id ILIKE $1 OR name ILIKE $1
         ORDER BY id LIMIT $2",
    )
    .bind(&pattern)
    .bind(limit)
    .fetch_all(pool())
    .await?;
    Ok(rows.into_iter().map(summary).collect())
}

/// Lightweight cases for a specific set of ids (see [`summary`]). Used to resolve
/// case names for display without loading every case.
pub async fn by_ids_lite(ids: &[String]) -> Result<Vec<Case>, sqlx::Error> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query_as::<_, CaseRow>(
        "SELECT id, name, status, owner_id FROM cases WHERE id = ANY($1) ORDER BY id",
    )
    .bind(ids)
    .fetch_all(pool())
    .await?;
    Ok(rows.into_iter().map(summary).collect())
}

/// One page of fully-hydrated cases, ordered by id, with an optional
/// case-insensitive search over id/name, scoped to the cases `viewer` holds
/// capabilities on.
///
/// Backs the case screen's server-side pagination ("Load more") so the UI never
/// has to pull every case into the browser.
pub async fn page(
    offset: i64,
    limit: i64,
    search: &str,
    viewer: &str,
) -> Result<Page<Case>, sqlx::Error> {
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
            term.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
        ))
    };

    // `$1` is the (nullable) search pattern; the viewer id scopes the results to
    // cases they hold capabilities on. Its bind position differs between the
    // COUNT query (`$2`) and the page query (`$4`, after LIMIT/OFFSET).
    const SEARCH: &str = "($1::text IS NULL OR id ILIKE $1 OR name ILIKE $1)";
    const SCOPE: &str = "EXISTS \
        (SELECT 1 FROM case_assignments a WHERE a.case_id = cases.id AND a.user_id = $VIEWER)";

    let count_sql = format!(
        "SELECT count(*) FROM cases WHERE {SEARCH} AND {}",
        SCOPE.replace("$VIEWER", "$2")
    );
    let total = sqlx::query_scalar::<_, i64>(&count_sql)
        .bind(&pattern)
        .bind(viewer)
        .fetch_one(pool())
        .await?;

    let page_sql = format!(
        "SELECT id, name, status, owner_id FROM cases WHERE {SEARCH} AND {} \
         ORDER BY id LIMIT $2 OFFSET $3",
        SCOPE.replace("$VIEWER", "$4")
    );
    let rows = sqlx::query_as::<_, CaseRow>(&page_sql)
        .bind(&pattern)
        .bind(limit)
        .bind(offset)
        .bind(viewer)
        .fetch_all(pool())
        .await?;

    let items = hydrate_many(rows).await?;
    Ok(Page { items, total })
}

/// A single case by id, fully hydrated.
pub async fn get(id: &str) -> Result<Option<Case>, sqlx::Error> {
    let row = sqlx::query_as::<_, CaseRow>(
        "SELECT id, name, status, owner_id FROM cases WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool())
    .await?;
    match row {
        Some(r) => Ok(Some(hydrate(r).await?)),
        None => Ok(None),
    }
}

/// The owner id of a case, if it exists (cheap authorization lookup).
pub async fn owner_id(case_id: &str) -> Result<Option<String>, sqlx::Error> {
    let owner: Option<String> = sqlx::query_scalar("SELECT owner_id FROM cases WHERE id = $1")
        .bind(case_id)
        .fetch_optional(pool())
        .await?;
    Ok(owner)
}

/// Create a case owned by `owner_id` with an initial status, cleaned property
/// set, and an optional first note. Grants the owner full capabilities. Returns
/// the new case id.
pub async fn create(
    owner_id: &str,
    owner_name: &str,
    name: &str,
    status: CaseStatus,
    properties: Vec<(String, String)>,
    first_note: Option<String>,
) -> Result<String, sqlx::Error> {
    let id = ids::next(pool(), "c").await?;

    let mut tx = pool().begin().await?;
    sqlx::query("INSERT INTO cases (id, name, status, owner_id) VALUES ($1, $2, $3, $4)")
        .bind(&id)
        .bind(name)
        .bind(status.slug())
        .bind(owner_id)
        .execute(&mut *tx)
        .await?;

    let mut ord: i32 = 0;
    for (k, v) in properties {
        let key = k.trim().to_string();
        let value = v.trim().to_string();
        if key.is_empty() || value.is_empty() {
            continue;
        }
        sqlx::query(
            "INSERT INTO case_properties (case_id, ord, key, value) VALUES ($1, $2, $3, $4)",
        )
        .bind(&id)
        .bind(ord)
        .bind(&key)
        .bind(&value)
        .execute(&mut *tx)
        .await?;
        ord += 1;
    }

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
    tx.commit().await?;

    // Give the owner an explicit full-control assignment (also audits it).
    users::assign_capabilities(owner_id, &id, &CaseCapability::ALL, owner_name).await?;
    Ok(id)
}

/// Update a case's status, auditing the change.
pub async fn set_status(
    case_id: &str,
    status: CaseStatus,
    actor: &str,
) -> Result<(), sqlx::Error> {
    let current: Option<String> = sqlx::query_scalar("SELECT status FROM cases WHERE id = $1")
        .bind(case_id)
        .fetch_optional(pool())
        .await?;
    let Some(current) = current else {
        return Ok(());
    };
    if current == status.slug() {
        return Ok(());
    }
    sqlx::query("UPDATE cases SET status = $1 WHERE id = $2")
        .bind(status.slug())
        .bind(case_id)
        .execute(pool())
        .await?;
    audit::record(
        pool(),
        audit::Entity::Case,
        case_id,
        actor,
        "status",
        &current,
        status.slug(),
    )
    .await
}

/// Rename a case, auditing the change.
pub async fn set_name(case_id: &str, name: &str, actor: &str) -> Result<(), sqlx::Error> {
    let current: Option<String> = sqlx::query_scalar("SELECT name FROM cases WHERE id = $1")
        .bind(case_id)
        .fetch_optional(pool())
        .await?;
    let Some(current) = current else {
        return Ok(());
    };
    if current == name {
        return Ok(());
    }
    sqlx::query("UPDATE cases SET name = $1 WHERE id = $2")
        .bind(name)
        .bind(case_id)
        .execute(pool())
        .await?;
    audit::record(pool(), audit::Entity::Case, case_id, actor, "name", &current, name).await
}

/// Change a case's owner. `owner_id` must reference an existing user. Audits
/// using display names for readability.
pub async fn set_owner(case_id: &str, owner_id: &str, actor: &str) -> Result<(), sqlx::Error> {
    let current: Option<String> = sqlx::query_scalar("SELECT owner_id FROM cases WHERE id = $1")
        .bind(case_id)
        .fetch_optional(pool())
        .await?;
    let Some(current) = current else {
        return Ok(());
    };
    if current == owner_id {
        return Ok(());
    }
    sqlx::query("UPDATE cases SET owner_id = $1 WHERE id = $2")
        .bind(owner_id)
        .bind(case_id)
        .execute(pool())
        .await?;
    audit::record(
        pool(),
        audit::Entity::Case,
        case_id,
        actor,
        "owner",
        &user_name(&current).await?,
        &user_name(owner_id).await?,
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

/// Replace a case's whole property set, auditing once when it changes.
pub async fn replace_properties(
    case_id: &str,
    props: Vec<(String, String)>,
    actor: &str,
) -> Result<(), sqlx::Error> {
    let cleaned: Vec<(String, String)> = props
        .into_iter()
        .filter_map(|(k, v)| {
            let key = k.trim().to_string();
            if key.is_empty() {
                None
            } else {
                Some((key, v.trim().to_string()))
            }
        })
        .collect();

    let existing = load_properties(case_id).await?;
    let changed = existing.len() != cleaned.len()
        || existing
            .iter()
            .zip(cleaned.iter())
            .any(|(e, (k, v))| &e.key != k || &e.value != v);

    let mut tx = pool().begin().await?;
    sqlx::query("DELETE FROM case_properties WHERE case_id = $1")
        .bind(case_id)
        .execute(&mut *tx)
        .await?;
    for (ord, (key, value)) in cleaned.iter().enumerate() {
        sqlx::query(
            "INSERT INTO case_properties (case_id, ord, key, value) VALUES ($1, $2, $3, $4)",
        )
        .bind(case_id)
        .bind(ord as i32)
        .bind(key)
        .bind(value)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    if changed {
        audit::record(
            pool(),
            audit::Entity::Case,
            case_id,
            actor,
            "properties",
            "",
            "updated",
        )
        .await?;
    }
    Ok(())
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

/// Attach evidence to a case.
pub async fn add_evidence(
    case_id: &str,
    name: &str,
    uploaded_by: &str,
    description: &str,
) -> Result<(), sqlx::Error> {
    let id = ids::next(pool(), "e").await?;
    sqlx::query(
        "INSERT INTO evidence (id, case_id, name, uploaded_by, uploaded_at, description)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(&id)
    .bind(case_id)
    .bind(name)
    .bind(uploaded_by)
    .bind(now_stamp())
    .bind(description)
    .execute(pool())
    .await?;
    Ok(())
}

/// Remove a piece of evidence from a case.
pub async fn delete_evidence(case_id: &str, evidence_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM evidence WHERE case_id = $1 AND id = $2")
        .bind(case_id)
        .bind(evidence_id)
        .execute(pool())
        .await?;
    Ok(())
}
