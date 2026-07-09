//! Shared types used by both the Axum JSON API (server) and the Leptos UI
//! (client). Keeping them in one place guarantees the website and the dedicated
//! API never drift apart.
//!
//! Two groups live here:
//! 1. The **RAG chatbot / API contract** types (chat, version, health).
//! 2. The **V1 domain model** (users, cases, evidence, grants, messages) — all
//!    mocked in-memory for now, to be backed by a real database later.

use serde::{Deserialize, Serialize};

// ===========================================================================
// RAG chatbot + API contract types
// ===========================================================================

/// A single cited source returned by the RAG chat endpoint.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceInfo {
    pub filename: String,
    pub heading: String,
    pub snippet: String,
    pub relevance: f64,
    pub anchor: String,
}

/// Where a chat answer came from — mirrors the legacy Python contract.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Documents,
    GeneralKnowledge,
    Mixed,
}

/// `POST /api/chat` request body.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    #[serde(default)]
    pub captcha_token: Option<String>,
    /// Existing conversation to continue. Omitted on the first message; the
    /// server echoes back an id the widget/UI can send on later turns.
    #[serde(default)]
    pub conversation_id: Option<String>,
}

/// `POST /api/chat` response body.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub answer: String,
    pub sources: Vec<SourceInfo>,
    pub source_type: SourceType,
    /// The conversation this turn belongs to. Echo it back on the next request
    /// to keep the thread continuous.
    #[serde(default)]
    pub conversation_id: Option<String>,
}

/// `GET /api/version` response body.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VersionResponse {
    pub version: String,
    pub chat_model: String,
    pub embedding_model: String,
    pub captcha_enabled: bool,
}

/// `GET /api/health` response body.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
}

// ===========================================================================
// V1 domain model (mocked in-memory)
// ===========================================================================

// ---------------------------------------------------------------------------
// Users, roles & permissions
// ---------------------------------------------------------------------------

/// The global account type a user has. This controls app-level access (e.g.
/// only an `Admin` reaches the Admin dashboard). It is intentionally separate
/// from per-case permissions: all account types view and work cases the same
/// way; what differs per case is their set of [`CaseCapability`]s.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountRole {
    /// A client the organization is helping.
    Client,
    /// A volunteer working cases on behalf of clients.
    Volunteer,
    /// An administrator who manages users and their permissions.
    Admin,
}

impl AccountRole {
    pub const ALL: [AccountRole; 3] = [
        AccountRole::Client,
        AccountRole::Volunteer,
        AccountRole::Admin,
    ];

    pub fn label(self) -> &'static str {
        match self {
            AccountRole::Client => "Client",
            AccountRole::Volunteer => "Volunteer",
            AccountRole::Admin => "Admin",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            AccountRole::Client => "client",
            AccountRole::Volunteer => "volunteer",
            AccountRole::Admin => "admin",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.slug() == s)
    }

    /// Only admins reach the Admin dashboard and can manage other users.
    pub fn is_admin(self) -> bool {
        matches!(self, AccountRole::Admin)
    }

    pub fn badge_classes(self) -> &'static str {
        match self {
            AccountRole::Admin => "bg-primary-500/15 text-primary-300 ring-1 ring-primary-500/30",
            AccountRole::Volunteer => "bg-sky-500/15 text-sky-300 ring-1 ring-sky-500/30",
            AccountRole::Client => "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30",
        }
    }
}

/// A discrete action a user may be permitted to take on a specific case. Access
/// is modeled as a **set** of these capabilities per assignment (rather than a
/// single tier), so control is fine-grained: e.g. a client may hold
/// `UploadEvidence` while a volunteer holds only `ViewEvidence`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseCapability {
    /// See the case at all (list it, open it).
    ViewCase,
    /// Change case status and properties.
    EditCase,
    /// Add case notes.
    AddNotes,
    /// See the case's evidence.
    ViewEvidence,
    /// Add new evidence.
    UploadEvidence,
    /// Remove evidence.
    DeleteEvidence,
    /// Read and post in the case chat.
    SendMessages,
}

impl CaseCapability {
    pub const ALL: [CaseCapability; 7] = [
        CaseCapability::ViewCase,
        CaseCapability::EditCase,
        CaseCapability::AddNotes,
        CaseCapability::ViewEvidence,
        CaseCapability::UploadEvidence,
        CaseCapability::DeleteEvidence,
        CaseCapability::SendMessages,
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
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.slug() == s)
    }
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
    /// Full control of the case.
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

/// An application user account (demo credentials only — never real auth).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: String,
    pub home_address: String,
    /// Plaintext for the local demo only. Do not use this pattern for real auth.
    pub password: String,
    pub role: AccountRole,
    /// Cases this user is assigned to, with their permission on each.
    #[serde(default)]
    pub assigned_cases: Vec<CaseAssignment>,
    /// Append-only log of who changed which field of this user, and when.
    #[serde(default)]
    pub audit_log: Vec<ChangeLogEntry>,
}

impl User {
    /// Convenience: the user's full display name.
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
            .trim()
            .to_string()
    }

    /// This user's capabilities on a given case (empty if not assigned).
    pub fn capabilities_for(&self, case_id: &str) -> Vec<CaseCapability> {
        self.assigned_cases
            .iter()
            .find(|a| a.case_id == case_id)
            .map(|a| a.capabilities.clone())
            .unwrap_or_default()
    }

    /// Whether this user is assigned to the given case at all.
    pub fn is_assigned_to(&self, case_id: &str) -> bool {
        self.assigned_cases.iter().any(|a| a.case_id == case_id)
    }
}

// ---------------------------------------------------------------------------
// Audit change log (shared by users and cases)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Grants
// ---------------------------------------------------------------------------

/// A funding grant. Minimal for V1 — just an id and a name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Grant {
    pub id: String,
    pub name: String,
}

// ---------------------------------------------------------------------------
// Cases
// ---------------------------------------------------------------------------

/// The lifecycle status of a case.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseStatus {
    Open,
    Monitor,
    Closed,
}

impl CaseStatus {
    pub const ALL: [CaseStatus; 3] = [CaseStatus::Open, CaseStatus::Monitor, CaseStatus::Closed];

    pub fn label(self) -> &'static str {
        match self {
            CaseStatus::Open => "Open",
            CaseStatus::Monitor => "Monitor",
            CaseStatus::Closed => "Closed",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            CaseStatus::Open => "open",
            CaseStatus::Monitor => "monitor",
            CaseStatus::Closed => "closed",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s2| s2.slug() == s)
    }

    pub fn badge_classes(self) -> &'static str {
        match self {
            CaseStatus::Open => "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30",
            CaseStatus::Monitor => "bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/30",
            CaseStatus::Closed => "bg-slate-500/15 text-slate-400 ring-1 ring-slate-500/30",
        }
    }
}

/// A free-text note recorded against a case.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CaseNote {
    pub id: String,
    pub author: String,
    pub body: String,
    pub created_at: String,
}

/// A piece of evidence attached to a case.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Evidence {
    pub id: String,
    pub name: String,
    /// The case this evidence belongs to.
    pub case_id: String,
    /// Display name of the user who uploaded it.
    pub uploaded_by: String,
    /// Human-readable upload timestamp (mock).
    pub uploaded_at: String,
    /// Free-text extra information / description.
    #[serde(default)]
    pub description: String,
}

/// A support case tracked by the organization.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Case {
    pub id: String,
    pub name: String,
    pub status: CaseStatus,
    /// The user who owns this case (implicitly has `Owner` permission).
    pub owner_id: String,
    /// Case notes, newest last.
    #[serde(default)]
    pub notes: Vec<CaseNote>,
    /// Evidence gathered for this case.
    #[serde(default)]
    pub evidence: Vec<Evidence>,
    /// Free-form case properties (e.g. "Opposing attorney" -> "J. Smith").
    #[serde(default)]
    pub properties: Vec<CaseProperty>,
    /// Append-only log of changes to this case.
    #[serde(default)]
    pub audit_log: Vec<ChangeLogEntry>,
}

/// A named key/value property on a case (e.g. attorney names, court, docket).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CaseProperty {
    pub key: String,
    pub value: String,
}

// ---------------------------------------------------------------------------
// Case chat messages
// ---------------------------------------------------------------------------

/// A message posted in a case's chat thread. Every case has its own thread that
/// any assigned user with the `SendMessages` capability can read and post to.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    /// The case whose chat this message belongs to.
    pub case_id: String,
    /// User id of the author.
    pub author_id: String,
    /// Display name of the author.
    pub author: String,
    pub body: String,
    /// Human-readable timestamp (mock).
    pub sent_at: String,
}
