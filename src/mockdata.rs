//! Local, in-memory demo data for volunteers, cases, and user accounts.
//!
//! Compiled for both the server and the browser so the whole volunteer/case
//! management experience works without a database. Replaced by a real backend
//! in a later phase.

use crate::types::{
    Case, CaseDocument, CaseNote, CasePriority, CaseStatus, Client, NeedCategory, Role,
    TimelineEvent, TimelineKind, User, Volunteer, VolunteerStatus,
};

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
        },
        Volunteer {
            id: "v-2".into(),
            name: "Maria Gonzalez".into(),
            email: "maria@mommysheart.org".into(),
            phone: "+1 (415) 555-0132".into(),
            specialty: "Legal advocacy".into(),
            status: VolunteerStatus::Active,
        },
        Volunteer {
            id: "v-3".into(),
            name: "James Okoye".into(),
            email: "james@mommysheart.org".into(),
            phone: "+1 (206) 555-0188".into(),
            specialty: "Housing support".into(),
            status: VolunteerStatus::OnLeave,
        },
        Volunteer {
            id: "v-4".into(),
            name: "Aisha Rahman".into(),
            email: "aisha@mommysheart.org".into(),
            phone: "+1 (617) 555-0143".into(),
            specialty: "Mental health".into(),
            status: VolunteerStatus::Pending,
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
        },
        Case {
            id: "c-1002".into(),
            title: "Family court advocacy".into(),
            client_id: "cl-1".into(),
            category: NeedCategory::FamilyCourt,
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
            timeline: vec![ev("e-4", "2026-06-28", TimelineKind::Opened, "Case opened")],
            opened_at: "2026-06-28".into(),
        },
        Case {
            id: "c-1003".into(),
            title: "Public benefits enrollment".into(),
            client_id: "cl-1".into(),
            category: NeedCategory::PublicBenefits,
            summary: "Assist with SNAP, Medicaid, and childcare subsidy applications.".into(),
            status: CaseStatus::OnHold,
            priority: CasePriority::Medium,
            assigned_volunteer_ids: vec!["v-1".into()],
            related_case_ids: vec!["c-1001".into(), "c-1002".into()],
            notes: Vec::new(),
            documents: Vec::new(),
            timeline: vec![ev("e-5", "2026-05-19", TimelineKind::Opened, "Case opened")],
            opened_at: "2026-05-19".into(),
        },
        // --- Other clients: single-need cases -------------------------------
        Case {
            id: "c-1004".into(),
            title: "Immigration support".into(),
            client_id: "cl-2".into(),
            category: NeedCategory::Immigration,
            summary: "Guidance on work authorization and document preparation.".into(),
            status: CaseStatus::Open,
            priority: CasePriority::Medium,
            assigned_volunteer_ids: vec!["v-2".into()],
            related_case_ids: Vec::new(),
            notes: Vec::new(),
            documents: Vec::new(),
            timeline: vec![ev("e-6", "2026-06-28", TimelineKind::Opened, "Case opened")],
            opened_at: "2026-06-28".into(),
        },
        Case {
            id: "c-1005".into(),
            title: "Mental health referral".into(),
            client_id: "cl-3".into(),
            category: NeedCategory::MentalHealth,
            summary: "Connect with a partner clinic for ongoing counseling.".into(),
            status: CaseStatus::InProgress,
            priority: CasePriority::Medium,
            assigned_volunteer_ids: vec!["v-4".into()],
            related_case_ids: Vec::new(),
            notes: Vec::new(),
            documents: Vec::new(),
            timeline: vec![ev("e-7", "2026-05-19", TimelineKind::Opened, "Case opened")],
            opened_at: "2026-05-19".into(),
        },
        Case {
            id: "c-1006".into(),
            title: "Survivor support intake".into(),
            client_id: "cl-4".into(),
            category: NeedCategory::Other,
            summary: "New intake — needs risk assessment and a wellness check-in schedule.".into(),
            status: CaseStatus::Open,
            priority: CasePriority::High,
            assigned_volunteer_ids: Vec::new(),
            related_case_ids: Vec::new(),
            notes: Vec::new(),
            documents: Vec::new(),
            timeline: vec![ev("e-8", "2026-07-05", TimelineKind::Opened, "Case opened")],
            opened_at: "2026-07-05".into(),
        },
    ]
}
