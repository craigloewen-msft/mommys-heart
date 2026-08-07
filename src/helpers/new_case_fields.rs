//! The volunteer-only properties every new case is created with — the team's own
//! intake and outtake record, filled in on each case.
//!
//! Every entry here is volunteer-only: it is the team's internal record and is
//! never shown to the client the case is about. That is a fixed property of this
//! list, not a per-entry setting, which is why the type, the list, and the
//! accessors all say `volunteer_only` by name.
//!
//! This list lives in code on purpose. It is small, applies org-wide, and
//! changes by deploy rather than at runtime, so a constant is simpler to review
//! and safer to change than a config table plus the admin UI to edit it. Adding,
//! renaming, reordering, or re-sectioning an entry is a one-line edit here —
//! never a schema change.
//!
//! Each entry becomes an ordinary case property with no value in it yet, so
//! everything that already works on properties — editing, auditing — works on
//! them with no special cases. The *files* a case starts with are folders, not
//! named slots; they live in
//! [`new_case_folders`](crate::helpers::new_case_folders).

use crate::helpers::sections;

/// One volunteer-only property a new case starts with.
#[derive(Clone, Copy, Debug)]
pub struct VolunteerOnlyField {
    /// The display heading this is grouped under. Free text; empty means the
    /// catch-all group.
    pub section: &'static str,
    /// The property key shown to the user.
    pub label: &'static str,
}

/// The volunteer-only fields every new case is created with.
///
/// Every entry is [`Visibility::VolunteerOnly`](crate::helpers::visibility::Visibility::VolunteerOnly):
/// the team's own intake/outtake record, never shown to the client the case is
/// about. Visibility is not stored per entry because it never varies here.
///
/// Order here is the order the rows are created in, and therefore the order they
/// are displayed within their section.
pub const VOLUNTEER_ONLY_FIELDS: &[VolunteerOnlyField] = &[
    // ── Intake ───────────────────────────────────────────────────────────────
    VolunteerOnlyField {
        section: sections::INTAKE,
        label: "Referral source",
    },
    VolunteerOnlyField {
        section: sections::INTAKE,
        label: "Intake completed on",
    },
    VolunteerOnlyField {
        section: sections::INTAKE,
        label: "Intake volunteer",
    },
    // ── Outtake ──────────────────────────────────────────────────────────────
    VolunteerOnlyField {
        section: sections::OUTTAKE,
        label: "Outcome",
    },
    VolunteerOnlyField {
        section: sections::OUTTAKE,
        label: "Case closed on",
    },
];

/// The volunteer-only properties a new case starts with, in order.
pub fn volunteer_only_properties() -> impl Iterator<Item = &'static VolunteerOnlyField> {
    VOLUNTEER_ONLY_FIELDS.iter()
}
