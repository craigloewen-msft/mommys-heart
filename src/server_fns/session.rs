//! Session bootstrap: everything the UI needs immediately after sign-in.

use leptos::prelude::*;

use crate::types::BootstrapResponse;

/// Load the signed-in user plus the data the UI caches, **scoped to what the
/// caller may see**:
///
/// * **Admins** get the full picture: every user, every case, and every grant.
/// * **Everyone else** gets only their visible cases, a redacted name-resolution
///   directory limited to the users referenced by those cases (owners +
///   assignees), and no grants (grants are an admin-only feature).
///
/// This keeps other users' PII and audit history on the server and avoids
/// loading or serializing the whole database for ordinary users.
#[server(prefix = "/api")]
pub async fn bootstrap() -> Result<BootstrapResponse, ServerFnError> {
    use crate::server::permissions::{require_user, visible};
    use crate::server::db::{cases, grants, users};
    use std::collections::BTreeSet;

    let user = require_user().await?;

    if user.role.is_admin() {
        return Ok(BootstrapResponse {
            current_user: Some(user),
            users: users::list().await.map_err(ServerFnError::new)?,
            cases: cases::list_all().await.map_err(ServerFnError::new)?,
            grants: grants::list().await.map_err(ServerFnError::new)?,
        });
    }

    let cases = visible(&user, cases::list_all().await.map_err(ServerFnError::new)?);

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
