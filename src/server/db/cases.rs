//! Cases and their sub-resources: notes, evidence, properties, and audit log.

use crate::server::db::{audit, ids, now_stamp, pool, users};
use crate::server_fns::cases::{Case, CaseNote, CaseProperty, CaseStatus, CaseSummary, Evidence};
use crate::server_fns::pagination::Page;
use crate::server_fns::permissions::CaseCapability;

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

/// Flat row shape for the sparse [`CaseSummary`] projection (header fields plus
/// resolved owner name and message count). Shared by every summary query.
#[derive(sqlx::FromRow)]
struct SummaryRow {
    id: String,
    name: String,
    status: String,
    owner_id: String,
    owner_name: String,
    message_count: i64,
}

impl SummaryRow {
    fn into_summary(self) -> CaseSummary {
        CaseSummary {
            // Fall back to the owner id if the user row is missing or unnamed.
            owner_name: if self.owner_name.trim().is_empty() {
                self.owner_id.clone()
            } else {
                self.owner_name
            },
            id: self.id,
            name: self.name,
            status: CaseStatus::from_slug(&self.status).unwrap_or(CaseStatus::Open),
            owner_id: self.owner_id,
            message_count: self.message_count.max(0) as usize,
        }
    }
}

/// The `SELECT` list that projects a `cases` row (aliased `c`) into a
/// [`SummaryRow`]: header fields, the owner's resolved display name, and the
/// case's message count. Callers append their own `WHERE`/`ORDER`/`LIMIT`.
const SUMMARY_SELECT: &str = "SELECT c.id, c.name, c.status, c.owner_id,
        TRIM(COALESCE(u.first_name, '') || ' ' || COALESCE(u.last_name, '')) AS owner_name,
        (SELECT COUNT(*) FROM messages m WHERE m.case_id = c.id) AS message_count
 FROM cases c
 LEFT JOIN users u ON u.id = c.owner_id";

/// The owner id of a case, if it exists (cheap authorization lookup).
pub async fn owner_id(case_id: &str) -> Result<Option<String>, sqlx::Error> {
    let owner: Option<String> = sqlx::query_scalar("SELECT owner_id FROM cases WHERE id = $1")
        .bind(case_id)
        .fetch_optional(pool())
        .await?;
    Ok(owner)
}

/// One page of sparse cases visible to a user_id
pub async fn get_summaries_for_user(
    offset: i64,
    limit: i64,
    search: &str,
    user_id: &str,
) -> Result<Page<CaseSummary>, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct DirRow {
        id: String,
        name: String,
        status: String,
        owner_id: String,
        owner_name: String,
        message_count: i64,
    }

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
    // cases they hold capabilities on. Its bind position differs between the
    // COUNT query (`$2`) and the page query (`$4`, after LIMIT/OFFSET).
    const SEARCH: &str = "($1::text IS NULL OR c.id ILIKE $1 OR c.name ILIKE $1)";
    const SCOPE: &str = "EXISTS \
        (SELECT 1 FROM case_assignments a WHERE a.case_id = c.id AND a.user_id = $VIEWER)";

    let count_sql = format!(
        "SELECT count(*) FROM cases c WHERE {SEARCH} AND {}",
        SCOPE.replace("$VIEWER", "$2")
    );
    let page_sql = format!(
        "SELECT c.id, c.name, c.status, c.owner_id,
                TRIM(COALESCE(u.first_name, '') || ' ' || COALESCE(u.last_name, '')) AS owner_name,
                (SELECT COUNT(*) FROM messages m WHERE m.case_id = c.id) AS message_count
         FROM cases c
         LEFT JOIN users u ON u.id = c.owner_id
         WHERE {SEARCH} AND {}
         ORDER BY c.id LIMIT $2 OFFSET $3",
        SCOPE.replace("$VIEWER", "$4")
    );

    // The count and the page fetch are independent, so run them concurrently —
    // one fewer sequential round-trip against a networked database.
    let count_fut = sqlx::query_scalar::<_, i64>(&count_sql)
        .bind(&pattern)
        .bind(user_id)
        .fetch_one(pool());
    let rows_fut = sqlx::query_as::<_, DirRow>(&page_sql)
        .bind(&pattern)
        .bind(limit)
        .bind(offset)
        .bind(user_id)
        .fetch_all(pool());
    let (total, rows) = tokio::try_join!(count_fut, rows_fut)?;

    let items = rows
        .into_iter()
        .map(|r| CaseSummary {
            // Fall back to the owner id if the user row is missing or unnamed.
            owner_name: if r.owner_name.is_empty() {
                r.owner_id.clone()
            } else {
                r.owner_name
            },
            id: r.id,
            name: r.name,
            status: CaseStatus::from_slug(&r.status).unwrap_or(CaseStatus::Open),
            owner_id: r.owner_id,
            message_count: r.message_count.max(0) as usize,
        })
        .collect();
    Ok(Page { items, total })
}

/// Up to `limit` lightweight cases whose id or name matches `search`
/// (case-insensitive, metacharacters escaped), ordered by id. Unscoped — used by
/// the admin permission tool to find any case to grant access to. Empty search
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
        "{SUMMARY_SELECT}
         WHERE $1::text IS NULL OR c.id ILIKE $1 OR c.name ILIKE $1
         ORDER BY c.id LIMIT $2"
    ))
    .bind(&pattern)
    .bind(limit)
    .fetch_all(pool())
    .await?;
    Ok(rows.into_iter().map(SummaryRow::into_summary).collect())
}

/// Lightweight [`CaseSummary`]s for a specific set of ids, ordered by id. Used to
/// resolve case names for display without loading every case. Empty input
/// returns an empty vec (no query).
pub async fn get_summaries_by_ids(ids: &[String]) -> Result<Vec<CaseSummary>, sqlx::Error> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query_as::<_, SummaryRow>(&format!(
        "{SUMMARY_SELECT} WHERE c.id = ANY($1) ORDER BY c.id"
    ))
    .bind(ids)
    .fetch_all(pool())
    .await?;
    Ok(rows.into_iter().map(SummaryRow::into_summary).collect())
}

/// A single case by id, fully hydrated (properties, evidence, and its newest
/// notes), or `None` if no such case exists.
pub async fn get(id: &str) -> Result<Option<Case>, sqlx::Error> {
    let Some(row) =
        sqlx::query_as::<_, CaseRow>("SELECT id, name, status, owner_id FROM cases WHERE id = $1")
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

    let evidence_rows = sqlx::query_as::<_, EvidenceRow>(
        "SELECT id, name, case_id, uploaded_by, uploaded_at, description
         FROM evidence WHERE case_id = $1 ORDER BY seq ASC",
    )
    .bind(id)
    .fetch_all(pool())
    .await?;
    let evidence = evidence_rows
        .into_iter()
        .map(|r| Evidence {
            id: r.id,
            name: r.name,
            case_id: r.case_id,
            uploaded_by: r.uploaded_by,
            uploaded_at: r.uploaded_at,
            description: r.description,
        })
        .collect();

    let property_rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT key, value FROM case_properties WHERE case_id = $1 ORDER BY ord ASC",
    )
    .bind(id)
    .fetch_all(pool())
    .await?;
    let properties = property_rows
        .into_iter()
        .map(|(key, value)| CaseProperty { key, value })
        .collect();

    Ok(Some(Case {
        id: row.id,
        name: row.name,
        status: CaseStatus::from_slug(&row.status).unwrap_or(CaseStatus::Open),
        owner_id: row.owner_id,
        notes,
        evidence,
        properties,
        message_count: 0,
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

    let existing: Vec<(String, String)> = sqlx::query_as(
        "SELECT key, value FROM case_properties WHERE case_id = $1 ORDER BY ord ASC",
    )
    .bind(case_id)
    .fetch_all(pool())
    .await?;
    let changed = existing.len() != cleaned.len()
        || existing
            .iter()
            .zip(cleaned.iter())
            .any(|((ek, ev), (k, v))| ek != k || ev != v);

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
