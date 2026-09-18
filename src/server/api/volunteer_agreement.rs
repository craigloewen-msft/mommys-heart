//! `GET /api/volunteers/{user_id}/volunteer-agreement.pdf`: the accepted
//! volunteer agreement as a generated PDF, for the volunteer themselves and for
//! operations admins.

use axum::{
    extract::Path,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};

use crate::server::auth::AuthUser;
use crate::server::db::{audit, users, volunteers};
use crate::server::permissions::require_operations_admin;
use crate::server::volunteer_agreement_pdf;

/// Reduce a name to something safe to put in a filename.
fn safe_filename(name: &str) -> String {
    let slug: String = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug.trim_matches('-').replace("--", "-");
    if slug.is_empty() {
        "volunteer-agreement.pdf".to_string()
    } else {
        format!("volunteer-agreement-{}.pdf", &slug[..slug.len().min(100)])
    }
}

pub fn install<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.route(
        "/api/volunteers/{user_id}/volunteer-agreement.pdf",
        get(download),
    )
}

async fn download(AuthUser(viewer): AuthUser, Path(user_id): Path<String>) -> Response {
    let missing = || (StatusCode::NOT_FOUND, "Agreement not found.").into_response();
    let is_self = viewer.id == user_id;
    if !is_self && require_operations_admin(&viewer).is_err() {
        return missing();
    }
    let Ok(Some(user)) = users::get(&user_id).await else {
        return missing();
    };
    let Ok(Some(volunteer)) = volunteers::get(&user_id).await else {
        return missing();
    };
    if !volunteer.has_agreement() {
        return missing();
    }

    let name = user.full_name();
    let bytes = volunteer_agreement_pdf::render(&name, &user.email, &volunteer);

    // An admin reading someone else's signed paperwork is recorded; reading your
    // own is not, in the same spirit as the SSN reveal.
    if !is_self {
        if let Err(error) = audit::record(
            crate::server::db::pool(),
            audit::Entity::User,
            &user_id,
            &viewer.full_name(),
            "volunteer_agreement_pdf",
            "",
            "downloaded",
        )
        .await
        {
            tracing::warn!("could not record volunteer agreement download: {error}");
        }
    }

    (
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", safe_filename(&name)),
            ),
            (header::CACHE_CONTROL, "no-store".to_string()),
        ],
        bytes,
    )
        .into_response()
}
