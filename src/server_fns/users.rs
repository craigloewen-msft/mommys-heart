//! User/admin server functions: role changes and per-case capability
//! assignments. All require operations-admin permissions.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::server_fns::capabilities::{CaseAssignment, CaseCapability};
use crate::server_fns::pagination::Page;

/// The global account type a user has. This controls app-level access (e.g.
/// operations and site admins can reach the Admin dashboard). It is
/// intentionally separate from per-case capabilities: all account types view
/// and work cases the same way; what differs per case is their set of
/// [`CaseCapability`](crate::server_fns::capabilities::CaseCapability)s.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountRole {
    /// A client the organization is helping.
    Client,
    /// A volunteer working cases on behalf of clients.
    Volunteer,
    /// An operations administrator who may manage their own case access and
    /// request changes to other users.
    OperationsAdmin,
    /// A site administrator with unrestricted administrative access.
    SiteAdmin,
}

impl AccountRole {
    pub const ALL: [AccountRole; 4] = [
        AccountRole::Client,
        AccountRole::Volunteer,
        AccountRole::OperationsAdmin,
        AccountRole::SiteAdmin,
    ];

    pub fn label(self) -> &'static str {
        match self {
            AccountRole::Client => "Client",
            AccountRole::Volunteer => "Volunteer",
            AccountRole::OperationsAdmin => "Operations admin",
            AccountRole::SiteAdmin => "Site admin",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            AccountRole::Client => "client",
            AccountRole::Volunteer => "volunteer",
            AccountRole::OperationsAdmin => "operations_admin",
            AccountRole::SiteAdmin => "site_admin",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.slug() == s)
    }

    /// Whether this is exactly the site-administrator role.
    pub fn is_site_admin(self) -> bool {
        matches!(self, AccountRole::SiteAdmin)
    }

    /// Whether this is exactly the operations-administrator role.
    pub fn is_operations_admin(self) -> bool {
        matches!(self, AccountRole::OperationsAdmin)
    }

    /// Whether this role may perform operations-admin actions.
    /// Site administrators inherit these permissions.
    pub fn has_operations_admin_permissions(self) -> bool {
        self.is_operations_admin() || self.is_site_admin()
    }

    pub fn has_volunteer_privileges(self) -> bool {
        matches!(
            self,
            AccountRole::Volunteer | AccountRole::OperationsAdmin | AccountRole::SiteAdmin
        )
    }

    pub fn badge_classes(self) -> &'static str {
        match self {
            AccountRole::SiteAdmin => {
                "bg-primary-500/15 text-primary-300 ring-1 ring-primary-500/30"
            }
            AccountRole::OperationsAdmin => {
                "bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/30"
            }
            AccountRole::Volunteer => "bg-sky-500/15 text-sky-300 ring-1 ring-sky-500/30",
            AccountRole::Client => "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30",
        }
    }
}

/// A user summary
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserSummary {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub role: AccountRole,
    /// Stored grant for the shared Contacts, Organizations, and Funding areas.
    pub information_management_access: bool,
}

impl UserSummary {
    /// Convenience: the user's full display name.
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
            .trim()
            .to_string()
    }

    /// Whether this account's role and stored grant both allow information access.
    pub fn has_information_management_access(&self) -> bool {
        self.role.has_volunteer_privileges() && self.information_management_access
    }
}

/// Whether a volunteer has completed the volunteer agreement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgreementStatus {
    /// They accepted a specific version of the volunteer agreement.
    Completed,
    /// No accepted agreement on file. True for volunteers who predate the
    /// agreement, who were backfilled by migration.
    Outstanding,
}

impl AgreementStatus {
    pub fn label(self) -> &'static str {
        match self {
            AgreementStatus::Completed => "Completed",
            AgreementStatus::Outstanding => "Outstanding",
        }
    }

    pub fn badge_classes(self) -> &'static str {
        match self {
            AgreementStatus::Completed => {
                "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30"
            }
            AgreementStatus::Outstanding => {
                "bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/30"
            }
        }
    }
}

/// One row of the admin "Volunteers" list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VolunteerListItem {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub agreement: AgreementStatus,
}

impl VolunteerListItem {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
            .trim()
            .to_string()
    }
}

/// Which exact account-role section of the grouped admin user directory to load.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserDirectoryRoleGroup {
    Volunteer,
    Client,
    Other,
}

/// One narrow row of the grouped admin user directory.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserDirectoryItem {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub role: AccountRole,
    /// Volunteer-agreement status for exact volunteer accounts only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agreement: Option<AgreementStatus>,
    /// Distinct cases this user can access.
    pub assignment_count: i64,
    /// Concise human-readable summary of that access.
    pub assignment_summary: String,
}

impl UserDirectoryItem {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
            .trim()
            .to_string()
    }
}

/// An application user account
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: String,
    pub home_address: String,
    pub role: AccountRole,
    /// Stored grant for the shared Contacts, Organizations, and Funding areas.
    pub information_management_access: bool,
    /// Cases this user is assigned to, with the capabilities they hold on each.
    #[serde(default)]
    pub assigned_cases: Vec<CaseAssignment>,
}

impl User {
    /// Convenience: the user's full display name.
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
            .trim()
            .to_string()
    }

    /// Whether this account's role and stored grant both allow information access.
    pub fn has_information_management_access(&self) -> bool {
        self.role.has_volunteer_privileges() && self.information_management_access
    }

    /// This user's capabilities on a given case (empty if not assigned).
    pub fn capabilities_for(&self, case_id: &str) -> Vec<CaseCapability> {
        self.assigned_cases
            .iter()
            .find(|a| a.case_id == case_id)
            .map(|a| a.capabilities.clone())
            .unwrap_or_default()
    }

    /// Whether this user is assigned to the given case at all.
    pub fn is_assigned_to(&self, case_id: &str) -> bool {
        self.assigned_cases.iter().any(|a| a.case_id == case_id)
    }
}

impl From<User> for UserSummary {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            first_name: user.first_name,
            last_name: user.last_name,
            role: user.role,
            information_management_access: user.information_management_access,
        }
    }
}

/// Server-side typeahead search over users for the case owner-picker. Requires
/// operations-admin permissions because it returns other users' confidential
/// names and its sole consumer is owner reassignment.
#[server(prefix = "/api")]
pub async fn search_users(query: String) -> Result<Vec<UserSummary>, ServerFnError> {
    use crate::server::db::users;
    use crate::server::permissions::{require_operations_admin, require_user};

    let actor = require_user().await?;
    require_operations_admin(&actor)?;
    users::search_user_summaries(&query, 10)
        .await
        .map_err(ServerFnError::new)
}

/// One page of users for the admin management screen, ordered by id, with an
/// optional case-insensitive search over id/name/email. Requires
/// operations-admin permissions.
///
/// Backs the admin dashboard's server-side pagination ("Load more") so the UI
/// never has to pull every user into the browser.
#[server(prefix = "/api")]
pub async fn list_users_page(
    offset: i64,
    limit: i64,
    search: String,
) -> Result<Page<User>, ServerFnError> {
    use crate::server::db::users;
    use crate::server::permissions::{require_operations_admin, require_user};

    let actor = require_user().await?;
    require_operations_admin(&actor)?;
    users::page(offset, limit, &search)
        .await
        .map_err(ServerFnError::new)
}

/// One page of volunteer accounts for the admin "Volunteers" tab, with the same
/// optional search as [`list_users_page`]. Requires operations-admin permissions.
#[server(prefix = "/api")]
pub async fn list_volunteers_page(
    offset: i64,
    limit: i64,
    search: String,
) -> Result<Page<VolunteerListItem>, ServerFnError> {
    use crate::server::db::users;
    use crate::server::permissions::{require_operations_admin, require_user};

    let actor = require_user().await?;
    require_operations_admin(&actor)?;
    users::volunteers_page(
        offset,
        limit,
        &search,
        crate::helpers::volunteer_terms::VOLUNTEER_AGREEMENT_VERSION,
    )
    .await
    .map_err(ServerFnError::new)
}

/// One grouped section of the admin user directory, using a narrow row DTO so
/// contact details and full assignments are fetched only when needed.
#[server(prefix = "/api")]
pub async fn list_user_directory_page(
    offset: i64,
    limit: i64,
    search: String,
    role_group: UserDirectoryRoleGroup,
) -> Result<Page<UserDirectoryItem>, ServerFnError> {
    use crate::server::db::users;
    use crate::server::permissions::{require_operations_admin, require_user};

    let actor = require_user().await?;
    require_operations_admin(&actor)?;
    users::directory_page(
        offset,
        limit,
        &search,
        role_group,
        crate::helpers::volunteer_terms::VOLUNTEER_AGREEMENT_VERSION,
    )
    .await
    .map_err(ServerFnError::new)
}

/// Lazily load one full user record for the admin management view.
#[server(prefix = "/api")]
pub async fn load_admin_user(user_id: String) -> Result<User, ServerFnError> {
    use crate::server::db::users;
    use crate::server::permissions::{require_operations_admin, require_user};

    let actor = require_user().await?;
    require_operations_admin(&actor)?;
    let user_id = user_id.trim().to_string();
    if user_id.is_empty() {
        return Err(ServerFnError::new("No user was requested."));
    }
    users::get(&user_id)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("User not found."))
}

/// Change a user's account role.
#[server(prefix = "/api")]
pub async fn set_user_role(user_id: String, role: AccountRole) -> Result<(), ServerFnError> {
    use crate::server::db::users;
    use crate::server::permissions::{require_site_admin, require_user};

    let actor = require_user().await?;
    require_site_admin(&actor)?;
    let user_id = user_id.trim().to_string();
    let changed = users::set_role(&user_id, role, &actor.full_name())
        .await
        .map_err(|error| {
            let message = error.to_string();
            if message.contains("You cannot demote the final site admin.") {
                ServerFnError::new("You cannot demote the final site admin.")
            } else {
                ServerFnError::new(error)
            }
        })?;
    if changed {
        crate::server::notifications::notify_account_permissions_changed(
            user_id,
            actor.full_name(),
            format!("changed your account role to {}", role.label()),
        );
    }
    Ok(())
}

/// Grant or revoke access to Contacts, Organizations, and Funding information.
#[server(prefix = "/api")]
pub async fn set_information_management_access(
    user_id: String,
    enabled: bool,
) -> Result<(), ServerFnError> {
    use crate::server::db::users;
    use crate::server::permissions::{require_site_admin, require_user};

    let actor = require_user().await?;
    require_site_admin(&actor)?;
    let user_id = user_id.trim().to_string();
    let changed =
        users::set_information_management_access(&user_id, enabled, &actor.id, &actor.full_name())
            .await
            .map_err(ServerFnError::new)?;
    if changed {
        let change = if enabled {
            "granted you access to Contacts, Organizations, and Funding"
        } else {
            "revoked your access to Contacts, Organizations, and Funding"
        };
        crate::server::notifications::notify_account_permissions_changed(
            user_id,
            actor.full_name(),
            change.to_string(),
        );
    }
    Ok(())
}

/// Assign a user to a case with an explicit set of capabilities.
#[server(prefix = "/api")]
pub async fn assign_case(
    user_id: String,
    case_id: String,
    capabilities: Vec<CaseCapability>,
) -> Result<(), ServerFnError> {
    use crate::server::db::users;
    use crate::server::permissions::{require_case_access_management, require_user};
    use crate::server_fns::capabilities::validate_capabilities;

    let actor = require_user().await?;
    require_case_access_management(&actor, &user_id)?;
    validate_capabilities(&capabilities).map_err(ServerFnError::new)?;
    let was_assigned = users::is_assigned(&user_id, &case_id).await.unwrap_or(true);
    users::assign_capabilities(&user_id, &case_id, &capabilities, &actor.full_name())
        .await
        .map_err(ServerFnError::new)?;
    if !was_assigned && capabilities.contains(&CaseCapability::ViewCase) {
        crate::server::notifications::notify_assignment(
            user_id.clone(),
            actor.full_name(),
            case_id.clone(),
        );
    }
    Ok(())
}

/// Toggle a single capability for a user on a case.
#[server(prefix = "/api")]
pub async fn toggle_capability(
    user_id: String,
    case_id: String,
    capability: CaseCapability,
    enabled: bool,
) -> Result<(), ServerFnError> {
    use crate::server::db::users;
    use crate::server::permissions::{require_case_access_management, require_user};

    let actor = require_user().await?;
    require_case_access_management(&actor, &user_id)?;
    users::toggle_capability(&user_id, &case_id, capability, enabled, &actor.full_name())
        .await
        .map_err(ServerFnError::new)
}

/// Remove a user's assignment to a case entirely.
#[server(prefix = "/api")]
pub async fn unassign_case(user_id: String, case_id: String) -> Result<(), ServerFnError> {
    use crate::server::db::users;
    use crate::server::permissions::{require_case_access_management, require_user};

    let actor = require_user().await?;
    require_case_access_management(&actor, &user_id)?;
    users::unassign(&user_id, &case_id, &actor.full_name())
        .await
        .map_err(ServerFnError::new)
}

/// Apply a batch of per-case capability changes for one user in a single
/// request. Each change is either a new capability set for a case (`Some`) or a
/// removal of the assignment (`None`). Backs the admin "Edit → Save" flow so a
/// whole draft applies in one round-trip instead of one server call per checkbox.
#[server(prefix = "/api", input = leptos::server_fn::codec::Json)]
pub async fn save_case_capabilities(
    user_id: String,
    changes: Vec<(String, Option<Vec<CaseCapability>>)>,
) -> Result<(), ServerFnError> {
    use crate::server::db::users;
    use crate::server::permissions::{require_case_access_management, require_user};
    use crate::server_fns::capabilities::validate_capabilities;

    let actor = require_user().await?;
    require_case_access_management(&actor, &user_id)?;
    for (_, caps) in &changes {
        if let Some(caps) = caps {
            validate_capabilities(caps).map_err(ServerFnError::new)?;
        }
    }
    let actor_name = actor.full_name();
    for (case_id, caps) in changes {
        match caps {
            Some(caps) => {
                let was_assigned = users::is_assigned(&user_id, &case_id).await.unwrap_or(true);
                users::assign_capabilities(&user_id, &case_id, &caps, &actor_name)
                    .await
                    .map_err(ServerFnError::new)?;
                if !was_assigned && caps.contains(&CaseCapability::ViewCase) {
                    crate::server::notifications::notify_assignment(
                        user_id.clone(),
                        actor_name.clone(),
                        case_id.clone(),
                    );
                }
            }
            None => users::unassign(&user_id, &case_id, &actor_name)
                .await
                .map_err(ServerFnError::new)?,
        }
    }
    Ok(())
}
