//! `POST /api/chat` — the RAG chatbot endpoint (widget + CRM UI), plus
//! `POST /api/reingest` to rebuild the vector store on demand.

use axum::{http::StatusCode, routing::post, Json, Router};

use crate::server::{captcha, service};
use crate::types::{ChatRequest, ChatResponse};

pub fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/api/chat", post(chat))
        .route("/api/reingest", post(reingest))
}

async fn chat(Json(req): Json<ChatRequest>) -> Result<Json<ChatResponse>, (StatusCode, String)> {
    let message = req.message.trim();
    if message.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Message cannot be empty".into()));
    }

    // CAPTCHA gate (only enforced when a Turnstile secret is configured).
    if let Some(secret) = service::turnstile_secret() {
        let ok = captcha::verify(&secret, req.captcha_token.as_deref(), None).await;
        if !ok {
            return Err((StatusCode::FORBIDDEN, "CAPTCHA verification failed".into()));
        }
    }

    // Run the RAG assistant and echo back the conversation id the caller sent.
    Ok(Json(service::chat(message, req.conversation_id).await))
}

async fn reingest() -> Result<Json<crate::server::rag::IngestStats>, (StatusCode, String)> {
    crate::server::rag::reingest()
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))
}
