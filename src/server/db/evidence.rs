//! Evidence persistence (SSR only): the `evidence` table backing a case's
//! uploaded files and legacy metadata-only entries. Kept separate from
//! [`crate::server::db::cases`] because evidence is its own concept — a
//! file-storage resource with its own lifecycle (reserve id → store bytes →
//! write row → delete row + blob).

use crate::server::db::{audit, ids, now_stamp, pool};
use crate::server_fns::evidence::Evidence;

/// Flat row shape for hydrating [`Evidence`] from the `evidence` table.
#[derive(sqlx::FromRow)]
struct EvidenceRow {
    id: String,
    name: String,
    case_id: String,
    uploaded_by: String,
    uploaded_at: String,
    description: String,
    original_filename: String,
    content_type: String,
    size_bytes: i64,
    sha256: String,
    blob_path: String,
    status: String,
}

/// The file details of an uploaded piece of evidence, used to persist a
/// file-backed evidence row.
pub struct EvidenceFile<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub original_filename: &'a str,
    pub content_type: &'a str,
    pub size_bytes: i64,
    pub sha256: &'a str,
    pub blob_path: &'a str,
    pub status: &'a str,
}

/// All evidence attached to a case, oldest first. Used to hydrate a full case.
pub async fn get_case_evidence(case_id: &str) -> Result<Vec<Evidence>, sqlx::Error> {
    let rows = sqlx::query_as::<_, EvidenceRow>(
        "SELECT id, name, case_id, uploaded_by, uploaded_at, description,
                original_filename, content_type, size_bytes, sha256, blob_path, status
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
            original_filename: r.original_filename,
            content_type: r.content_type,
            size_bytes: r.size_bytes,
            sha256: r.sha256,
            has_file: !r.blob_path.is_empty() && r.status == "stored",
        })
        .collect())
}

/// Reserve an evidence id for a case. Callers use this to name the blob before
/// the row exists, so the blob path is known up front and the DB row can be
/// written only after the bytes land (avoiding orphaned rows).
pub async fn reserve_id() -> Result<String, sqlx::Error> {
    ids::next(pool(), "e").await
}

/// Insert a file-backed evidence row with a pre-reserved id (see
/// [`reserve_id`]).
pub async fn add_evidence(
    id: &str,
    case_id: &str,
    uploaded_by: &str,
    file: &EvidenceFile<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO evidence
            (id, case_id, name, uploaded_by, uploaded_at, description,
             original_filename, content_type, size_bytes, sha256, blob_path, status)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
    )
    .bind(id)
    .bind(case_id)
    .bind(file.name)
    .bind(uploaded_by)
    .bind(now_stamp())
    .bind(file.description)
    .bind(file.original_filename)
    .bind(file.content_type)
    .bind(file.size_bytes)
    .bind(file.sha256)
    .bind(file.blob_path)
    .bind(file.status)
    .execute(pool())
    .await?;
    audit::record(
        pool(),
        audit::Entity::Case,
        case_id,
        uploaded_by,
        "evidence",
        "",
        file.original_filename,
    )
    .await?;
    Ok(())
}

/// The blob path backing a piece of evidence, or an empty string for a
/// metadata-only entry. `None` when the evidence does not exist on the case.
pub async fn blob_path(case_id: &str, evidence_id: &str) -> Result<Option<String>, sqlx::Error> {
    let path: Option<String> =
        sqlx::query_scalar("SELECT blob_path FROM evidence WHERE case_id = $1 AND id = $2")
            .bind(case_id)
            .bind(evidence_id)
            .fetch_optional(pool())
            .await?;
    Ok(path)
}

/// The download filename + content type recorded for a piece of evidence, used
/// to set the response headers when streaming the file back.
pub async fn download_meta(
    case_id: &str,
    evidence_id: &str,
) -> Result<Option<(String, String)>, sqlx::Error> {
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT original_filename, content_type FROM evidence WHERE case_id = $1 AND id = $2",
    )
    .bind(case_id)
    .bind(evidence_id)
    .fetch_optional(pool())
    .await?;
    Ok(row)
}

/// Remove a piece of evidence from a case. Returns the blob path that backed it
/// (empty for metadata-only evidence) so the caller can delete the blob too.
pub async fn delete(case_id: &str, evidence_id: &str) -> Result<String, sqlx::Error> {
    let blob_path: Option<String> = sqlx::query_scalar(
        "DELETE FROM evidence WHERE case_id = $1 AND id = $2 RETURNING blob_path",
    )
    .bind(case_id)
    .bind(evidence_id)
    .fetch_optional(pool())
    .await?;
    Ok(blob_path.unwrap_or_default())
}
