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

/// Whether the organization has decided to take a case.
///
/// Deliberately separate from [`CaseStatus`]: status says where a case is in its
/// life, this says whether we accepted it at all. Only an operations or site
/// admin can change it (see `set_case_review_state`), which is what makes it the
/// one trustworthy signal — unlike status, which the per-case `EditCase`
/// capability puts within reach of the client who owns the case.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseReviewState {
    /// Submitted (usually via public signup) and waiting for an admin decision.
    PendingReview,
    /// The organization has taken this case on.
    Accepted,
    /// The organization is not taking this case; `review_reason` says why.
    Declined,
}

impl CaseReviewState {
    pub const ALL: [CaseReviewState; 3] = [
        CaseReviewState::PendingReview,
        CaseReviewState::Accepted,
        CaseReviewState::Declined,
    ];

    pub fn label(self) -> &'static str {
        match self {
            CaseReviewState::PendingReview => "Pending review",
            CaseReviewState::Accepted => "Accepted",
            CaseReviewState::Declined => "Declined",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            CaseReviewState::PendingReview => "pending_review",
            CaseReviewState::Accepted => "accepted",
            CaseReviewState::Declined => "declined",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.slug() == s)
    }
}

/// What is actually happening on a case right now, in one value.
///
/// This is **derived**, never stored: see [`CaseSummary::work_state`]. Storing an
/// "is being worked on" flag alongside the assignments that already answer the
/// question would give two sources of truth and one of them would go stale the
/// first time someone assigned a volunteer without remembering to flip it.
///
/// Every surface — the client's banner, the staff chip, the admin Cases tab —
/// renders this same value, so they cannot disagree about a case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaseWorkState {
    /// Submitted, no admin decision yet.
    AwaitingReview,
    /// Accepted, but nobody is assigned to work it yet.
    AwaitingVolunteer,
    /// Accepted and staffed. Carries the assigned names for display.
    InProgress(Vec<String>),
    /// Not taken on. Carries the reason given, which may be empty.
    Declined(String),
    /// Finished. Outranks the rest: a closed case is not "awaiting" anything.
    Closed,
}

impl CaseWorkState {
    /// The state said plainly, for the client who owns the case. Clients should
    /// never have to decode staff vocabulary to find out whether anyone is
    /// helping them.
    pub fn client_message(&self) -> String {
        match self {
            CaseWorkState::AwaitingReview => {
                "Submitted \u{2014} a coordinator is reviewing your case.".to_string()
            }
            CaseWorkState::AwaitingVolunteer => {
                "Accepted \u{2014} we're arranging support for you.".to_string()
            }
            CaseWorkState::InProgress(names) => match names.as_slice() {
                [] => "Accepted \u{2014} your case is being worked on.".to_string(),
                [one] => format!("Being worked on by {one}."),
                [first, rest @ ..] => {
                    format!("Being worked on by {first} and {} others.", rest.len())
                }
            },
            CaseWorkState::Declined(reason) if reason.trim().is_empty() => {
                "This case was not accepted.".to_string()
            }
            CaseWorkState::Declined(reason) => format!("This case was not accepted: {reason}"),
            CaseWorkState::Closed => "This case is closed.".to_string(),
        }
    }

    pub fn badge_classes(&self) -> &'static str {
        match self {
            CaseWorkState::AwaitingReview => {
                "bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/30"
            }
            CaseWorkState::AwaitingVolunteer => "bg-sky-500/15 text-sky-300 ring-1 ring-sky-500/30",
            CaseWorkState::InProgress(_) => {
                "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30"
            }
            CaseWorkState::Declined(_) => "bg-rose-500/15 text-rose-300 ring-1 ring-rose-500/30",
            CaseWorkState::Closed => "bg-slate-500/15 text-slate-400 ring-1 ring-slate-500/30",
        }
    }
}

/// Derive the work state from a case's review state, lifecycle status, and who
/// is assigned. The single place the rule lives, so every caller agrees.
fn work_state_of(
    review_state: CaseReviewState,
    review_reason: &str,
    status: CaseStatus,
    assigned: &[String],
) -> CaseWorkState {
    match review_state {
        CaseReviewState::Declined => CaseWorkState::Declined(review_reason.to_string()),
        CaseReviewState::PendingReview => CaseWorkState::AwaitingReview,
        // Closed is checked only after the decision states: a declined case that
        // someone also closed should still explain *why* it was declined.
        CaseReviewState::Accepted if status == CaseStatus::Closed => CaseWorkState::Closed,
        CaseReviewState::Accepted if assigned.is_empty() => CaseWorkState::AwaitingVolunteer,
        CaseReviewState::Accepted => CaseWorkState::InProgress(assigned.to_vec()),
    }
}

/// Serde default for `review_state` on payloads written before the field
/// existed, matching the migration's column default: an old case is one we are
/// already working, not one awaiting a decision.
fn accepted() -> CaseReviewState {
    CaseReviewState::Accepted
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
    /// Whether the org has accepted this case. Admin-owned; see
    /// [`CaseReviewState`].
    #[serde(default = "accepted")]
    pub review_state: CaseReviewState,
    /// Why the case was declined, if it was. Empty otherwise.
    #[serde(default)]
    pub review_reason: String,
    /// Display names of the volunteers/admins assigned to work this case (i.e.
    /// holders of `EditCase` who are not the client owner). Feeds the derived
    /// work state; never a stored "is staffed" flag.
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
    #[serde(default = "accepted")]
    pub review_state: CaseReviewState,
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
    /// What is happening on this case, derived from the review decision, the
    /// lifecycle status, and who is assigned. Render this rather than
    /// interpreting the individual fields, so every screen tells the same story.
    pub fn work_state(&self) -> CaseWorkState {
        work_state_of(
            self.review_state,
            &self.review_reason,
            self.status,
            &self.assigned_volunteers,
        )
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
    // client creates for themselves still needs a decision.
    let review_state = if has_volunteer_access(&user) {
        CaseReviewState::Accepted
    } else {
        CaseReviewState::PendingReview
    };
    cases::create(
        &user.id,
        &user.full_name(),
        &name,
        status,
        review_state,
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

/// Accept or decline a case (operations-admin or site-admin only).
///
/// Gated on the **account role**, not a [`CaseCapability`], and that distinction
/// is the point: deciding whether the organization takes a case is an
/// organizational act, so it must not be reachable through a per-case grant. The
/// client who owns the case holds the full capability set on it (the signup flow
/// grants it), and must never be able to approve their own case.
#[server(prefix = "/api")]
pub async fn set_case_review_state(
    case_id: String,
    state: CaseReviewState,
    reason: String,
) -> Result<(), ServerFnError> {
    use crate::server::db::cases;
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    require_operations_admin(&user)?;

    let reason = reason.trim().to_string();
    // A decline the client cannot understand is worse than no answer: it leaves
    // them with nowhere to go and nothing to act on.
    if state == CaseReviewState::Declined && reason.is_empty() {
        return Err(ServerFnError::new(
            "A reason is required when declining a case.",
        ));
    }
    // The reason belongs to the decline. Carrying it over to a later acceptance
    // would leave a stale "we said no because…" attached to a case we took on.
    let reason = if state == CaseReviewState::Declined {
        reason
    } else {
        String::new()
    };

    let changed = cases::set_review_state(&case_id, state, &reason, &user.full_name())
        .await
        .map_err(ServerFnError::new)?;
    if !changed {
        return Ok(());
    }

    let detail = match state {
        CaseReviewState::Accepted => "accepted this case".to_string(),
        CaseReviewState::Declined => format!("declined this case: {reason}"),
        CaseReviewState::PendingReview => "reopened this case for review".to_string(),
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
    // is told about it (see `work_state`) rather than making it.
    if !has_volunteer_access(&user) {
        return Err(ServerFnError::new("Only staff can change a case's status."));
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
