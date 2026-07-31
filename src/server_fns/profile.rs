//! User profiles: the self-service "who am I" screen at `/profile` and the
//! read-only view of a colleague's profile at `/profile/:id`.
//!
//! Profiles are **need-to-know**, not public: a profile is only readable by its
//! owner, by an administrator, or by someone who shares a case with that user
//! (both hold the `view_case` capability on the same case, or own it).
//!
//! What a reader sees is narrower still. Name and role identify a colleague and
//! are shown to anyone who may open the profile at all; the contact details
//! (email, phone, home address) are personal information and are sent **only**
//! to the profile's owner and to administrators. That filtering happens
//! server-side in [`load_profile`] — [`UserProfile::contact`] is simply absent
//! from the response for everyone else, so the details never reach the browser.
//!
//! Editing is strictly self-service: [`save_my_profile`] always writes the
//! caller's own row. Account-level fields that are *not* the user's to change
//! (their email, which identifies the account for sign-in, and their role) stay
//! read-only here and remain admin-managed in [`crate::server_fns::users`].

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::server_fns::users::{AccountRole, User};

/// The personal contact details on a profile. Only ever populated for the
/// profile's owner and for administrators; omitted entirely otherwise.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProfileContact {
    pub email: String,
    pub phone: String,
    pub home_address: String,
}

/// A user's profile, resolved for one specific viewer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub role: AccountRole,
    /// Contact details, present only when the viewer is the profile's owner or
    /// an administrator.
    #[serde(default)]
    pub contact: Option<ProfileContact>,
    /// Whether this profile belongs to the caller (drives the edit affordances).
    #[serde(default)]
    pub is_self: bool,
    /// Whether the viewer works a case with this user — the reason a colleague's
    /// profile is visible. Always false on your own profile, and false for an
    /// admin viewing someone they share no case with.
    #[serde(default)]
    pub shares_case: bool,
}

impl UserProfile {
    /// Narrow a stored [`User`] down to what one particular viewer is allowed
    /// to see.
    ///
    /// This is the one place the privacy rule lives, and it is deliberately a
    /// *constructor* rather than a filter applied afterwards: you cannot get a
    /// `UserProfile` at all without first answering "is this the owner or an
    /// admin?", so a caller cannot forget to strip the contact block.
    pub fn for_viewer(user: User, is_self: bool, is_admin: bool, shares_case: bool) -> Self {
        let User {
            id,
            first_name,
            last_name,
            email,
            phone,
            home_address,
            role,
            ..
        } = user;
        Self {
            id,
            first_name,
            last_name,
            role,
            contact: (is_self || is_admin).then_some(ProfileContact {
                email,
                phone,
                home_address,
            }),
            is_self,
            shares_case,
        }
    }

    /// The profile's display name, falling back to the user id when unnamed.
    pub fn full_name(&self) -> String {
        let name = format!("{} {}", self.first_name, self.last_name)
            .trim()
            .to_string();
        if name.is_empty() {
            self.id.clone()
        } else {
            name
        }
    }

    /// Up to two uppercase initials for the avatar bubble.
    pub fn initials(&self) -> String {
        self.full_name()
            .split_whitespace()
            .filter_map(|part| part.chars().next())
            .take(2)
            .flat_map(|c| c.to_uppercase())
            .collect()
    }

    /// The editable subset of this profile, for seeding the edit form. Only
    /// meaningful on your own profile, where the contact details are present.
    pub fn to_edit(&self) -> ProfileEdit {
        let contact = self.contact.clone().unwrap_or_default();
        ProfileEdit {
            first_name: self.first_name.clone(),
            last_name: self.last_name.clone(),
            phone: contact.phone,
            home_address: contact.home_address,
        }
    }
}

/// The fields a user may change on their own profile.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProfileEdit {
    pub first_name: String,
    pub last_name: String,
    pub phone: String,
    pub home_address: String,
}

/// Load a profile as seen by the signed-in caller.
///
/// Visible when the profile is the caller's own, the caller is an admin, or the
/// two share a case; otherwise the call fails rather than leaking that the
/// account exists in any more detail. Contact details ride along only for the
/// owner and for admins.
#[server(prefix = "/api")]
pub async fn load_profile(user_id: String) -> Result<UserProfile, ServerFnError> {
    use crate::server::db::users;
    use crate::server::permissions::require_user;

    let viewer = require_user().await?;
    let user_id = user_id.trim().to_string();
    if user_id.is_empty() {
        return Err(ServerFnError::new("No profile was requested."));
    }

    const UNAVAILABLE: &str =
        "This profile isn't available. You can only view the profiles of people you share a case with.";

    let record = users::get(&user_id)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new(UNAVAILABLE))?;

    let is_self = viewer.id == record.id;
    let shares_case = if is_self {
        false
    } else {
        users::shares_case(&viewer.id, &record.id)
            .await
            .map_err(ServerFnError::new)?
    };
    let is_admin = viewer.role.is_admin();
    if !is_self && !shares_case && !is_admin {
        return Err(ServerFnError::new(UNAVAILABLE));
    }

    Ok(UserProfile::for_viewer(
        record,
        is_self,
        is_admin,
        shares_case,
    ))
}

/// Save the caller's own profile and return it as freshly stored. Always scoped
/// to the signed-in user: there is no way to edit somebody else's profile here.
#[server(prefix = "/api")]
pub async fn save_my_profile(edit: ProfileEdit) -> Result<UserProfile, ServerFnError> {
    use crate::server::db::users;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    let edit = ProfileEdit {
        first_name: edit.first_name.trim().to_string(),
        last_name: edit.last_name.trim().to_string(),
        phone: edit.phone.trim().to_string(),
        home_address: edit.home_address.trim().to_string(),
    };
    if edit.first_name.is_empty() || edit.last_name.is_empty() {
        return Err(ServerFnError::new("First and last name are required."));
    }

    users::update_profile(&user.id, &edit, &user.full_name())
        .await
        .map_err(ServerFnError::new)?;

    let saved = users::get(&user.id)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("This profile isn't available."))?;

    // Editing is always self-service, so the contact block is always included.
    Ok(UserProfile::for_viewer(
        saved,
        true,
        user.role.is_admin(),
        false,
    ))
}
