//! Volunteers as an object built on top of a user: the volunteer agreement they
//! accepted, their application to become one, and the admin decision on it.
//!
//! A user is the base record (identity, contact details, role); this module adds
//! what is true *because* someone is a volunteer. There is exactly one volunteer
//! record per person and it doubles as their application, so a decision updates
//! it in place. A denied applicant may accept the agreement again, which returns
//! them to `Pending`.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::server_fns::users::User;

/// Where a volunteer application stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VolunteerStatus {
    Pending,
    Approved,
    Denied,
}

impl VolunteerStatus {
    pub fn slug(self) -> &'static str {
        match self {
            VolunteerStatus::Pending => "pending",
            VolunteerStatus::Approved => "approved",
            VolunteerStatus::Denied => "denied",
        }
    }

    pub fn from_slug(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(VolunteerStatus::Pending),
            "approved" => Some(VolunteerStatus::Approved),
            "denied" => Some(VolunteerStatus::Denied),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            VolunteerStatus::Pending => "Pending review",
            VolunteerStatus::Approved => "Approved",
            VolunteerStatus::Denied => "Declined",
        }
    }

    pub fn badge_classes(self) -> &'static str {
        match self {
            VolunteerStatus::Pending => "bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/30",
            VolunteerStatus::Approved => {
                "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30"
            }
            VolunteerStatus::Denied => "bg-rose-500/15 text-rose-300 ring-1 ring-rose-500/30",
        }
    }
}

/// One person's volunteer record: the agreement they accepted and the state of
/// their application.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VolunteerApplication {
    pub status: VolunteerStatus,
    /// The agreement wording they accepted. Empty for volunteers who predate the
    /// agreement (backfilled by migration), which is why it is not an `Option`:
    /// "no version" is a real, meaningful state rather than missing data.
    pub agreement_version: String,
    /// Pre-formatted for display; these are only ever shown, never compared.
    pub agreed_at: String,
    pub decided_by_name: String,
    pub decision_note: String,
    pub decided_at: String,
}

impl VolunteerApplication {
    /// Whether this record carries the wording of a specific agreement version.
    /// False for the migration backfill, whose volunteers never saw one.
    pub fn has_agreement(&self) -> bool {
        !self.agreement_version.is_empty()
    }
}

/// A volunteer: the base user plus the volunteer-specific record built on it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Volunteer {
    pub user: User,
    pub application: VolunteerApplication,
}

impl Volunteer {
    /// Convenience passthrough to the base record.
    pub fn full_name(&self) -> String {
        self.user.full_name()
    }
}

/// Accept the volunteer agreement and apply to become a volunteer.
///
/// Rejects a caller who already has volunteer privileges, or who has an
/// application awaiting a decision. A previously declined applicant may apply
/// again, which returns their record to pending.
#[server(prefix = "/api")]
pub async fn apply_to_volunteer(agreement_version: String) -> Result<(), ServerFnError> {
    use crate::server::db::volunteers;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    if user.role.has_volunteer_privileges() {
        return Err(ServerFnError::new(
            "Your account already has volunteer access.",
        ));
    }
    // An old tab holding a stale version never saw the wording it claims to
    // accept, so make it re-read rather than record consent it cannot evidence.
    if !crate::helpers::volunteer_terms::is_current(agreement_version.trim()) {
        return Err(ServerFnError::new(
            "Please read and accept the current volunteer agreement.",
        ));
    }
    if volunteers::get(&user.id)
        .await
        .map_err(ServerFnError::new)?
        .is_some_and(|application| application.status == VolunteerStatus::Pending)
    {
        return Err(ServerFnError::new(
            "Your volunteer application is already awaiting review.",
        ));
    }
    volunteers::apply(&user.id, agreement_version.trim())
        .await
        .map_err(ServerFnError::new)
}

/// Every volunteer application waiting on a decision, oldest first. Visible to
/// anyone with operations-admin permissions, though only site admins may decide.
#[server(prefix = "/api")]
pub async fn list_pending_volunteer_applications() -> Result<Vec<Volunteer>, ServerFnError> {
    use crate::server::db::volunteers;
    use crate::server::permissions::{require_operations_admin, require_user};

    let actor = require_user().await?;
    require_operations_admin(&actor)?;
    volunteers::list_pending().await.map_err(ServerFnError::new)
}

/// How many volunteer applications are waiting on a decision.
#[server(prefix = "/api")]
pub async fn pending_volunteer_application_count() -> Result<i64, ServerFnError> {
    use crate::server::db::volunteers;
    use crate::server::permissions::{require_operations_admin, require_user};

    let actor = require_user().await?;
    require_operations_admin(&actor)?;
    volunteers::pending_count()
        .await
        .map_err(ServerFnError::new)
}

/// Approve or decline a volunteer application.
///
/// Site admins only: approving grants the Volunteer role, and changing a user's
/// role is site-admin-only everywhere else in the app (see
/// [`crate::server_fns::users::set_user_role`]). Either outcome emails the
/// applicant; a decline has no other visible effect, and they may apply again.
#[server(prefix = "/api")]
pub async fn decide_volunteer_application(
    user_id: String,
    approve: bool,
    note: String,
) -> Result<(), ServerFnError> {
    use crate::server::db::volunteers;
    use crate::server::permissions::{require_site_admin, require_user};

    let actor = require_user().await?;
    require_site_admin(&actor)?;
    let user_id = user_id.trim().to_string();
    if user_id.is_empty() {
        return Err(ServerFnError::new("No application was chosen."));
    }
    if note.chars().count() > 1_000 {
        return Err(ServerFnError::new(
            "Notes must be 1,000 characters or fewer.",
        ));
    }
    let note = note.trim().to_string();
    volunteers::decide(&user_id, approve, &actor.id, &actor.full_name(), &note)
        .await
        .map_err(ServerFnError::new)?;
    crate::server::notifications::notify_volunteer_decision(user_id, approve, note);
    Ok(())
}
