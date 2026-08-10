//! `POST /api/chat` — the RAG chatbot endpoint (widget + app UI), plus
//! `POST /api/reingest` to rebuild the vector store on demand.

use axum::{http::StatusCode, routing::post, Json, Router};
use serde::{Deserialize, Serialize};

use crate::server::rag::ChatResponse;
use crate::server::{captcha, service};

/// `POST /api/chat` request body.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    #[serde(default)]
    pub captcha_token: Option<String>,
    /// Existing conversation to continue. Omitted on the first message; the
    /// server echoes back an id the widget/UI can send on later turns.
    #[serde(default)]
    pub conversation_id: Option<String>,
}

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
