//! The audit change-log entry shared by the client and the server.
//!
//! A single [`ChangeLogEntry`] records who changed which field of an entity from
//! what to what, and when. The same shape is produced by the server's audit
//! store ([`crate::server::db::audit`], which logs changes to both users and
//! cases) and rendered by the admin and case pages, so it lives here in the
//! shared `server_fns` layer rather than in the server-only DB module.

use serde::{Deserialize, Serialize};

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
    /// Human-readable timestamp (mock; ISO or "just now").
    pub at: String,
}
