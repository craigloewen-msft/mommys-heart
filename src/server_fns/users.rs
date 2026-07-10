//! User/admin server functions: role changes and per-case capability
//! assignments. All require administrator rights.

use leptos::prelude::*;

use crate::types::{AccountRole, CaseCapability, Page, User};

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
