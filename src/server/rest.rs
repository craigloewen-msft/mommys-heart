//! The dedicated JSON API (SSR only).
//!
//! Plain Axum routes under `/api/*` returning JSON. This is the single API
//! consumed by both the CRM website (via `crate::api_client`) and the embeddable
//! Squarespace chat widget (cross-origin, hence the CORS layer).

use axum::{
    extract::Path,
    http::{header, Method, StatusCode},
    routing::{get, post},
    Json, Router,
};
use tower_http::cors::{Any, CorsLayer};

use crate::server::{captcha, service};
use crate::types::{ChatRequest, ChatResponse, Contact, HealthResponse, VersionResponse};

/// Build the `/api/*` router. Generic over state so it can be merged into the
/// Leptos router (which carries `LeptosOptions` state); the handlers ignore it.
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/api/health", get(health))
        .route("/api/version", get(version))
        .route("/api/chat", post(chat))
        .route("/api/contacts", get(contacts))
        .route("/api/contacts/{id}", get(contact))
}

/// CORS policy for the API, driven by `ALLOWED_ORIGINS` (comma-separated, or
/// `*` for any origin) — parity with the legacy FastAPI CORSMiddleware.
pub fn cors_layer() -> CorsLayer {
    let origins = std::env::var("ALLOWED_ORIGINS").unwrap_or_else(|_| "*".into());
    let layer = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

    if origins.trim() == "*" {
        layer.allow_origin(Any)
    } else {
        let list: Vec<_> = origins
            .split(',')
            .filter_map(|o| o.trim().parse().ok())
            .collect();
        layer.allow_origin(list)
    }
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
    })
}

async fn version() -> Json<VersionResponse> {
    Json(service::version())
}

async fn contacts() -> Json<Vec<Contact>> {
    Json(service::list_contacts())
}

async fn contact(Path(id): Path<String>) -> Result<Json<Contact>, (StatusCode, String)> {
    service::get_contact(&id)
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, format!("Contact '{id}' not found")))
}

async fn chat(Json(req): Json<ChatRequest>) -> Result<Json<ChatResponse>, (StatusCode, String)> {
    let message = req.message.trim();
    if message.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Message cannot be empty".into()));
    }

    if let Some(secret) = service::turnstile_secret() {
        let ok = captcha::verify(&secret, req.captcha_token.as_deref(), None).await;
        if !ok {
            return Err((StatusCode::FORBIDDEN, "CAPTCHA verification failed".into()));
        }
    }

    Ok(Json(service::chat_reply(message)))
}
