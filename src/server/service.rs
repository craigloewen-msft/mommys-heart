//! Server-side business logic (SSR only). Single source of truth shared by both
//! the dedicated REST API (`server::rest`) and the UI data loaders
//! (`crate::api_client`, SSR branch).

use crate::server::data;
use crate::types::{ChatResponse, Contact, SourceType, VersionResponse};

pub fn list_contacts() -> Vec<Contact> {
    data::contacts()
}

pub fn get_contact(id: &str) -> Option<Contact> {
    data::contacts().into_iter().find(|c| c.id == id)
}

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

/// Chat reply.
///
/// NOTE: this is a STUB. It preserves the legacy response contract
/// (`answer`, `sources`, `source_type`) but returns a placeholder answer.
/// Porting the real RAG pipeline (Azure OpenAI embeddings + vector search) is a
/// later phase.
pub fn chat_reply(message: &str) -> ChatResponse {
    ChatResponse {
        answer: format!(
            "Thanks for your message — the chat API is not wired up to the RAG \
             pipeline yet. This is a placeholder response so the UI and widget can \
             integrate against the real /api/chat contract. You asked: \"{message}\""
        ),
        sources: Vec::new(),
        source_type: SourceType::GeneralKnowledge,
    }
}
