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

/// Where a volunteer application stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VolunteerStatus {
    Pending,
    Approved,
    Denied,
    /// Was approved, then lost the volunteer role. The accepted agreement is
    /// kept rather than deleted because it is a real historical fact; a revoked
    /// person may accept again and re-apply, exactly like a declined one.
    Revoked,
}

impl VolunteerStatus {
    pub fn slug(self) -> &'static str {
        match self {
            VolunteerStatus::Pending => "pending",
            VolunteerStatus::Approved => "approved",
            VolunteerStatus::Denied => "denied",
            VolunteerStatus::Revoked => "revoked",
        }
    }

    pub fn from_slug(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(VolunteerStatus::Pending),
            "approved" => Some(VolunteerStatus::Approved),
            "denied" => Some(VolunteerStatus::Denied),
            "revoked" => Some(VolunteerStatus::Revoked),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            VolunteerStatus::Pending => "Pending review",
            VolunteerStatus::Approved => "Approved",
            VolunteerStatus::Denied => "Declined",
            VolunteerStatus::Revoked => "Revoked",
        }
    }

    pub fn badge_classes(self) -> &'static str {
        match self {
            VolunteerStatus::Pending => "bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/30",
            VolunteerStatus::Approved => {
                "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30"
            }
            VolunteerStatus::Denied | VolunteerStatus::Revoked => {
                "bg-rose-500/15 text-rose-300 ring-1 ring-rose-500/30"
            }
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

/// One row of the admin's pending-application queue: who applied and when.
///
/// Deliberately narrow rather than a whole [`User`] plus their record — the queue
/// renders a name, an email and a date, and a pending applicant has no case
/// assignments worth loading.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Volunteer {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    /// Pre-formatted for display; only ever shown, never compared.
    pub agreed_at: String,
}

impl Volunteer {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
            .trim()
            .to_string()
    }
}

/// Accept the volunteer agreement.
///
/// For a client this files an application for an admin to review. For someone
/// who already holds the Volunteer role — an admin set it directly, or they
/// predate the agreement — it simply records the signed agreement against their
/// existing record, because there is nothing left to approve.
///
/// Administrators are refused. They have volunteer *privileges* without being
/// volunteers, and the `volunteers` table tracks the Volunteer role specifically:
/// recording one for an admin would contradict
/// [`crate::server::db::users::apply_role_in`]'s invariant, and treating it as an
/// application would demote them to Volunteer on approval.
#[server(prefix = "/api")]
pub async fn apply_to_volunteer(agreement_version: String) -> Result<(), ServerFnError> {
    use crate::server::db::volunteers;
    use crate::server::permissions::require_user;
    use crate::server_fns::users::AccountRole;

    let user = require_user().await?;
    if user.role.has_operations_admin_permissions() {
        return Err(ServerFnError::new(
            "Administrator accounts cannot apply to volunteer.",
        ));
    }
    // An old tab holding a stale version never saw the wording it claims to
    // accept, so make it re-read rather than record consent it cannot evidence.
    if !crate::helpers::volunteer_terms::is_current(agreement_version.trim()) {
        return Err(ServerFnError::new(
            "Please read and accept the current volunteer agreement.",
        ));
    }
    let existing = volunteers::get(&user.id)
        .await
        .map_err(ServerFnError::new)?;
    if existing
        .as_ref()
        .is_some_and(|application| application.status == VolunteerStatus::Pending)
    {
        return Err(ServerFnError::new(
            "Your volunteer application is already awaiting review.",
        ));
    }
    let already_a_volunteer = user.role == AccountRole::Volunteer;
    if already_a_volunteer
        && existing
            .as_ref()
            .is_some_and(VolunteerApplication::has_agreement)
    {
        return Err(ServerFnError::new(
            "You have already accepted the volunteer agreement.",
        ));
    }
    volunteers::apply(&user.id, agreement_version.trim(), already_a_volunteer)
        .await
        .map_err(ServerFnError::new)?;
    // Only a genuine application needs a decision, so only that emails the site
    // admins. An existing volunteer signing the paperwork has nothing to review.
    if !already_a_volunteer {
        crate::server::notifications::notify_volunteer_application_filed(
            user.full_name(),
            user.email.clone(),
        );
    }
    Ok(())
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
        .map_err(|error| ServerFnError::new(error.to_string()))?;
    crate::server::notifications::notify_volunteer_decision(user_id, approve, note);
    Ok(())
}
