//! Case-chat unread notifications: the small client/server surface behind the
//! "Case Chat" nav badge and the per-channel unread dots.
//!
//! A [`ChannelUnread`] is a compact "you have N unread messages in this channel"
//! fact for the signed-in user. [`load_unread_notifications`] returns the full
//! set (the nav highlights whenever it is non-empty); [`mark_channel_read`]
//! clears one channel's entries the moment the user opens it. The rows
//! themselves are created server-side when a message is posted — see
//! [`crate::server::db::channel_notifications`].

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// The signed-in user's unread message count for one channel. Carries the owning
/// case id too, so the case chat can light up both the channel and the case it
/// belongs to without a second lookup.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChannelUnread {
    pub case_id: String,
    pub channel_id: String,
    pub count: i64,
}

/// Every channel the signed-in user has unread messages in. An empty list means
/// nothing is unread; the nav badge is driven purely by whether this is empty.
#[server(prefix = "/api")]
pub async fn load_unread_notifications() -> Result<Vec<ChannelUnread>, ServerFnError> {
    use crate::server::db::channel_notifications;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    channel_notifications::unread_for_user(&user.id)
        .await
        .map_err(ServerFnError::new)
}

/// Clear the signed-in user's unread notifications for a channel — called when
/// they open it. The channel is resolved through the same access gate the chat
/// itself uses, so a caller can only ever clear notifications on a channel they
/// are actually allowed to read.
#[server(prefix = "/api")]
pub async fn mark_channel_read(channel_id: String) -> Result<(), ServerFnError> {
    use crate::server::db::channel_notifications;
    use crate::server::permissions::{require_channel, require_user};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    require_channel(&user, &channel_id, CaseCapability::ViewCase).await?;
    channel_notifications::mark_channel_read(&user.id, &channel_id)
        .await
        .map_err(ServerFnError::new)?;
    Ok(())
}
