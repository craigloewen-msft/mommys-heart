//! Evidence server functions and the companion file endpoints.
//!
//! Evidence is its own concept: a case sub-resource whose *bytes* live in Azure
//! Blob Storage while its metadata lives in Postgres. This module owns the whole
//! surface:
//!
//! * [`upload_evidence`] — a multipart [`#[server]`](macro@leptos::server)
//!   function that validates every byte (size + content-sniffed type allowlist),
//!   hashes it, streams it to Blob Storage, and records a file-backed row.
//! * [`add_case_file`] — add a file entry with nothing in it yet, for a
//!   document somebody is expected to provide later.
//! * [`delete_case_evidence`] — the delete (which also removes the backing
//!   blob).
//! * [`routes`] — a plain Axum **GET** download route. A binary file download
//!   with a `Content-Disposition` attachment is fundamentally an HTTP GET that
//!   does not map onto a `#[server]` function (the streaming codec is POST-only
//!   and server-fn URLs are hashed), so — like the chat widget — it stays a
//!   plain route, but lives here with the rest of the evidence surface rather
//!   than in the generic `server::api` aggregator.

use leptos::prelude::*;
use leptos::server_fn::codec::{MultipartData, MultipartFormData};
use serde::{Deserialize, Serialize};

use crate::helpers::visibility::Visibility;

/// A file on a case: a named entry, plus the bytes behind it once they exist.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Evidence {
    pub id: String,
    pub name: String,
    /// The case this evidence belongs to.
    pub case_id: String,
    /// Display name of the user who uploaded it.
    pub uploaded_by: String,
    /// Human-readable upload timestamp (mock).
    pub uploaded_at: String,
    /// Free-text extra information / description.
    #[serde(default)]
    pub description: String,
    /// The original uploaded file name (empty until a file is provided).
    #[serde(default)]
    pub original_filename: String,
    /// Validated MIME type of the file (empty until a file is provided).
    #[serde(default)]
    pub content_type: String,
    /// Size of the file in bytes (0 until a file is provided).
    #[serde(default)]
    pub size_bytes: i64,
    /// Hex SHA-256 of the file bytes (empty until a file is provided).
    #[serde(default)]
    pub sha256: String,
    /// Whether this evidence has a downloadable file backing it.
    #[serde(default)]
    pub has_file: bool,
    /// Display grouping heading, e.g. "Intake". Free text; empty groups the row
    /// under [`sections::DEFAULT_LABEL`](crate::helpers::sections::DEFAULT_LABEL).
    #[serde(default)]
    pub section: String,
    /// Who may see this file.
    #[serde(default)]
    pub visibility: Visibility,
}

/// Maximum accepted evidence file size (25 MB). Enforced by an explicit byte
/// check in the upload handler (the router body limit allows a little slack for
/// multipart overhead).
pub const MAX_SIZE_BYTES: usize = 25 * 1024 * 1024;

/// Content-sniffed MIME types accepted for evidence uploads: PDFs, common
/// images, and Office documents (modern + legacy). The declared type is ignored;
/// only the bytes' true type (via `infer`) is trusted, defeating extension or
/// `Content-Type` spoofing.
#[cfg(feature = "ssr")]
const ALLOWED_MIME: &[&str] = &[
    "application/pdf",
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/webp",
    // Office Open XML (docx / xlsx / pptx)
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    // Legacy Office (doc / xls / ppt)
    "application/msword",
    "application/vnd.ms-excel",
    "application/vnd.ms-powerpoint",
];

/// Upload a file as evidence on a case (requires the `UploadEvidence`
/// capability). Returns the evidence id the bytes ended up on.
///
/// The multipart body carries the `file` plus either:
///
/// * `evidence_id` — put the file into an existing entry, typically one created
///   with the case ("Intake letter" and friends). The entry keeps its name,
///   section, and visibility: providing the file answers what the entry was
///   already asking for rather than redefining it, and the case it belongs to is
///   read from the database rather than taken from the request.
/// * `case_id`, plus optional `name`, `description`, `section`, and
///   `visibility` — create a new row.
///
/// The server re-validates every byte regardless of what the client claims:
/// size cap, content-sniffed type allowlist, and a SHA-256 integrity hash. The
/// bytes are streamed to Blob Storage and only then is the DB row written; any
/// failure after the blob lands triggers a compensating delete so an upload
/// never leaves an orphaned blob or row.
#[server(prefix = "/api", input = MultipartFormData)]
pub async fn upload_evidence(data: MultipartData) -> Result<String, ServerFnError> {
    use crate::server::db::evidence as db;
    use crate::server::permissions::{require_cap, require_user, require_visibility};
    use crate::server::storage;
    use crate::server_fns::capabilities::CaseCapability;
    use sha2::{Digest, Sha256};
    use std::collections::HashMap;

    let user = require_user().await?;

    // On the server side `into_inner()` is always `Some`.
    let mut multipart = data
        .into_inner()
        .ok_or_else(|| ServerFnError::new("Malformed upload."))?;

    let mut case_id = String::new();
    let mut target_id = String::new();
    let mut name = String::new();
    let mut description = String::new();
    let mut section = String::new();
    let mut visibility_slug = String::new();
    let mut filename: Option<String> = None;
    let mut bytes: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ServerFnError::new(format!("Malformed upload: {e}")))?
    {
        let field_name = field.name().map(str::to_owned);
        let file_name = field.file_name().map(str::to_owned);
        match field_name.as_deref() {
            Some("file") => {
                filename = Some(file_name.unwrap_or_else(|| "upload".to_string()));
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| ServerFnError::new(format!("Could not read file: {e}")))?;
                bytes = Some(data.to_vec());
            }
            Some("case_id") => {
                case_id = field.text().await.unwrap_or_default();
            }
            Some("evidence_id") => {
                target_id = field.text().await.unwrap_or_default();
            }
            Some("name") => {
                name = field.text().await.unwrap_or_default();
            }
            Some("description") => {
                description = field.text().await.unwrap_or_default();
            }
            Some("section") => {
                section = field.text().await.unwrap_or_default();
            }
            Some("visibility") => {
                visibility_slug = field.text().await.unwrap_or_default();
            }
            _ => {
                let _ = field.bytes().await;
            }
        }
    }

    // An upload into an existing row takes everything about *where* it lands
    // from that row, never from the request: the posted case id is not trusted.
    let target_id = target_id.trim().to_string();
    let target = if target_id.is_empty() {
        None
    } else {
        Some(
            db::get(&target_id)
                .await
                .map_err(ServerFnError::new)?
                .ok_or_else(|| ServerFnError::new("That file no longer exists."))?,
        )
    };

    let (case_id, section, visibility) = match &target {
        Some(existing) => (
            existing.case_id.clone(),
            existing.section.clone(),
            existing.visibility,
        ),
        None => {
            let case_id = case_id.trim().to_string();
            if case_id.is_empty() {
                return Err(ServerFnError::new("Missing case id in upload."));
            }
            let visibility = if visibility_slug.trim().is_empty() {
                Visibility::default()
            } else {
                Visibility::from_slug(visibility_slug.trim())
                    .ok_or_else(|| ServerFnError::new("Unknown visibility."))?
            };
            (case_id, section.trim().to_string(), visibility)
        }
    };

    require_cap(&user, &case_id, CaseCapability::UploadEvidence).await?;
    require_visibility(&user, visibility)?;

    if !storage::is_configured() {
        return Err(ServerFnError::new(
            "Evidence storage is not configured on this server.",
        ));
    }

    let (filename, bytes) = match (filename, bytes) {
        (Some(f), Some(b)) => (f, b),
        _ => return Err(ServerFnError::new("No file was provided in the upload.")),
    };

    // Content-based validation: size + true (sniffed) type allowlist.
    let content_type = validate_bytes(&bytes).map_err(ServerFnError::new)?;

    let filename = sanitize_filename(&filename);
    // An upload into an existing entry keeps that entry's name; a fresh upload
    // is named by the uploader, falling back to the file's own name.
    let display_name = match &target {
        Some(existing) => existing.name.clone(),
        None if !name.trim().is_empty() => name.trim().to_string(),
        None => filename.clone(),
    };

    let sha256 = hex::encode(Sha256::digest(&bytes));
    let size_bytes = bytes.len() as i64;

    // Reserve the id first so the blob is named before the row exists; on any
    // failure after the blob is written we delete it, so a failed DB write never
    // leaves an orphaned blob and a failed upload never leaves an orphaned row.
    let evidence_id = match &target {
        Some(existing) => existing.id.clone(),
        None => db::reserve_id().await.map_err(ServerFnError::new)?,
    };
    let blob_path = storage::blob_path(&case_id, &evidence_id);

    let mut metadata = HashMap::new();
    metadata.insert("case_id".to_string(), case_id.clone());
    metadata.insert("evidence_id".to_string(), evidence_id.clone());
    metadata.insert("uploaded_by".to_string(), user.full_name());
    metadata.insert("sha256".to_string(), sha256.clone());

    storage::put(&blob_path, bytes, content_type, metadata)
        .await
        .map_err(|e| ServerFnError::new(format!("Storage error: {e}")))?;

    let file = db::EvidenceFile {
        original_filename: &filename,
        content_type,
        size_bytes,
        sha256: &sha256,
        blob_path: &blob_path,
    };
    let written = match &target {
        Some(_) => db::set_file(&evidence_id, &user.full_name(), &file)
            .await
            .map(|previous_blob| {
                // Re-uploading over an entry that already had a file reuses the
                // same blob path, so only a path that actually changed leaves
                // bytes behind.
                if !previous_blob.is_empty() && previous_blob != blob_path {
                    let stale = previous_blob;
                    tokio::spawn(async move {
                        let _ = storage::delete(&stale).await;
                    });
                }
            }),
        None => {
            db::add(
                &evidence_id,
                &user.full_name(),
                &db::NewEvidence {
                    case_id: &case_id,
                    name: &display_name,
                    description: description.trim(),
                    section: &section,
                    visibility,
                    file: Some(file),
                },
            )
            .await
        }
    };
    if let Err(e) = written {
        // Compensating cleanup: the bytes are stored but the row failed, so drop
        // the now-orphaned blob before reporting the error.
        let _ = storage::delete(&blob_path).await;
        return Err(ServerFnError::new(format!("Database error: {e}")));
    }

    tracing::info!(
        "evidence uploaded: case={case_id} id={evidence_id} file='{filename}' \
         type={content_type} size={size_bytes}"
    );
    crate::server::notifications::notify_case(
        case_id,
        user.id.clone(),
        user.full_name(),
        crate::server_fns::settings::NotificationKind::EvidenceChanged,
        format!("uploaded \"{display_name}\""),
        crate::server::notifications::audience_for(visibility),
    );
    Ok(evidence_id)
}

/// Add a file to a case without providing the file itself (requires the
/// `UploadEvidence` capability) — a named entry listing a document the case is
/// waiting on, which someone uploads into later. Returns the new evidence id.
///
/// This is the same thing a new case is created with; making it available on
/// demand means the list of expected paperwork is not frozen at case creation.
#[server(prefix = "/api")]
pub async fn add_case_file(
    case_id: String,
    name: String,
    description: String,
    section: String,
    visibility: Visibility,
) -> Result<String, ServerFnError> {
    use crate::server::db::{evidence as db, pool};
    use crate::server::permissions::{require_cap, require_user, require_visibility};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    require_cap(&user, &case_id, CaseCapability::UploadEvidence).await?;
    require_visibility(&user, visibility)?;

    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ServerFnError::new("A file needs a name."));
    }

    let mut tx = pool().begin().await.map_err(ServerFnError::new)?;
    let id = db::insert_in(
        &mut tx,
        &user.full_name(),
        &db::NewEvidence {
            case_id: &case_id,
            name: &name,
            description: description.trim(),
            section: section.trim(),
            visibility,
            file: None,
        },
    )
    .await
    .map_err(ServerFnError::new)?;
    tx.commit().await.map_err(ServerFnError::new)?;

    crate::server::notifications::notify_case(
        case_id,
        user.id.clone(),
        user.full_name(),
        crate::server_fns::settings::NotificationKind::EvidenceChanged,
        format!("asked for \"{name}\""),
        crate::server::notifications::audience_for(visibility),
    );
    Ok(id)
}

/// Remove a file from a case (requires the `DeleteEvidence` capability). Also
/// deletes the backing blob when a file had been provided.
#[server(prefix = "/api")]
pub async fn delete_case_evidence(
    case_id: String,
    evidence_id: String,
) -> Result<(), ServerFnError> {
    use crate::server::db::evidence as db;
    use crate::server::permissions::{require_cap, require_user, require_visibility};
    use crate::server::storage;
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    require_cap(&user, &case_id, CaseCapability::DeleteEvidence).await?;
    let existing = db::get(&evidence_id)
        .await
        .map_err(ServerFnError::new)?
        .filter(|e| e.case_id == case_id)
        .ok_or_else(|| ServerFnError::new("That file no longer exists."))?;
    require_visibility(&user, existing.visibility)?;
    let blob_path = db::delete(&case_id, &evidence_id)
        .await
        .map_err(ServerFnError::new)?;

    // Best-effort blob cleanup: the row is already gone, so a storage hiccup
    // must not fail the operation. A leftover blob is orphaned, not dangerous.
    if !blob_path.is_empty() {
        if let Err(e) = storage::delete(&blob_path).await {
            tracing::warn!("failed to delete evidence blob '{blob_path}': {e}");
        }
    }
    crate::server::notifications::notify_case(
        case_id,
        user.id.clone(),
        user.full_name(),
        crate::server_fns::settings::NotificationKind::EvidenceChanged,
        format!("removed \"{}\"", existing.name),
        crate::server::notifications::audience_for(existing.visibility),
    );
    Ok(())
}

/// Validate size and content-sniffed type, returning the canonical MIME type to
/// store (derived from the bytes, never from the client's declared type).
#[cfg(feature = "ssr")]
fn validate_bytes(bytes: &[u8]) -> Result<&'static str, String> {
    if bytes.is_empty() {
        return Err("The uploaded file is empty.".to_string());
    }
    if bytes.len() > MAX_SIZE_BYTES {
        return Err(format!(
            "File is too large ({:.1} MB); the limit is {} MB.",
            bytes.len() as f64 / (1024.0 * 1024.0),
            MAX_SIZE_BYTES / (1024 * 1024)
        ));
    }
    let kind = infer::get(bytes).ok_or_else(|| {
        "Unrecognized file type. Allowed: PDF, images, and Office documents.".to_string()
    })?;
    let mime = kind.mime_type();
    if ALLOWED_MIME.contains(&mime) {
        Ok(mime)
    } else {
        Err(format!(
            "File type '{mime}' is not allowed. Allowed: PDF, images, and Office documents."
        ))
    }
}

/// Reduce a client-supplied filename to a safe display/download name: strip any
/// path components and control characters, and cap the length.
#[cfg(feature = "ssr")]
pub(crate) fn sanitize_filename(raw: &str) -> String {
    let base = raw
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(raw)
        .trim()
        .replace(['\r', '\n', '\0'], "");
    let base = base.trim_matches('.').to_string();
    let cleaned = if base.is_empty() {
        "upload".to_string()
    } else {
        base
    };
    cleaned.chars().take(255).collect()
}

/// Validate a signed agreement as a genuine Word Open XML package.
#[cfg(feature = "ssr")]
pub(crate) fn validate_docx(bytes: &[u8]) -> Result<&'static str, String> {
    use std::io::Cursor;

    if bytes.is_empty() {
        return Err("The signed agreement is empty.".to_string());
    }
    if bytes.len() > MAX_SIZE_BYTES {
        return Err(format!(
            "The signed agreement is too large ({:.1} MB); the limit is {} MB.",
            bytes.len() as f64 / (1024.0 * 1024.0),
            MAX_SIZE_BYTES / (1024 * 1024)
        ));
    }
    let archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| "The signed agreement must be a valid .docx file.".to_string())?;
    let has_content_types = archive.file_names().any(|name| name == "[Content_Types].xml");
    let has_relationships = archive.file_names().any(|name| name == "_rels/.rels");
    let has_document = archive.file_names().any(|name| name == "word/document.xml");
    if !has_content_types || !has_relationships || !has_document {
        return Err("The signed agreement must be a valid .docx file.".to_string());
    }
    Ok("application/vnd.openxmlformats-officedocument.wordprocessingml.document")
}

// ---------------------------------------------------------------------------
// Download route (plain Axum GET — see the module docs for why it is not a
// `#[server]` function).
// ---------------------------------------------------------------------------

#[cfg(feature = "ssr")]
mod download {
    use axum::{
        body::Body,
        extract::{DefaultBodyLimit, Path},
        http::{header, StatusCode},
        response::{IntoResponse, Response},
        routing::get,
        Router,
    };

    use crate::server::auth::AuthUser;
    use crate::server::db::evidence as db;
    use crate::server::permissions::{require_cap, require_visibility};
    use crate::server::storage;
    use crate::server_fns::capabilities::CaseCapability;

    /// Wire the whole evidence HTTP surface into the app router: merge the binary
    /// download **GET** route and raise the request body limit so multipart
    /// uploads (25 MB + multipart overhead) get past Axum's small default body
    /// limit. The upload handler still enforces the real 25 MB cap on the bytes.
    ///
    /// Keeping this here means `main.rs` doesn't need to know evidence's route
    /// paths or size limits — the whole surface is owned by this module.
    pub fn install<S>(app: Router<S>) -> Router<S>
    where
        S: Clone + Send + Sync + 'static,
    {
        app.merge(routes())
            .layer(DefaultBodyLimit::max(super::MAX_SIZE_BYTES + 1024 * 1024))
    }

    /// The evidence download route, ready to be merged into the app router.
    pub fn routes<S>() -> Router<S>
    where
        S: Clone + Send + Sync + 'static,
    {
        Router::new().route(
            "/api/cases/{case_id}/evidence/{evidence_id}/download",
            get(download),
        )
    }

    /// `GET /api/cases/{case_id}/evidence/{evidence_id}/download` — stream the
    /// stored file back with a download disposition (requires `ViewEvidence`)
    async fn download(
        AuthUser(user): AuthUser,
        Path((case_id, evidence_id)): Path<(String, String)>,
    ) -> Response {
        if let Err(e) = require_cap(&user, &case_id, CaseCapability::ViewEvidence).await {
            return (StatusCode::FORBIDDEN, e.to_string()).into_response();
        }
        match db::get(&evidence_id).await {
            Ok(Some(e)) if e.case_id == case_id => {
                if let Err(e) = require_visibility(&user, e.visibility) {
                    return (StatusCode::FORBIDDEN, e.to_string()).into_response();
                }
            }
            Ok(_) => {
                return (StatusCode::NOT_FOUND, "Evidence not found.".to_string()).into_response()
            }
            Err(e) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {e}"))
                    .into_response()
            }
        }

        let blob_path = match db::blob_path(&case_id, &evidence_id).await {
            Ok(Some(path)) if !path.is_empty() => path,
            Ok(Some(_)) => {
                return (
                    StatusCode::NOT_FOUND,
                    "This evidence entry has no downloadable file.".to_string(),
                )
                    .into_response()
            }
            Ok(None) => {
                return (StatusCode::NOT_FOUND, "Evidence not found.".to_string()).into_response()
            }
            Err(e) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {e}"))
                    .into_response()
            }
        };

        let (filename, content_type) = db::download_meta(&case_id, &evidence_id)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| (evidence_id.clone(), "application/octet-stream".to_string()));

        match storage::get(&blob_path).await {
            Ok(bytes) => {
                let disposition = format!("attachment; filename=\"{}\"", filename.replace('"', ""));
                (
                    [
                        (header::CONTENT_TYPE, content_type),
                        (header::CONTENT_DISPOSITION, disposition),
                    ],
                    Body::from(bytes),
                )
                    .into_response()
            }
            Err(e) => (StatusCode::BAD_GATEWAY, format!("storage error: {e}")).into_response(),
        }
    }
}

#[cfg(feature = "ssr")]
pub use download::install;
