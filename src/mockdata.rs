//! Local, deterministic demo data used to seed the database (see
//! [`crate::server::db::seed`]).
//!
//! Everything here is generated in simple loops so there is realistic *volume*
//! to exercise pagination, search, and "Load more": ~100 users (an admin plus
//! 50 volunteers and 50 clients), 500 cases, and a few thousand chat messages
//! (unevenly spread, so some cases have long, paginated threads).
//!
//! Fixtures use their own small, fixed ids (`u-1`, `c-1`, `m-1`, …). Records the
//! running app creates get their ids from the database sequence, which starts
//! well above these, so the two never collide.

use crate::types::{
    AccountRole, Case, CaseAssignment, CasePreset, CaseProperty, CaseStatus, Grant, Message, User,
};

/// Organization display name.
pub const ORG_NAME: &str = "Mommy's Heart";

/// How much demo data to generate. Tune these to change the volume.
const VOLUNTEERS: usize = 50;
const CLIENTS: usize = 50;
const CASES: usize = 500;
/// Total users: one admin, then the volunteers, then the clients.
const USERS: usize = 1 + VOLUNTEERS + CLIENTS;

const FIRST_NAMES: [&str; 20] = [
    "Maria", "James", "Aisha", "Liam", "Sofia", "Noah", "Emma", "Lucas", "Olivia", "Ethan", "Ava",
    "Mason", "Isabella", "Logan", "Mia", "Elijah", "Amara", "Daniel", "Chloe", "Kai",
];
const LAST_NAMES: [&str; 20] = [
    "Johnson", "Garcia", "Patel", "Nguyen", "Kim", "Okafor", "Rossi", "Silva", "Haddad", "Ali",
    "Brown", "Martinez", "Cohen", "Wang", "Diallo", "Santos", "Ivanov", "Reyes", "Novak", "Khan",
];
const MATTERS: [&str; 6] = [
    "custody matter",
    "housing assistance",
    "benefits appeal",
    "guardianship petition",
    "support modification",
    "protective order",
];
const MSG_BODIES: [&str; 5] = [
    "Following up on the latest filing.",
    "Client confirmed the appointment for next week.",
    "Uploaded the requested documents to the case.",
    "Court date has been scheduled — details to follow.",
    "Thanks, I'll update the case notes accordingly.",
];

/// Deterministic (first, last) name for slot `i`, spread across the pools.
fn name(i: usize) -> (&'static str, &'static str) {
    (
        FIRST_NAMES[(i * 3) % FIRST_NAMES.len()],
        LAST_NAMES[(i * 7 + 3) % LAST_NAMES.len()],
    )
}

fn user_id(i: usize) -> String {
    format!("u-{}", i + 1)
}

fn case_id(i: usize) -> String {
    format!("c-{}", i + 1)
}

fn assign(case: usize, preset: CasePreset) -> CaseAssignment {
    CaseAssignment {
        case_id: case_id(case),
        capabilities: preset.capabilities(),
    }
}

fn prop(key: &str, value: &str) -> CaseProperty {
    CaseProperty {
        key: key.into(),
        value: value.into(),
    }
}

/// The users: user 0 is the admin, the next `VOLUNTEERS` are volunteers, the
/// rest are clients. The first user of each role keeps a fixed, memorable
/// email/password so the login page's "Demo autofill" buttons work.
pub fn users() -> Vec<User> {
    (0..USERS)
        .map(|i| {
            let (role, demo) = if i == 0 {
                (
                    AccountRole::Admin,
                    Some(("admin@mommysheart.org", "admin123")),
                )
            } else if i <= VOLUNTEERS {
                let demo = (i == 1).then_some(("dana@mommysheart.org", "volunteer123"));
                (AccountRole::Volunteer, demo)
            } else {
                let demo = (i == VOLUNTEERS + 1).then_some(("jamie@example.com", "client123"));
                (AccountRole::Client, demo)
            };

            let (first, last) = name(i);
            let (email, password) = match demo {
                Some((email, password)) => (email.to_string(), password.to_string()),
                None => (
                    format!(
                        "{}.{}{}@example.com",
                        first.to_lowercase(),
                        last.to_lowercase(),
                        i + 1
                    ),
                    if role == AccountRole::Volunteer {
                        "volunteer123".into()
                    } else {
                        "client123".into()
                    },
                ),
            };

            User {
                id: user_id(i),
                first_name: first.into(),
                last_name: last.into(),
                email,
                phone: format!("(555) {:03}-{:04}", (i * 13) % 1000, (i * 97) % 10000),
                home_address: format!("{} {} Street, Springfield", 100 + i, last),
                password,
                role,
                // Assign each user to a couple of cases (cycling presets) so
                // assignments and case visibility have data too.
                assigned_cases: vec![
                    assign(i % CASES, CasePreset::ALL[i % 3]),
                    assign((i + CASES / 2) % CASES, CasePreset::ALL[(i + 1) % 3]),
                ],
                audit_log: Vec::new(),
            }
        })
        .collect()
}

/// A handful of funding grants (an admin-only feature).
pub fn grants() -> Vec<Grant> {
    [
        "Family Stability Fund",
        "Legal Aid Access Grant",
        "Community Housing Initiative",
    ]
    .iter()
    .enumerate()
    .map(|(i, name)| Grant {
        id: format!("g-{}", i + 1),
        name: (*name).into(),
    })
    .collect()
}

/// The cases, each owned by one of the users.
pub fn cases() -> Vec<Case> {
    (0..CASES)
        .map(|i| {
            let owner = i % USERS;
            let (owner_first, owner_last) = name(owner);
            Case {
                id: case_id(i),
                name: format!(
                    "{}-{} {}",
                    owner_first,
                    owner_last,
                    MATTERS[i % MATTERS.len()]
                ),
                status: CaseStatus::ALL[i % CaseStatus::ALL.len()],
                owner_id: user_id(owner),
                notes: Vec::new(),
                evidence: Vec::new(),
                properties: vec![
                    prop("Court", "Springfield Family Court"),
                    prop("Docket", &format!("FC-2026-{:04}", i + 1)),
                ],
                audit_log: Vec::new(),
                message_count: 0,
            }
        })
        .collect()
}

/// A tiny deterministic hash (the SplitMix64 finalizer) so the demo data looks
/// random but stays identical across runs.
fn hash64(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// How many chat messages a given case gets. Most cases have only a handful, but
/// a random ~1-in-25 are "busy" with 50–100 messages so there is plenty to page
/// through when testing message pagination.
fn message_count(case: usize) -> usize {
    let h = hash64(case as u64);
    if h.is_multiple_of(3) {
        50 + (hash64(h) % 51) as usize // 50..=100
    } else {
        (h % 5) as usize // 0..=4
    }
}

/// The chat messages, authored by each case's owner. The per-case volume is
/// deliberately uneven (see [`message_count`]) so some threads are long enough
/// to exercise pagination. Messages are emitted in send order per case (the DB
/// preserves insertion order via its `seq` column).
pub fn messages() -> Vec<Message> {
    let mut out = Vec::new();
    let mut n = 0usize;
    for ci in 0..CASES {
        let owner = ci % USERS;
        let (first, last) = name(owner);
        let author = format!("{first} {last}");
        let author_id = user_id(owner);
        let cid = case_id(ci);
        for k in 0..message_count(ci) {
            let seed = hash64(((ci as u64) << 20) ^ k as u64);
            out.push(Message {
                id: format!("m-{}", n + 1),
                case_id: cid.clone(),
                author_id: author_id.clone(),
                author: author.clone(),
                body: MSG_BODIES[(seed as usize) % MSG_BODIES.len()].into(),
                sent_at: format!(
                    "2026-{:02}-{:02} {:02}:{:02}",
                    1 + (k % 12),
                    1 + (k % 27),
                    seed % 24,
                    hash64(seed) % 60
                ),
            });
            n += 1;
        }
    }
    out
}
