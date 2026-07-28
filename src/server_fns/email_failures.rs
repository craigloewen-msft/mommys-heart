//! The email-delivery-failure record shared by the client and the server.
//!
//! A single [`EmailFailure`] captures one outbound email that ultimately failed
//! to send: who it was addressed to, what it was, and the error. The server's
//! store ([`crate::server::db::email_failures`]) produces these and the admin
//! dashboard renders them, so — like [`crate::server_fns::audit::ChangeLogEntry`]
//! — the shape lives here in the shared `server_fns` layer rather than in the
//! SSR-only DB module.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::server_fns::pagination::Page;

/// One recorded outbound-email delivery failure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EmailFailure {
    pub id: String,
    /// The address we tried to email.
    pub recipient: String,
    /// The subject line of the message we tried to send.
    pub subject: String,
    /// Which flow produced it, e.g. `Case notification` or `Authentication email`.
    pub context: String,
    /// The final error surfaced by the send.
    pub error: String,
    /// Human-readable timestamp (`YYYY-MM-DD HH:MM`, local time).
    pub at: String,
}

/// List recorded email failures, newest first. Admin-only.
#[server(prefix = "/api")]
pub async fn list_email_failures_page(
    offset: i64,
    limit: i64,
) -> Result<Page<EmailFailure>, ServerFnError> {
    use crate::server::db::email_failures;
    use crate::server::permissions::{require_admin, require_user};

    /// Hard server-side cap on rows per request, regardless of what the client
    /// asks for — the admin view paginates in small windows.
    const MAX_LIMIT: i64 = 200;

    let user = require_user().await?;
    require_admin(&user)?;

    email_failures::page(offset.max(0), limit.clamp(1, MAX_LIMIT))
        .await
        .map_err(ServerFnError::new)
}
