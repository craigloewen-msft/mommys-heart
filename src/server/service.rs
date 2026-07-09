//! Server-side business logic (SSR only). Single source of truth shared by both
//! the dedicated REST API (`server::api`) and the UI data loaders
//! (`crate::api_client`, SSR branch).

use crate::server::comm_store::{repo, ConversationFilter, NewConversation, NewMessage};
use crate::server::{data, rag};
use crate::types::{
    AuthorKind, Channel, ChatResponse, Contact, Conversation, ConversationStatus,
    ConversationThread, Message, MessageDirection, VersionResponse,
};

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

// ---------------------------------------------------------------------------
// Communications management.
//
// Every web-chat turn is retained in the org-owned `comm_store` so history
// survives past the individual volunteer, and staff can pick up, reply to,
// reassign, and fold conversations into case records from the inbox console.
// ---------------------------------------------------------------------------

/// Handle one web-chat turn: retain the visitor's message, run the RAG
/// assistant, retain its answer, and return the answer plus the conversation id
/// the caller should echo back on the next turn.
///
/// Never hard-fails: retention is best-effort so the widget always gets a reply.
pub async fn record_web_chat_turn(message: &str, conversation_id: Option<String>) -> ChatResponse {
    let store = repo();

    // Find or create the org-owned conversation this turn belongs to.
    let conversation = match conversation_id
        .as_deref()
        .and_then(|id| store.get(id).map(|t| t.conversation))
    {
        Some(existing) => existing,
        None => store.create(NewConversation {
            channel: Channel::WebChat,
            subject: chat_subject(message),
            status: ConversationStatus::New,
            contact_id: None,
            case_id: None,
            assigned_volunteer_id: None,
        }),
    };

    // Retain the inbound visitor message before answering.
    store.append_message(
        &conversation.id,
        NewMessage {
            channel: Channel::WebChat,
            direction: MessageDirection::Inbound,
            author_kind: AuthorKind::Visitor,
            author_id: None,
            author_label: "Website visitor".into(),
            body: message.to_string(),
            sources: Vec::new(),
        },
    );

    // Run the assistant (guard dropped — no store lock is held across the await).
    let mut response = rag::answer(message).await;

    // Retain the assistant's outbound answer with its citations.
    store.append_message(
        &conversation.id,
        NewMessage {
            channel: Channel::WebChat,
            direction: MessageDirection::Outbound,
            author_kind: AuthorKind::Bot,
            author_id: None,
            author_label: "Mommy's Heart Assistant".into(),
            body: response.answer.clone(),
            sources: response.sources.clone(),
        },
    );

    response.conversation_id = Some(conversation.id);
    response
}

/// A short subject line derived from the visitor's first message.
fn chat_subject(message: &str) -> String {
    let trimmed = message.trim().replace('\n', " ");
    if trimmed.is_empty() {
        return "Website chat".into();
    }
    if trimmed.chars().count() > 60 {
        let truncated: String = trimmed.chars().take(60).collect();
        format!("{truncated}…")
    } else {
        trimmed
    }
}

pub fn list_conversations(filter: &ConversationFilter) -> Vec<Conversation> {
    repo().list(filter)
}

pub fn get_conversation(id: &str) -> Option<ConversationThread> {
    repo().get(id)
}

/// Post a staff/volunteer reply into a conversation and move it to
/// `AwaitingReply` (we've responded; the ball is with the visitor).
pub fn reply_to_conversation(
    id: &str,
    body: &str,
    author_id: Option<String>,
    author_label: Option<String>,
) -> Option<Message> {
    let store = repo();
    let thread = store.get(id)?;
    let msg = store.append_message(
        id,
        NewMessage {
            channel: thread.conversation.channel,
            direction: MessageDirection::Outbound,
            author_kind: AuthorKind::Staff,
            author_id,
            author_label: author_label.unwrap_or_else(|| "Staff".into()),
            body: body.to_string(),
            sources: Vec::new(),
        },
    )?;
    if thread.conversation.status != ConversationStatus::Closed {
        store.set_status(id, ConversationStatus::AwaitingReply);
    }
    Some(msg)
}

pub fn assign_conversation(id: &str, volunteer_id: Option<String>) -> Option<Conversation> {
    repo().assign(id, volunteer_id)
}

pub fn link_conversation_case(
    id: &str,
    case_id: Option<String>,
    contact_id: Option<String>,
) -> Option<Conversation> {
    repo().link_case(id, case_id, contact_id)
}

pub fn set_conversation_status(id: &str, status: ConversationStatus) -> Option<Conversation> {
    repo().set_status(id, status)
}

pub fn conversations_for_case(case_id: &str) -> Vec<Conversation> {
    repo().conversations_for_case(case_id)
}

pub fn conversations_for_contact(contact_id: &str) -> Vec<Conversation> {
    repo().conversations_for_contact(contact_id)
}

/// Offboard a volunteer: clear them from every conversation they own while
/// keeping all history, returning how many conversations were reassigned.
pub fn offboard_volunteer(volunteer_id: &str) -> usize {
    repo().unassign_volunteer(volunteer_id)
}
