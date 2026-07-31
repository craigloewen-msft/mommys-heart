//! Authorization — the server-side *gate* for the server-function layer (SSR
//! only). This is where per-case [`CaseCapability`]s (the vocabulary defined in
//! [`crate::server_fns::capabilities`]) and account roles are actually turned
//! into allow/deny decisions.
//!
//! This is the *authorization* half of access control ("what may you do?"),
//! kept separate from [`crate::server::auth`], which handles *authentication*
//! ("who are you?" — passwords, sessions, the [`AuthUser`] extractor).
//!
//! Every `#[server]` function in [`crate::server_fns`] resolves the caller with
//! [`require_user`] (from the session cookie) and then checks authorization with
//! the helpers here: [`require_admin`] for the account-level role gate,
//! [`require_cap`] for a specific per-case capability, and [`require_channel`]
//! for the case chat, which combines a capability check with the account-role
//! rule that keeps clients out of a case's volunteer-only channel. Keeping
//! access control in one place — rather than repeating it in each operation —
//! means every authenticated entry point is guarded the same way before it ever
//! touches the database in [`crate::server::db`].
//!
//! [`AuthUser`]: crate::server::auth::AuthUser

use leptos::prelude::ServerFnError;

use crate::server::auth::AuthUser;
use crate::server::db::{cases, channels};
use crate::server_fns::capabilities::CaseCapability;
use crate::server_fns::channels::Channel;
use crate::server_fns::users::{AccountRole, User};

/// Resolve the signed-in user from the request's session cookie, or an error if
/// there is no valid session. Uses the same [`AuthUser`] extractor the rest of
/// the server relies on, pulled from the Leptos request context.
pub async fn require_user() -> Result<User, ServerFnError> {
    let AuthUser(user) = leptos_axum::extract().await?;
    Ok(user)
}

/// Reject unless the caller is an administrator.
pub fn require_admin(user: &User) -> Result<(), ServerFnError> {
    if user.role.is_admin() {
        Ok(())
    } else {
        Err(ServerFnError::new("Admin access required."))
    }
}

/// Whether this account may see a case's restricted (volunteer-only) chat
/// channel. This is an **account-role** gate, deliberately orthogonal to the
/// per-case capability gate: staff — volunteers and admins — get the private
/// back-channel, and a client never does, no matter how many capabilities they
/// have been granted on the case.
pub fn sees_restricted_channels(user: &User) -> bool {
    !matches!(user.role, AccountRole::Client)
}

/// The caller's capabilities on a case: exactly the set granted by their
/// assignment (no implicit grants for owners or admins — access is governed
/// solely by the stored capabilities). Errors if the case does not exist.
pub async fn capabilities_on(
    user: &User,
    case_id: &str,
) -> Result<Vec<CaseCapability>, ServerFnError> {
    // Confirm the case exists so callers get a clear "not found" error rather
    // than an ambiguous empty-capabilities result.
    cases::owner_id(case_id)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("Case not found."))?;
    Ok(user.capabilities_for(case_id))
}

/// Require a specific capability on a case, else a `Forbidden`-style error.
pub async fn require_cap(
    user: &User,
    case_id: &str,
    cap: CaseCapability,
) -> Result<(), ServerFnError> {
    if capabilities_on(user, case_id).await?.contains(&cap) {
        Ok(())
    } else {
        Err(ServerFnError::new(format!(
            "You do not have permission to {} on this case.",
            cap.label().to_lowercase()
        )))
    }
}

/// The single gate for acting inside a case's chat channel.
///
/// The channel is resolved from the database *first*, so the case the
/// capability is checked against always comes from the stored channel and never
/// from a case id supplied by the caller. Both access checks are then applied:
/// `cap` on the owning case, and — for the volunteer-only channel — the
/// [`sees_restricted_channels`] account-role gate that keeps clients out.
///
/// A client asking for the volunteer-only channel gets the same "not found"
/// answer as for a channel id that does not exist, so the private thread's
/// existence is never disclosed.
pub async fn require_channel(
    user: &User,
    channel_id: &str,
    cap: CaseCapability,
) -> Result<Channel, ServerFnError> {
    let channel = channels::get(channel_id)
        .await
        .map_err(ServerFnError::new)?
        .filter(|c| sees_restricted_channels(user) || !c.kind.is_restricted())
        .ok_or_else(|| ServerFnError::new("Channel not found."))?;
    require_cap(user, &channel.case_id, cap).await?;
    Ok(channel)
}
