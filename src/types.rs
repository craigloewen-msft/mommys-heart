//! Shared API contract types used by both the Axum JSON API (server) and the
//! Leptos UI (client). Keeping them in one place guarantees the website and the
//! dedicated API never drift apart.

use serde::{Deserialize, Serialize};

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
}

/// `POST /api/chat` response body.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub answer: String,
    pub sources: Vec<SourceInfo>,
    pub source_type: SourceType,
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

/// CRM contact lifecycle stage.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContactStatus {
    Lead,
    Active,
    Inactive,
}

impl ContactStatus {
    /// Human label for display.
    pub fn label(self) -> &'static str {
        match self {
            ContactStatus::Lead => "Lead",
            ContactStatus::Active => "Active",
            ContactStatus::Inactive => "Inactive",
        }
    }

    /// Tailwind badge classes for this status.
    pub fn badge_classes(self) -> &'static str {
        match self {
            ContactStatus::Active => "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30",
            ContactStatus::Lead => "bg-sky-500/15 text-sky-300 ring-1 ring-sky-500/30",
            ContactStatus::Inactive => "bg-slate-500/15 text-slate-400 ring-1 ring-slate-500/30",
        }
    }
}

/// A CRM contact.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub company: String,
    pub status: ContactStatus,
    pub notes: String,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// Auth + volunteer/case management types (local demo data).
// ---------------------------------------------------------------------------

/// Who a user is within the organization.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Volunteer,
}

impl Role {
    pub fn label(self) -> &'static str {
        match self {
            Role::Admin => "Admin",
            Role::Volunteer => "Volunteer",
        }
    }
}

/// An application user account (demo credentials only — never real auth).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    /// Plaintext for the local demo only. Do not use this pattern for real auth.
    pub password: String,
    pub role: Role,
    /// For volunteer accounts, links to the matching `Volunteer` record.
    #[serde(default)]
    pub volunteer_id: Option<String>,
}

/// Lifecycle status of a volunteer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VolunteerStatus {
    Active,
    OnLeave,
    Pending,
    Inactive,
}

impl VolunteerStatus {
    pub const ALL: [VolunteerStatus; 4] = [
        VolunteerStatus::Active,
        VolunteerStatus::OnLeave,
        VolunteerStatus::Pending,
        VolunteerStatus::Inactive,
    ];

    pub fn label(self) -> &'static str {
        match self {
            VolunteerStatus::Active => "Active",
            VolunteerStatus::OnLeave => "On leave",
            VolunteerStatus::Pending => "Pending",
            VolunteerStatus::Inactive => "Inactive",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            VolunteerStatus::Active => "active",
            VolunteerStatus::OnLeave => "on_leave",
            VolunteerStatus::Pending => "pending",
            VolunteerStatus::Inactive => "inactive",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.slug() == s)
    }

    pub fn badge_classes(self) -> &'static str {
        match self {
            VolunteerStatus::Active => "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30",
            VolunteerStatus::OnLeave => "bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/30",
            VolunteerStatus::Pending => "bg-sky-500/15 text-sky-300 ring-1 ring-sky-500/30",
            VolunteerStatus::Inactive => "bg-slate-500/15 text-slate-400 ring-1 ring-slate-500/30",
        }
    }
}

/// A volunteer record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Volunteer {
    pub id: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub specialty: String,
    pub status: VolunteerStatus,
}

/// Lifecycle status of a support case.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseStatus {
    Open,
    InProgress,
    OnHold,
    Closed,
}

impl CaseStatus {
    pub const ALL: [CaseStatus; 4] = [
        CaseStatus::Open,
        CaseStatus::InProgress,
        CaseStatus::OnHold,
        CaseStatus::Closed,
    ];

    pub fn label(self) -> &'static str {
        match self {
            CaseStatus::Open => "Open",
            CaseStatus::InProgress => "In progress",
            CaseStatus::OnHold => "On hold",
            CaseStatus::Closed => "Closed",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            CaseStatus::Open => "open",
            CaseStatus::InProgress => "in_progress",
            CaseStatus::OnHold => "on_hold",
            CaseStatus::Closed => "closed",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.slug() == s)
    }

    pub fn badge_classes(self) -> &'static str {
        match self {
            CaseStatus::Open => "bg-sky-500/15 text-sky-300 ring-1 ring-sky-500/30",
            CaseStatus::InProgress => "bg-violet-500/15 text-violet-300 ring-1 ring-violet-500/30",
            CaseStatus::OnHold => "bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/30",
            CaseStatus::Closed => "bg-slate-500/15 text-slate-400 ring-1 ring-slate-500/30",
        }
    }
}

/// Priority of a support case.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CasePriority {
    Low,
    Medium,
    High,
}

impl CasePriority {
    pub const ALL: [CasePriority; 3] =
        [CasePriority::Low, CasePriority::Medium, CasePriority::High];

    pub fn label(self) -> &'static str {
        match self {
            CasePriority::Low => "Low",
            CasePriority::Medium => "Medium",
            CasePriority::High => "High",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            CasePriority::Low => "low",
            CasePriority::Medium => "medium",
            CasePriority::High => "high",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.slug() == s)
    }

    pub fn badge_classes(self) -> &'static str {
        match self {
            CasePriority::Low => "bg-slate-500/15 text-slate-300 ring-1 ring-slate-500/30",
            CasePriority::Medium => "bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/30",
            CasePriority::High => "bg-rose-500/15 text-rose-300 ring-1 ring-rose-500/30",
        }
    }
}

/// A document attached to a case.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CaseDocument {
    pub id: String,
    pub name: String,
    pub uploaded_at: String,
}

/// A support case tracked by the organization.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Case {
    pub id: String,
    pub title: String,
    pub client_name: String,
    pub summary: String,
    pub status: CaseStatus,
    pub priority: CasePriority,
    pub assigned_volunteer_ids: Vec<String>,
    pub documents: Vec<CaseDocument>,
    pub opened_at: String,
}
