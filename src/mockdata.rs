//! Local, in-memory demo data for volunteers, cases, and user accounts.
//!
//! Compiled for both the server and the browser so the whole volunteer/case
//! management experience works without a database. Replaced by a real backend
//! in a later phase.

use crate::types::{
    Case, CaseDocument, CasePriority, CaseStatus, Role, User, Volunteer, VolunteerStatus,
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

pub fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "c-1001".into(),
            title: "Emergency housing placement".into(),
            client_name: "Client A. (confidential)".into(),
            summary: "Single mother of two needs emergency shelter and a longer-term housing plan.".into(),
            status: CaseStatus::InProgress,
            priority: CasePriority::High,
            assigned_volunteer_ids: vec!["v-1".into(), "v-3".into()],
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
            opened_at: "2026-06-12".into(),
        },
        Case {
            id: "c-1002".into(),
            title: "Family court advocacy".into(),
            client_name: "Client B. (confidential)".into(),
            summary: "Support through custody proceedings and safety planning.".into(),
            status: CaseStatus::Open,
            priority: CasePriority::Medium,
            assigned_volunteer_ids: vec!["v-2".into()],
            documents: vec![CaseDocument {
                id: "d-3".into(),
                name: "Court schedule.pdf".into(),
                uploaded_at: "2026-07-01".into(),
            }],
            opened_at: "2026-06-28".into(),
        },
        Case {
            id: "c-1003".into(),
            title: "Public benefits enrollment".into(),
            client_name: "Client C. (confidential)".into(),
            summary: "Assist with SNAP, Medicaid, and childcare subsidy applications.".into(),
            status: CaseStatus::OnHold,
            priority: CasePriority::Low,
            assigned_volunteer_ids: vec!["v-1".into()],
            documents: Vec::new(),
            opened_at: "2026-05-19".into(),
        },
        Case {
            id: "c-1004".into(),
            title: "Survivor support intake".into(),
            client_name: "Client D. (confidential)".into(),
            summary: "New intake — needs risk assessment and a wellness check-in schedule.".into(),
            status: CaseStatus::Open,
            priority: CasePriority::High,
            assigned_volunteer_ids: Vec::new(),
            documents: Vec::new(),
            opened_at: "2026-07-05".into(),
        },
    ]
}
