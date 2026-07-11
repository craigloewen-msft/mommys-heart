//! User/admin server functions: role changes and per-case capability
//! assignments. All require administrator rights.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::types::{AccountRole, CaseAssignment, CaseCapability, Page};

/// A user summary
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserSummary {
    pub id: String,
    pub name: String,
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
    /// Plaintext for the local demo only. Do not use this pattern for real auth.
    pub password: String,
    pub role: AccountRole,
    /// Cases this user is assigned to, with their permission on each.
    #[serde(default)]
    pub assigned_cases: Vec<CaseAssignment>
}

impl User {
    /// Convenience: the user's full display name.
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
            .trim()
            .to_string()
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

/// Server-side typeahead search over users for the case owner-picker
#[server(prefix = "/api")]
pub async fn search_users(query: String) -> Result<Vec<UserSummary>, ServerFnError> {
    use crate::server::db::users;
    use crate::server::permissions::require_user;

    require_user().await?;
    users::search_directory(&query, 10)
        .await
        .map_err(ServerFnError::new)
}

/// One page of users for the admin management screen, ordered by id, with an
/// optional case-insensitive search over id/name/email. Admin only.
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
    use crate::server::permissions::{require_admin, require_user};

    let actor = require_user().await?;
    require_admin(&actor)?;
    users::page(offset, limit, &search)
        .await
        .map_err(ServerFnError::new)
}

/// Change a user's account role.
#[server(prefix = "/api")]
pub async fn set_user_role(user_id: String, role: AccountRole) -> Result<(), ServerFnError> {
    use crate::server::permissions::{require_admin, require_user};
    use crate::server::db::users;

    let actor = require_user().await?;
    require_admin(&actor)?;
    users::set_role(&user_id, role, &actor.full_name())
        .await
        .map_err(ServerFnError::new)
}

/// Assign a user to a case with an explicit set of capabilities.
#[server(prefix = "/api")]
pub async fn assign_case(
    user_id: String,
    case_id: String,
    capabilities: Vec<CaseCapability>,
) -> Result<(), ServerFnError> {
    use crate::server::permissions::{require_admin, require_user};
    use crate::server::db::users;

    let actor = require_user().await?;
    require_admin(&actor)?;
    users::assign_capabilities(&user_id, &case_id, &capabilities, &actor.full_name())
        .await
        .map_err(ServerFnError::new)
}

/// Toggle a single capability for a user on a case.
#[server(prefix = "/api")]
pub async fn toggle_capability(
    user_id: String,
    case_id: String,
    capability: CaseCapability,
    enabled: bool,
) -> Result<(), ServerFnError> {
    use crate::server::permissions::{require_admin, require_user};
    use crate::server::db::users;

    let actor = require_user().await?;
    require_admin(&actor)?;
    users::toggle_capability(&user_id, &case_id, capability, enabled, &actor.full_name())
        .await
        .map_err(ServerFnError::new)
}

/// Remove a user's assignment to a case entirely.
#[server(prefix = "/api")]
pub async fn unassign_case(user_id: String, case_id: String) -> Result<(), ServerFnError> {
    use crate::server::permissions::{require_admin, require_user};
    use crate::server::db::users;

    let actor = require_user().await?;
    require_admin(&actor)?;
    users::unassign(&user_id, &case_id, &actor.full_name())
        .await
        .map_err(ServerFnError::new)
}

/// Apply a batch of per-case permission changes for one user in a single
/// request. Each change is either a new capability set for a case (`Some`) or a
/// removal of the assignment (`None`). Backs the admin "Edit → Save" flow so a
/// whole draft applies in one round-trip instead of one server call per checkbox.
#[server(prefix = "/api")]
pub async fn save_case_permissions(
    user_id: String,
    changes: Vec<(String, Option<Vec<CaseCapability>>)>,
) -> Result<(), ServerFnError> {
    use crate::server::permissions::{require_admin, require_user};
    use crate::server::db::users;

    let actor = require_user().await?;
    require_admin(&actor)?;
    let actor_name = actor.full_name();
    for (case_id, caps) in changes {
        match caps {
            Some(caps) => users::assign_capabilities(&user_id, &case_id, &caps, &actor_name)
                .await
                .map_err(ServerFnError::new)?,
            None => users::unassign(&user_id, &case_id, &actor_name)
                .await
                .map_err(ServerFnError::new)?,
        }
    }
    Ok(())
}
