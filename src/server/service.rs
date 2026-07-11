//! Server-side business logic (SSR only). Single source of truth shared by both
//! the dedicated REST API (`server::api`) and the UI data loaders
//! (`crate::api_client`, SSR branch).

use crate::server::api::version::VersionResponse;
use crate::server::rag;
use crate::server::rag::ChatResponse;

/// The Turnstile secret, if CAPTCHA enforcement is configured.
pub fn turnstile_secret() -> Option<String> {
    std::env::var("TURNSTILE_SECRET_KEY")
        .ok()
        .filter(|s| !s.is_empty())
}

pub fn version() -> VersionResponse {
    VersionResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        chat_model: std::env::var("AZURE_OPENAI_CHAT_DEPLOYMENT")
            .unwrap_or_else(|_| "gpt-4o".into()),
        embedding_model: std::env::var("AZURE_OPENAI_EMBEDDING_DEPLOYMENT")
            .unwrap_or_else(|_| "text-embedding-ada-002".into()),
        captcha_enabled: turnstile_secret().is_some(),
    }
}

/// Handle one web-chat turn: run the RAG assistant and return its answer,
/// echoing back the conversation id the caller provided (if any) so the widget
/// can keep a thread together client-side.
///
/// Never hard-fails: the widget always gets a reply.
pub async fn chat(message: &str, conversation_id: Option<String>) -> ChatResponse {
    let mut response = rag::answer(message).await;
    response.conversation_id = conversation_id;
    response
}
