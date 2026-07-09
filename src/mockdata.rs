//! Local, in-memory demo data for the V1 app.
//!
//! Compiled for both the server and the browser so the whole experience works
//! without a database. Replaced by a real backend in a later phase.

use crate::types::{
    AccountRole, Case, CaseAssignment, CaseCapability, CaseNote, CasePreset, CaseProperty,
    CaseStatus, ChangeLogEntry, Evidence, Grant, Message, User,
};

/// Organization display name.
pub const ORG_NAME: &str = "Mommy's Heart";

/// Build a case assignment from a preset for compact fixtures.
fn assign(case_id: &str, preset: CasePreset) -> CaseAssignment {
    CaseAssignment {
        case_id: case_id.into(),
        capabilities: preset.capabilities(),
    }
}

fn note(id: &str, author: &str, body: &str, created_at: &str) -> CaseNote {
    CaseNote {
        id: id.into(),
        author: author.into(),
        body: body.into(),
        created_at: created_at.into(),
    }
}

fn evidence(
    id: &str,
    name: &str,
    case_id: &str,
    uploaded_by: &str,
    uploaded_at: &str,
    description: &str,
) -> Evidence {
    Evidence {
        id: id.into(),
        name: name.into(),
        case_id: case_id.into(),
        uploaded_by: uploaded_by.into(),
        uploaded_at: uploaded_at.into(),
        description: description.into(),
    }
}

fn prop(key: &str, value: &str) -> CaseProperty {
    CaseProperty {
        key: key.into(),
        value: value.into(),
    }
}

fn change(
    id: &str,
    actor: &str,
    field: &str,
    old_value: &str,
    new_value: &str,
    at: &str,
) -> ChangeLogEntry {
    ChangeLogEntry {
        id: id.into(),
        actor: actor.into(),
        field: field.into(),
        old_value: old_value.into(),
        new_value: new_value.into(),
        at: at.into(),
    }
}

/// Seed user accounts (one per account role, plus extras).
pub fn users() -> Vec<User> {
    vec![
        User {
            id: "u-admin".into(),
            first_name: "Alex".into(),
            last_name: "Rivera".into(),
            email: "admin@mommysheart.org".into(),
            phone: "(555) 100-2000".into(),
            home_address: "12 Chestnut St, Springfield".into(),
            password: "admin123".into(),
            role: AccountRole::Admin,
            assigned_cases: vec![assign("c-1001", CasePreset::Manager)],
            audit_log: vec![
                change(
                    "ul-a1",
                    "Dana Cole",
                    "phone",
                    "(555) 100-1999",
                    "(555) 100-2000",
                    "2026-07-07 09:12",
                ),
                change(
                    "ul-a2",
                    "system",
                    "email",
                    "arivera@old.org",
                    "admin@mommysheart.org",
                    "2026-06-27 15:40",
                ),
                change(
                    "ul-a3",
                    "system",
                    "role",
                    "volunteer",
                    "admin",
                    "2026-01-04 10:00",
                ),
            ],
        },
        User {
            id: "u-vol".into(),
            first_name: "Dana".into(),
            last_name: "Cole".into(),
            email: "dana@mommysheart.org".into(),
            phone: "(555) 200-3000".into(),
            home_address: "48 Maple Ave, Springfield".into(),
            password: "volunteer123".into(),
            role: AccountRole::Volunteer,
            assigned_cases: vec![
                assign("c-1001", CasePreset::Contributor),
                assign("c-1002", CasePreset::Manager),
            ],
            audit_log: vec![
                change(
                    "ul-v1",
                    "Alex Rivera",
                    "case:c-1002",
                    "",
                    "manager",
                    "2026-07-05 11:03",
                ),
                change(
                    "ul-v2",
                    "Alex Rivera",
                    "home_address",
                    "40 Maple Ave",
                    "48 Maple Ave, Springfield",
                    "2026-06-30 08:20",
                ),
                change(
                    "ul-v3",
                    "Alex Rivera",
                    "phone",
                    "(555) 200-2999",
                    "(555) 200-3000",
                    "2026-02-11 14:15",
                ),
            ],
        },
        User {
            id: "u-client".into(),
            first_name: "Jamie".into(),
            last_name: "Nguyen".into(),
            email: "jamie@example.com".into(),
            phone: "(555) 300-4000".into(),
            home_address: "301 Oak Blvd, Springfield".into(),
            password: "client123".into(),
            role: AccountRole::Client,
            // A client who can upload evidence for their own case.
            assigned_cases: vec![assign("c-1002", CasePreset::Contributor)],
            audit_log: vec![
                change(
                    "ul-c1",
                    "Alex Rivera",
                    "case:c-1002",
                    "",
                    "contributor",
                    "2026-07-08 16:45",
                ),
                change(
                    "ul-c2",
                    "Dana Cole",
                    "phone",
                    "(555) 300-3999",
                    "(555) 300-4000",
                    "2026-06-14 10:05",
                ),
            ],
        },
        User {
            id: "u-vol2".into(),
            first_name: "Priya".into(),
            last_name: "Shah".into(),
            email: "priya@mommysheart.org".into(),
            phone: "(555) 400-5000".into(),
            home_address: "77 Birch Ln, Springfield".into(),
            password: "volunteer123".into(),
            role: AccountRole::Volunteer,
            // A volunteer who can read evidence but not upload or delete it.
            assigned_cases: vec![CaseAssignment {
                case_id: "c-1001".into(),
                capabilities: vec![
                    CaseCapability::ViewCase,
                    CaseCapability::ViewEvidence,
                    CaseCapability::SendMessages,
                ],
            }],
            audit_log: vec![
                change(
                    "ul-p1",
                    "Alex Rivera",
                    "case:c-1001",
                    "",
                    "view_case, view_evidence, send_messages",
                    "2026-07-02 13:30",
                ),
                change(
                    "ul-p2",
                    "system",
                    "role",
                    "client",
                    "volunteer",
                    "2026-06-09 09:00",
                ),
            ],
        },
    ]
}

/// Seed grants.
pub fn grants() -> Vec<Grant> {
    vec![
        Grant {
            id: "g-1".into(),
            name: "Family Stability Fund".into(),
        },
        Grant {
            id: "g-2".into(),
            name: "Legal Aid Access Grant".into(),
        },
        Grant {
            id: "g-3".into(),
            name: "Community Housing Initiative".into(),
        },
    ]
}

/// Seed cases.
pub fn cases() -> Vec<Case> {
    vec![
        Case {
            id: "c-1001".into(),
            name: "Nguyen custody matter".into(),
            status: CaseStatus::Open,
            owner_id: "u-admin".into(),
            notes: vec![
                note(
                    "n-1",
                    "Alex Rivera",
                    "Initial intake completed. Client seeking custody support.",
                    "2026-03-01",
                ),
                note(
                    "n-2",
                    "Dana Cole",
                    "Filed initial paperwork with the county clerk.",
                    "2026-03-06",
                ),
            ],
            evidence: vec![evidence(
                "e-1",
                "Text message thread (March)",
                "c-1001",
                "Dana Cole",
                "2026-03-05",
                "Screenshots of scheduling messages.",
            )],
            properties: vec![
                prop("Opposing attorney", "J. Smith"),
                prop("Court", "Springfield Family Court"),
                prop("Docket", "FC-2026-0421"),
            ],
            audit_log: vec![change(
                "cl-1",
                "Alex Rivera",
                "status",
                "monitor",
                "open",
                "2026-03-01",
            )],
        },
        Case {
            id: "c-1002".into(),
            name: "Nguyen housing assistance".into(),
            status: CaseStatus::Monitor,
            owner_id: "u-vol".into(),
            notes: vec![note(
                "n-3",
                "Dana Cole",
                "Connected client with housing initiative resources.",
                "2026-02-20",
            )],
            evidence: Vec::new(),
            properties: vec![prop("Caseworker", "Dana Cole")],
            audit_log: Vec::new(),
        },
    ]
}

/// Seed case chat messages.
pub fn messages() -> Vec<Message> {
    vec![
        Message {
            id: "m-1".into(),
            case_id: "c-1001".into(),
            author_id: "u-vol".into(),
            author: "Dana Cole".into(),
            body: "The county clerk confirmed receipt of the paperwork.".into(),
            sent_at: "2026-03-06 09:14".into(),
        },
        Message {
            id: "m-2".into(),
            case_id: "c-1001".into(),
            author_id: "u-admin".into(),
            author: "Alex Rivera".into(),
            body: "Great, thanks for the quick turnaround.".into(),
            sent_at: "2026-03-06 10:02".into(),
        },
        Message {
            id: "m-3".into(),
            case_id: "c-1002".into(),
            author_id: "u-vol".into(),
            author: "Dana Cole".into(),
            body: "Shared the housing initiative contact with the client.".into(),
            sent_at: "2026-02-20 14:30".into(),
        },
    ]
}
