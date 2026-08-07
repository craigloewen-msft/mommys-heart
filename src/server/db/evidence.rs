//! Evidence persistence (SSR only): the `evidence` table backing a case's files.
//!
//! Kept separate from [`crate::server::db::cases`] because evidence is its own
//! concept — a file-storage resource with its own lifecycle (reserve id → store
//! bytes → write row → delete row + blob).
//!
//! A piece of evidence either has a file in it or it does not. Both are the same
//! row: `blob_path` and the other file columns stay empty until a file is
//! provided, and [`set_file`] fills them in later. There is no second kind of
//! evidence and no separate table — which is what lets a case be created already
//! listing the documents it is waiting on.
//!
//! Every row lives in one of the case's folders ([`crate::server::db::case_folders`]),
//! and the folder is what decides the row's case and audience: both are read off
//! the folder on insert and on move, never taken from a caller.

use crate::helpers::visibility::Visibility;
use crate::server::db::{audit, ids, now_stamp, pool};
use crate::server_fns::case_folders::CaseFolder;
use crate::server_fns::evidence::Evidence;

/// The columns needed to hydrate an [`Evidence`], in a fixed order so every
/// query selects exactly the same shape.
const COLUMNS: &str = "id, name, case_id, uploaded_by, uploaded_at, description,
     original_filename, content_type, size_bytes, sha256, blob_path,
     folder_id, visibility";

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
    folder_id: String,
    visibility: String,
}

impl EvidenceRow {
    fn into_evidence(self) -> Evidence {
        Evidence {
            id: self.id,
            name: self.name,
            case_id: self.case_id,
            uploaded_by: self.uploaded_by,
            uploaded_at: self.uploaded_at,
            description: self.description,
            original_filename: self.original_filename,
            content_type: self.content_type,
            size_bytes: self.size_bytes,
            sha256: self.sha256,
            has_file: !self.blob_path.is_empty(),
            folder_id: self.folder_id,
            visibility: Visibility::from_slug(&self.visibility).unwrap_or_default(),
        }
    }
}

/// The file details of a stored upload.
pub struct EvidenceFile<'a> {
    pub original_filename: &'a str,
    pub content_type: &'a str,
    pub size_bytes: i64,
    pub sha256: &'a str,
    pub blob_path: &'a str,
}

/// A new piece of evidence, going into `folder`. `file` is `None` when it has
/// been named but no file has been provided yet.
///
/// The folder carries the case and the audience, so neither can be passed in
/// disagreeing with where the file actually lands.
pub struct NewEvidence<'a> {
    pub folder: &'a CaseFolder,
    pub name: &'a str,
    pub description: &'a str,
    pub file: Option<EvidenceFile<'a>>,
}

/// All evidence on a case that the caller may see, oldest first.
pub async fn get_case_evidence(
    case_id: &str,
    include_volunteer_only: bool,
) -> Result<Vec<Evidence>, sqlx::Error> {
    let rows = sqlx::query_as::<_, EvidenceRow>(&format!(
        "SELECT {COLUMNS} FROM evidence
         WHERE case_id = $1 AND ($2 OR visibility <> $3) ORDER BY seq ASC"
    ))
    .bind(case_id)
    .bind(include_volunteer_only)
    .bind(Visibility::VolunteerOnly.slug())
    .fetch_all(pool())
    .await?;

    Ok(rows.into_iter().map(EvidenceRow::into_evidence).collect())
}

/// A single piece of evidence by id, or `None` if it does not exist.
pub async fn get(evidence_id: &str) -> Result<Option<Evidence>, sqlx::Error> {
    let row =
        sqlx::query_as::<_, EvidenceRow>(&format!("SELECT {COLUMNS} FROM evidence WHERE id = $1"))
            .bind(evidence_id)
            .fetch_optional(pool())
            .await?;
    Ok(row.map(EvidenceRow::into_evidence))
}

/// Reserve an evidence id. Callers storing a file use this to name the blob
/// before the row exists, so the blob path is known up front and the row can be
/// written only after the bytes land (avoiding orphaned rows).
pub async fn reserve_id() -> Result<String, sqlx::Error> {
    ids::next(pool(), "e").await
}

/// Insert a piece of evidence with a pre-reserved id (see [`reserve_id`]),
/// auditing it.
pub async fn add(id: &str, added_by: &str, new: &NewEvidence<'_>) -> Result<(), sqlx::Error> {
    let mut tx = pool().begin().await?;
    insert(&mut tx, id, added_by, new).await?;
    tx.commit().await?;
    audit::record(
        pool(),
        audit::Entity::Case,
        &new.folder.case_id,
        added_by,
        "evidence",
        "",
        new.name,
    )
    .await
}

/// Insert a piece of evidence inside an existing transaction, so a case can be
/// created together with the files it starts out waiting on and either both land
/// or neither does. Returns the new id.
pub async fn insert_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    added_by: &str,
    new: &NewEvidence<'_>,
) -> Result<String, sqlx::Error> {
    let id = ids::next(&mut **tx, "e").await?;
    insert(tx, &id, added_by, new).await?;
    Ok(id)
}

async fn insert(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: &str,
    added_by: &str,
    new: &NewEvidence<'_>,
) -> Result<(), sqlx::Error> {
    let file = new.file.as_ref();
    sqlx::query(
        "INSERT INTO evidence
            (id, case_id, name, uploaded_by, uploaded_at, description,
             original_filename, content_type, size_bytes, sha256, blob_path,
             folder_id, visibility)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
    )
    .bind(id)
    .bind(&new.folder.case_id)
    .bind(new.name)
    .bind(added_by)
    .bind(now_stamp())
    .bind(new.description)
    .bind(file.map(|f| f.original_filename).unwrap_or_default())
    .bind(file.map(|f| f.content_type).unwrap_or_default())
    .bind(file.map(|f| f.size_bytes).unwrap_or_default())
    .bind(file.map(|f| f.sha256).unwrap_or_default())
    .bind(file.map(|f| f.blob_path).unwrap_or_default())
    .bind(&new.folder.id)
    .bind(new.folder.visibility.slug())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Put a file into an existing piece of evidence, auditing it.
///
/// The row's `name`, `description`, and folder are left alone: providing the
/// file answers what the entry was already asking for, it does not redefine the
/// entry. Returns the blob path the row held before (empty when it had no file),
/// so a replacement upload can clean up the blob it superseded.
pub async fn set_file(
    evidence_id: &str,
    uploaded_by: &str,
    file: &EvidenceFile<'_>,
) -> Result<String, sqlx::Error> {
    let row: Option<(String, String, String)> =
        sqlx::query_as("SELECT case_id, name, blob_path FROM evidence WHERE id = $1")
            .bind(evidence_id)
            .fetch_optional(pool())
            .await?;
    let Some((case_id, name, previous_blob)) = row else {
        return Ok(String::new());
    };

    sqlx::query(
        "UPDATE evidence SET uploaded_by = $2, uploaded_at = $3, original_filename = $4,
                content_type = $5, size_bytes = $6, sha256 = $7, blob_path = $8
         WHERE id = $1",
    )
    .bind(evidence_id)
    .bind(uploaded_by)
    .bind(now_stamp())
    .bind(file.original_filename)
    .bind(file.content_type)
    .bind(file.size_bytes)
    .bind(file.sha256)
    .bind(file.blob_path)
    .execute(pool())
    .await?;

    audit::record(
        pool(),
        audit::Entity::Case,
        &case_id,
        uploaded_by,
        "evidence",
        "",
        &name,
    )
    .await?;
    Ok(previous_blob)
}

/// The blob path backing a piece of evidence, or an empty string when it has no
/// file. `None` when the evidence does not exist on the case.
pub async fn blob_path(case_id: &str, evidence_id: &str) -> Result<Option<String>, sqlx::Error> {
    let path: Option<String> =
        sqlx::query_scalar("SELECT blob_path FROM evidence WHERE case_id = $1 AND id = $2")
            .bind(case_id)
            .bind(evidence_id)
            .fetch_optional(pool())
            .await?;
    Ok(path)
}

/// Move a piece of evidence into another folder, auditing it.
///
/// The destination folder supplies the new audience, so a move is also how a
/// file changes who can see it. The stored bytes are not involved: `blob_path`
/// names them by id, not by where they sit in the tree.
pub async fn move_to_folder(
    evidence_id: &str,
    folder: &CaseFolder,
    actor: &str,
) -> Result<(), sqlx::Error> {
    let name: Option<String> = sqlx::query_scalar(
        "UPDATE evidence SET folder_id = $2, visibility = $3
         WHERE id = $1 RETURNING name",
    )
    .bind(evidence_id)
    .bind(&folder.id)
    .bind(folder.visibility.slug())
    .fetch_optional(pool())
    .await?;
    let Some(name) = name else {
        return Err(sqlx::Error::RowNotFound);
    };
    audit::record(
        pool(),
        audit::Entity::Case,
        &folder.case_id,
        actor,
        "evidence",
        &name,
        &format!("{name} \u{2192} {}", folder.name),
    )
    .await
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
/// (empty when it had no file) so the caller can delete the blob too.
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
