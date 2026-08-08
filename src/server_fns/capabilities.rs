//! The per-case **capability** model shared by the client and the server.
//!
//! These types are the vocabulary of case-level authorization: the discrete
//! [`CaseCapability`]s a user may hold, convenience [`CasePreset`]s that expand
//! to a common set of them, and the [`CaseAssignment`] that links a user to a
//! case with the capabilities they hold on it. They are consumed symmetrically
//! by users (assignments live on a [`crate::server_fns::users::User`]) and by
//! cases (each operation checks a capability), so they live in their own module
//! rather than being owned by either side.
//!
//! This module is only the *vocabulary* (the nouns). The server-side *gate* that
//! actually grants or denies access from these capabilities lives in
//! [`crate::server::permissions`]. Account-level access (who reaches the Admin
//! dashboard) is a separate concern: see
//! [`crate::server_fns::users::AccountRole`].

use serde::{Deserialize, Serialize};

/// A discrete action a user may be permitted to take on a specific case. Access
/// is modeled as a **set** of these capabilities per assignment (rather than a
/// single tier), so control is fine-grained: e.g. a client may hold
/// `UploadEvidence` while a volunteer holds only `ViewEvidence`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseCapability {
    /// See the case at all (list it, open it).
    ViewCase,
    EditCase,
    AddNotes,
    ViewEvidence,
    UploadEvidence,
    DeleteEvidence,
    SendMessages,
    ManageChannels,
}

impl CaseCapability {
    pub const ALL: [CaseCapability; 8] = [
        CaseCapability::ViewCase,
        CaseCapability::EditCase,
        CaseCapability::AddNotes,
        CaseCapability::ViewEvidence,
        CaseCapability::UploadEvidence,
        CaseCapability::DeleteEvidence,
        CaseCapability::SendMessages,
        CaseCapability::ManageChannels,
    ];

    pub fn label(self) -> &'static str {
        match self {
            CaseCapability::ViewCase => "View case",
            CaseCapability::EditCase => "Edit case",
            CaseCapability::AddNotes => "Add notes",
            CaseCapability::ViewEvidence => "View evidence",
            CaseCapability::UploadEvidence => "Upload evidence",
            CaseCapability::DeleteEvidence => "Delete evidence",
            CaseCapability::SendMessages => "Send messages",
            CaseCapability::ManageChannels => "Manage message channels",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            CaseCapability::ViewCase => "view_case",
            CaseCapability::EditCase => "edit_case",
            CaseCapability::AddNotes => "add_notes",
            CaseCapability::ViewEvidence => "view_evidence",
            CaseCapability::UploadEvidence => "upload_evidence",
            CaseCapability::DeleteEvidence => "delete_evidence",
            CaseCapability::SendMessages => "send_messages",
            CaseCapability::ManageChannels => "manage_channels",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.slug() == s)
    }

    /// Whether exercising this capability changes the case, rather than only
    /// reading it. Used to hold a declined case read-only.
    pub fn is_write(self) -> bool {
        use CaseCapability::*;
        match self {
            ViewCase | ViewEvidence => false,
            EditCase | AddNotes | UploadEvidence | DeleteEvidence | SendMessages
            | ManageChannels => true,
        }
    }

    /// The other capabilities that must also be held for this one to make sense.
    /// Access is layered: you cannot act on a case (or its evidence) you cannot
    /// even see, so every capability implies at least [`ViewCase`], and the
    /// evidence-mutating ones additionally imply [`ViewEvidence`].
    ///
    /// [`ViewCase`]: CaseCapability::ViewCase
    /// [`ViewEvidence`]: CaseCapability::ViewEvidence
    /// [`ManageChannels`]: CaseCapability::ManageChannels
    pub fn requires(self) -> &'static [CaseCapability] {
        use CaseCapability::*;
        match self {
            ViewCase => &[],
            EditCase => &[ViewCase],
            AddNotes => &[ViewCase],
            ViewEvidence => &[ViewCase],
            UploadEvidence => &[ViewCase, ViewEvidence],
            DeleteEvidence => &[ViewCase, ViewEvidence, UploadEvidence],
            SendMessages => &[ViewCase],
            ManageChannels => &[ViewCase, EditCase, SendMessages],
        }
    }
}

/// Check that a capability set has no duplicates and every capability's
/// prerequisites (see [`CaseCapability::requires`]) are also present. Returns a
/// human-readable error describing the first problem, or `Ok(())` when the set
/// makes sense (e.g. you cannot grant "Edit case" without "View case").
pub fn validate_capabilities(capabilities: &[CaseCapability]) -> Result<(), String> {
    for (index, &cap) in capabilities.iter().enumerate() {
        if capabilities[..index].contains(&cap) {
            return Err(format!("\"{}\" appears more than once.", cap.label()));
        }
        for &req in cap.requires() {
            if !capabilities.contains(&req) {
                return Err(format!("\"{}\" requires \"{}\".", cap.label(), req.label()));
            }
        }
    }
    Ok(())
}

/// Convenience presets that expand to a common set of [`CaseCapability`]s. These
/// seed an assignment quickly; admins can then toggle individual capabilities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CasePreset {
    /// Read-only: view the case and its evidence.
    Viewer,
    /// Day-to-day worker: view, notes, evidence upload, and chat (no delete/edit).
    Contributor,
    /// Full control of the case — the "Full access" set, and the only preset
    /// that carries [`CaseCapability::ManageChannels`].
    Manager,
}

impl CasePreset {
    pub const ALL: [CasePreset; 3] = [
        CasePreset::Viewer,
        CasePreset::Contributor,
        CasePreset::Manager,
    ];

    pub fn label(self) -> &'static str {
        match self {
            CasePreset::Viewer => "Viewer",
            CasePreset::Contributor => "Contributor",
            CasePreset::Manager => "Manager",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            CasePreset::Viewer => "viewer",
            CasePreset::Contributor => "contributor",
            CasePreset::Manager => "manager",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.slug() == s)
    }

    /// The capability set this preset expands to.
    pub fn capabilities(self) -> Vec<CaseCapability> {
        use CaseCapability::*;
        match self {
            CasePreset::Viewer => vec![ViewCase, ViewEvidence],
            CasePreset::Contributor => {
                vec![
                    ViewCase,
                    AddNotes,
                    ViewEvidence,
                    UploadEvidence,
                    SendMessages,
                ]
            }
            CasePreset::Manager => CaseCapability::ALL.to_vec(),
        }
    }
}

/// A link from a user to a case, carrying the capabilities that user holds on it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CaseAssignment {
    pub case_id: String,
    pub capabilities: Vec<CaseCapability>,
}
