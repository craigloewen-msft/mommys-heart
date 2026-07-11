//! Case server functions: creation, edits, notes, evidence, and the per-case
//! chat. Each operation resolves the caller and checks the required capability
//! before touching the database.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::types::{
    CaseNote, CaseProperty, CaseStatus, Evidence, Page,
};
use crate::server_fns::message::Message;

/// A support case tracked by the organization.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Case {
    pub id: String,
    pub name: String,
    pub status: CaseStatus,
    /// The user who owns this case (implicitly has `Owner` permission).
    pub owner_id: String,
    /// Case notes, newest last.
    #[serde(default)]
    pub notes: Vec<CaseNote>,
    /// Evidence gathered for this case.
    #[serde(default)]
    pub evidence: Vec<Evidence>,
    /// Free-form case properties (e.g. "Opposing attorney" -> "J. Smith").
    #[serde(default)]
    pub properties: Vec<CaseProperty>,
    /// Count only of messages
    #[serde(default)]
    pub message_count: usize,
}

/// A sparse view of a case for list/directory screens: the header fields only
/// (id, name, status, owner id + resolved owner name, and chat message count),
/// with none of the heavy sub-resources (notes, evidence, properties, audit
/// log). Shared by the DB layer that produces it and the pages that render it,
/// so it is defined exactly once.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CaseSummary {
    pub id: String,
    pub name: String,
    pub status: CaseStatus,
    pub owner_id: String,
    pub owner_name: String,
    pub message_count: usize,
}

/// One page of summarized cases the caller owns or is assigned to, ordered by
/// id, with an optional case-insensitive search over id/name. Owner names and
/// message counts are resolved server-side. Paginated ("Load more") so the
/// browser never pulls every case at once; the full detail for one case is
/// loaded on demand via [`load_case`].
#[server(prefix = "/api")]
pub async fn load_case_summaries_for_user(
    offset: i64,
    limit: i64,
    search: String,
) -> Result<Page<CaseSummary>, ServerFnError> {
    use crate::server::db::cases;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    cases::get_summaries_for_user(offset, limit, &search, &user.id)
        .await
        .map_err(ServerFnError::new)
}

/// Load a fully hydrated case
#[server(prefix = "/api")]
pub async fn load_case(case_id: String) -> Result<Option<Case>, ServerFnError> {
    use crate::server::db::cases;
    use crate::server::permissions::{require_cap, require_user};
    use crate::types::CaseCapability;

    let user = require_user().await?;
    require_cap(&user, &case_id, CaseCapability::ViewCase).await?;
    cases::get(&case_id).await.map_err(ServerFnError::new)
}

/// Admin-only lightweight case search for the permission tool: find any case
/// (across the whole system) by id or name so an admin can assign a user to it.
/// This is a management action, not a case view — it never hydrates or exposes
/// case contents.
#[server(prefix = "/api")]
pub async fn admin_search_cases(query: String) -> Result<Vec<CaseSummary>, ServerFnError> {
    use crate::server::db::cases;
    use crate::server::permissions::{require_admin, require_user};

    let user = require_user().await?;
    require_admin(&user)?;
    cases::search_lite(&query, 10)
        .await
        .map_err(ServerFnError::new)
}

/// Admin-only lightweight resolution of specific case ids to names, so the
/// permission tool can show which cases a user is already assigned to without
/// loading every case.
#[server(prefix = "/api")]
pub async fn admin_cases_by_ids(ids: Vec<String>) -> Result<Vec<CaseSummary>, ServerFnError> {
    use crate::server::db::cases;
    use crate::server::permissions::{require_admin, require_user};

    let user = require_user().await?;
    require_admin(&user)?;
    cases::get_summaries_by_ids(&ids)
        .await
        .map_err(ServerFnError::new)
}

/// Create a case owned by the caller. Any signed-in user may create one.
#[server(prefix = "/api")]
pub async fn create_case(
    name: String,
    status: CaseStatus,
    properties: Vec<(String, String)>,
    first_note: Option<String>,
) -> Result<String, ServerFnError> {
    use crate::server::permissions::require_user;
    use crate::server::db::cases;

    let user = require_user().await?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ServerFnError::new("Case name is required."));
    }
    cases::create(
        &user.id,
        &user.full_name(),
        &name,
        status,
        properties,
        first_note,
    )
    .await
    .map_err(ServerFnError::new)
}

/// Change a case's status (requires the `EditCase` capability).
#[server(prefix = "/api")]
pub async fn set_case_status(case_id: String, status: CaseStatus) -> Result<(), ServerFnError> {
    use crate::server::permissions::{require_cap, require_user};
    use crate::server::db::cases;
    use crate::types::CaseCapability;

    let user = require_user().await?;
    require_cap(&user, &case_id, CaseCapability::EditCase).await?;
    cases::set_status(&case_id, status, &user.full_name())
        .await
        .map_err(ServerFnError::new)
}

/// Rename a case (requires the `EditCase` capability).
#[server(prefix = "/api")]
pub async fn set_case_name(case_id: String, name: String) -> Result<(), ServerFnError> {
    use crate::server::permissions::{require_cap, require_user};
    use crate::server::db::cases;
    use crate::types::CaseCapability;

    let user = require_user().await?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ServerFnError::new("Case name is required."));
    }
    require_cap(&user, &case_id, CaseCapability::EditCase).await?;
    cases::set_name(&case_id, &name, &user.full_name())
        .await
        .map_err(ServerFnError::new)
}

/// Reassign a case's owner (requires the `EditCase` capability).
#[server(prefix = "/api")]
pub async fn set_case_owner(case_id: String, owner_id: String) -> Result<(), ServerFnError> {
    use crate::server::permissions::{require_cap, require_user};
    use crate::server::db::{cases, users};
    use crate::types::CaseCapability;

    let user = require_user().await?;
    require_cap(&user, &case_id, CaseCapability::EditCase).await?;
    if users::get(&owner_id).await.map_err(ServerFnError::new)?.is_none() {
        return Err(ServerFnError::new("Unknown owner."));
    }
    cases::set_owner(&case_id, &owner_id, &user.full_name())
        .await
        .map_err(ServerFnError::new)
}

/// Replace a case's free-form properties (requires the `EditCase` capability).
#[server(prefix = "/api")]
pub async fn set_case_properties(
    case_id: String,
    properties: Vec<(String, String)>,
) -> Result<(), ServerFnError> {
    use crate::server::permissions::{require_cap, require_user};
    use crate::server::db::cases;
    use crate::types::CaseCapability;

    let user = require_user().await?;
    require_cap(&user, &case_id, CaseCapability::EditCase).await?;
    cases::replace_properties(&case_id, properties, &user.full_name())
        .await
        .map_err(ServerFnError::new)
}

/// Add a note to a case (requires the `AddNotes` capability).
#[server(prefix = "/api")]
pub async fn add_case_note(case_id: String, body: String) -> Result<(), ServerFnError> {
    use crate::server::permissions::{require_cap, require_user};
    use crate::server::db::cases;
    use crate::types::CaseCapability;

    let user = require_user().await?;
    let body = body.trim().to_string();
    if body.is_empty() {
        return Err(ServerFnError::new("Note cannot be empty."));
    }
    require_cap(&user, &case_id, CaseCapability::AddNotes).await?;
    cases::add_note(&case_id, &user.full_name(), &body)
        .await
        .map_err(ServerFnError::new)
}

/// Attach an evidence entry to a case (requires the `UploadEvidence` capability).
#[server(prefix = "/api")]
pub async fn add_case_evidence(
    case_id: String,
    name: String,
    description: String,
) -> Result<(), ServerFnError> {
    use crate::server::permissions::{require_cap, require_user};
    use crate::server::db::cases;
    use crate::types::CaseCapability;

    let user = require_user().await?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ServerFnError::new("Evidence name is required."));
    }
    require_cap(&user, &case_id, CaseCapability::UploadEvidence).await?;
    cases::add_evidence(&case_id, &name, &user.full_name(), description.trim())
        .await
        .map_err(ServerFnError::new)
}

/// Remove an evidence entry (requires the `DeleteEvidence` capability).
#[server(prefix = "/api")]
pub async fn delete_case_evidence(
    case_id: String,
    evidence_id: String,
) -> Result<(), ServerFnError> {
    use crate::server::permissions::{require_cap, require_user};
    use crate::server::db::cases;
    use crate::types::CaseCapability;

    let user = require_user().await?;
    require_cap(&user, &case_id, CaseCapability::DeleteEvidence).await?;
    cases::delete_evidence(&case_id, &evidence_id)
        .await
        .map_err(ServerFnError::new)
}

/// One page of a case's chat: the most recent `limit` messages (oldest-first)
/// plus the thread's total message count (requires the `SendMessages`
/// capability). Backs the chat's "Load more" pagination.
#[server(prefix = "/api")]
pub async fn list_messages_page(
    case_id: String,
    limit: i64,
) -> Result<Page<Message>, ServerFnError> {
    use crate::server::db::messages;
    use crate::server::permissions::{require_cap, require_user};
    use crate::types::CaseCapability;

    let user = require_user().await?;
    require_cap(&user, &case_id, CaseCapability::SendMessages).await?;
    messages::page(&case_id, limit)
        .await
        .map_err(ServerFnError::new)
}

/// Post a message to a case's chat as the signed-in user (requires the
/// `SendMessages` capability). Returns the stored message.
#[server(prefix = "/api")]
pub async fn send_message(case_id: String, body: String) -> Result<Message, ServerFnError> {
    use crate::server::permissions::{require_cap, require_user};
    use crate::server::db::messages;
    use crate::types::CaseCapability;

    let user = require_user().await?;
    let body = body.trim().to_string();
    if body.is_empty() {
        return Err(ServerFnError::new("Message cannot be empty."));
    }
    require_cap(&user, &case_id, CaseCapability::SendMessages).await?;
    messages::create(&case_id, &user.id, &user.full_name(), &body)
        .await
        .map_err(ServerFnError::new)
}
