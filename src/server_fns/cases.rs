//! Case server functions: creation, edits, notes, evidence, and the per-case
//! chat. Each operation resolves the caller and checks the required capability
//! before touching the database.

use leptos::prelude::*;

use crate::types::{CaseStatus, Message};

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

/// The chat messages for a case (requires the `SendMessages` capability).
#[server(prefix = "/api")]
pub async fn list_messages(case_id: String) -> Result<Vec<Message>, ServerFnError> {
    use crate::server::permissions::{require_cap, require_user};
    use crate::server::db::messages;
    use crate::types::CaseCapability;

    let user = require_user().await?;
    require_cap(&user, &case_id, CaseCapability::SendMessages).await?;
    messages::for_case(&case_id)
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
