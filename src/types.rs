//! Shared API contract types used by both the Axum JSON API (server) and the
//! Leptos UI (client). Keeping them in one place guarantees the website and the
//! dedicated API never drift apart.

use serde::{Deserialize, Serialize};

use crate::taxonomy::ServiceType;

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
    /// server returns a fresh id the widget/UI should echo back on later turns
    /// so the whole thread is retained together.
    #[serde(default)]
    pub conversation_id: Option<String>,
}

/// `POST /api/chat` response body.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub answer: String,
    pub sources: Vec<SourceInfo>,
    pub source_type: SourceType,
    /// The conversation this turn was retained under. Echo it back on the next
    /// request to keep the thread continuous.
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
            ContactStatus::Active => {
                "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30"
            }
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

/// Who a user is within the organization. Roles map to authorization *levels*
/// via a permission matrix (see [`Permission`] and [`Role::permissions`]) rather
/// than features checking role equality directly, so new levels slot in cleanly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Full administrative control, including governance and access management.
    Admin,
    /// Employed staff: manage cases/volunteers/knowledge, but not org settings.
    Staff,
    /// Time-limited volunteer/intern: work only their assigned cases.
    Volunteer,
    /// Read-only observer (e.g. auditor, board member): can view, never edit.
    ReadOnly,
}

impl Role {
    pub const ALL: [Role; 4] = [Role::Admin, Role::Staff, Role::Volunteer, Role::ReadOnly];

    pub fn label(self) -> &'static str {
        match self {
            Role::Admin => "Admin",
            Role::Staff => "Staff",
            Role::Volunteer => "Volunteer",
            Role::ReadOnly => "Read-only",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Staff => "staff",
            Role::Volunteer => "volunteer",
            Role::ReadOnly => "read_only",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.slug() == s)
    }

    /// Short description of the authorization level, for admin screens.
    pub fn description(self) -> &'static str {
        match self {
            Role::Admin => "Full control: governance, access management, all records.",
            Role::Staff => "Manage cases, volunteers, and knowledge across the organization.",
            Role::Volunteer => "Work only their own assigned cases and documents.",
            Role::ReadOnly => "View-only access; cannot create or change records.",
        }
    }

    pub fn badge_classes(self) -> &'static str {
        match self {
            Role::Admin => "bg-primary-500/15 text-primary-300 ring-1 ring-primary-500/30",
            Role::Staff => "bg-violet-500/15 text-violet-300 ring-1 ring-violet-500/30",
            Role::Volunteer => "bg-sky-500/15 text-sky-300 ring-1 ring-sky-500/30",
            Role::ReadOnly => "bg-slate-500/15 text-slate-300 ring-1 ring-slate-500/30",
        }
    }

    /// Whether this role lands in the admin/staff control center (vs. the
    /// volunteer view) after signing in.
    pub fn is_staff_level(self) -> bool {
        matches!(self, Role::Admin | Role::Staff)
    }

    /// The set of permissions granted to this authorization level. This is the
    /// single source of truth for role-based access control.
    pub fn permissions(self) -> &'static [Permission] {
        use Permission::*;
        match self {
            Role::Admin => &[
                ViewAllCases,
                EditCases,
                ManageVolunteers,
                ManageUsers,
                ViewAuditLog,
                ManageRetention,
                ViewConfidentialDocs,
                ManageKnowledge,
                OffboardVolunteers,
            ],
            Role::Staff => &[
                ViewAllCases,
                EditCases,
                ManageVolunteers,
                ViewConfidentialDocs,
                ManageKnowledge,
                OffboardVolunteers,
            ],
            Role::Volunteer => &[EditCases],
            Role::ReadOnly => &[ViewAllCases],
        }
    }

    /// Whether this role is granted a given permission.
    pub fn can(self, perm: Permission) -> bool {
        self.permissions().contains(&perm)
    }
}

/// A discrete action gated by role-based access control. Features check
/// `Role::can(permission)` rather than comparing roles directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    /// See every case in the organization (vs. only one's own assignments).
    ViewAllCases,
    /// Create cases and change case data (status, assignments, documents).
    EditCases,
    /// Add volunteers and change their lifecycle status.
    ManageVolunteers,
    /// Change other users' roles / authorization levels.
    ManageUsers,
    /// Read the audit trail.
    ViewAuditLog,
    /// Manage records-retention policy: legal holds and disposal.
    ManageRetention,
    /// Open documents classified Confidential or Restricted.
    ViewConfidentialDocs,
    /// Create and edit shared institutional-knowledge resources.
    ManageKnowledge,
    /// Offboard a volunteer, reassigning their work to the organization.
    OffboardVolunteers,
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
            VolunteerStatus::Active => {
                "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30"
            }
            VolunteerStatus::OnLeave => "bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/30",
            VolunteerStatus::Pending => "bg-sky-500/15 text-sky-300 ring-1 ring-sky-500/30",
            VolunteerStatus::Inactive => "bg-slate-500/15 text-slate-400 ring-1 ring-slate-500/30",
        }
    }
}

/// A completed (or scheduled) training a volunteer participated in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrainingRecord {
    pub name: String,
    /// ISO `YYYY-MM-DD`. Empty string means enrolled but not yet completed.
    pub completed_on: String,
}

impl TrainingRecord {
    /// Whether this training has been completed.
    pub fn is_completed(&self) -> bool {
        !self.completed_on.trim().is_empty()
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
    /// Total service hours logged to date.
    #[serde(default)]
    pub hours_logged: f64,
    /// Typical weekly availability in hours (used for scheduling insight).
    #[serde(default)]
    pub weekly_availability_hours: f64,
    /// Count of logged client contacts / follow-up touches.
    #[serde(default)]
    pub client_contacts: u32,
    /// Trainings the volunteer has enrolled in or completed.
    #[serde(default)]
    pub trainings: Vec<TrainingRecord>,
}

/// The programmatic category (matter type) a case falls under. Drives the
/// programmatic metrics used for service-utilization and grant reporting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatterType {
    Housing,
    Immigration,
    Divorce,
    CustodyVisitation,
    ChildSupport,
    OrderOfProtection,
    DomesticViolence,
    MentalHealth,
    PublicBenefits,
    SafetyPlanning,
    Other,
}

impl MatterType {
    pub const ALL: [MatterType; 11] = [
        MatterType::Housing,
        MatterType::Immigration,
        MatterType::Divorce,
        MatterType::CustodyVisitation,
        MatterType::ChildSupport,
        MatterType::OrderOfProtection,
        MatterType::DomesticViolence,
        MatterType::MentalHealth,
        MatterType::PublicBenefits,
        MatterType::SafetyPlanning,
        MatterType::Other,
    ];

    pub fn label(self) -> &'static str {
        match self {
            MatterType::Housing => "Housing",
            MatterType::Immigration => "Immigration",
            MatterType::Divorce => "Divorce",
            MatterType::CustodyVisitation => "Custody & visitation",
            MatterType::ChildSupport => "Child support",
            MatterType::OrderOfProtection => "Orders of protection",
            MatterType::DomesticViolence => "Domestic violence advocacy",
            MatterType::MentalHealth => "Mental health referral",
            MatterType::PublicBenefits => "Public benefits",
            MatterType::SafetyPlanning => "Safety planning",
            MatterType::Other => "Other",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            MatterType::Housing => "housing",
            MatterType::Immigration => "immigration",
            MatterType::Divorce => "divorce",
            MatterType::CustodyVisitation => "custody_visitation",
            MatterType::ChildSupport => "child_support",
            MatterType::OrderOfProtection => "order_of_protection",
            MatterType::DomesticViolence => "domestic_violence",
            MatterType::MentalHealth => "mental_health",
            MatterType::PublicBenefits => "public_benefits",
            MatterType::SafetyPlanning => "safety_planning",
            MatterType::Other => "other",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.slug() == s)
    }
}

/// The resolution outcome of a case, for outcome tracking and impact reporting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseOutcome {
    Ongoing,
    Resolved,
    ReferredOut,
    Withdrawn,
    Unresolved,
}

impl CaseOutcome {
    pub const ALL: [CaseOutcome; 5] = [
        CaseOutcome::Ongoing,
        CaseOutcome::Resolved,
        CaseOutcome::ReferredOut,
        CaseOutcome::Withdrawn,
        CaseOutcome::Unresolved,
    ];

    pub fn label(self) -> &'static str {
        match self {
            CaseOutcome::Ongoing => "Ongoing",
            CaseOutcome::Resolved => "Resolved",
            CaseOutcome::ReferredOut => "Referred out",
            CaseOutcome::Withdrawn => "Withdrawn",
            CaseOutcome::Unresolved => "Unresolved",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            CaseOutcome::Ongoing => "ongoing",
            CaseOutcome::Resolved => "resolved",
            CaseOutcome::ReferredOut => "referred_out",
            CaseOutcome::Withdrawn => "withdrawn",
            CaseOutcome::Unresolved => "unresolved",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.slug() == s)
    }

    pub fn badge_classes(self) -> &'static str {
        match self {
            CaseOutcome::Resolved => {
                "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30"
            }
            CaseOutcome::Ongoing => "bg-sky-500/15 text-sky-300 ring-1 ring-sky-500/30",
            CaseOutcome::ReferredOut => {
                "bg-violet-500/15 text-violet-300 ring-1 ring-violet-500/30"
            }
            CaseOutcome::Withdrawn => "bg-slate-500/15 text-slate-400 ring-1 ring-slate-500/30",
            CaseOutcome::Unresolved => "bg-rose-500/15 text-rose-300 ring-1 ring-rose-500/30",
        }
    }
}

/// A referral made on behalf of a client to an external agency or program.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Referral {
    /// The agency or program the client was referred to.
    pub agency: String,
    /// ISO `YYYY-MM-DD`.
    pub date: String,
}

/// A discrete service provided as part of a case.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServiceRecord {
    /// Short description of the service (e.g. "Legal clinic", "Counseling").
    pub kind: String,
    /// ISO `YYYY-MM-DD`.
    pub date: String,
}

/// A scheduled or completed follow-up touchpoint for a case.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FollowUp {
    /// ISO `YYYY-MM-DD` the follow-up was due / performed.
    pub date: String,
    /// Whether the follow-up was completed.
    pub completed: bool,
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

/// A category of need a case addresses. A single client often has several of
/// these running at once (housing *and* family court *and* benefits) — each is
/// tracked as its own case so the pathway can be seen as a whole.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NeedCategory {
    Housing,
    FamilyCourt,
    Immigration,
    PublicBenefits,
    MentalHealth,
    Other,
}

impl NeedCategory {
    pub const ALL: [NeedCategory; 6] = [
        NeedCategory::Housing,
        NeedCategory::FamilyCourt,
        NeedCategory::Immigration,
        NeedCategory::PublicBenefits,
        NeedCategory::MentalHealth,
        NeedCategory::Other,
    ];

    pub fn label(self) -> &'static str {
        match self {
            NeedCategory::Housing => "Housing",
            NeedCategory::FamilyCourt => "Family Court",
            NeedCategory::Immigration => "Immigration",
            NeedCategory::PublicBenefits => "Public Benefits",
            NeedCategory::MentalHealth => "Mental Health",
            NeedCategory::Other => "Other",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            NeedCategory::Housing => "housing",
            NeedCategory::FamilyCourt => "family_court",
            NeedCategory::Immigration => "immigration",
            NeedCategory::PublicBenefits => "public_benefits",
            NeedCategory::MentalHealth => "mental_health",
            NeedCategory::Other => "other",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.slug() == s)
    }

    pub fn badge_classes(self) -> &'static str {
        match self {
            NeedCategory::Housing => "bg-teal-500/15 text-teal-300 ring-1 ring-teal-500/30",
            NeedCategory::FamilyCourt => {
                "bg-indigo-500/15 text-indigo-300 ring-1 ring-indigo-500/30"
            }
            NeedCategory::Immigration => "bg-cyan-500/15 text-cyan-300 ring-1 ring-cyan-500/30",
            NeedCategory::PublicBenefits => "bg-lime-500/15 text-lime-300 ring-1 ring-lime-500/30",
            NeedCategory::MentalHealth => {
                "bg-fuchsia-500/15 text-fuchsia-300 ring-1 ring-fuchsia-500/30"
            }
            NeedCategory::Other => "bg-slate-500/15 text-slate-300 ring-1 ring-slate-500/30",
        }
    }
}

/// A person served by the organization. One client owns many cases — one per
/// need area — so their interconnected needs can be viewed as a single pathway.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Client {
    pub id: String,
    /// May be a confidential reference (e.g. "Client A.") rather than a legal name.
    pub display_name: String,
    pub phone: String,
    pub email: String,
    pub intake_date: String,
    pub summary: String,
}

/// A free-text note recorded against a case (what was said / observed / done).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CaseNote {
    pub id: String,
    pub author: String,
    pub body: String,
    pub created_at: String,
}

/// The kind of action recorded on a case timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineKind {
    Opened,
    StatusChanged,
    NoteAdded,
    DocumentAdded,
    VolunteerAssigned,
    VolunteerUnassigned,
}

impl TimelineKind {
    pub fn label(self) -> &'static str {
        match self {
            TimelineKind::Opened => "Opened",
            TimelineKind::StatusChanged => "Status changed",
            TimelineKind::NoteAdded => "Note added",
            TimelineKind::DocumentAdded => "Document added",
            TimelineKind::VolunteerAssigned => "Volunteer assigned",
            TimelineKind::VolunteerUnassigned => "Volunteer unassigned",
        }
    }

    pub fn dot_classes(self) -> &'static str {
        match self {
            TimelineKind::Opened => "bg-sky-400",
            TimelineKind::StatusChanged => "bg-violet-400",
            TimelineKind::NoteAdded => "bg-emerald-400",
            TimelineKind::DocumentAdded => "bg-amber-400",
            TimelineKind::VolunteerAssigned => "bg-primary-400",
            TimelineKind::VolunteerUnassigned => "bg-slate-400",
        }
    }
}

/// A single event on a case's timeline — the record of how a case progresses
/// over time and what actions were taken.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub id: String,
    pub at: String,
    pub kind: TimelineKind,
    pub summary: String,
}

/// A document attached to a case.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CaseDocument {
    pub id: String,
    pub name: String,
    pub uploaded_at: String,
    /// Sensitivity classification, used for access control.
    #[serde(default = "default_classification")]
    pub classification: DocumentClassification,
}

fn default_classification() -> DocumentClassification {
    DocumentClassification::Internal
}

/// The kind of evidence an [`EvidenceItem`] represents. Mirrors the categories
/// clients typically collect across family court, DV, custody, and related
/// matters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceType {
    TextMessage,
    Email,
    Screenshot,
    CourtFiling,
    Photograph,
    AudioRecording,
    Affidavit,
    SupportingDocument,
    #[default]
    Other,
}

impl EvidenceType {
    pub const ALL: [EvidenceType; 9] = [
        EvidenceType::TextMessage,
        EvidenceType::Email,
        EvidenceType::Screenshot,
        EvidenceType::CourtFiling,
        EvidenceType::Photograph,
        EvidenceType::AudioRecording,
        EvidenceType::Affidavit,
        EvidenceType::SupportingDocument,
        EvidenceType::Other,
    ];

    pub fn label(self) -> &'static str {
        match self {
            EvidenceType::TextMessage => "Text message",
            EvidenceType::Email => "Email",
            EvidenceType::Screenshot => "Screenshot",
            EvidenceType::CourtFiling => "Court filing",
            EvidenceType::Photograph => "Photograph",
            EvidenceType::AudioRecording => "Audio recording",
            EvidenceType::Affidavit => "Affidavit",
            EvidenceType::SupportingDocument => "Supporting document",
            EvidenceType::Other => "Other",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            EvidenceType::TextMessage => "text_message",
            EvidenceType::Email => "email",
            EvidenceType::Screenshot => "screenshot",
            EvidenceType::CourtFiling => "court_filing",
            EvidenceType::Photograph => "photograph",
            EvidenceType::AudioRecording => "audio_recording",
            EvidenceType::Affidavit => "affidavit",
            EvidenceType::SupportingDocument => "supporting_document",
            EvidenceType::Other => "other",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.slug() == s)
    }

    pub fn badge_classes(self) -> &'static str {
        match self {
            EvidenceType::TextMessage => "bg-sky-500/15 text-sky-300 ring-1 ring-sky-500/30",
            EvidenceType::Email => "bg-indigo-500/15 text-indigo-300 ring-1 ring-indigo-500/30",
            EvidenceType::Screenshot => "bg-cyan-500/15 text-cyan-300 ring-1 ring-cyan-500/30",
            EvidenceType::CourtFiling => "bg-rose-500/15 text-rose-300 ring-1 ring-rose-500/30",
            EvidenceType::Photograph => {
                "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30"
            }
            EvidenceType::AudioRecording => {
                "bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/30"
            }
            EvidenceType::Affidavit => "bg-violet-500/15 text-violet-300 ring-1 ring-violet-500/30",
            EvidenceType::SupportingDocument => {
                "bg-teal-500/15 text-teal-300 ring-1 ring-teal-500/30"
            }
            EvidenceType::Other => "bg-slate-500/15 text-slate-300 ring-1 ring-slate-500/30",
        }
    }
}

/// Review workflow state of an [`EvidenceItem`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    Unreviewed,
    InReview,
    Reviewed,
    Flagged,
}

impl ReviewStatus {
    pub const ALL: [ReviewStatus; 4] = [
        ReviewStatus::Unreviewed,
        ReviewStatus::InReview,
        ReviewStatus::Reviewed,
        ReviewStatus::Flagged,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ReviewStatus::Unreviewed => "Unreviewed",
            ReviewStatus::InReview => "In review",
            ReviewStatus::Reviewed => "Reviewed",
            ReviewStatus::Flagged => "Flagged",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            ReviewStatus::Unreviewed => "unreviewed",
            ReviewStatus::InReview => "in_review",
            ReviewStatus::Reviewed => "reviewed",
            ReviewStatus::Flagged => "flagged",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.slug() == s)
    }

    pub fn badge_classes(self) -> &'static str {
        match self {
            ReviewStatus::Unreviewed => "bg-slate-500/15 text-slate-400 ring-1 ring-slate-500/30",
            ReviewStatus::InReview => "bg-sky-500/15 text-sky-300 ring-1 ring-sky-500/30",
            ReviewStatus::Reviewed => {
                "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30"
            }
            ReviewStatus::Flagged => "bg-rose-500/15 text-rose-300 ring-1 ring-rose-500/30",
        }
    }
}

/// A single piece of evidence attached to a case. Metadata-only in the current
/// proof-of-concept (no binary file is stored); a `file_ref` can be added later
/// without reworking the UI.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EvidenceItem {
    pub id: String,
    /// Short title / description of the item.
    pub name: String,
    pub evidence_type: EvidenceType,
    /// Free-text notes and context.
    pub description: String,
    /// Where it came from — platform, device, or person.
    pub source: String,
    /// Who it relates to / is from — drives pattern grouping.
    pub party: String,
    /// The date the underlying event occurred (`YYYY-MM-DD`). Drives timelines.
    pub occurred_on: String,
    pub tags: Vec<String>,
    pub review_status: ReviewStatus,
    /// When the item was added to the system.
    pub uploaded_at: String,
}

/// A support case tracked by the organization. Each case addresses one need
/// area for one client; related cases for the same client are cross-linked via
/// `related_case_ids` to capture interconnected service pathways.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Case {
    pub id: String,
    pub title: String,
    pub client_id: String,
    pub category: NeedCategory,
    /// Taxonomy services this case addresses. A case may span more than one
    /// service type (e.g. a custody matter that also needs safety planning).
    pub service_types: Vec<ServiceType>,
    pub summary: String,
    pub status: CaseStatus,
    pub priority: CasePriority,
    pub assigned_volunteer_ids: Vec<String>,
    pub related_case_ids: Vec<String>,
    pub notes: Vec<CaseNote>,
    pub documents: Vec<CaseDocument>,
    pub evidence: Vec<EvidenceItem>,
    pub timeline: Vec<TimelineEvent>,
    pub opened_at: String,
    /// Programmatic category, for service-utilization / grant reporting.
    #[serde(default = "default_matter_type")]
    pub matter_type: MatterType,
    /// Resolution outcome, for outcome tracking.
    #[serde(default = "default_case_outcome")]
    pub outcome: CaseOutcome,
    /// ISO `YYYY-MM-DD` the client was intaked.
    #[serde(default)]
    pub intake_date: String,
    /// ISO `YYYY-MM-DD` the case was resolved, if it has been.
    #[serde(default)]
    pub resolved_date: Option<String>,
    /// Referrals made on behalf of the client.
    #[serde(default)]
    pub referrals: Vec<Referral>,
    /// Services provided as part of the case.
    #[serde(default)]
    pub services: Vec<ServiceRecord>,
    /// Follow-up touchpoints (completed and outstanding).
    #[serde(default)]
    pub follow_ups: Vec<FollowUp>,
    /// Name of the user who opened the case (provenance).
    #[serde(default)]
    pub created_by: String,
    /// Name of the party currently accountable for the case.
    #[serde(default)]
    pub steward: String,
    /// Retention class governing how long the record is kept.
    #[serde(default = "default_retention")]
    pub retention: RetentionClass,
    /// When true, disposal is blocked regardless of retention (e.g. litigation).
    #[serde(default)]
    pub legal_hold: bool,
}

fn default_matter_type() -> MatterType {
    MatterType::Other
}

fn default_case_outcome() -> CaseOutcome {
    CaseOutcome::Ongoing
}

fn default_retention() -> RetentionClass {
    RetentionClass::Standard
}

// ---------------------------------------------------------------------------
// Communications management — org-owned, multi-channel conversation records.
//
// These model communications so they are retained by the organization rather
// than by an individual volunteer, and so text / chat / email / phone all share
// one record shape. The POC populates the `WebChat` channel end-to-end; the
// other channels reuse the same types once an external provider is wired in.
// ---------------------------------------------------------------------------

/// The medium a communication came in / went out on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    WebChat,
    Sms,
    Email,
    Phone,
}

impl Channel {
    pub const ALL: [Channel; 4] = [
        Channel::WebChat,
        Channel::Sms,
        Channel::Email,
        Channel::Phone,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Channel::WebChat => "Web chat",
            Channel::Sms => "SMS",
            Channel::Email => "Email",
            Channel::Phone => "Phone",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Channel::WebChat => "web_chat",
            Channel::Sms => "sms",
            Channel::Email => "email",
            Channel::Phone => "phone",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.slug() == s)
    }

    pub fn badge_classes(self) -> &'static str {
        match self {
            Channel::WebChat => "bg-primary-500/15 text-primary-300 ring-1 ring-primary-500/30",
            Channel::Sms => "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30",
            Channel::Email => "bg-sky-500/15 text-sky-300 ring-1 ring-sky-500/30",
            Channel::Phone => "bg-violet-500/15 text-violet-300 ring-1 ring-violet-500/30",
        }
    }
}

/// Whether a message came into the organization or went out from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageDirection {
    Inbound,
    Outbound,
}

/// Who authored a message within a conversation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorKind {
    /// The outside person (client/lead) contacting the organization.
    Visitor,
    /// The automated RAG assistant.
    Bot,
    /// A human volunteer or staff member acting on behalf of the org.
    Staff,
}

impl AuthorKind {
    pub fn label(self) -> &'static str {
        match self {
            AuthorKind::Visitor => "Visitor",
            AuthorKind::Bot => "Assistant",
            AuthorKind::Staff => "Staff",
        }
    }
}

/// Lifecycle of an org-owned conversation as staff work it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationStatus {
    /// Incoming, not yet claimed by a human.
    New,
    /// Claimed by a volunteer/staff member.
    Assigned,
    /// Waiting on the visitor to reply.
    AwaitingReply,
    /// Resolved / archived.
    Closed,
}

impl ConversationStatus {
    pub const ALL: [ConversationStatus; 4] = [
        ConversationStatus::New,
        ConversationStatus::Assigned,
        ConversationStatus::AwaitingReply,
        ConversationStatus::Closed,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ConversationStatus::New => "New",
            ConversationStatus::Assigned => "Assigned",
            ConversationStatus::AwaitingReply => "Awaiting reply",
            ConversationStatus::Closed => "Closed",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            ConversationStatus::New => "new",
            ConversationStatus::Assigned => "assigned",
            ConversationStatus::AwaitingReply => "awaiting_reply",
            ConversationStatus::Closed => "closed",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.slug() == s)
    }

    pub fn badge_classes(self) -> &'static str {
        match self {
            ConversationStatus::New => "bg-rose-500/15 text-rose-300 ring-1 ring-rose-500/30",
            ConversationStatus::Assigned => {
                "bg-violet-500/15 text-violet-300 ring-1 ring-violet-500/30"
            }
            ConversationStatus::AwaitingReply => {
                "bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/30"
            }
            ConversationStatus::Closed => "bg-slate-500/15 text-slate-400 ring-1 ring-slate-500/30",
        }
    }
}

/// A single message retained within a conversation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub channel: Channel,
    pub direction: MessageDirection,
    pub author_kind: AuthorKind,
    /// Volunteer/user id when authored by staff; `None` for visitor/bot.
    #[serde(default)]
    pub author_id: Option<String>,
    pub author_label: String,
    pub body: String,
    /// RAG citations when the assistant authored this message.
    #[serde(default)]
    pub sources: Vec<SourceInfo>,
    pub created_at: String,
}

/// An org-owned thread of communication with a client/lead across one channel.
///
/// Ownership lives here, not with the assigned volunteer: `assigned_volunteer_id`
/// can be reassigned or cleared (e.g. when a volunteer leaves) without ever
/// losing the retained history, and `case_id` / `contact_id` fold the activity
/// into the client's case record automatically.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub channel: Channel,
    pub subject: String,
    pub status: ConversationStatus,
    #[serde(default)]
    pub contact_id: Option<String>,
    #[serde(default)]
    pub case_id: Option<String>,
    #[serde(default)]
    pub assigned_volunteer_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub last_message_preview: String,
    pub message_count: usize,
}

/// A conversation together with its full retained message history.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConversationThread {
    pub conversation: Conversation,
    pub messages: Vec<Message>,
}

/// `POST /api/conversations/{id}/messages` — a staff/volunteer reply.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplyRequest {
    pub body: String,
    #[serde(default)]
    pub author_id: Option<String>,
    #[serde(default)]
    pub author_label: Option<String>,
}

/// `POST /api/conversations/{id}/assign` — (re)assign or unassign an owner.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssignRequest {
    /// `None` unassigns (e.g. when the previous owner leaves the org).
    #[serde(default)]
    pub volunteer_id: Option<String>,
}

/// `POST /api/conversations/{id}/link-case` — fold activity into a case record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkCaseRequest {
    #[serde(default)]
    pub case_id: Option<String>,
    #[serde(default)]
    pub contact_id: Option<String>,
}

/// `POST /api/conversations/{id}/status` — move the conversation lifecycle.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StatusRequest {
    pub status: ConversationStatus,
}

// ---------------------------------------------------------------------------
// Privacy, security & data-governance types.
// ---------------------------------------------------------------------------

/// Sensitivity classification for a stored document. Drives access control
/// (who may open it) and retention handling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentClassification {
    /// Shareable outside the organization.
    Public,
    /// Internal use; visible to any authenticated staff/volunteer on the case.
    Internal,
    /// Sensitive; requires the `ViewConfidentialDocs` permission.
    Confidential,
    /// Highly sensitive (e.g. safety plans, court records); staff-level only.
    Restricted,
}

impl DocumentClassification {
    pub const ALL: [DocumentClassification; 4] = [
        DocumentClassification::Public,
        DocumentClassification::Internal,
        DocumentClassification::Confidential,
        DocumentClassification::Restricted,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DocumentClassification::Public => "Public",
            DocumentClassification::Internal => "Internal",
            DocumentClassification::Confidential => "Confidential",
            DocumentClassification::Restricted => "Restricted",
        }
    }

    /// Whether opening this document requires elevated permission.
    pub fn is_sensitive(self) -> bool {
        matches!(
            self,
            DocumentClassification::Confidential | DocumentClassification::Restricted
        )
    }

    pub fn badge_classes(self) -> &'static str {
        match self {
            DocumentClassification::Public => {
                "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30"
            }
            DocumentClassification::Internal => "bg-sky-500/15 text-sky-300 ring-1 ring-sky-500/30",
            DocumentClassification::Confidential => {
                "bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/30"
            }
            DocumentClassification::Restricted => {
                "bg-rose-500/15 text-rose-300 ring-1 ring-rose-500/30"
            }
        }
    }
}

/// Records-retention class applied to a case. Determines how long records are
/// kept before they become eligible for disposal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionClass {
    /// Kept for the life of the case plus a short tail.
    Standard,
    /// Longer statutory retention (e.g. legal/court involvement).
    Extended,
    /// Never auto-dispose (e.g. ongoing safety concern).
    Permanent,
}

impl RetentionClass {
    pub const ALL: [RetentionClass; 3] = [
        RetentionClass::Standard,
        RetentionClass::Extended,
        RetentionClass::Permanent,
    ];

    pub fn label(self) -> &'static str {
        match self {
            RetentionClass::Standard => "Standard (7 yrs)",
            RetentionClass::Extended => "Extended (10 yrs)",
            RetentionClass::Permanent => "Permanent",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            RetentionClass::Standard => "standard",
            RetentionClass::Extended => "extended",
            RetentionClass::Permanent => "permanent",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.slug() == s)
    }
}

// ---------------------------------------------------------------------------
// Audit trail (demo, in-memory).
// ---------------------------------------------------------------------------

/// The category of action recorded in the audit trail.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    Login,
    Logout,
    Register,
    ViewCase,
    CreateCase,
    UpdateCaseStatus,
    AssignVolunteer,
    UploadDocument,
    AccessDocument,
    DeniedAccess,
    AddVolunteer,
    UpdateVolunteerStatus,
    ChangeUserRole,
    OffboardVolunteer,
    LegalHoldChange,
    DisposeRecord,
    EditKnowledge,
}

impl AuditAction {
    pub fn label(self) -> &'static str {
        match self {
            AuditAction::Login => "Signed in",
            AuditAction::Logout => "Signed out",
            AuditAction::Register => "Registered account",
            AuditAction::ViewCase => "Viewed case",
            AuditAction::CreateCase => "Created case",
            AuditAction::UpdateCaseStatus => "Changed case status",
            AuditAction::AssignVolunteer => "Changed case assignment",
            AuditAction::UploadDocument => "Uploaded document",
            AuditAction::AccessDocument => "Opened document",
            AuditAction::DeniedAccess => "Access denied",
            AuditAction::AddVolunteer => "Added volunteer",
            AuditAction::UpdateVolunteerStatus => "Changed volunteer status",
            AuditAction::ChangeUserRole => "Changed user role",
            AuditAction::OffboardVolunteer => "Offboarded volunteer",
            AuditAction::LegalHoldChange => "Changed legal hold",
            AuditAction::DisposeRecord => "Disposed record",
            AuditAction::EditKnowledge => "Edited knowledge resource",
        }
    }

    /// Whether this event represents a security-relevant denial.
    pub fn is_denial(self) -> bool {
        matches!(self, AuditAction::DeniedAccess)
    }
}

/// A single entry in the audit trail: who did what, to which target, and when.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: String,
    /// Name of the actor who performed the action.
    pub actor: String,
    /// The actor's role at the time of the action.
    pub actor_role: Role,
    pub action: AuditAction,
    /// Human-readable description of the affected record.
    pub target: String,
    /// Timestamp label (demo: a display string, not a real clock).
    pub at: String,
}

// ---------------------------------------------------------------------------
// Institutional knowledge base (demo, in-memory).
// ---------------------------------------------------------------------------

/// The kind of institutional-knowledge resource, so knowledge is retained by
/// the organization rather than leaving with individual volunteers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeCategory {
    /// Reusable document template (intake forms, letters, checklists).
    Template,
    /// External resource or referral (shelters, hotlines, legal aid).
    Resource,
    /// Best-practice guidance or standard operating procedure.
    BestPractice,
    /// Retained case context / handover notes.
    CaseContext,
}

impl KnowledgeCategory {
    pub const ALL: [KnowledgeCategory; 4] = [
        KnowledgeCategory::Template,
        KnowledgeCategory::Resource,
        KnowledgeCategory::BestPractice,
        KnowledgeCategory::CaseContext,
    ];

    pub fn label(self) -> &'static str {
        match self {
            KnowledgeCategory::Template => "Template",
            KnowledgeCategory::Resource => "Resource",
            KnowledgeCategory::BestPractice => "Best practice",
            KnowledgeCategory::CaseContext => "Case context",
        }
    }

    pub fn badge_classes(self) -> &'static str {
        match self {
            KnowledgeCategory::Template => "bg-sky-500/15 text-sky-300 ring-1 ring-sky-500/30",
            KnowledgeCategory::Resource => {
                "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30"
            }
            KnowledgeCategory::BestPractice => {
                "bg-violet-500/15 text-violet-300 ring-1 ring-violet-500/30"
            }
            KnowledgeCategory::CaseContext => {
                "bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/30"
            }
        }
    }
}

/// An institutional-knowledge entry owned by the organization.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeItem {
    pub id: String,
    pub title: String,
    pub category: KnowledgeCategory,
    pub summary: String,
    /// Who contributed it (provenance) — ownership stays with the organization.
    pub contributed_by: String,
    pub updated_at: String,
}
