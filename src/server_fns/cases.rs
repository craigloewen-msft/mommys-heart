//! Case server functions: creation, edits, notes, and the per-case chat. Each
//! operation resolves the caller and checks the required capability before
//! touching the database. (A case's files live in
//! [`crate::server_fns::evidence`] and its properties in
//! [`crate::server_fns::case_properties`].)

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::helpers::case_intake::CaseIntake;
use crate::server_fns::capabilities::CaseCapability;
use crate::server_fns::case_folders::CaseFolder;
use crate::server_fns::case_properties::CaseProperty;
use crate::server_fns::evidence::Evidence;
use crate::server_fns::message::Message;
use crate::server_fns::pagination::Page;

/// The single state of a case: where it is in its life, including whether the
/// organization has accepted it at all.
///
/// One enum on purpose. An earlier draft kept the accept/decline decision in a
/// separate `review_state` column, which made states like "Open + Declined"
/// representable -- of the nine combinations, only three meant anything. The
/// decision and the lifecycle are the same dimension, so they are one value.
///
/// Who may make a given transition still differs: only an operations or site
/// admin may accept or decline (see `set_case_review_decision`), while staff
/// move a case between the working states (see `set_case_status`). That is a
/// rule about transitions, not a reason for a second field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseStatus {
    /// Submitted (usually via public signup) and waiting for an admin decision.
    /// Nobody is working it yet, by definition.
    PendingReview,
    /// Accepted and being worked.
    Open,
    /// Accepted, but only being kept an eye on.
    Monitor,
    /// Finished.
    Closed,
    /// The organization is not taking this case on. Terminal, and carries a
    /// reason the client is shown.
    Declined,
}

impl CaseStatus {
    /// Every status, for slug round-tripping. Not what the staff dropdown
    /// offers -- see [`CaseStatus::STAFF_SELECTABLE`].
    pub const ALL: [CaseStatus; 5] = [
        CaseStatus::PendingReview,
        CaseStatus::Open,
        CaseStatus::Monitor,
        CaseStatus::Closed,
        CaseStatus::Declined,
    ];

    /// The statuses staff may move a case between directly. Entering or leaving
    /// review is an admin decision that needs a reason, so those two are not
    /// items on a dropdown.
    pub const STAFF_SELECTABLE: [CaseStatus; 3] =
        [CaseStatus::Open, CaseStatus::Monitor, CaseStatus::Closed];

    pub fn label(self) -> &'static str {
        match self {
            CaseStatus::PendingReview => "Pending review",
            CaseStatus::Open => "Open",
            CaseStatus::Monitor => "Monitor",
            CaseStatus::Closed => "Closed",
            CaseStatus::Declined => "Declined",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            CaseStatus::PendingReview => "pending_review",
            CaseStatus::Open => "open",
            CaseStatus::Monitor => "monitor",
            CaseStatus::Closed => "closed",
            CaseStatus::Declined => "declined",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s2| s2.slug() == s)
    }

    /// Whether the organization has taken this case on, i.e. whether work on it
    /// is legitimate. False while it awaits a decision and after a decline.
    pub fn is_accepted(self) -> bool {
        matches!(
            self,
            CaseStatus::Open | CaseStatus::Monitor | CaseStatus::Closed
        )
    }

    pub fn badge_classes(self) -> &'static str {
        match self {
            CaseStatus::PendingReview => "bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/30",
            CaseStatus::Open => "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30",
            CaseStatus::Monitor => "bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/30",
            CaseStatus::Closed => "bg-slate-500/15 text-slate-400 ring-1 ring-slate-500/30",
            CaseStatus::Declined => "bg-rose-500/15 text-rose-300 ring-1 ring-rose-500/30",
        }
    }

    /// What this status means for the client who owns the case, in plain words.
    ///
    /// `assigned` is the staff working it, which turns a bare "Open" into the
    /// thing the client actually wants to know: whether a person is on it. It is
    /// derived from the live assignments rather than stored, so assigning a
    /// volunteer updates this with no second step to forget.
    pub fn client_message(self, assigned: &[String], reason: &str) -> String {
        match self {
            CaseStatus::PendingReview => {
                "Submitted \u{2014} a coordinator is reviewing your case.".to_string()
            }
            CaseStatus::Declined if reason.trim().is_empty() => {
                "This case was not accepted.".to_string()
            }
            CaseStatus::Declined => format!("This case was not accepted: {reason}"),
            CaseStatus::Closed => "This case is closed.".to_string(),
            CaseStatus::Open | CaseStatus::Monitor => match assigned {
                [] => "Accepted \u{2014} we're arranging support for you.".to_string(),
                [one] => format!("Being worked on by {one}."),
                [first, rest @ ..] => {
                    format!("Being worked on by {first} and {} others.", rest.len())
                }
            },
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

/// A support case tracked by the organization.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Case {
    pub id: String,
    pub name: String,
    pub status: CaseStatus,
    /// Why the case was declined, if it was. Empty otherwise.
    #[serde(default)]
    pub review_reason: String,
    /// Display names of the volunteers/admins assigned to work this case (i.e.
    /// holders of `EditCase` who are not the client owner). Derived from the
    /// live assignments; never a stored "is staffed" flag.
    #[serde(default)]
    pub assigned_volunteers: Vec<String>,
    /// The user who owns this case. Owners hold no implicit rights; they are
    /// granted a full capability assignment explicitly when the case is created.
    pub owner_id: String,
    /// Case notes, newest last.
    #[serde(default)]
    pub notes: Vec<CaseNote>,
    /// Evidence gathered for this case.
    #[serde(default)]
    pub evidence: Vec<Evidence>,
    /// The folders the evidence is organized into, creation order. Only the
    /// folders the viewer may see are included.
    #[serde(default)]
    pub folders: Vec<CaseFolder>,
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
    /// When the client accepted the Terms and Conditions to open this case, and
    /// which version they accepted — `None` for cases opened by staff, which
    /// never went through the public signup. Pre-formatted for display because
    /// it is only ever shown, never compared.
    #[serde(default)]
    pub terms_accepted: Option<String>,
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
    #[serde(default)]
    pub review_reason: String,
    #[serde(default)]
    pub assigned_volunteers: Vec<String>,
    pub owner_id: String,
    pub owner_first_name: String,
    pub owner_last_name: String,
    pub message_count: usize,
    #[serde(default)]
    pub inactive: bool,
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

impl Case {
    /// What this case's state means for the client who owns it, in plain words.
    /// Reads the live assignments, so "Open" becomes "Being worked on by X"
    /// without anything extra being stored.
    pub fn client_message(&self) -> String {
        self.status
            .client_message(&self.assigned_volunteers, &self.review_reason)
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
    use crate::server::permissions::{has_volunteer_access, require_cap, require_user};

    let user = require_user().await?;
    require_cap(&user, &case_id, CaseCapability::ViewCase).await?;
    cases::get(&case_id, &user.id, has_volunteer_access(&user))
        .await
        .map_err(ServerFnError::new)
}

/// Lightweight case search for the capability tool. Requires operations-admin
/// permissions because it supports self-management and approval requests. It
/// returns only case ids and names, never case contents.
#[server(prefix = "/api")]
pub async fn admin_search_cases(query: String) -> Result<Vec<CaseSummary>, ServerFnError> {
    use crate::server::db::cases;
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    require_operations_admin(&user)?;
    cases::search_lite(&query, 10)
        .await
        .map_err(ServerFnError::new)
}

/// Resolve specific case ids to names for the capability tool. Requires
/// operations-admin permissions and never loads case contents.
#[server(prefix = "/api")]
pub async fn admin_cases_by_ids(ids: Vec<String>) -> Result<Vec<CaseSummary>, ServerFnError> {
    use crate::server::db::cases;
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    require_operations_admin(&user)?;
    cases::get_summaries_by_ids(&ids)
        .await
        .map_err(ServerFnError::new)
}

/// Create a case owned by the caller. Any signed-in user may create one.
#[server(prefix = "/api")]
pub async fn create_case(
    name: String,
    status: CaseStatus,
    intake: CaseIntake,
    first_note: Option<String>,
) -> Result<String, ServerFnError> {
    use crate::server::db::cases;
    use crate::server::permissions::{has_volunteer_access, require_user, require_visibility};

    let user = require_user().await?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ServerFnError::new("Case name is required."));
    }
    intake.validate().map_err(ServerFnError::new)?;
    let properties = intake.properties();
    for property in &properties {
        require_visibility(&user, property.visibility)?;
    }
    // A staff member creating a case *is* the acceptance — the decision has
    // already been made by the person doing it, so sending it to a review queue
    // would only ask an admin to rubber-stamp their own colleague. A case a
    // client creates for themselves still needs a decision, and the status they
    // asked for is ignored until it has one.
    let status = if has_volunteer_access(&user) {
        status
    } else {
        CaseStatus::PendingReview
    };
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

/// The cases waiting for an admin to accept or decline them.
///
/// Requires operations-admin permissions. Deliberately unscoped by assignment,
/// unlike [`load_case_summaries_for_user`]: a case awaiting review has nobody
/// assigned to it yet by definition, so an assignment-scoped query would return
/// exactly nothing. Header fields only, never case contents.
#[server(prefix = "/api")]
pub async fn list_pending_case_requests() -> Result<Vec<CaseSummary>, ServerFnError> {
    use crate::server::db::cases;
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    require_operations_admin(&user)?;
    cases::pending_review_cases()
        .await
        .map_err(ServerFnError::new)
}

/// How many cases are waiting for an admin decision. Drives the count badge on
/// the Cases tab, so "something needs you" is visible without opening it.
#[server(prefix = "/api")]
pub async fn pending_case_review_count() -> Result<i64, ServerFnError> {
    use crate::server::db::cases;
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    require_operations_admin(&user)?;
    cases::pending_review_count()
        .await
        .map_err(ServerFnError::new)
}

/// Accept or decline a case awaiting review (operations-admin or site-admin
/// only).
///
/// Gated on the **account role**, not a [`CaseCapability`], and that distinction
/// is the point: deciding whether the organization takes a case is an
/// organizational act, so it must not be reachable through a per-case grant. The
/// client who owns the case holds the full capability set on it (the signup flow
/// grants it), and must never be able to approve their own case.
///
/// Only the two decision transitions live here. Moving an accepted case between
/// its working states is [`set_case_status`], which staff do.
#[server(prefix = "/api")]
pub async fn set_case_review_decision(
    case_id: String,
    accept: bool,
    reason: String,
) -> Result<(), ServerFnError> {
    use crate::server::db::cases;
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    require_operations_admin(&user)?;

    let reason = reason.trim().to_string();
    // A decline the client cannot understand is worse than no answer: it leaves
    // them with nowhere to go and nothing to act on.
    if !accept && reason.is_empty() {
        return Err(ServerFnError::new(
            "A reason is required when declining a case.",
        ));
    }
    // The reason belongs to the decline. Carrying it over to an acceptance would
    // leave a stale "we said no because…" attached to a case we took on.
    let reason = if accept { String::new() } else { reason };
    // Accepting starts the case at the front of the working lifecycle; declining
    // ends it. Both are one status write, because a case has one state.
    let next = if accept {
        CaseStatus::Open
    } else {
        CaseStatus::Declined
    };

    let current = cases::status(&case_id)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("Case not found."))?;
    // Only a case actually awaiting review can be decided. Without this, a
    // decision could silently reopen or discard a case already being worked --
    // and "decline" on a live case would strand its client mid-support.
    if current != CaseStatus::PendingReview {
        return Err(ServerFnError::new(format!(
            "This case is already {} and is not awaiting review.",
            current.label().to_lowercase()
        )));
    }

    cases::set_status_with_reason(&case_id, next, &reason, &user.full_name())
        .await
        .map_err(ServerFnError::new)?;

    let detail = if accept {
        "accepted this case".to_string()
    } else {
        format!("declined this case: {reason}")
    };
    crate::server::notifications::notify_case(
        case_id,
        user.id.clone(),
        user.full_name(),
        crate::server_fns::settings::NotificationKind::CaseData,
        detail,
        // The decision is the one thing the client is most owed an answer on, so
        // it reaches everyone on the case rather than staff alone.
        crate::server::notifications::Audience::Everyone,
    );
    Ok(())
}

/// Change a case's status (requires the `EditCase` capability).
#[server(prefix = "/api")]
pub async fn set_case_status(case_id: String, status: CaseStatus) -> Result<(), ServerFnError> {
    use crate::server::db::cases;
    use crate::server::permissions::{has_volunteer_access, require_cap, require_user};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    require_cap(&user, &case_id, CaseCapability::EditCase).await?;
    // The capability alone is not enough here. The signup flow grants the client
    // owner every capability on their own case, so without this an intake could
    // mark itself Closed. Status is a staff judgement about the work; the client
    // is told about it (see `client_message`) rather than making it.
    if !has_volunteer_access(&user) {
        return Err(ServerFnError::new("Only staff can change a case's status."));
    }
    // Accepting and declining are admin decisions that carry a reason and a
    // decider, so they are not reachable from the staff status control. Both
    // ends of the lifecycle belong to `set_case_review_decision`.
    if !status.is_accepted() {
        return Err(ServerFnError::new(
            "Accepting or declining a case is an admin decision, not a status change.",
        ));
    }
    // A case that has not been accepted has no working status to set, and a
    // declined one is finished. Letting staff move either would route around the
    // decision entirely.
    let current = cases::status(&case_id)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("Case not found."))?;
    if !current.is_accepted() {
        return Err(ServerFnError::new(format!(
            "This case is {} \u{2014} an admin has to decide it first.",
            current.label().to_lowercase()
        )));
    }
    cases::set_status(&case_id, status, &user.full_name())
        .await
        .map_err(ServerFnError::new)?;
    crate::server::notifications::notify_case(
        case_id,
        user.id.clone(),
        user.full_name(),
        crate::server_fns::settings::NotificationKind::CaseData,
        format!("changed the status to \"{}\"", status.label()),
        crate::server::notifications::Audience::Everyone,
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
        crate::server::notifications::Audience::Everyone,
    );
    Ok(())
}

/// Reassign a case's owner. Site Admin only: ownership decides whose personal
/// information (e.g. other users' names, surfaced through the owner picker) a
/// case exposes, so it is deliberately *not* covered by the per-case `EditCase`
/// capability that a case owner themselves holds.
#[server(prefix = "/api")]
pub async fn set_case_owner(case_id: String, owner_id: String) -> Result<(), ServerFnError> {
    use crate::server::db::{cases, users};
    use crate::server::permissions::{require_site_admin, require_user};

    let user = require_user().await?;
    require_site_admin(&user)?;
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
        crate::server::notifications::Audience::Everyone,
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
        crate::server::notifications::Audience::Everyone,
    );
    Ok(())
}

/// One page of a chat **channel**: the most recent `limit` messages
/// (oldest-first) plus the channel's total message count. Backs the chat's
/// "Load more" pagination.
#[server(prefix = "/api")]
pub async fn list_messages_page(
    channel_id: String,
    limit: i64,
) -> Result<Page<Message>, ServerFnError> {
    use crate::server::db::messages;
    use crate::server::permissions::{require_channel, require_user};

    let user = require_user().await?;
    require_channel(&user, &channel_id, CaseCapability::ViewCase).await?;
    messages::page(&channel_id, limit)
        .await
        .map_err(ServerFnError::new)
}

/// Post a message to a chat channel as the signed-in user
#[server(prefix = "/api")]
pub async fn send_message(channel_id: String, body: String) -> Result<Message, ServerFnError> {
    use crate::server::db::messages;
    use crate::server::permissions::{require_channel, require_user};
    use crate::server_fns::capabilities::CaseCapability;
    use crate::server_fns::channels::ChannelKind;

    let user = require_user().await?;
    let body = body.trim().to_string();
    if body.is_empty() {
        return Err(ServerFnError::new("Message cannot be empty."));
    }
    let channel = require_channel(&user, &channel_id, CaseCapability::SendMessages).await?;
    let message = messages::create(
        &channel.case_id,
        &channel.id,
        &user.id,
        &user.full_name(),
        &body,
    )
    .await
    .map_err(ServerFnError::new)?;

    if let Err(e) = crate::server::db::channel_notifications::record_for_channel_message(
        &channel.case_id,
        &channel.id,
        &message.id,
        &user.id,
        channel.kind,
    )
    .await
    {
        tracing::warn!(
            "failed to record chat notifications for channel {}: {e}",
            channel.id
        );
    }
    let preview: String = body.chars().take(80).collect();
    let ellipsis = if body.chars().count() > 80 { "…" } else { "" };
    let detail = format!(
        "posted a new message in \"{}\": \"{preview}{ellipsis}\"",
        channel.name
    );
    // A volunteer-only message must never be summarized into a client's inbox,
    // so the notification audience is narrowed to the same people who can read
    // the channel.
    let audience = if channel.kind == ChannelKind::VolunteerOnly {
        crate::server::notifications::Audience::StaffOnly
    } else {
        crate::server::notifications::Audience::Everyone
    };
    crate::server::notifications::notify_case(
        channel.case_id,
        user.id.clone(),
        user.full_name(),
        crate::server_fns::settings::NotificationKind::NewMessage,
        detail,
        audience,
    );
    Ok(message)
}
