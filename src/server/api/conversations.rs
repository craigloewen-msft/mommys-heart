//! `/api/conversations` — the org-owned communications inbox.
//!
//! Backs the staff inbox console and the case/contact communication timelines.
//! Every conversation and message is retained server-side (see
//! [`crate::server::comm_store`]), so records outlive the individual volunteer
//! who handled them.

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

use crate::server::comm_store::ConversationFilter;
use crate::server::service;
use crate::types::{
    AssignRequest, Channel, Conversation, ConversationStatus, ConversationThread, LinkCaseRequest,
    Message, ReplyRequest, StatusRequest,
};

pub fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/api/conversations", get(list))
        .route("/api/conversations/{id}", get(thread))
        .route("/api/conversations/{id}/messages", post(reply))
        .route("/api/conversations/{id}/assign", post(assign))
        .route("/api/conversations/{id}/link-case", post(link_case))
        .route("/api/conversations/{id}/status", post(set_status))
        .route("/api/volunteers/{id}/offboard", post(offboard))
}

/// Query-string filters for `GET /api/conversations` (all optional).
#[derive(Debug, Default, Deserialize)]
struct ListParams {
    status: Option<String>,
    channel: Option<String>,
    assigned_volunteer_id: Option<String>,
    case_id: Option<String>,
    contact_id: Option<String>,
}

async fn list(Query(params): Query<ListParams>) -> Json<Vec<Conversation>> {
    let filter = ConversationFilter {
        status: params
            .status
            .as_deref()
            .and_then(ConversationStatus::from_slug),
        channel: params.channel.as_deref().and_then(Channel::from_slug),
        assigned_volunteer_id: params.assigned_volunteer_id.filter(|s| !s.is_empty()),
        case_id: params.case_id.filter(|s| !s.is_empty()),
        contact_id: params.contact_id.filter(|s| !s.is_empty()),
    };
    Json(service::list_conversations(&filter))
}

async fn thread(Path(id): Path<String>) -> Result<Json<ConversationThread>, (StatusCode, String)> {
    service::get_conversation(&id).map(Json).ok_or((
        StatusCode::NOT_FOUND,
        format!("Conversation '{id}' not found"),
    ))
}

async fn reply(
    Path(id): Path<String>,
    Json(req): Json<ReplyRequest>,
) -> Result<Json<Message>, (StatusCode, String)> {
    let body = req.body.trim();
    if body.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Reply cannot be empty".into()));
    }
    service::reply_to_conversation(&id, body, req.author_id, req.author_label)
        .map(Json)
        .ok_or((
            StatusCode::NOT_FOUND,
            format!("Conversation '{id}' not found"),
        ))
}

async fn assign(
    Path(id): Path<String>,
    Json(req): Json<AssignRequest>,
) -> Result<Json<Conversation>, (StatusCode, String)> {
    service::assign_conversation(&id, req.volunteer_id)
        .map(Json)
        .ok_or((
            StatusCode::NOT_FOUND,
            format!("Conversation '{id}' not found"),
        ))
}

async fn link_case(
    Path(id): Path<String>,
    Json(req): Json<LinkCaseRequest>,
) -> Result<Json<Conversation>, (StatusCode, String)> {
    service::link_conversation_case(&id, req.case_id, req.contact_id)
        .map(Json)
        .ok_or((
            StatusCode::NOT_FOUND,
            format!("Conversation '{id}' not found"),
        ))
}

async fn set_status(
    Path(id): Path<String>,
    Json(req): Json<StatusRequest>,
) -> Result<Json<Conversation>, (StatusCode, String)> {
    service::set_conversation_status(&id, req.status)
        .map(Json)
        .ok_or((
            StatusCode::NOT_FOUND,
            format!("Conversation '{id}' not found"),
        ))
}

/// Offboard a volunteer: unassign them from every conversation they own while
/// preserving all history. Returns how many conversations were reassigned.
async fn offboard(Path(id): Path<String>) -> Json<usize> {
    Json(service::offboard_volunteer(&id))
}
