//! Local, deterministic demo data used to seed the database (see
//! [`crate::server::db::seed`]).
//!
//! Unlike a bulk data generator, this is a small, **hand-crafted** fixture: a
//! handful of users and exactly 8 targeted cases that are all interconnected —
//! every case has an owner plus several assigned users, and every user works a
//! few cases. That keeps the demo realistic and easy to reason about while still
//! exercising assignments, case visibility, chat threads, notes, and evidence.
//!
//! Fixtures use their own small, fixed ids (`u-1`, `c-1`, `m-1`, …). Records the
//! running app creates get their ids from the database sequence, which starts
//! well above these, so the two never collide.

use crate::helpers::visibility::Visibility;
use crate::server_fns::capabilities::{CaseAssignment, CasePreset};
use crate::server_fns::case_properties::CaseProperty;
use crate::server_fns::cases::{Case, CaseNote, CaseStatus};
use crate::server_fns::channels::ChannelKind;
use crate::server_fns::evidence::Evidence;
use crate::server_fns::message::Message;
use crate::server_fns::users::{AccountRole, User};

/// Organization display name.
pub const ORG_NAME: &str = "Mommy's Heart";

/// A demo user and the cases they are assigned to (with the preset that seeds
/// their capabilities on each). Case ids are given as their 1-based number, so
/// `1` means `c-1`. The first user of each role keeps a fixed, memorable
/// email/password so the login page's "Demo autofill" buttons work; the rest
/// share the same role password.
struct SeedUser {
    first: &'static str,
    last: &'static str,
    email: &'static str,
    password: &'static str,
    role: AccountRole,
    assignments: &'static [(u32, CasePreset)],
}

use AccountRole::{Client, OperationsAdmin, SiteAdmin, Volunteer};
use CasePreset::{Contributor, Manager, Viewer};

/// The nine demo users: one site admin, three volunteers, four clients, and one
/// operations admin. Their
/// assignments cross-link them to the cases below so no case or user is an
/// island.
const USERS: [SeedUser; 9] = [
    SeedUser {
        first: "Maria",
        last: "Nguyen",
        email: "admin@mommysheart.org",
        password: "admin123",
        role: SiteAdmin,
        assignments: &[(1, Manager), (3, Manager), (5, Viewer), (7, Viewer)],
    },
    SeedUser {
        first: "Dana",
        last: "Patel",
        email: "dana@mommysheart.org",
        password: "volunteer123",
        role: Volunteer,
        assignments: &[
            (2, Manager),
            (1, Contributor),
            (4, Contributor),
            (6, Contributor),
        ],
    },
    SeedUser {
        first: "James",
        last: "Garcia",
        email: "james.garcia@mommysheart.org",
        password: "volunteer123",
        role: Volunteer,
        assignments: &[
            (3, Contributor),
            (5, Contributor),
            (7, Contributor),
            (2, Viewer),
        ],
    },
    SeedUser {
        first: "Aisha",
        last: "Okafor",
        email: "aisha.okafor@mommysheart.org",
        password: "volunteer123",
        role: Volunteer,
        assignments: &[(6, Contributor), (8, Contributor), (1, Viewer), (4, Viewer)],
    },
    SeedUser {
        first: "Jamie",
        last: "Rivera",
        email: "jamie@example.com",
        password: "client123",
        role: Client,
        assignments: &[(1, Contributor), (6, Contributor), (2, Viewer)],
    },
    SeedUser {
        first: "Sofia",
        last: "Silva",
        email: "sofia.silva@example.com",
        password: "client123",
        role: Client,
        assignments: &[(3, Contributor), (7, Contributor), (8, Viewer)],
    },
    SeedUser {
        first: "Noah",
        last: "Kim",
        email: "noah.kim@example.com",
        password: "client123",
        role: Client,
        assignments: &[(4, Contributor), (8, Contributor), (5, Viewer)],
    },
    SeedUser {
        first: "Emma",
        last: "Johnson",
        email: "emma.johnson@example.com",
        password: "client123",
        role: Client,
        assignments: &[(5, Contributor), (3, Viewer)],
    },
    SeedUser {
        first: "Owen",
        last: "Brooks",
        email: "operations@mommysheart.org",
        password: "operations123",
        role: OperationsAdmin,
        assignments: &[(2, Manager), (4, Viewer)],
    },
];

/// The id of the `i`th demo user.
///
/// Real accounts get a cryptographically random id (see
/// [`crate::server::db::users::next_id`]); these mirror that *shape* so the
/// demo behaves like production, but are derived deterministically from the
/// index (SplitMix64) so re-seeding reproduces the same object graph and the
/// fixed cross-references below keep pointing at the right rows.
pub fn user_id(i: usize) -> String {
    let mut z = (i as u64)
        .wrapping_add(1)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    format!("u-{z:016x}")
}

fn case_id(n: u32) -> String {
    format!("c-{n}")
}

/// The users, with their per-case assignments expanded from presets.
///
/// Demo users are paired with their plaintext demo password. The password is
/// returned separately (not stored on [`User`]) so the seed can hash it before
/// inserting; the domain type never carries a password.
pub fn users() -> Vec<(User, String)> {
    USERS
        .iter()
        .enumerate()
        .map(|(i, su)| {
            let user = User {
                id: user_id(i),
                first_name: su.first.into(),
                last_name: su.last.into(),
                email: su.email.into(),
                phone: format!(
                    "(555) {:03}-{:04}",
                    (i * 13 + 7) % 1000,
                    (i * 97 + 11) % 10000
                ),
                home_address: format!("{} {} Street, Springfield", 100 + i * 7, su.last),
                role: su.role,
                assigned_cases: su
                    .assignments
                    .iter()
                    .map(|(case, preset)| CaseAssignment {
                        case_id: case_id(*case),
                        capabilities: preset.capabilities(),
                    })
                    .collect(),
            };
            (user, su.password.to_string())
        })
        .collect()
}

/// A note recorded against a case: `(author, body, created_at)`.
type SeedNote = (&'static str, &'static str, &'static str);
/// A metadata-only piece of evidence: `(name, uploaded_by, uploaded_at, description)`.
type SeedEvidence = (&'static str, &'static str, &'static str, &'static str);

/// One hand-crafted case with its owner, court metadata, notes, and evidence.
struct SeedCase {
    name: &'static str,
    status: CaseStatus,
    /// 1-based owner user number (`5` means `u-5`).
    owner: u32,
    docket: &'static str,
    notes: &'static [SeedNote],
    evidence: &'static [SeedEvidence],
}

use CaseStatus::{Closed, Monitor, Open};

/// The eight targeted cases. Each is owned by one user and (via the assignments
/// on [`USERS`]) worked by several others, so the case directory, assignment
/// screens, and chat threads all have interconnected data.
const CASES: [SeedCase; 8] = [
    SeedCase {
        name: "Nguyen custody matter",
        status: Open,
        owner: 5,
        docket: "FC-2026-0001",
        notes: &[(
            "Dana Patel",
            "Client prefers phone contact in the evenings.",
            "2026-02-04 09:00",
        )],
        evidence: &[(
            "Court summons",
            "Jamie Rivera",
            "2026-02-03 14:30",
            "Original summons served to the client.",
        )],
    },
    SeedCase {
        name: "Rivera housing assistance",
        status: Monitor,
        owner: 2,
        docket: "FC-2026-0002-A",
        notes: &[],
        evidence: &[],
    },
    SeedCase {
        name: "Silva benefits appeal",
        status: Closed,
        owner: 6,
        docket: "FC-2026-0003",
        notes: &[(
            "Maria Nguyen",
            "Case closed favorably; retain records for three years.",
            "2026-02-20 10:00",
        )],
        evidence: &[(
            "Benefits appeal packet",
            "James Garcia",
            "2026-01-12 08:40",
            "Complete filed appeal packet.",
        )],
    },
    SeedCase {
        name: "Kim guardianship petition",
        status: Open,
        owner: 7,
        docket: "FC-2026-0004",
        notes: &[],
        evidence: &[(
            "Signed guardianship petition",
            "Noah Kim",
            "2026-04-03 12:05",
            "Executed petition, all pages.",
        )],
    },
    SeedCase {
        name: "Johnson support modification",
        status: Monitor,
        owner: 8,
        docket: "FC-2026-0005",
        notes: &[],
        evidence: &[],
    },
    SeedCase {
        name: "Rivera protective order",
        status: Open,
        owner: 5,
        docket: "FC-2026-0006",
        notes: &[(
            "Aisha Okafor",
            "Safety plan reviewed with the client.",
            "2026-05-04 09:30",
        )],
        evidence: &[],
    },
    SeedCase {
        name: "Silva housing assistance",
        status: Open,
        owner: 6,
        docket: "FC-2026-0007",
        notes: &[],
        evidence: &[],
    },
    SeedCase {
        name: "Kim custody matter",
        status: Monitor,
        owner: 7,
        docket: "FC-2026-0008",
        notes: &[],
        evidence: &[],
    },
];

/// A chat message keyed to its case: `(case number, author display name, body,
/// sent_at)`. Authors are always users assigned to that case with the
/// `SendMessages` capability, and every thread mixes a volunteer/admin with the
/// client so conversations feel real.
const MESSAGES: &[(u32, &str, &str, &str)] = &[
    // c-1 Nguyen custody matter
    (
        1,
        "Dana Patel",
        "Hi Jamie, I've opened your custody case and I'll be your main contact.",
        "2026-02-03 09:15",
    ),
    (
        1,
        "Jamie Rivera",
        "Thank you, Dana. I uploaded the court summons this morning.",
        "2026-02-03 14:40",
    ),
    (
        1,
        "Dana Patel",
        "Got it — I see the summons. I'll draft our response this week.",
        "2026-02-05 10:05",
    ),
    (
        1,
        "Maria Nguyen",
        "Reviewed the file. Let's keep this Open for now.",
        "2026-02-09 16:20",
    ),
    // c-2 Rivera housing assistance
    (
        2,
        "Dana Patel",
        "Moving this housing matter to Monitor while we wait on the landlord.",
        "2026-03-01 11:00",
    ),
    (
        2,
        "Dana Patel",
        "Updated the docket after the clerk reassigned it.",
        "2026-03-06 13:30",
    ),
    // c-3 Silva benefits appeal
    (
        3,
        "James Garcia",
        "Sofia, your benefits appeal packet is complete and filed.",
        "2026-01-12 08:45",
    ),
    (
        3,
        "Sofia Silva",
        "Wonderful, thank you James!",
        "2026-01-12 15:10",
    ),
    (
        3,
        "Maria Nguyen",
        "Appeal granted — closing this case. Great work, everyone.",
        "2026-02-20 09:30",
    ),
    // c-4 Kim guardianship petition
    (
        4,
        "Dana Patel",
        "Noah, the guardianship petition is ready for your signature.",
        "2026-04-02 10:20",
    ),
    (
        4,
        "Noah Kim",
        "Signed and scanned back to you.",
        "2026-04-03 12:00",
    ),
    // c-5 Johnson support modification
    (
        5,
        "James Garcia",
        "Emma, I've requested the updated income records for the modification.",
        "2026-03-18 09:00",
    ),
    (
        5,
        "Emma Johnson",
        "I'll send my latest pay stubs today.",
        "2026-03-18 17:45",
    ),
    // c-6 Rivera protective order
    (
        6,
        "Aisha Okafor",
        "Protective order paperwork is drafted and ready to file.",
        "2026-05-04 10:15",
    ),
    (
        6,
        "Jamie Rivera",
        "Please file it as soon as possible.",
        "2026-05-04 10:40",
    ),
    (
        6,
        "Dana Patel",
        "Filed with the court this afternoon.",
        "2026-05-04 15:20",
    ),
    // c-7 Silva housing assistance
    (
        7,
        "James Garcia",
        "Sofia, the housing authority confirmed your application is under review.",
        "2026-05-11 11:30",
    ),
    (
        7,
        "Sofia Silva",
        "Thanks for the update.",
        "2026-05-12 09:05",
    ),
    // c-8 Kim custody matter
    (
        8,
        "Aisha Okafor",
        "Noah, I've scheduled the custody mediation for next month.",
        "2026-06-01 14:00",
    ),
    (
        8,
        "Noah Kim",
        "That works for me, thank you.",
        "2026-06-02 08:30",
    ),
];

/// Messages for each case's **volunteer-only** channel, in the same
/// `(case number, author display name, body, sent_at)` shape as [`MESSAGES`].
/// Authors here are only ever volunteers or admins — a client can neither read
/// nor post in this channel — so the fixture doubles as a check that the
/// restricted thread really is staff-only.
const VOLUNTEER_MESSAGES: &[(u32, &str, &str, &str)] = &[
    // c-1 Nguyen custody matter
    (
        1,
        "Dana Patel",
        "Heads up: Jamie is anxious about the hearing date. Let's keep updates gentle.",
        "2026-02-03 09:20",
    ),
    (
        1,
        "Maria Nguyen",
        "Agreed. I'll review the filing before we share anything with the client.",
        "2026-02-04 08:10",
    ),
    // c-3 Silva benefits appeal
    (
        3,
        "James Garcia",
        "Internal note: the first packet was rejected on a technicality; refiled today.",
        "2026-01-10 16:05",
    ),
    // c-6 Rivera protective order
    (
        6,
        "Aisha Okafor",
        "Safety planning call scheduled with the shelter coordinator.",
        "2026-05-03 09:45",
    ),
    (
        6,
        "Dana Patel",
        "Noted. I'll cover the filing so Aisha can stay on the safety plan.",
        "2026-05-03 10:30",
    ),
];

/// A seeded chat message together with which of its case's two default channels
/// it belongs to.
pub struct SeedMessage {
    pub message: Message,
    pub channel: ChannelKind,
}

fn prop(key: &str, value: &str) -> CaseProperty {
    CaseProperty::new(key, value)
}

/// The cases, each owned by one of the users and carrying its notes, evidence,
/// properties, and a resolved message count.
pub fn cases() -> Vec<Case> {
    let mut note_n = 0usize;
    let mut evidence_n = 0usize;
    CASES
        .iter()
        .enumerate()
        .map(|(i, sc)| {
            let n = (i as u32) + 1;
            let notes = sc
                .notes
                .iter()
                .map(|(author, body, created_at)| {
                    note_n += 1;
                    CaseNote {
                        id: format!("n-{note_n}"),
                        author: (*author).into(),
                        body: (*body).into(),
                        created_at: (*created_at).into(),
                    }
                })
                .collect();
            let evidence = sc
                .evidence
                .iter()
                .map(|(name, uploaded_by, uploaded_at, description)| {
                    evidence_n += 1;
                    Evidence {
                        id: format!("e-{evidence_n}"),
                        name: (*name).into(),
                        case_id: case_id(n),
                        uploaded_by: (*uploaded_by).into(),
                        uploaded_at: (*uploaded_at).into(),
                        description: (*description).into(),
                        original_filename: String::new(),
                        content_type: String::new(),
                        size_bytes: 0,
                        sha256: String::new(),
                        has_file: false,
                        section: String::new(),
                        visibility: Visibility::Shared,
                    }
                })
                .collect();
            Case {
                id: case_id(n),
                name: sc.name.into(),
                status: sc.status,
                owner_id: user_id((sc.owner - 1) as usize),
                notes,
                evidence,
                properties: vec![
                    prop("Court", "Springfield Family Court"),
                    prop("Docket", sc.docket),
                ],
                message_count: MESSAGES.iter().filter(|(case, ..)| *case == n).count(),
                capabilities: Vec::new(),
            }
        })
        .collect()
}

/// Resolve a display name (as used in [`MESSAGES`]) to the matching user id.
fn user_id_for_name(name: &str) -> String {
    USERS
        .iter()
        .position(|su| format!("{} {}", su.first, su.last) == name)
        .map(user_id)
        .unwrap_or_default()
}

/// The case chat messages, hand-authored per case so every thread is a coherent
/// conversation between the case's assigned users. Emitted in send order (the DB
/// preserves insertion order via its `seq` column), with each message tagged
/// with the channel it belongs to: the shared "General" channel or the private
/// volunteer-only one.
pub fn messages() -> Vec<SeedMessage> {
    MESSAGES
        .iter()
        .map(|m| (ChannelKind::Standard, m))
        .chain(
            VOLUNTEER_MESSAGES
                .iter()
                .map(|m| (ChannelKind::VolunteerOnly, m)),
        )
        .enumerate()
        .map(
            |(idx, (channel, (case, author, body, sent_at)))| SeedMessage {
                message: Message {
                    id: format!("m-{}", idx + 1),
                    case_id: case_id(*case),
                    // Resolved to the real channel row id by the seed, which is the
                    // only place the two default channels' ids exist.
                    channel_id: String::new(),
                    author_id: user_id_for_name(author),
                    author: (*author).into(),
                    body: (*body).into(),
                    sent_at: (*sent_at).into(),
                },
                channel,
            },
        )
        .collect()
}
