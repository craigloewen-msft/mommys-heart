//! The unread/attention counts the app chrome shows, fetched in one request.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::server_fns::channel_notifications::ChannelUnread;

/// Everything the nav and admin tabs badge, resolved together.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AppBadges {
    pub unread: Vec<ChannelUnread>,
    pub admin_requests_pending: i64,
    pub cases_pending_review: i64,
}

/// Load every badge count for the signed-in user.
///
/// One round trip because these are always wanted together on sign-in, and each
/// count is zero for anyone not permitted to see it rather than an error.
#[server(prefix = "/api")]
pub async fn load_app_badges() -> Result<AppBadges, ServerFnError> {
    use crate::server::db::{admin_requests, cases, channel_notifications};
    use crate::server::permissions::require_user;

    let user = require_user().await?;

    let unread = channel_notifications::unread_for_user(&user.id)
        .await
        .map_err(ServerFnError::new)?;
    let admin_requests_pending = if user.role.is_site_admin() {
        admin_requests::pending_count()
            .await
            .map_err(ServerFnError::new)?
    } else {
        0
    };
    // Any admin may review a case, unlike approval requests.
    let cases_pending_review = if user.role.has_operations_admin_permissions() {
        cases::pending_review_count()
            .await
            .map_err(ServerFnError::new)?
    } else {
        0
    };

    Ok(AppBadges {
        unread,
        admin_requests_pending,
        cases_pending_review,
    })
}
