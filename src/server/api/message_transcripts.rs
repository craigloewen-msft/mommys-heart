//! Authenticated CSV transcript download route.

use axum::{
    extract::Path,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};

use crate::server::auth::AuthUser;
use crate::server::db::messages;
use crate::server::permissions::{require_channel, require_operations_admin};
use crate::server_fns::capabilities::CaseCapability;

fn safe_filename(value: &str) -> String {
    let name: String = value
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
        .take(180)
        .collect();
    if name.is_empty() {
        "message-transcript.csv".to_string()
    } else {
        name
    }
}

pub fn install<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.route("/api/channels/{channel_id}/transcript.csv", get(download))
}

async fn download(AuthUser(user): AuthUser, Path(channel_id): Path<String>) -> Response {
    if require_operations_admin(&user).is_err() {
        return (StatusCode::NOT_FOUND, "Transcript not found.").into_response();
    }
    let channel = match require_channel(&user, &channel_id, CaseCapability::ViewCase).await {
        Ok(channel) => channel,
        Err(_) => return (StatusCode::NOT_FOUND, "Transcript not found.").into_response(),
    };
    match messages::export_channel(&channel, &user.full_name()).await {
        Ok(export) => (
            [
                (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
                (
                    header::CONTENT_DISPOSITION,
                    format!(
                        "attachment; filename=\"{}\"",
                        safe_filename(&export.filename)
                    ),
                ),
                (header::CACHE_CONTROL, "no-store".to_string()),
            ],
            export.csv,
        )
            .into_response(),
        Err(error) => {
            tracing::warn!("message transcript export failed for {channel_id}: {error}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Could not export this transcript.",
            )
                .into_response()
        }
    }
}
