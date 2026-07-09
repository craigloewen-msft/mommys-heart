//! Typed client the CRM website uses to talk to the dedicated API.
//!
//! Compiled for both targets. During SSR the functions call the server service
//! layer directly (no self-HTTP round-trip); in the browser they issue real
//! HTTP requests to `/api/*` — the same endpoints external consumers (the
//! Squarespace widget) use.

use crate::types::{
    ChatRequest, ChatResponse, Contact, Conversation, ConversationStatus, ConversationThread,
    Message, VersionResponse,
};

// ---------------------------------------------------------------------------
// SSR branch — call the service layer directly.
// ---------------------------------------------------------------------------
#[cfg(feature = "ssr")]
pub async fn get_contacts() -> Result<Vec<Contact>, String> {
    Ok(crate::server::service::list_contacts())
}

#[cfg(feature = "ssr")]
pub async fn get_contact(id: String) -> Result<Contact, String> {
    crate::server::service::get_contact(&id).ok_or_else(|| format!("Contact '{id}' not found"))
}

#[cfg(feature = "ssr")]
pub async fn get_version() -> Result<VersionResponse, String> {
    Ok(crate::server::service::version())
}

#[cfg(feature = "ssr")]
pub async fn send_chat(req: ChatRequest) -> Result<ChatResponse, String> {
    Ok(crate::server::service::record_web_chat_turn(req.message.trim(), req.conversation_id).await)
}

#[cfg(feature = "ssr")]
pub async fn list_conversations() -> Result<Vec<Conversation>, String> {
    Ok(crate::server::service::list_conversations(
        &Default::default(),
    ))
}

#[cfg(feature = "ssr")]
pub async fn get_conversation(id: String) -> Result<ConversationThread, String> {
    crate::server::service::get_conversation(&id)
        .ok_or_else(|| format!("Conversation '{id}' not found"))
}

#[cfg(feature = "ssr")]
pub async fn conversations_for_case(case_id: String) -> Result<Vec<Conversation>, String> {
    Ok(crate::server::service::conversations_for_case(&case_id))
}

#[cfg(feature = "ssr")]
pub async fn conversations_for_contact(contact_id: String) -> Result<Vec<Conversation>, String> {
    Ok(crate::server::service::conversations_for_contact(
        &contact_id,
    ))
}

#[cfg(feature = "ssr")]
pub async fn reply_conversation(
    id: String,
    body: String,
    author_id: Option<String>,
    author_label: Option<String>,
) -> Result<Message, String> {
    crate::server::service::reply_to_conversation(&id, body.trim(), author_id, author_label)
        .ok_or_else(|| format!("Conversation '{id}' not found"))
}

#[cfg(feature = "ssr")]
pub async fn assign_conversation(
    id: String,
    volunteer_id: Option<String>,
) -> Result<Conversation, String> {
    crate::server::service::assign_conversation(&id, volunteer_id)
        .ok_or_else(|| format!("Conversation '{id}' not found"))
}

#[cfg(feature = "ssr")]
pub async fn link_conversation_case(
    id: String,
    case_id: Option<String>,
    contact_id: Option<String>,
) -> Result<Conversation, String> {
    crate::server::service::link_conversation_case(&id, case_id, contact_id)
        .ok_or_else(|| format!("Conversation '{id}' not found"))
}

#[cfg(feature = "ssr")]
pub async fn set_conversation_status(
    id: String,
    status: ConversationStatus,
) -> Result<Conversation, String> {
    crate::server::service::set_conversation_status(&id, status)
        .ok_or_else(|| format!("Conversation '{id}' not found"))
}

#[cfg(feature = "ssr")]
pub async fn offboard_volunteer(volunteer_id: String) -> Result<usize, String> {
    Ok(crate::server::service::offboard_volunteer(&volunteer_id))
}

// ---------------------------------------------------------------------------
// Browser branch — fetch the dedicated API over HTTP.
// ---------------------------------------------------------------------------
#[cfg(not(feature = "ssr"))]
async fn get_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, String> {
    let resp = gloo_net::http::Request::get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.json::<T>().await.map_err(|e| e.to_string())
}

#[cfg(not(feature = "ssr"))]
async fn post_json<B: serde::Serialize, T: serde::de::DeserializeOwned>(
    url: &str,
    body: &B,
) -> Result<T, String> {
    let resp = gloo_net::http::Request::post(url)
        .json(body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.json::<T>().await.map_err(|e| e.to_string())
}

#[cfg(not(feature = "ssr"))]
pub async fn get_contacts() -> Result<Vec<Contact>, String> {
    send_wrapper::SendWrapper::new(async move { get_json("/api/contacts").await }).await
}

#[cfg(not(feature = "ssr"))]
pub async fn get_contact(id: String) -> Result<Contact, String> {
    send_wrapper::SendWrapper::new(async move { get_json(&format!("/api/contacts/{id}")).await })
        .await
}

#[cfg(not(feature = "ssr"))]
pub async fn get_version() -> Result<VersionResponse, String> {
    send_wrapper::SendWrapper::new(async move { get_json("/api/version").await }).await
}

#[cfg(not(feature = "ssr"))]
pub async fn send_chat(req: ChatRequest) -> Result<ChatResponse, String> {
    send_wrapper::SendWrapper::new(async move { post_json("/api/chat", &req).await }).await
}

#[cfg(not(feature = "ssr"))]
pub async fn list_conversations() -> Result<Vec<Conversation>, String> {
    send_wrapper::SendWrapper::new(async move { get_json("/api/conversations").await }).await
}

#[cfg(not(feature = "ssr"))]
pub async fn get_conversation(id: String) -> Result<ConversationThread, String> {
    send_wrapper::SendWrapper::new(
        async move { get_json(&format!("/api/conversations/{id}")).await },
    )
    .await
}

#[cfg(not(feature = "ssr"))]
pub async fn conversations_for_case(case_id: String) -> Result<Vec<Conversation>, String> {
    send_wrapper::SendWrapper::new(async move {
        get_json(&format!("/api/conversations?case_id={case_id}")).await
    })
    .await
}

#[cfg(not(feature = "ssr"))]
pub async fn conversations_for_contact(contact_id: String) -> Result<Vec<Conversation>, String> {
    send_wrapper::SendWrapper::new(async move {
        get_json(&format!("/api/conversations?contact_id={contact_id}")).await
    })
    .await
}

#[cfg(not(feature = "ssr"))]
pub async fn reply_conversation(
    id: String,
    body: String,
    author_id: Option<String>,
    author_label: Option<String>,
) -> Result<Message, String> {
    use crate::types::ReplyRequest;
    send_wrapper::SendWrapper::new(async move {
        let req = ReplyRequest {
            body,
            author_id,
            author_label,
        };
        post_json(&format!("/api/conversations/{id}/messages"), &req).await
    })
    .await
}

#[cfg(not(feature = "ssr"))]
pub async fn assign_conversation(
    id: String,
    volunteer_id: Option<String>,
) -> Result<Conversation, String> {
    use crate::types::AssignRequest;
    send_wrapper::SendWrapper::new(async move {
        let req = AssignRequest { volunteer_id };
        post_json(&format!("/api/conversations/{id}/assign"), &req).await
    })
    .await
}

#[cfg(not(feature = "ssr"))]
pub async fn link_conversation_case(
    id: String,
    case_id: Option<String>,
    contact_id: Option<String>,
) -> Result<Conversation, String> {
    use crate::types::LinkCaseRequest;
    send_wrapper::SendWrapper::new(async move {
        let req = LinkCaseRequest {
            case_id,
            contact_id,
        };
        post_json(&format!("/api/conversations/{id}/link-case"), &req).await
    })
    .await
}

#[cfg(not(feature = "ssr"))]
pub async fn set_conversation_status(
    id: String,
    status: ConversationStatus,
) -> Result<Conversation, String> {
    use crate::types::StatusRequest;
    send_wrapper::SendWrapper::new(async move {
        let req = StatusRequest { status };
        post_json(&format!("/api/conversations/{id}/status"), &req).await
    })
    .await
}

#[cfg(not(feature = "ssr"))]
pub async fn offboard_volunteer(volunteer_id: String) -> Result<usize, String> {
    send_wrapper::SendWrapper::new(async move {
        post_json(&format!("/api/volunteers/{volunteer_id}/offboard"), &()).await
    })
    .await
}
