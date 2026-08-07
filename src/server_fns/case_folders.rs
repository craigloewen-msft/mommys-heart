//! Case file folders: the tree a case's files are organized into.
//!
//! A case's files live in folders rather than in one flat list. The tree is
//! rooted in a set of standing top-level folders — defined by
//! [`NEW_CASE_FOLDERS`](crate::helpers::new_case_folders::NEW_CASE_FOLDERS) and
//! created with the case — each of which names the audience for everything
//! filed under it. They cannot be deleted ([`CaseFolder::is_root`] guards the
//! delete path). Everything else is a folder somebody made inside one of them.
//!
//! The root a folder hangs from decides its audience, and a file's audience is
//! its folder's. That is the whole access-control story for the tree: moving a
//! file into a folder is what changes who can see it, so there is no way to
//! produce a file whose audience disagrees with where it sits.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::helpers::visibility::Visibility;

/// The longest a folder name may be.
pub const MAX_NAME_LEN: usize = 60;

/// How deep the tree may go, counting the standing top-level folder as level 1.
/// A cap keeps the breadcrumb and the "move to" picker readable.
pub const MAX_DEPTH: usize = 4;

/// One folder in a case's file tree.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CaseFolder {
    pub id: String,
    pub case_id: String,
    /// The folder this one sits in. `None` for the two standing top-level
    /// folders.
    #[serde(default)]
    pub parent_id: Option<String>,
    pub name: String,
    /// Who may see the files in here, inherited from the root this folder hangs
    /// from.
    pub visibility: Visibility,
}

impl CaseFolder {
    /// Whether this is one of the two standing top-level folders, which cannot
    /// be renamed, moved, or deleted.
    pub fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }
}

/// Check a user-supplied folder name, returning the cleaned version.
///
/// Names are shown in the UI *and* become a path segment in the blob container,
/// so the characters that would change the shape of a path are rejected here
/// rather than quietly rewritten — a folder called `a/b` is a mistake worth
/// reporting, not two folders.
pub fn clean_name(raw: &str) -> Result<String, String> {
    let name = raw.trim();
    if name.is_empty() {
        return Err("Give the folder a name.".to_string());
    }
    if name.chars().count() > MAX_NAME_LEN {
        return Err(format!(
            "Folder names are limited to {MAX_NAME_LEN} characters."
        ));
    }
    if name.contains(['/', '\\']) || name.chars().any(char::is_control) {
        return Err("A folder name cannot contain slashes.".to_string());
    }
    if name.chars().all(|c| c == '.') {
        return Err("Give the folder a real name.".to_string());
    }
    Ok(name.to_string())
}

/// Create a folder inside `parent_id` (requires the `UploadEvidence`
/// capability). Returns the new folder's id.
///
/// The parent decides everything except the name: which case the folder belongs
/// to and who may see what goes in it. Neither is taken from the request, so a
/// folder can only ever be made somewhere the caller can already see.
#[server(prefix = "/api")]
pub async fn create_case_folder(parent_id: String, name: String) -> Result<String, ServerFnError> {
    use crate::server::db::case_folders as db;
    use crate::server::permissions::{require_cap, require_user, require_visibility};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    let parent = db::get(&parent_id)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("That folder no longer exists."))?;
    require_cap(&user, &parent.case_id, CaseCapability::UploadEvidence).await?;
    require_visibility(&user, parent.visibility)?;

    let name = clean_name(&name).map_err(ServerFnError::new)?;
    if db::depth_of(&parent.id).await.map_err(ServerFnError::new)? >= MAX_DEPTH {
        return Err(ServerFnError::new(
            "Folders cannot be nested any deeper here.",
        ));
    }
    if db::child_exists(&parent.id, &name)
        .await
        .map_err(ServerFnError::new)?
    {
        return Err(ServerFnError::new(
            "There is already a folder with that name here.",
        ));
    }

    let id = db::create(&parent, &name, &user.full_name())
        .await
        .map_err(ServerFnError::new)?;

    crate::server::notifications::notify_case(
        parent.case_id,
        user.id.clone(),
        user.full_name(),
        crate::server_fns::settings::NotificationKind::EvidenceChanged,
        format!("added the folder \"{name}\""),
        crate::server::notifications::audience_for(parent.visibility),
    );
    Ok(id)
}

/// Delete an empty folder (requires the `DeleteEvidence` capability).
///
/// Only empty folders go: a folder with files or sub-folders in it has to be
/// cleared out first. Deleting a whole tree in one click is how people lose
/// paperwork they meant to keep, and the files still have their own delete.
#[server(prefix = "/api")]
pub async fn delete_case_folder(folder_id: String) -> Result<(), ServerFnError> {
    use crate::server::db::case_folders as db;
    use crate::server::permissions::{require_cap, require_user, require_visibility};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    let folder = db::get(&folder_id)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("That folder no longer exists."))?;
    require_cap(&user, &folder.case_id, CaseCapability::DeleteEvidence).await?;
    require_visibility(&user, folder.visibility)?;

    if folder.is_root() {
        return Err(ServerFnError::new(
            "The top-level folders are part of every case and cannot be deleted.",
        ));
    }
    if !db::is_empty(&folder.id).await.map_err(ServerFnError::new)? {
        return Err(ServerFnError::new(
            "Empty the folder before deleting it.".to_string(),
        ));
    }

    db::delete(&folder.id, &user.full_name())
        .await
        .map_err(ServerFnError::new)?;

    crate::server::notifications::notify_case(
        folder.case_id,
        user.id.clone(),
        user.full_name(),
        crate::server_fns::settings::NotificationKind::EvidenceChanged,
        format!("deleted the folder \"{}\"", folder.name),
        crate::server::notifications::audience_for(folder.visibility),
    );
    Ok(())
}
