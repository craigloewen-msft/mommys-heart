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
}

fn default_matter_type() -> MatterType {
    MatterType::Other
}

fn default_case_outcome() -> CaseOutcome {
    CaseOutcome::Ongoing
}
