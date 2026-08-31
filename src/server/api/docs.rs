//! `GET /api/docs/{filename}` (styled HTML viewer) and
//! `GET /api/docs/{filename}/download` (original `.docx`).
//!
//! These back the chat widget's per-source "Read more" / "Download" links.

use axum::{
    body::Body,
    extract::Path,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};

use crate::server::docs;

pub fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/api/docs/{filename}", get(view))
        .route("/api/docs/{filename}/download", get(download))
}

/// Render a `.docx` as a styled HTML page.
async fn view(Path(filename): Path<String>) -> Response {
    match docs::render_docx_to_html(&filename) {
        Some(html) => Html(html).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            format!("Document '{filename}' not found"),
        )
            .into_response(),
    }
}

/// Serve the original `.docx` file as a download.
async fn download(Path(filename): Path<String>) -> Response {
    match docs::read_docx_bytes(&filename) {
        Some((bytes, name)) => {
            let disposition = format!("attachment; filename=\"{}\"", name.replace('"', ""));
            (
                [
                    (header::CONTENT_TYPE, docs::DOCX_MIME.to_string()),
                    (header::CONTENT_DISPOSITION, disposition),
                ],
                Body::from(bytes),
            )
                .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            format!("Document '{filename}' not found"),
        )
            .into_response(),
    }
}
