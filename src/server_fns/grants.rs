//! Grant server functions (admin only).

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// A grant as shown on the grants screen: id + name. A durable, serializable
/// list-view shape shared by the DB layer that produces it and the page that
/// renders it, so it is defined exactly once.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GrantSummary {
    pub id: String,
    pub name: String,
}

/// Every grant, ready to render on the grants screen (admin only). This is the
/// grants page's single source of data.
#[server(prefix = "/api")]
pub async fn load_grants() -> Result<Vec<GrantSummary>, ServerFnError> {
    use crate::server::db::grants;
    use crate::server::permissions::{require_admin, require_user};

    let user = require_user().await?;
    require_admin(&user)?;
    grants::summaries().await.map_err(ServerFnError::new)
}

/// Create a new grant. Returns its id.
#[server(prefix = "/api")]
pub async fn add_grant(name: String) -> Result<String, ServerFnError> {
    use crate::server::permissions::{require_admin, require_user};
    use crate::server::db::grants;

    let user = require_user().await?;
    require_admin(&user)?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ServerFnError::new("Grant name is required."));
    }
    grants::create(&name).await.map_err(ServerFnError::new)
}

/// Rename an existing grant.
#[server(prefix = "/api")]
pub async fn rename_grant(grant_id: String, name: String) -> Result<(), ServerFnError> {
    use crate::server::permissions::{require_admin, require_user};
    use crate::server::db::grants;

    let user = require_user().await?;
    require_admin(&user)?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ServerFnError::new("Grant name is required."));
    }
    grants::rename(&grant_id, &name)
        .await
        .map_err(ServerFnError::new)
}

/// Delete a grant.
#[server(prefix = "/api")]
pub async fn delete_grant(grant_id: String) -> Result<(), ServerFnError> {
    use crate::server::permissions::{require_admin, require_user};
    use crate::server::db::grants;

    let user = require_user().await?;
    require_admin(&user)?;
    grants::delete(&grant_id).await.map_err(ServerFnError::new)
}
