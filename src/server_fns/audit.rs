//! The audit change-log entry shared by the client and the server.
//!
//! A single [`ChangeLogEntry`] records who changed which field of an entity from
//! what to what, and when. The same shape is produced by the server's audit
//! store ([`crate::server::db::audit`], which logs changes to both users and
//! cases) and rendered by the admin and case pages, so it lives here in the
//! shared `server_fns` layer rather than in the server-only DB module.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::server_fns::pagination::Page;

/// A single audited change: who changed which field from what to what, and when.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChangeLogEntry {
    pub id: String,
    /// Display name of the actor who made the change.
    pub actor: String,
    /// The field or property that changed.
    pub field: String,
    pub old_value: String,
    pub new_value: String,
    /// Human-readable timestamp (`YYYY-MM-DD HH:MM`, local time).
    pub at: String,
}

/// Which kind of entity an audit query targets. Mirrors the server-only
/// `db::audit::Entity`, but lives here so the client can name the scope it wants
/// without depending on the SSR-only DB layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditScope {
    User,
    Case,
    Contact,
    Organization,
    Grant,
}

#[server(prefix = "/api")]
pub async fn list_audit_page(
    scope: AuditScope,
    entity_id: String,
    start: String,
    end: String,
    offset: i64,
    limit: i64,
) -> Result<Page<ChangeLogEntry>, ServerFnError> {
    use crate::server::db::audit::{self, Entity};
    use crate::server::permissions::{
        has_volunteer_access, require_case_view_or_admin_read, require_operations_admin,
        require_site_admin, require_user,
    };

    /// Hard cap on how many audit rows a single request may return, regardless
    /// of what the client asks for. The client paginates in small windows, but
    /// this enforces the "no view ever pulls a whole audit history into the
    /// browser at once" guarantee on the server too, rather than trusting the
    /// caller.
    const MAX_LIMIT: i64 = 200;

    let user = require_user().await?;
    let entity = match scope {
        AuditScope::User => {
            require_site_admin(&user)?;
            Entity::User
        }
        AuditScope::Case => {
            require_operations_admin(&user)?;
            require_case_view_or_admin_read(&user, &entity_id).await?;
            Entity::Case
        }
        // CRM history is back-office reading: operations admins and above, and
        // never a client, who cannot see the underlying records at all.
        AuditScope::Contact => {
            require_operations_admin(&user)?;
            Entity::Contact
        }
        AuditScope::Organization => {
            require_operations_admin(&user)?;
            Entity::Organization
        }
        AuditScope::Grant => {
            require_operations_admin(&user)?;
            Entity::Grant
        }
    };

    audit::page(
        entity,
        &entity_id,
        &start,
        &end,
        offset.max(0),
        limit.clamp(1, MAX_LIMIT),
        has_volunteer_access(&user),
    )
    .await
    .map_err(ServerFnError::new)
}
