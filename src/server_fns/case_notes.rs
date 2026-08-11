//! Structured Case Notes server functions and shared domain types.
//!
//! New Case Notes are staff-only structured records. Existing free-text notes are
//! exposed only as immutable legacy notes through the old case detail shape.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::server_fns::pagination::Page;
use crate::server_fns::users::AccountRole;

pub const SAFETY_WARNING: &str = "Recording an urgent or emergency concern here does not contact emergency services or replace Mommy's Heart escalation procedures.";
pub const MAX_SHORT_TEXT_CHARS: usize = 200;
pub const MAX_MEDIUM_TEXT_CHARS: usize = 1_000;
pub const MAX_LONG_TEXT_CHARS: usize = 5_000;
pub const MAX_NARRATIVE_CHARS: usize = 10_000;
pub const MAX_MULTISELECT_CHOICES: usize = 12;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseNoteAccessSummary {
    pub can_view: bool,
    pub can_add: bool,
    pub can_inspect_drafts: bool,
}

macro_rules! coded_enum {
    ($name:ident { $($variant:ident => ($slug:literal, $label:literal)),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            pub fn slug(self) -> &'static str {
                match self {
                    $($name::$variant => $slug),+
                }
            }

            pub fn label(self) -> &'static str {
                match self {
                    $($name::$variant => $label),+
                }
            }

            pub fn from_slug(s: &str) -> Option<Self> {
                Self::ALL.iter().copied().find(|v| v.slug() == s)
            }
        }
    };
}

coded_enum!(CaseNoteState {
    Draft => ("draft", "Draft"),
    Finalized => ("finalized", "Finalized"),
    Discarded => ("discarded", "Discarded"),
    Legacy => ("legacy", "Legacy"),
});

impl CaseNoteState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Finalized | Self::Discarded | Self::Legacy)
    }
}

coded_enum!(CaseNoteAudience {
    VolunteerOnly => ("volunteer_only", "Volunteers and admins only"),
    SharedLegacy => ("shared_legacy", "Legacy shared audience"),
});

coded_enum!(CaseNoteInteractionType {
    TelephoneCall => ("telephone_call", "Telephone call"),
    Email => ("email", "Email"),
    TextMessage => ("text_message", "Text message"),
    Videoconference => ("videoconference", "Zoom or videoconference"),
    InPersonMeeting => ("in_person_meeting", "In-person meeting"),
    CourtAppearance => ("court_appearance", "Court appearance"),
    ClientAccompaniment => ("client_accompaniment", "Client accompaniment"),
    InternalCasework => ("internal_casework", "Internal casework"),
    DocumentReview => ("document_review", "Document review"),
    LegalResearch => ("legal_research", "Legal research"),
    CaseConsultation => ("case_consultation", "Case consultation"),
    InterdisciplinaryMeeting => ("interdisciplinary_meeting", "Interdisciplinary team meeting"),
    Referral => ("referral", "Referral"),
    AdvocacyOutreach => ("advocacy_outreach", "Advocacy or outreach"),
    OutsideProviderCommunication => ("outside_provider_communication", "Communication with an outside provider"),
    AttorneyCommunication => ("attorney_communication", "Communication with an attorney"),
    CourtProfessionalCommunication => ("court_professional_communication", "Communication with a court professional"),
    GovernmentAgencyCommunication => ("government_agency_communication", "Communication with a government agency"),
    AttemptedContact => ("attempted_contact", "Attempted contact"),
    Other => ("other", "Other"),
});

coded_enum!(ContactCategory {
    DirectClientContact => ("direct_client_contact", "Direct client contact"),
    CollateralContact => ("collateral_contact", "Collateral contact"),
    InternalCasework => ("internal_casework", "Internal casework"),
    GroupTeamActivity => ("group_team_activity", "Group or team activity"),
    AttemptedContact => ("attempted_contact", "Attempted contact"),
    Other => ("other", "Other"),
    Unknown => ("unknown", "Unknown"),
    ClientDeclined => ("client_declined", "Client declined"),
    NotApplicable => ("not_applicable", "Not applicable"),
});

coded_enum!(ContactDirection {
    Incoming => ("incoming", "Incoming"),
    Outgoing => ("outgoing", "Outgoing"),
    Bidirectional => ("bidirectional", "Bidirectional"),
    NotApplicable => ("not_applicable", "Not applicable"),
    Unknown => ("unknown", "Unknown"),
});

coded_enum!(CompletionOutcome {
    Yes => ("yes", "Yes"),
    No => ("no", "No"),
    Partially => ("partially", "Partially"),
    NotApplicable => ("not_applicable", "Not applicable"),
    Unknown => ("unknown", "Unknown"),
});

coded_enum!(ServiceArea {
    Legal => ("legal", "Legal"),
    MentalHealth => ("mental_health", "Mental health"),
    DomesticViolenceSupport => ("domestic_violence_support", "Domestic-violence support"),
    SafetyPlanning => ("safety_planning", "Safety planning"),
    SocialServices => ("social_services", "Social services"),
    Housing => ("housing", "Housing"),
    BenefitsPublicAssistance => ("benefits_public_assistance", "Benefits/public assistance"),
    FinancialAssistance => ("financial_assistance", "Financial assistance"),
    CareerBuilding => ("career_building", "Career building"),
    Employment => ("employment", "Employment"),
    EducationTraining => ("education_training", "Education/training"),
    ChildFamilyServices => ("child_family_services", "Child/family services"),
    DisabilityServices => ("disability_services", "Disability services"),
    Immigration => ("immigration", "Immigration"),
    Advocacy => ("advocacy", "Advocacy"),
    Referrals => ("referrals", "Referrals"),
    CaseManagement => ("case_management", "Case management"),
    Other => ("other", "Other"),
    Unknown => ("unknown", "Unknown"),
    ClientDeclined => ("client_declined", "Client declined"),
    NotApplicable => ("not_applicable", "Not applicable"),
});

coded_enum!(InformationSource {
    ClientReported => ("client_reported", "Client report"),
    DirectObservation => ("direct_observation", "Direct observation"),
    CourtDocument => ("court_document", "Court document"),
    AttorneyCommunication => ("attorney_communication", "Attorney communication"),
    ServiceProviderCommunication => ("service_provider_communication", "Service-provider communication"),
    GovernmentAgencyCommunication => ("government_agency_communication", "Government-agency communication"),
    PoliceChildProtectionRecord => ("police_child_protection_record", "Police or ACS/CPS record"),
    Other => ("other", "Other"),
    Unknown => ("unknown", "Unknown"),
    ClientDeclined => ("client_declined", "Client declined"),
    NotApplicable => ("not_applicable", "Not applicable"),
});

coded_enum!(UrgencyLevel {
    Routine => ("routine", "Routine"),
    Elevated => ("elevated", "Elevated"),
    Urgent => ("urgent", "Urgent"),
});

coded_enum!(AddendumCategory {
    ActivityTiming => ("activity_timing", "Activity timing"),
    Interaction => ("interaction", "Interaction"),
    Participants => ("participants", "Participants"),
    ServiceAreas => ("service_areas", "Service areas"),
    ReportedInformation => ("reported_information", "Reported information"),
    ObservedInformation => ("observed_information", "Observed information"),
    Actions => ("actions", "Actions"),
    Outcome => ("outcome", "Outcome"),
    Urgency => ("urgency", "Urgency"),
    NextSteps => ("next_steps", "Next steps"),
    Narrative => ("narrative", "Narrative"),
    Other => ("other", "Other"),
});

coded_enum!(CaseNoteAuditAction {
    CreateDraft => ("create_draft", "Created draft"),
    SaveDraft => ("save_draft", "Saved draft"),
    DiscardDraft => ("discard_draft", "Discarded draft"),
    FinalizeNote => ("finalize_note", "Finalized note"),
    AdminInspectDraft => ("admin_inspect_draft", "Admin inspected draft"),
    AddAddendum => ("add_addendum", "Added addendum"),
    ExportNote => ("export_note", "Exported note"),
});

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CaseNoteDraftInput {
    #[serde(default)]
    pub activity_date: String,
    #[serde(default)]
    pub start_time: String,
    #[serde(default)]
    pub end_time: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub delayed_entry_reason: String,
    #[serde(default)]
    pub primary_interaction: Option<CaseNoteInteractionType>,
    #[serde(default)]
    pub contact_category: Option<ContactCategory>,
    #[serde(default)]
    pub contact_direction: Option<ContactDirection>,
    #[serde(default)]
    pub completion_outcome: Option<CompletionOutcome>,
    #[serde(default)]
    pub participant_summary: String,
    #[serde(default)]
    pub service_areas: Vec<ServiceArea>,
    #[serde(default)]
    pub purpose: String,
    #[serde(default)]
    pub client_reported_info: String,
    #[serde(default)]
    pub verified_observed_info: String,
    #[serde(default)]
    pub information_sources: Vec<InformationSource>,
    #[serde(default)]
    pub actions_taken: String,
    #[serde(default)]
    pub outcome_response: String,
    #[serde(default)]
    pub progress_barriers: String,
    #[serde(default)]
    pub urgency: Option<UrgencyLevel>,
    #[serde(default)]
    pub urgency_details: String,
    #[serde(default)]
    pub next_steps: String,
    #[serde(default)]
    pub next_steps_not_applicable: bool,
    #[serde(default)]
    pub narrative: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseNoteValidationError {
    pub field: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedCaseNoteFinalization {
    pub input: CaseNoteDraftInput,
    pub total_minutes: i32,
    pub signature_name: String,
    pub accuracy_confirmed: bool,
}

impl CaseNoteDraftInput {
    pub fn normalized(&self) -> Self {
        let mut out = self.clone();
        out.activity_date = normalize_space(&out.activity_date);
        out.start_time = normalize_space(&out.start_time);
        out.end_time = normalize_space(&out.end_time);
        out.location = normalize_space(&out.location);
        out.delayed_entry_reason = normalize_space(&out.delayed_entry_reason);
        out.participant_summary = normalize_space(&out.participant_summary);
        out.purpose = normalize_space(&out.purpose);
        out.client_reported_info = normalize_space(&out.client_reported_info);
        out.verified_observed_info = normalize_space(&out.verified_observed_info);
        out.actions_taken = normalize_space(&out.actions_taken);
        out.outcome_response = normalize_space(&out.outcome_response);
        out.progress_barriers = normalize_space(&out.progress_barriers);
        out.urgency_details = normalize_space(&out.urgency_details);
        out.next_steps = normalize_space(&out.next_steps);
        out.narrative = normalize_space(&out.narrative);
        if out.next_steps_not_applicable {
            out.next_steps.clear();
        }
        out.service_areas = dedup_choices(out.service_areas);
        out.information_sources = dedup_choices(out.information_sources);
        out
    }

    pub fn calculated_total_minutes(&self) -> Option<i32> {
        let start = parse_time_minutes(&self.start_time)?;
        let end = parse_time_minutes(&self.end_time)?;
        (end > start).then_some(end - start)
    }

    /// Draft saves may be incomplete, but supplied dates/times and bounded text are
    /// still validated server-side.
    pub fn validate_draft(&self) -> Result<(), Vec<CaseNoteValidationError>> {
        let mut errors = Vec::new();
        validate_optional_date("activity_date", &self.activity_date, &mut errors);
        validate_optional_time("start_time", &self.start_time, &mut errors);
        validate_optional_time("end_time", &self.end_time, &mut errors);
        if let (Some(start), Some(end)) = (
            parse_time_minutes(&self.start_time),
            parse_time_minutes(&self.end_time),
        ) {
            if end <= start {
                push_error(
                    &mut errors,
                    "end_time",
                    "End time must be after start time.",
                );
            }
        }
        validate_text(
            "location",
            &self.location,
            MAX_SHORT_TEXT_CHARS,
            &mut errors,
        );
        validate_text(
            "delayed_entry_reason",
            &self.delayed_entry_reason,
            MAX_MEDIUM_TEXT_CHARS,
            &mut errors,
        );
        validate_text(
            "participant_summary",
            &self.participant_summary,
            MAX_MEDIUM_TEXT_CHARS,
            &mut errors,
        );
        validate_text("purpose", &self.purpose, MAX_LONG_TEXT_CHARS, &mut errors);
        validate_text(
            "client_reported_info",
            &self.client_reported_info,
            MAX_LONG_TEXT_CHARS,
            &mut errors,
        );
        validate_text(
            "verified_observed_info",
            &self.verified_observed_info,
            MAX_LONG_TEXT_CHARS,
            &mut errors,
        );
        validate_text(
            "actions_taken",
            &self.actions_taken,
            MAX_LONG_TEXT_CHARS,
            &mut errors,
        );
        validate_text(
            "outcome_response",
            &self.outcome_response,
            MAX_LONG_TEXT_CHARS,
            &mut errors,
        );
        validate_text(
            "progress_barriers",
            &self.progress_barriers,
            MAX_LONG_TEXT_CHARS,
            &mut errors,
        );
        validate_text(
            "urgency_details",
            &self.urgency_details,
            MAX_LONG_TEXT_CHARS,
            &mut errors,
        );
        validate_text(
            "next_steps",
            &self.next_steps,
            MAX_LONG_TEXT_CHARS,
            &mut errors,
        );
        validate_text(
            "narrative",
            &self.narrative,
            MAX_NARRATIVE_CHARS,
            &mut errors,
        );
        validate_choices("service_areas", self.service_areas.len(), &mut errors);
        validate_choices(
            "information_sources",
            self.information_sources.len(),
            &mut errors,
        );
        finish_validation(errors)
    }

    pub fn validate_for_finalization(
        &self,
        today: &str,
        current_time: &str,
        typed_signature_name: &str,
        current_full_name: &str,
        accuracy_confirmed: bool,
    ) -> Result<ValidatedCaseNoteFinalization, Vec<CaseNoteValidationError>> {
        let input = self.normalized();
        let mut errors = input.validate_draft().err().unwrap_or_default();

        if input.activity_date.is_empty() {
            push_error(&mut errors, "activity_date", "Activity date is required.");
        } else if parse_date(&input.activity_date).is_some() && input.activity_date.as_str() > today
        {
            push_error(
                &mut errors,
                "activity_date",
                "Activity date cannot be in the future.",
            );
        }

        if input.start_time.is_empty() {
            push_error(&mut errors, "start_time", "Start time is required.");
        }
        if input.end_time.is_empty() {
            push_error(&mut errors, "end_time", "End time is required.");
        }
        let total_minutes = match input.calculated_total_minutes() {
            Some(minutes) => minutes,
            None => {
                push_error(
                    &mut errors,
                    "total_minutes",
                    "A valid start and end time are required.",
                );
                0
            }
        };
        if input.activity_date == today {
            if let (Some(end), Some(now)) = (
                parse_time_minutes(&input.end_time),
                parse_time_minutes(current_time),
            ) {
                if end > now {
                    push_error(
                        &mut errors,
                        "end_time",
                        "Activity end time cannot be in the future.",
                    );
                }
            }
        }

        if input.activity_date != today && input.delayed_entry_reason.is_empty() {
            push_error(
                &mut errors,
                "delayed_entry_reason",
                "Delayed entry reason is required when the activity date is not today.",
            );
        }
        if input.primary_interaction.is_none() {
            push_error(
                &mut errors,
                "primary_interaction",
                "Primary interaction is required.",
            );
        }
        if input.contact_category.is_none() {
            push_error(
                &mut errors,
                "contact_category",
                "Contact category is required.",
            );
        }
        if input.contact_direction.is_none() {
            push_error(
                &mut errors,
                "contact_direction",
                "Contact direction is required.",
            );
        }
        if input.completion_outcome.is_none() {
            push_error(
                &mut errors,
                "completion_outcome",
                "Completion outcome is required.",
            );
        }
        if input.service_areas.is_empty() {
            push_error(
                &mut errors,
                "service_areas",
                "At least one service area is required.",
            );
        }
        require_text(
            &input.purpose,
            "purpose",
            "Purpose/objective is required.",
            &mut errors,
        );
        require_text(
            &input.actions_taken,
            "actions_taken",
            "Actions taken are required.",
            &mut errors,
        );
        require_text(
            &input.outcome_response,
            "outcome_response",
            "Client response/outcome is required.",
            &mut errors,
        );
        match input.urgency {
            None => push_error(&mut errors, "urgency", "Urgency is required."),
            Some(UrgencyLevel::Routine) => {}
            Some(_) if input.urgency_details.is_empty() => push_error(
                &mut errors,
                "urgency_details",
                "Elevated or urgent notes require urgency details.",
            ),
            Some(_) => {}
        }
        if input.next_steps.is_empty() && !input.next_steps_not_applicable {
            push_error(
                &mut errors,
                "next_steps",
                "Next steps are required unless marked not applicable.",
            );
        }
        require_text(
            &input.narrative,
            "narrative",
            "A concise factual narrative is required.",
            &mut errors,
        );

        let signature_name = normalize_space(typed_signature_name);
        let current_full_name = normalize_space(current_full_name);
        if signature_name.is_empty() {
            push_error(
                &mut errors,
                "signature_name",
                "Type your full name to finalize the note.",
            );
        } else if signature_name != current_full_name {
            push_error(
                &mut errors,
                "signature_name",
                "Signature name must match your current account name.",
            );
        }

        if !accuracy_confirmed {
            push_error(
                &mut errors,
                "accuracy_confirmed",
                "Confirm accuracy before finalizing.",
            );
        }

        if errors.is_empty() {
            Ok(ValidatedCaseNoteFinalization {
                input,
                total_minutes,
                signature_name,
                accuracy_confirmed,
            })
        } else {
            Err(errors)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseNoteAddendumInput {
    pub reason: String,
    pub information: String,
    #[serde(default)]
    pub affected_categories: Vec<AddendumCategory>,
    #[serde(default)]
    pub follow_up: String,
    pub signature_name: String,
}

impl CaseNoteAddendumInput {
    pub fn normalized(&self) -> Self {
        Self {
            reason: normalize_space(&self.reason),
            information: normalize_space(&self.information),
            affected_categories: dedup_choices(self.affected_categories.clone()),
            follow_up: normalize_space(&self.follow_up),
            signature_name: normalize_space(&self.signature_name),
        }
    }

    pub fn validate(&self, current_full_name: &str) -> Result<Self, Vec<CaseNoteValidationError>> {
        let input = self.normalized();
        let mut errors = Vec::new();
        require_text(
            &input.reason,
            "reason",
            "Addendum reason is required.",
            &mut errors,
        );
        require_text(
            &input.information,
            "information",
            "Supplemental or corrected information is required.",
            &mut errors,
        );
        if input.affected_categories.is_empty() {
            push_error(
                &mut errors,
                "affected_categories",
                "Choose at least one affected category.",
            );
        }
        validate_choices(
            "affected_categories",
            input.affected_categories.len(),
            &mut errors,
        );
        validate_text("reason", &input.reason, MAX_MEDIUM_TEXT_CHARS, &mut errors);
        validate_text(
            "information",
            &input.information,
            MAX_LONG_TEXT_CHARS,
            &mut errors,
        );
        validate_text(
            "follow_up",
            &input.follow_up,
            MAX_LONG_TEXT_CHARS,
            &mut errors,
        );
        let current_full_name = normalize_space(current_full_name);
        if input.signature_name.is_empty() {
            push_error(
                &mut errors,
                "signature_name",
                "Type your full name to sign the addendum.",
            );
        } else if input.signature_name != current_full_name {
            push_error(
                &mut errors,
                "signature_name",
                "Signature name must match your current account name.",
            );
        }
        if errors.is_empty() {
            Ok(input)
        } else {
            Err(errors)
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseNoteListFilters {
    #[serde(default)]
    pub start_date: String,
    #[serde(default)]
    pub end_date: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub primary_interaction: Option<CaseNoteInteractionType>,
    #[serde(default)]
    pub state: Option<CaseNoteState>,
    #[serde(default)]
    pub urgency: Option<UrgencyLevel>,
    #[serde(default)]
    pub keyword: String,
}

impl CaseNoteListFilters {
    pub fn normalized(&self) -> Self {
        Self {
            start_date: normalize_space(&self.start_date),
            end_date: normalize_space(&self.end_date),
            author: normalize_space(&self.author),
            primary_interaction: self.primary_interaction,
            state: self.state,
            urgency: self.urgency,
            keyword: normalize_space(&self.keyword),
        }
    }

    pub fn validate(&self) -> Result<Self, Vec<CaseNoteValidationError>> {
        let filters = self.normalized();
        let mut errors = Vec::new();
        validate_optional_date("start_date", &filters.start_date, &mut errors);
        validate_optional_date("end_date", &filters.end_date, &mut errors);
        if !filters.start_date.is_empty()
            && !filters.end_date.is_empty()
            && filters.start_date > filters.end_date
        {
            push_error(
                &mut errors,
                "end_date",
                "End date must be on or after start date.",
            );
        }
        validate_text("author", &filters.author, MAX_SHORT_TEXT_CHARS, &mut errors);
        validate_text(
            "keyword",
            &filters.keyword,
            MAX_SHORT_TEXT_CHARS,
            &mut errors,
        );
        if errors.is_empty() {
            Ok(filters)
        } else {
            Err(errors)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseNoteListItem {
    pub id: String,
    pub case_id: String,
    pub state: CaseNoteState,
    pub audience: CaseNoteAudience,
    pub author_user_id: String,
    pub author: String,
    pub author_role_snapshot: Option<AccountRole>,
    pub created_at: String,
    pub updated_at: String,
    pub finalized_at: String,
    pub discarded_at: String,
    pub activity_date: String,
    pub total_minutes: Option<i32>,
    pub primary_interaction: Option<CaseNoteInteractionType>,
    pub urgency: Option<UrgencyLevel>,
    pub addendum_count: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CaseNoteDetail {
    pub id: String,
    pub case_id: String,
    pub state: CaseNoteState,
    pub audience: CaseNoteAudience,
    pub author_user_id: String,
    pub author: String,
    pub author_role_snapshot: Option<AccountRole>,
    pub created_at: String,
    pub updated_at: String,
    pub finalized_at: String,
    pub discarded_at: String,
    pub signature_name: String,
    pub signature_signed_at: String,
    pub total_minutes: Option<i32>,
    pub draft: CaseNoteDraftInput,
    /// Free-text body for migrated legacy notes only. Structured notes keep their
    /// content in `draft` and clients never receive this detail type.
    pub legacy_body: String,
    /// False only for an administrator's redacted pre-inspection draft shell.
    #[serde(default)]
    pub content_revealed: bool,
    #[serde(default)]
    pub addenda: Vec<CaseNoteAddendum>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseNoteAddendum {
    pub id: String,
    pub note_id: String,
    pub case_id: String,
    pub audience: CaseNoteAudience,
    pub author_user_id: String,
    pub author: String,
    pub author_role_snapshot: AccountRole,
    pub reason: String,
    pub information: String,
    #[serde(default)]
    pub affected_categories: Vec<AddendumCategory>,
    pub follow_up: String,
    pub signature_name: String,
    pub signed_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseNoteAuditEntry {
    pub id: String,
    pub note_id: String,
    pub case_id: String,
    pub addendum_id: String,
    pub actor_user_id: String,
    pub actor: String,
    pub actor_role_snapshot: AccountRole,
    pub action: CaseNoteAuditAction,
    pub at: String,
    /// JSON object with identifiers/states only; never note body or client facts.
    pub metadata: String,
}

#[server(prefix = "/api")]
pub async fn case_note_access(case_id: String) -> Result<CaseNoteAccessSummary, ServerFnError> {
    use crate::server::permissions::{capabilities_on, require_user};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    if !user.role.has_volunteer_privileges() {
        return Ok(CaseNoteAccessSummary::default());
    }
    let capabilities = capabilities_on(&user, &case_id).await.unwrap_or_default();
    Ok(CaseNoteAccessSummary {
        can_view: capabilities.contains(&CaseCapability::ViewCase),
        can_add: capabilities.contains(&CaseCapability::AddNotes),
        can_inspect_drafts: user.role.has_operations_admin_permissions()
            && capabilities.contains(&CaseCapability::ViewCase),
    })
}

#[server(prefix = "/api")]
pub async fn list_case_notes(
    case_id: String,
    filters: CaseNoteListFilters,
    offset: i64,
    limit: i64,
) -> Result<Page<CaseNoteListItem>, ServerFnError> {
    use crate::server::db::case_notes;
    use crate::server::permissions::{require_cap, require_user};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    require_staff(&user)?;
    require_cap(&user, &case_id, CaseCapability::ViewCase).await?;
    let filters = filters.validate().map_err(validation_text)?;
    case_notes::page(&case_id, &filters, offset, limit, &user)
        .await
        .map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn load_case_note(note_id: String) -> Result<Option<CaseNoteDetail>, ServerFnError> {
    use crate::server::db::case_notes;
    use crate::server::permissions::require_user;
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    require_staff(&user)?;
    let _case_id = authorized_note_case(&user, &note_id, CaseCapability::ViewCase).await?;
    case_notes::get_visible(
        &note_id,
        &user,
        user.role.has_operations_admin_permissions(),
    )
    .await
    .map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn create_case_note_draft(
    case_id: String,
    draft: CaseNoteDraftInput,
) -> Result<CaseNoteDetail, ServerFnError> {
    use crate::server::db::case_notes;
    use crate::server::permissions::{require_cap, require_user};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    require_staff(&user)?;
    require_cap(&user, &case_id, CaseCapability::AddNotes).await?;
    let draft = draft.normalized();
    draft.validate_draft().map_err(validation_text)?;
    case_notes::create_draft(&case_id, &user, &draft)
        .await
        .map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn create_and_finalize_case_note(
    case_id: String,
    draft: CaseNoteDraftInput,
    signature_name: String,
    accuracy_confirmed: bool,
) -> Result<CaseNoteDetail, ServerFnError> {
    use crate::server::db::case_notes;
    use crate::server::permissions::{require_cap, require_user};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    require_staff(&user)?;
    require_cap(&user, &case_id, CaseCapability::AddNotes).await?;
    let (today, current_time) = now_local_date_time();
    let finalization = draft
        .validate_for_finalization(
            &today,
            &current_time,
            &signature_name,
            &user.full_name(),
            accuracy_confirmed,
        )
        .map_err(validation_text)?;
    case_notes::create_finalized(&case_id, &user, &finalization)
        .await
        .map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn save_case_note_draft(
    note_id: String,
    draft: CaseNoteDraftInput,
) -> Result<CaseNoteDetail, ServerFnError> {
    use crate::server::db::case_notes;
    use crate::server::permissions::require_user;
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    require_staff(&user)?;
    let _case_id = authorized_note_case(&user, &note_id, CaseCapability::AddNotes).await?;
    let draft = draft.normalized();
    draft.validate_draft().map_err(validation_text)?;
    case_notes::save_draft(&note_id, &user, &draft)
        .await
        .map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn finalize_case_note_draft(
    note_id: String,
    draft: CaseNoteDraftInput,
    signature_name: String,
    accuracy_confirmed: bool,
) -> Result<CaseNoteDetail, ServerFnError> {
    use crate::server::db::case_notes;
    use crate::server::permissions::require_user;
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    require_staff(&user)?;
    let _case_id = authorized_note_case(&user, &note_id, CaseCapability::AddNotes).await?;
    let (today, current_time) = now_local_date_time();
    let finalization = draft
        .validate_for_finalization(
            &today,
            &current_time,
            &signature_name,
            &user.full_name(),
            accuracy_confirmed,
        )
        .map_err(validation_text)?;
    case_notes::finalize_draft(&note_id, &user, &finalization)
        .await
        .map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn discard_case_note_draft(note_id: String) -> Result<(), ServerFnError> {
    use crate::server::db::case_notes;
    use crate::server::permissions::require_user;
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    require_staff(&user)?;
    let _case_id = authorized_note_case(&user, &note_id, CaseCapability::AddNotes).await?;
    case_notes::discard_draft(&note_id, &user)
        .await
        .map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn admin_inspect_case_note_draft(
    note_id: String,
) -> Result<CaseNoteDetail, ServerFnError> {
    use crate::server::db::case_notes;
    use crate::server::permissions::{require_operations_admin, require_user};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    require_staff(&user)?;
    require_operations_admin(&user)?;
    let _case_id = authorized_note_case(&user, &note_id, CaseCapability::ViewCase).await?;
    case_notes::admin_inspect_draft(&note_id, &user)
        .await
        .map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn add_case_note_addendum(
    note_id: String,
    input: CaseNoteAddendumInput,
) -> Result<CaseNoteAddendum, ServerFnError> {
    use crate::server::db::case_notes;
    use crate::server::permissions::require_user;
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    require_staff(&user)?;
    let _case_id = authorized_note_case(&user, &note_id, CaseCapability::AddNotes).await?;
    let input = input.validate(&user.full_name()).map_err(validation_text)?;
    case_notes::add_addendum(&note_id, &user, &input)
        .await
        .map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn list_case_note_audit(
    case_id: String,
    note_id: String,
    offset: i64,
    limit: i64,
) -> Result<Page<CaseNoteAuditEntry>, ServerFnError> {
    use crate::server::db::case_notes;
    use crate::server::permissions::{require_cap, require_operations_admin, require_user};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    require_staff(&user)?;
    require_operations_admin(&user)?;
    require_cap(&user, &case_id, CaseCapability::ViewCase).await?;
    case_notes::audit_page(&case_id, &note_id, offset, limit, &user)
        .await
        .map_err(ServerFnError::new)
}

#[cfg(feature = "ssr")]
async fn authorized_note_case(
    user: &crate::server_fns::users::User,
    note_id: &str,
    capability: crate::server_fns::capabilities::CaseCapability,
) -> Result<String, ServerFnError> {
    use crate::server::db::case_notes;
    use crate::server::permissions::require_cap;

    let case_id = case_notes::case_id(note_id)
        .await
        .map_err(|_| ServerFnError::new("Note not found."))?
        .ok_or_else(|| ServerFnError::new("Note not found."))?;
    if require_cap(user, &case_id, capability).await.is_err() {
        return Err(ServerFnError::new("Note not found."));
    }
    Ok(case_id)
}

#[cfg(feature = "ssr")]
fn now_local_date_time() -> (String, String) {
    let now = chrono::Local::now();
    (
        now.format("%Y-%m-%d").to_string(),
        now.format("%H:%M").to_string(),
    )
}

#[cfg(feature = "ssr")]
fn require_staff(user: &crate::server_fns::users::User) -> Result<(), ServerFnError> {
    if user.role.has_volunteer_privileges() {
        Ok(())
    } else {
        Err(ServerFnError::new(
            "Structured Case Notes are visible only to staff assigned to the case.",
        ))
    }
}

#[cfg(feature = "ssr")]
fn validation_text(errors: Vec<CaseNoteValidationError>) -> ServerFnError {
    ServerFnError::new(validation_message(&errors))
}

pub fn validation_message(errors: &[CaseNoteValidationError]) -> String {
    errors
        .iter()
        .map(|e| format!("{}: {}", e.field, e.message))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn normalize_space(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn dedup_choices<T: Copy + Eq>(choices: Vec<T>) -> Vec<T> {
    let mut out = Vec::new();
    for choice in choices {
        if !out.contains(&choice) {
            out.push(choice);
        }
    }
    out
}

fn require_text(
    value: &str,
    field: &str,
    message: &str,
    errors: &mut Vec<CaseNoteValidationError>,
) {
    if value.trim().is_empty() {
        push_error(errors, field, message);
    }
}

fn validate_text(
    field: &str,
    value: &str,
    max_chars: usize,
    errors: &mut Vec<CaseNoteValidationError>,
) {
    if value.chars().count() > max_chars {
        push_error(
            errors,
            field,
            &format!("Must be {max_chars} characters or fewer."),
        );
    }
}

fn validate_choices(field: &str, len: usize, errors: &mut Vec<CaseNoteValidationError>) {
    if len > MAX_MULTISELECT_CHOICES {
        push_error(
            errors,
            field,
            &format!("Choose no more than {MAX_MULTISELECT_CHOICES} options."),
        );
    }
}

fn validate_optional_date(field: &str, value: &str, errors: &mut Vec<CaseNoteValidationError>) {
    if !value.is_empty() && parse_date(value).is_none() {
        push_error(errors, field, "Use a valid date in YYYY-MM-DD format.");
    }
}

fn validate_optional_time(field: &str, value: &str, errors: &mut Vec<CaseNoteValidationError>) {
    if !value.is_empty() && parse_time_minutes(value).is_none() {
        push_error(errors, field, "Use a valid time in HH:MM format.");
    }
}

fn finish_validation(
    errors: Vec<CaseNoteValidationError>,
) -> Result<(), Vec<CaseNoteValidationError>> {
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn push_error(errors: &mut Vec<CaseNoteValidationError>, field: &str, message: &str) {
    errors.push(CaseNoteValidationError {
        field: field.to_string(),
        message: message.to_string(),
    });
}

fn parse_date(value: &str) -> Option<(i32, u32, u32)> {
    let mut parts = value.split('-');
    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parts.next()?.parse::<u32>().ok()?;
    let day = parts.next()?.parse::<u32>().ok()?;
    if parts.next().is_some() || value.len() != 10 || !(1..=12).contains(&month) {
        return None;
    }
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => return None,
    };
    (1..=max_day).contains(&day).then_some((year, month, day))
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn parse_time_minutes(value: &str) -> Option<i32> {
    let mut parts = value.split(':');
    let hour = parts.next()?.parse::<i32>().ok()?;
    let minute = parts.next()?.parse::<i32>().ok()?;
    if parts.next().is_some()
        || value.len() != 5
        || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute)
    {
        return None;
    }
    Some(hour * 60 + minute)
}
