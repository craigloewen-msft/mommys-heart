//! Session bootstrap: everything the UI needs immediately after sign-in.

use leptos::prelude::*;

use crate::types::BootstrapResponse;

/// Load the signed-in user plus the data the UI caches, **scoped to what the
/// caller may see**:
///
/// * **Cases** are always scoped to the caller — the cases they own or are
///   assigned to (admins are not special here; they assign themselves to cases
///   like anyone else). Only a lightweight directory (names/status only) is
///   shipped; full, paginated detail is loaded on demand by the case screen
///   ([`crate::server_fns::cases::list_cases_page`]).
/// * **Admins** additionally get a lightweight directory of every user
///   (names/role only) plus all grants, since they manage users and permissions.
///   Full, paginated user detail is loaded on demand by the admin dashboard
///   ([`crate::server_fns::users::list_users_page`]).
/// * **Everyone else** gets a redacted name-resolution directory limited to the
///   users referenced by their cases (owners + assignees), and no grants (grants
///   are an admin-only feature).
///
/// This keeps other users' PII and audit history on the server and avoids
/// loading or serializing the whole database on every bootstrap.
#[server(prefix = "/api")]
pub async fn bootstrap() -> Result<BootstrapResponse, ServerFnError> {
    use crate::server::permissions::require_user;
    use crate::server::db::{cases, grants, users};
    use std::collections::BTreeSet;

    let user = require_user().await?;

    // Cases are always scoped to the caller (owned or assigned), so bootstrap
    // stays cheap no matter how many cases exist system-wide.
    let cases = cases::directory_for(&user.id)
        .await
        .map_err(ServerFnError::new)?;

    if user.role.is_admin() {
        return Ok(BootstrapResponse {
            current_user: Some(user),
            // A lightweight directory of every user (names + role only) — enough
            // for name resolution and owner pickers. The admin dashboard loads
            // full, paginated user detail on demand via `list_users_page`, so we
            // never hydrate (and ship) every user's PII + audit log here.
            users: users::directory_all().await.map_err(ServerFnError::new)?,
            cases,
            grants: grants::list().await.map_err(ServerFnError::new)?,
        });
    }

    // Only the users referenced by the caller's visible cases need to be
    // resolvable by name in the UI (case owners + assignees), plus the caller.
    let case_ids: Vec<String> = cases.iter().map(|c| c.id.clone()).collect();
    let mut referenced: BTreeSet<String> = BTreeSet::new();
    referenced.insert(user.id.clone());
    for c in &cases {
        referenced.insert(c.owner_id.clone());
    }
    for id in users::ids_assigned_to_cases(&case_ids)
        .await
        .map_err(ServerFnError::new)?
    {
        referenced.insert(id);
    }
    let ids: Vec<String> = referenced.into_iter().collect();

    Ok(BootstrapResponse {
        current_user: Some(user),
        users: users::directory(&ids).await.map_err(ServerFnError::new)?,
        cases,
        grants: Vec::new(),
    })
}
