//! Local, in-memory demo data for volunteers, cases, and user accounts.
//!
//! Compiled for both the server and the browser so the whole volunteer/case
//! management experience works without a database. Replaced by a real backend
//! in a later phase.

use crate::taxonomy::ServiceType;
use crate::types::{
    Case, CaseDocument, CaseNote, CaseOutcome, CasePriority, CaseStatus, Client, EvidenceItem,
    EvidenceType, FollowUp, MatterType, NeedCategory, Referral, ReviewStatus, Role, ServiceRecord,
    TimelineEvent, TimelineKind, TrainingRecord, User, Volunteer, VolunteerStatus,
};

/// Convenience constructor for seed evidence, keeping the case fixtures compact.
#[allow(clippy::too_many_arguments)]
fn evi(
    id: &str,
    name: &str,
    evidence_type: EvidenceType,
    party: &str,
    source: &str,
    occurred_on: &str,
    review_status: ReviewStatus,
    tags: &[&str],
    description: &str,
) -> EvidenceItem {
    EvidenceItem {
        id: id.into(),
        name: name.into(),
        evidence_type,
        description: description.into(),
        source: source.into(),
        party: party.into(),
        occurred_on: occurred_on.into(),
        tags: tags.iter().map(|t| t.to_string()).collect(),
        review_status,
        uploaded_at: occurred_on.into(),
    }
}

/// Demo login accounts. Passwords are plaintext on purpose — this is fake data.
pub fn users() -> Vec<User> {
    vec![
        User {
            id: "u-admin".into(),
            name: "Sarah Mitchell".into(),
            email: "admin@mommysheart.org".into(),
            password: "admin123".into(),
            role: Role::Admin,
            volunteer_id: None,
        },
        User {
            id: "u-priya".into(),
            name: "Priya Nair".into(),
            email: "priya@mommysheart.org".into(),
            password: "volunteer123".into(),
            role: Role::Volunteer,
            volunteer_id: Some("v-1".into()),
        },
        User {
            id: "u-maria".into(),
            name: "Maria Gonzalez".into(),
            email: "maria@mommysheart.org".into(),
            password: "volunteer123".into(),
            role: Role::Volunteer,
            volunteer_id: Some("v-2".into()),
        },
    ]
}

pub fn volunteers() -> Vec<Volunteer> {
    vec![
        Volunteer {
            id: "v-1".into(),
            name: "Priya Nair".into(),
            email: "priya@mommysheart.org".into(),
            phone: "+1 (312) 555-0175".into(),
            specialty: "Case management".into(),
            status: VolunteerStatus::Active,
            hours_logged: 142.5,
            weekly_availability_hours: 16.0,
            client_contacts: 48,
            trainings: vec![
                TrainingRecord {
                    name: "Trauma-informed care".into(),
                    completed_on: "2026-02-11".into(),
                },
                TrainingRecord {
                    name: "Safety planning".into(),
                    completed_on: "2026-04-03".into(),
                },
            ],
        },
        Volunteer {
            id: "v-2".into(),
            name: "Maria Gonzalez".into(),
            email: "maria@mommysheart.org".into(),
            phone: "+1 (415) 555-0132".into(),
            specialty: "Legal advocacy".into(),
            status: VolunteerStatus::Active,
            hours_logged: 98.0,
            weekly_availability_hours: 12.0,
            client_contacts: 31,
            trainings: vec![
                TrainingRecord {
                    name: "Trauma-informed care".into(),
                    completed_on: "2026-02-11".into(),
                },
                TrainingRecord {
                    name: "Immigration law basics".into(),
                    completed_on: "2026-05-20".into(),
                },
            ],
        },
        Volunteer {
            id: "v-3".into(),
            name: "James Okoye".into(),
            email: "james@mommysheart.org".into(),
            phone: "+1 (206) 555-0188".into(),
            specialty: "Housing support".into(),
            status: VolunteerStatus::OnLeave,
            hours_logged: 61.5,
            weekly_availability_hours: 6.0,
            client_contacts: 19,
            trainings: vec![TrainingRecord {
                name: "Trauma-informed care".into(),
                completed_on: "2026-03-18".into(),
            }],
        },
        Volunteer {
            id: "v-4".into(),
            name: "Aisha Rahman".into(),
            email: "aisha@mommysheart.org".into(),
            phone: "+1 (617) 555-0143".into(),
            specialty: "Mental health".into(),
            status: VolunteerStatus::Pending,
            hours_logged: 12.0,
            weekly_availability_hours: 8.0,
            client_contacts: 4,
            trainings: vec![TrainingRecord {
                name: "Trauma-informed care".into(),
                completed_on: String::new(),
            }],
        },
    ]
}

/// People served by the organization. `cl-1` deliberately has several
/// interconnected cases to demonstrate the service-pathway view.
pub fn clients() -> Vec<Client> {
    vec![
        Client {
            id: "cl-1".into(),
            display_name: "Client A. (confidential)".into(),
            phone: "+1 (312) 555-0110".into(),
            email: String::new(),
            intake_date: "2026-06-12".into(),
            summary: "Single mother of two presenting with multiple, interconnected needs: emergency housing, an active custody matter, and unfiled public benefits.".into(),
        },
        Client {
            id: "cl-2".into(),
            display_name: "Client B. (confidential)".into(),
            phone: "+1 (312) 555-0121".into(),
            email: String::new(),
            intake_date: "2026-06-28".into(),
            summary: "Recent arrival seeking immigration support and work authorization guidance.".into(),
        },
        Client {
            id: "cl-3".into(),
            display_name: "Client C. (confidential)".into(),
            phone: "+1 (312) 555-0133".into(),
            email: String::new(),
            intake_date: "2026-05-19".into(),
            summary: "Referred for mental health support after a period of crisis.".into(),
        },
        Client {
            id: "cl-4".into(),
            display_name: "Client D. (confidential)".into(),
            phone: "+1 (312) 555-0144".into(),
            email: String::new(),
            intake_date: "2026-07-05".into(),
            summary: "New survivor intake — awaiting risk assessment.".into(),
        },
    ]
}

/// Build a timeline event for seed data.
fn ev(id: &str, at: &str, kind: TimelineKind, summary: &str) -> TimelineEvent {
    TimelineEvent {
        id: id.into(),
        at: at.into(),
        kind,
        summary: summary.into(),
    }
}

pub fn cases() -> Vec<Case> {
    vec![
        // --- Client A.: three interconnected, cross-linked cases -------------
        Case {
            id: "c-1001".into(),
            title: "Emergency housing placement".into(),
            client_id: "cl-1".into(),
            category: NeedCategory::Housing,
            service_types: vec![ServiceType::ShelterPlacement, ServiceType::HousingSubsidies],
            summary: "Needs emergency shelter and a longer-term housing plan.".into(),
            status: CaseStatus::InProgress,
            priority: CasePriority::High,
            assigned_volunteer_ids: vec!["v-1".into(), "v-3".into()],
            related_case_ids: vec!["c-1002".into(), "c-1003".into()],
            notes: vec![CaseNote {
                id: "n-1".into(),
                author: "Priya Nair".into(),
                body: "Placed in short-term shelter; applying for transitional housing this week."
                    .into(),
                created_at: "2026-06-15".into(),
            }],
            documents: vec![
                CaseDocument {
                    id: "d-1".into(),
                    name: "Intake assessment.pdf".into(),
                    uploaded_at: "2026-06-14".into(),
                },
                CaseDocument {
                    id: "d-2".into(),
                    name: "Housing application.docx".into(),
                    uploaded_at: "2026-06-20".into(),
                },
            ],
            evidence: vec![
                evi(
                    "ev-1",
                    "Intake assessment",
                    EvidenceType::SupportingDocument,
                    "Client A.",
                    "In-person intake",
                    "2026-06-14",
                    ReviewStatus::Reviewed,
                    &["intake", "housing"],
                    "Initial needs assessment completed at the shelter.",
                ),
                evi(
                    "ev-2",
                    "Threatening texts from ex-partner",
                    EvidenceType::TextMessage,
                    "Ex-partner",
                    "Client's phone (screenshots)",
                    "2026-06-16",
                    ReviewStatus::Flagged,
                    &["threats", "safety"],
                    "Series of intimidating messages sent overnight; relevant to safety planning.",
                ),
                evi(
                    "ev-3",
                    "Bruising photograph",
                    EvidenceType::Photograph,
                    "Client A.",
                    "Client's phone",
                    "2026-06-17",
                    ReviewStatus::InReview,
                    &["injury", "safety"],
                    "Photo documenting injury, dated by phone metadata.",
                ),
                evi(
                    "ev-4",
                    "Housing application",
                    EvidenceType::SupportingDocument,
                    "Client A.",
                    "Housing authority portal",
                    "2026-06-20",
                    ReviewStatus::Reviewed,
                    &["housing"],
                    "Submitted emergency housing application confirmation.",
                ),
                evi(
                    "ev-5",
                    "Voicemail from landlord",
                    EvidenceType::AudioRecording,
                    "Landlord",
                    "Client's voicemail",
                    "2026-06-22",
                    ReviewStatus::Unreviewed,
                    &["housing", "eviction"],
                    "Landlord voicemail regarding move-out timeline.",
                ),
            ],
            timeline: vec![
                ev("e-1", "2026-06-12", TimelineKind::Opened, "Case opened"),
                ev(
                    "e-2",
                    "2026-06-14",
                    TimelineKind::VolunteerAssigned,
                    "Assigned Priya Nair",
                ),
                ev("e-3", "2026-06-15", TimelineKind::NoteAdded, "Note added"),
            ],
            opened_at: "2026-06-12".into(),
            matter_type: MatterType::Housing,
            outcome: CaseOutcome::Ongoing,
            intake_date: "2026-06-10".into(),
            resolved_date: None,
            referrals: vec![Referral {
                agency: "City Housing Authority".into(),
                date: "2026-06-20".into(),
            }],
            services: vec![
                ServiceRecord {
                    kind: "Legal clinic".into(),
                    date: "2026-06-14".into(),
                },
                ServiceRecord {
                    kind: "Housing navigation".into(),
                    date: "2026-06-25".into(),
                },
            ],
            follow_ups: vec![
                FollowUp {
                    date: "2026-06-30".into(),
                    completed: true,
                },
                FollowUp {
                    date: "2026-07-10".into(),
                    completed: false,
                },
            ],
        },
        Case {
            id: "c-1002".into(),
            title: "Family court advocacy".into(),
            client_id: "cl-1".into(),
            category: NeedCategory::FamilyCourt,
            service_types: vec![
                ServiceType::CustodyVisitation,
                ServiceType::OrdersOfProtection,
            ],
            summary: "Support through custody proceedings and safety planning.".into(),
            status: CaseStatus::Open,
            priority: CasePriority::High,
            assigned_volunteer_ids: vec!["v-2".into()],
            related_case_ids: vec!["c-1001".into(), "c-1003".into()],
            notes: Vec::new(),
            documents: vec![CaseDocument {
                id: "d-3".into(),
                name: "Court schedule.pdf".into(),
                uploaded_at: "2026-07-01".into(),
            }],
            evidence: vec![
                evi(
                    "ev-6",
                    "Court hearing schedule",
                    EvidenceType::CourtFiling,
                    "Family Court",
                    "Court portal",
                    "2026-07-01",
                    ReviewStatus::Reviewed,
                    &["court", "custody"],
                    "Notice of upcoming custody hearing dates.",
                ),
                evi(
                    "ev-7",
                    "Custody exchange emails",
                    EvidenceType::Email,
                    "Ex-partner",
                    "Client's email",
                    "2026-07-03",
                    ReviewStatus::InReview,
                    &["custody", "communication"],
                    "Email thread showing repeated missed exchanges.",
                ),
                evi(
                    "ev-8",
                    "Affidavit of witness",
                    EvidenceType::Affidavit,
                    "Neighbor (witness)",
                    "Signed affidavit",
                    "2026-07-05",
                    ReviewStatus::Unreviewed,
                    &["court", "witness"],
                    "Neighbor's sworn statement about an incident.",
                ),
            ],
            timeline: vec![ev("e-4", "2026-06-28", TimelineKind::Opened, "Case opened")],
            opened_at: "2026-06-28".into(),
            matter_type: MatterType::CustodyVisitation,
            outcome: CaseOutcome::Ongoing,
            intake_date: "2026-06-26".into(),
            resolved_date: None,
            referrals: vec![Referral {
                agency: "Family Court Self-Help Center".into(),
                date: "2026-07-01".into(),
            }],
            services: vec![ServiceRecord {
                kind: "Legal advice".into(),
                date: "2026-06-30".into(),
            }],
            follow_ups: vec![FollowUp {
                date: "2026-07-08".into(),
                completed: false,
            }],
        },
        Case {
            id: "c-1003".into(),
            title: "Public benefits enrollment".into(),
            client_id: "cl-1".into(),
            category: NeedCategory::PublicBenefits,
            service_types: vec![
                ServiceType::Snap,
                ServiceType::CashAssistance,
                ServiceType::ChildCareAssistance,
            ],
            summary: "Assist with SNAP, Medicaid, and childcare subsidy applications.".into(),
            status: CaseStatus::OnHold,
            priority: CasePriority::Medium,
            assigned_volunteer_ids: vec!["v-1".into()],
            related_case_ids: vec!["c-1001".into(), "c-1002".into()],
            notes: Vec::new(),
            documents: Vec::new(),
            evidence: vec![evi(
                "ev-9",
                "Denial letter screenshot",
                EvidenceType::Screenshot,
                "State agency",
                "Benefits portal",
                "2026-05-22",
                ReviewStatus::Reviewed,
                &["benefits", "snap"],
                "Screenshot of initial SNAP denial for appeal reference.",
            )],
            timeline: vec![ev("e-5", "2026-05-19", TimelineKind::Opened, "Case opened")],
            opened_at: "2026-05-19".into(),
            matter_type: MatterType::PublicBenefits,
            outcome: CaseOutcome::Resolved,
            intake_date: "2026-05-15".into(),
            resolved_date: Some("2026-06-20".into()),
            referrals: vec![Referral {
                agency: "SNAP Benefits Office".into(),
                date: "2026-05-22".into(),
            }],
            services: vec![
                ServiceRecord {
                    kind: "Benefits application".into(),
                    date: "2026-05-18".into(),
                },
                ServiceRecord {
                    kind: "Appeal filing".into(),
                    date: "2026-06-05".into(),
                },
            ],
            follow_ups: vec![
                FollowUp {
                    date: "2026-06-01".into(),
                    completed: true,
                },
                FollowUp {
                    date: "2026-06-18".into(),
                    completed: true,
                },
            ],
        },
        // --- Other clients: single-need cases -------------------------------
        Case {
            id: "c-1004".into(),
            title: "Immigration support".into(),
            client_id: "cl-2".into(),
            category: NeedCategory::Immigration,
            service_types: vec![ServiceType::WorkAuthorization, ServiceType::Vawa],
            summary: "Guidance on work authorization and document preparation.".into(),
            status: CaseStatus::Open,
            priority: CasePriority::Medium,
            assigned_volunteer_ids: vec!["v-2".into()],
            related_case_ids: Vec::new(),
            notes: Vec::new(),
            documents: Vec::new(),
            evidence: Vec::new(),
            timeline: vec![ev("e-6", "2026-06-28", TimelineKind::Opened, "Case opened")],
            opened_at: "2026-06-28".into(),
            matter_type: MatterType::Immigration,
            outcome: CaseOutcome::Ongoing,
            intake_date: "2026-06-26".into(),
            resolved_date: None,
            referrals: vec![Referral {
                agency: "Immigration Legal Aid".into(),
                date: "2026-07-02".into(),
            }],
            services: vec![ServiceRecord {
                kind: "Consultation".into(),
                date: "2026-06-30".into(),
            }],
            follow_ups: vec![FollowUp {
                date: "2026-07-05".into(),
                completed: false,
            }],
        },
        Case {
            id: "c-1005".into(),
            title: "Mental health referral".into(),
            client_id: "cl-3".into(),
            category: NeedCategory::MentalHealth,
            service_types: vec![
                ServiceType::TherapyReferrals,
                ServiceType::SupportGroups,
            ],
            summary: "Connect with a partner clinic for ongoing counseling.".into(),
            status: CaseStatus::InProgress,
            priority: CasePriority::Medium,
            assigned_volunteer_ids: vec!["v-4".into()],
            related_case_ids: Vec::new(),
            notes: Vec::new(),
            documents: Vec::new(),
            evidence: Vec::new(),
            timeline: vec![ev("e-7", "2026-05-19", TimelineKind::Opened, "Case opened")],
            opened_at: "2026-05-19".into(),
            matter_type: MatterType::MentalHealth,
            outcome: CaseOutcome::Ongoing,
            intake_date: "2026-05-15".into(),
            resolved_date: None,
            referrals: vec![Referral {
                agency: "Community Counseling Center".into(),
                date: "2026-05-25".into(),
            }],
            services: vec![
                ServiceRecord {
                    kind: "Counseling referral".into(),
                    date: "2026-05-20".into(),
                },
                ServiceRecord {
                    kind: "Wellness check".into(),
                    date: "2026-06-02".into(),
                },
            ],
            follow_ups: vec![FollowUp {
                date: "2026-06-10".into(),
                completed: true,
            }],
        },
        Case {
            id: "c-1006".into(),
            title: "Survivor support intake".into(),
            client_id: "cl-4".into(),
            category: NeedCategory::Other,
            service_types: vec![
                ServiceType::CrisisIntervention,
                ServiceType::SafetyPlanning,
            ],
            summary: "New intake — needs risk assessment and a wellness check-in schedule.".into(),
            status: CaseStatus::Open,
            priority: CasePriority::High,
            assigned_volunteer_ids: Vec::new(),
            related_case_ids: Vec::new(),
            notes: Vec::new(),
            documents: Vec::new(),
            evidence: Vec::new(),
            timeline: vec![ev("e-8", "2026-07-05", TimelineKind::Opened, "Case opened")],
            opened_at: "2026-07-05".into(),
            matter_type: MatterType::Other,
            outcome: CaseOutcome::ReferredOut,
            intake_date: "2026-07-03".into(),
            resolved_date: Some("2026-07-04".into()),
            referrals: vec![Referral {
                agency: "Partner Advocacy Agency".into(),
                date: "2026-07-04".into(),
            }],
            services: Vec::new(),
            follow_ups: vec![FollowUp {
                date: "2026-07-12".into(),
                completed: false,
            }],
        },
    ]
}
