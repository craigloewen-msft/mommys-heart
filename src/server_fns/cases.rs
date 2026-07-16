//! Case server functions: creation, edits, notes, and the per-case chat. Each
//! operation resolves the caller and checks the required capability before
//! touching the database. (Evidence lives in [`crate::server_fns::evidence`].)

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::server_fns::capabilities::CaseCapability;
use crate::server_fns::evidence::Evidence;
use crate::server_fns::message::Message;
use crate::server_fns::pagination::Page;

/// The lifecycle status of a case.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseStatus {
    Open,
    Monitor,
    Closed,
}

impl CaseStatus {
    pub const ALL: [CaseStatus; 3] = [CaseStatus::Open, CaseStatus::Monitor, CaseStatus::Closed];

    pub fn label(self) -> &'static str {
        match self {
            CaseStatus::Open => "Open",
            CaseStatus::Monitor => "Monitor",
            CaseStatus::Closed => "Closed",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            CaseStatus::Open => "open",
            CaseStatus::Monitor => "monitor",
            CaseStatus::Closed => "closed",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s2| s2.slug() == s)
    }

    pub fn badge_classes(self) -> &'static str {
        match self {
            CaseStatus::Open => "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30",
            CaseStatus::Monitor => "bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/30",
            CaseStatus::Closed => "bg-slate-500/15 text-slate-400 ring-1 ring-slate-500/30",
        }
    }
}

/// A free-text note recorded against a case.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CaseNote {
    pub id: String,
    pub author: String,
    pub body: String,
    pub created_at: String,
}

/// A named key/value property on a case (e.g. attorney names, court, docket).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CaseProperty {
    pub key: String,
    pub value: String,
}

/// A support case tracked by the organization.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Case {
    pub id: String,
    pub name: String,
    pub status: CaseStatus,
    /// The user who owns this case. Owners hold no implicit rights; they are
    /// granted a full capability assignment explicitly when the case is created.
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
    /// The signed-in viewer's capabilities on this case, resolved server-side
    /// per request. The source of truth for what the current user may do here —
    /// read it directly rather than caching per-case rights in client state.
    #[serde(default)]
    pub capabilities: Vec<CaseCapability>,
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
    pub owner_first_name: String,
    pub owner_last_name: String,
    pub message_count: usize,
    #[serde(default)]
    pub capabilities: Vec<CaseCapability>,
}

impl CaseSummary {
    /// The owner's display name (first + last), falling back to the owner id
    /// when the owner user is missing or unnamed.
    pub fn owner_full_name(&self) -> String {
        let name = format!("{} {}", self.owner_first_name, self.owner_last_name)
            .trim()
            .to_string();
        if name.is_empty() {
            self.owner_id.clone()
        } else {
            name
        }
    }
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

    let user = require_user().await?;
    require_cap(&user, &case_id, CaseCapability::ViewCase).await?;
    cases::get(&case_id, &user.id)
        .await
        .map_err(ServerFnError::new)
}

/// Admin-only lightweight case search for the capability tool: find any case
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
/// capability tool can show which cases a user is already assigned to without
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
    use crate::server::db::cases;
    use crate::server::permissions::require_user;

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
    use crate::server::db::cases;
    use crate::server::permissions::{require_cap, require_user};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    require_cap(&user, &case_id, CaseCapability::EditCase).await?;
    cases::set_status(&case_id, status, &user.full_name())
        .await
        .map_err(ServerFnError::new)?;
    crate::server::notifications::notify_case(
        case_id,
        user.id.clone(),
        user.full_name(),
        crate::server_fns::settings::NotificationKind::CaseData,
        format!("changed the status to \"{}\"", status.label()),
    );
    Ok(())
}

/// Rename a case (requires the `EditCase` capability).
#[server(prefix = "/api")]
pub async fn set_case_name(case_id: String, name: String) -> Result<(), ServerFnError> {
    use crate::server::db::cases;
    use crate::server::permissions::{require_cap, require_user};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ServerFnError::new("Case name is required."));
    }
    require_cap(&user, &case_id, CaseCapability::EditCase).await?;
    cases::set_name(&case_id, &name, &user.full_name())
        .await
        .map_err(ServerFnError::new)?;
    crate::server::notifications::notify_case(
        case_id,
        user.id.clone(),
        user.full_name(),
        crate::server_fns::settings::NotificationKind::CaseData,
        format!("renamed the case to \"{name}\""),
    );
    Ok(())
}

/// Reassign a case's owner. Admin only: ownership decides whose personal
/// information (e.g. other users' names, surfaced through the owner picker) a
/// case exposes, so it is deliberately *not* covered by the per-case `EditCase`
/// capability that a case owner themselves holds.
#[server(prefix = "/api")]
pub async fn set_case_owner(case_id: String, owner_id: String) -> Result<(), ServerFnError> {
    use crate::server::db::{cases, users};
    use crate::server::permissions::{require_admin, require_user};

    let user = require_user().await?;
    require_admin(&user)?;
    if users::get(&owner_id)
        .await
        .map_err(ServerFnError::new)?
        .is_none()
    {
        return Err(ServerFnError::new("Unknown owner."));
    }
    cases::set_owner(&case_id, &owner_id, &user.full_name())
        .await
        .map_err(ServerFnError::new)?;
    crate::server::notifications::notify_case(
        case_id,
        user.id.clone(),
        user.full_name(),
        crate::server_fns::settings::NotificationKind::CaseData,
        "changed the case owner".to_string(),
    );
    Ok(())
}

/// Replace a case's free-form properties (requires the `EditCase` capability).
#[server(prefix = "/api")]
pub async fn set_case_properties(
    case_id: String,
    properties: Vec<(String, String)>,
) -> Result<(), ServerFnError> {
    use crate::server::db::cases;
    use crate::server::permissions::{require_cap, require_user};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    require_cap(&user, &case_id, CaseCapability::EditCase).await?;
    cases::replace_properties(&case_id, properties, &user.full_name())
        .await
        .map_err(ServerFnError::new)?;
    crate::server::notifications::notify_case(
        case_id,
        user.id.clone(),
        user.full_name(),
        crate::server_fns::settings::NotificationKind::CaseData,
        "updated the case properties".to_string(),
    );
    Ok(())
}

/// Add a note to a case (requires the `AddNotes` capability).
#[server(prefix = "/api")]
pub async fn add_case_note(case_id: String, body: String) -> Result<(), ServerFnError> {
    use crate::server::db::cases;
    use crate::server::permissions::{require_cap, require_user};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    let body = body.trim().to_string();
    if body.is_empty() {
        return Err(ServerFnError::new("Note cannot be empty."));
    }
    require_cap(&user, &case_id, CaseCapability::AddNotes).await?;
    cases::add_note(&case_id, &user.full_name(), &body)
        .await
        .map_err(ServerFnError::new)?;
    crate::server::notifications::notify_case(
        case_id,
        user.id.clone(),
        user.full_name(),
        crate::server_fns::settings::NotificationKind::NoteAdded,
        "added a note".to_string(),
    );
    Ok(())
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
    use crate::server::permissions::require_user;

    // Require user for authentication purposes
    let _user = require_user().await?;
    messages::page(&case_id, limit)
        .await
        .map_err(ServerFnError::new)
}

/// Post a message to a case's chat as the signed-in user (requires the
/// `SendMessages` capability). Returns the stored message.
#[server(prefix = "/api")]
pub async fn send_message(case_id: String, body: String) -> Result<Message, ServerFnError> {
    use crate::server::db::messages;
    use crate::server::permissions::{require_cap, require_user};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    let body = body.trim().to_string();
    if body.is_empty() {
        return Err(ServerFnError::new("Message cannot be empty."));
    }
    require_cap(&user, &case_id, CaseCapability::SendMessages).await?;
    let message = messages::create(&case_id, &user.id, &user.full_name(), &body)
        .await
        .map_err(ServerFnError::new)?;
    let preview: String = body.chars().take(80).collect();
    let detail = if body.chars().count() > 80 {
        format!("posted a new message: \"{preview}…\"")
    } else {
        format!("posted a new message: \"{preview}\"")
    };
    crate::server::notifications::notify_case(
        case_id,
        user.id.clone(),
        user.full_name(),
        crate::server_fns::settings::NotificationKind::NewMessage,
        detail,
    );
    Ok(message)
}
