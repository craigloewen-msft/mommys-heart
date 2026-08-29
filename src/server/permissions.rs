//! Authorization — the server-side *gate* for the server-function layer (SSR
//! only). This is where per-case [`CaseCapability`]s (the vocabulary defined in
//! [`crate::server_fns::capabilities`]) and account roles are actually turned
//! into allow/deny decisions.
//!
//! This is the *authorization* half of access control ("what may you do?"),
//! kept separate from [`crate::server::auth`], which handles *authentication*
//! ("who are you?" — passwords, sessions, the [`AuthUser`] extractor).
//!
//! Account role and case capabilities meet in one place: a site admin holds
//! every capability on every case without a stored assignment (see
//! [`AccountRole::has_full_case_access`]), so there is no separate
//! "admin inspection" path — everyone goes through [`require_cap`].
//!
//! Every `#[server]` function in [`crate::server_fns`] resolves the caller with
//! [`require_user`] (from the session cookie) and then checks authorization with
//! the helpers here: [`require_operations_admin`] for the account-level role gate,
//! [`require_cap`] for a specific per-case capability, and [`require_channel`]
//! for the case chat, which combines a capability check with the account-role
//! rule that keeps clients out of a case's volunteer-only channel. Keeping
//! access control in one place — rather than repeating it in each operation —
//! means every authenticated entry point is guarded the same way before it ever
//! touches the database in [`crate::server::db`].
//!
//! [`AuthUser`]: crate::server::auth::AuthUser

use leptos::prelude::ServerFnError;

use crate::helpers::visibility::Visibility;
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

/// Reject unless the caller is a site administrator.
pub fn require_site_admin(user: &User) -> Result<(), ServerFnError> {
    if user.role.is_site_admin() {
        Ok(())
    } else {
        Err(ServerFnError::new("Site admin access required."))
    }
}

/// Reject unless the caller has operations-admin permissions.
pub fn require_operations_admin(user: &User) -> Result<(), ServerFnError> {
    if user.role.has_operations_admin_permissions() {
        Ok(())
    } else {
        Err(ServerFnError::new("Operations-admin permissions required."))
    }
}

/// Whether the account's role and stored grant allow the information areas.
pub fn has_information_management_access(user: &User) -> bool {
    user.has_information_management_access()
}

/// Reject unless Contacts, Organizations, and Funding access has been granted.
pub fn require_information_management_access(user: &User) -> Result<(), ServerFnError> {
    if has_information_management_access(user) {
        Ok(())
    } else {
        Err(ServerFnError::new(
            "Access to contacts, organizations, and funding information has not been granted.",
        ))
    }
}

/// Reject unless the caller may manage the target user's case assignments and
/// capabilities. Site admins may manage anyone; operations admins may manage
/// only themselves.
pub fn require_case_access_management(
    actor: &User,
    target_user_id: &str,
) -> Result<(), ServerFnError> {
    let allowed = actor.role.is_site_admin()
        || (actor.role.is_operations_admin() && actor.id == target_user_id);
    if allowed {
        Ok(())
    } else {
        Err(ServerFnError::new(
            "You may only manage your own case access.",
        ))
    }
}

/// Whether this account may see the volunteer-only info of a case
/// Including properties, evidence and channels
pub fn has_volunteer_access(user: &User) -> bool {
    !matches!(user.role, AccountRole::Client)
}

/// Reject unless the caller may act on the given visibility. Used by every write
/// path so a client can never create or edit a volunteer-only property or file
/// by posting the visibility directly.
pub fn require_visibility(user: &User, visibility: Visibility) -> Result<(), ServerFnError> {
    if !visibility.is_restricted() || has_volunteer_access(user) {
        Ok(())
    } else {
        Err(ServerFnError::new(
            "You do not have access to volunteer-only case information.",
        ))
    }
}

/// The caller's capabilities on a case: every capability for a role with full
/// case access (site admin), else exactly the set granted by their stored
/// assignment. Errors if the case does not exist.
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
///
/// A site admin holds every capability on every case, so only the lifecycle
/// check below can stop them here.
///
/// Also enforces the case lifecycle: a declined or withdrawn case is held
/// read-only for everyone, since signup grants the client owner every capability
/// up front.
pub async fn require_cap(
    user: &User,
    case_id: &str,
    cap: CaseCapability,
) -> Result<(), ServerFnError> {
    if !capabilities_on(user, case_id).await?.contains(&cap) {
        return Err(ServerFnError::new(format!(
            "You do not have permission to {} on this case.",
            cap.label().to_lowercase()
        )));
    }
    if cap.is_write() {
        let status = cases::status(case_id)
            .await
            .map_err(ServerFnError::new)?
            .ok_or_else(|| ServerFnError::new("Case not found."))?;
        if !status.accepts_changes() {
            return Err(ServerFnError::new(format!(
                "This case is {} and can no longer be changed.",
                status.label().to_lowercase()
            )));
        }
    }
    Ok(())
}

/// The single gate for acting inside a case's chat channel.
///
/// The channel is resolved from the database *first*, so the case the
/// capability is checked against always comes from the stored channel and never
/// from a case id supplied by the caller. Both access checks are then applied:
/// `cap` on the owning case, and — for the volunteer-only channel — the
/// [`has_volunteer_access`] account-role gate that keeps clients out.
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
        .filter(|channel| has_volunteer_access(user) || !channel.kind.is_restricted())
        .ok_or_else(|| ServerFnError::new("Channel not found."))?;
    if require_cap(user, &channel.case_id, cap).await.is_err() {
        return Err(ServerFnError::new("Channel not found."));
    }
    Ok(channel)
}
