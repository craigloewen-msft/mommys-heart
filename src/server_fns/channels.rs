//! Case chat **channels** (threads) shared by the client and the server: the
//! [`Channel`] type, its [`ChannelKind`], and the server functions that list,
//! create, and delete them.
//!
//! A case's chat is a set of channels rather than one flat thread. Every case
//! has exactly one [`ChannelKind::VolunteerOnly`] channel — the private staff
//! back-channel that assigned *clients* can neither see nor post in — plus one
//! or more [`ChannelKind::Standard`] channels (starting with "General") that
//! everyone with access to the case shares.
//!
//! Two independent gates decide what a caller may do here, and both are always
//! applied server-side:
//!
//! * **Case capabilities** — [`ViewCase`] to read a channel, [`SendMessages`] to
//!   post in it, and [`ManageChannels`] (the "Full access" set only) to create
//!   or delete one.
//! * **Account role** — a [`AccountRole::Client`] is excluded from
//!   volunteer-only channels no matter which capabilities they hold on the case.
//!
//! [`ViewCase`]: crate::server_fns::capabilities::CaseCapability::ViewCase
//! [`SendMessages`]: crate::server_fns::capabilities::CaseCapability::SendMessages
//! [`ManageChannels`]: crate::server_fns::capabilities::CaseCapability::ManageChannels
//! [`AccountRole::Client`]: crate::server_fns::users::AccountRole::Client

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// The longest a channel name may be, so the channel list stays readable.
pub const MAX_CHANNEL_NAME_LEN: usize = 40;

/// What kind of audience a channel is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelKind {
    Standard,
    VolunteerOnly,
}

impl ChannelKind {
    pub const ALL: [ChannelKind; 2] = [ChannelKind::Standard, ChannelKind::VolunteerOnly];

    pub fn slug(self) -> &'static str {
        match self {
            ChannelKind::Standard => "standard",
            ChannelKind::VolunteerOnly => "volunteer_only",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.slug() == s)
    }

    /// Whether a channel of this kind is hidden from client accounts.
    pub fn is_restricted(self) -> bool {
        matches!(self, ChannelKind::VolunteerOnly)
    }
}

/// The name given to the volunteer-only channel every case is created with.
pub const VOLUNTEER_CHANNEL_NAME: &str = "Volunteer only";

/// The name of the standard channel every case starts with.
pub const DEFAULT_CHANNEL_NAME: &str = "General";

/// One channel within a case's chat.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub case_id: String,
    pub name: String,
    pub kind: ChannelKind,
    /// Number of messages in this channel (shown beside its name in the list).
    #[serde(default)]
    pub message_count: usize,
}

impl Channel {
    /// Whether this channel may be deleted. The volunteer-only back-channel is
    /// permanent — the UI hides its delete control and the server refuses the
    /// request regardless.
    pub fn is_deletable(&self) -> bool {
        self.kind != ChannelKind::VolunteerOnly
    }
}

/// Validate and normalize a user-supplied channel name.
pub fn normalize_channel_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Channel name is required.".to_string());
    }
    if name.chars().count() > MAX_CHANNEL_NAME_LEN {
        return Err(format!(
            "Channel name must be {MAX_CHANNEL_NAME_LEN} characters or fewer."
        ));
    }
    if name.eq_ignore_ascii_case(VOLUNTEER_CHANNEL_NAME) {
        return Err(format!(
            "\"{VOLUNTEER_CHANNEL_NAME}\" is reserved for the private volunteer channel."
        ));
    }
    Ok(name.to_string())
}

/// The channels of a case the caller may actually read: every standard channel
/// plus, for volunteers and admins only, the volunteer-only channel. Requires
/// the `ViewCase` capability.
#[server(prefix = "/api")]
pub async fn list_channels(case_id: String) -> Result<Vec<Channel>, ServerFnError> {
    use crate::server::db::channels;
    use crate::server::permissions::{require_cap, require_user, sees_restricted_channels};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    require_cap(&user, &case_id, CaseCapability::ViewCase).await?;
    channels::list_visible(&case_id, sees_restricted_channels(&user))
        .await
        .map_err(ServerFnError::new)
}

/// Create a new standard channel on a case. Requires the `ManageChannels`
/// capability, which only the "Full access" capability set can carry. New
/// channels are always standard: the single volunteer-only channel is created
/// with the case and can never be added a second time.
#[server(prefix = "/api")]
pub async fn create_channel(case_id: String, name: String) -> Result<Channel, ServerFnError> {
    use crate::server::db::channels;
    use crate::server::permissions::{require_cap, require_user};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    require_cap(&user, &case_id, CaseCapability::ManageChannels).await?;
    let name = normalize_channel_name(&name).map_err(ServerFnError::new)?;
    let channel = channels::create(&case_id, &name, &user.full_name())
        .await
        .map_err(|e| {
            if channels::is_duplicate_name(&e) {
                ServerFnError::new(format!("This case already has a \"{name}\" channel."))
            } else {
                ServerFnError::new(e)
            }
        })?;
    crate::server::notifications::notify_case(
        case_id,
        user.id.clone(),
        user.full_name(),
        crate::server_fns::settings::NotificationKind::CaseData,
        format!("created the \"{name}\" message channel"),
        crate::server::notifications::Audience::Everyone,
    );
    Ok(channel)
}

/// Delete a channel and every message in it. Requires the `ManageChannels`
/// capability on the channel's case; the volunteer-only channel is rejected.
#[server(prefix = "/api")]
pub async fn delete_channel(channel_id: String) -> Result<(), ServerFnError> {
    use crate::server::db::channels;
    use crate::server::permissions::{require_cap, require_user};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    // Resolve the channel first so the capability is checked against the case it
    // actually belongs to, never a case id supplied by the caller.
    let channel = channels::get(&channel_id)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("Channel not found."))?;
    require_cap(&user, &channel.case_id, CaseCapability::ManageChannels).await?;
    if !channel.is_deletable() {
        return Err(ServerFnError::new(
            "The volunteer-only channel cannot be deleted.",
        ));
    }
    channels::delete(&channel_id, &channel.case_id, &user.full_name())
        .await
        .map_err(ServerFnError::new)?;
    crate::server::notifications::notify_case(
        channel.case_id,
        user.id.clone(),
        user.full_name(),
        crate::server_fns::settings::NotificationKind::CaseData,
        format!("deleted the \"{}\" message channel", channel.name),
        crate::server::notifications::Audience::Everyone,
    );
    Ok(())
}
