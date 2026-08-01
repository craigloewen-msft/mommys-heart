//! The fields every new case is created with — the intake and outtake paperwork
//! the team expects to fill in on each case.
//!
//! This list lives in code on purpose. It is small, applies org-wide, and
//! changes by deploy rather than at runtime, so a constant is simpler to review
//! and safer to change than a config table plus the admin UI to edit it. Adding,
//! renaming, reordering, re-sectioning, or changing an entry's audience is a
//! one-line edit here — never a schema change.
//!
//! Each entry becomes an ordinary, *empty* row on the case: a case property with
//! no value yet, or a case file with no file in it yet. Because they are
//! ordinary rows, everything that already works on properties and files —
//! editing, uploading, deleting, auditing — works on them with no special cases.

use crate::helpers::visibility::Visibility;

/// Which of a case's two kinds of information an entry becomes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldTarget {
    /// A case property, created with an empty value.
    Property,
    /// A case file, created with no file in it yet.
    File,
}

/// One field a new case starts with.
#[derive(Clone, Copy, Debug)]
pub struct NewCaseField {
    /// Whether this becomes a property or a file.
    pub target: FieldTarget,
    /// Display grouping heading, e.g. "Intake". Free text; empty means the
    /// catch-all group.
    pub section: &'static str,
    /// Who may see the created row.
    pub visibility: Visibility,
    /// The property key / file name shown to the user.
    pub label: &'static str,
    /// Optional help text. Stored as the file's description; ignored for
    /// properties, which have no description column.
    pub description: &'static str,
}

/// The fields every new case is created with.
///
/// All of these are [`Visibility::VolunteerOnly`]: they are the team's own
/// intake/outtake record and are never shown to the client the case is about. An
/// entry can be made client-visible simply by changing its `visibility` — the
/// two ideas are independent, so a client-facing checklist needs no new code.
///
/// Order here is the order the rows are created in, and therefore the order they
/// are displayed within their section.
pub const NEW_CASE_FIELDS: &[NewCaseField] = &[
    // ── Intake ───────────────────────────────────────────────────────────────
    NewCaseField {
        target: FieldTarget::File,
        section: "Intake",
        visibility: Visibility::VolunteerOnly,
        label: "Intake interview",
        description: "Recording or written summary of the first interview with the client.",
    },
    NewCaseField {
        target: FieldTarget::File,
        section: "Intake",
        visibility: Visibility::VolunteerOnly,
        label: "Intake letter",
        description: "The letter sent to the client when the case was opened.",
    },
    NewCaseField {
        target: FieldTarget::File,
        section: "Intake",
        visibility: Visibility::VolunteerOnly,
        label: "Signed service agreement",
        description: "The service agreement signed by the client.",
    },
    NewCaseField {
        target: FieldTarget::Property,
        section: "Intake",
        visibility: Visibility::VolunteerOnly,
        label: "Referral source",
        description: "How the client found us.",
    },
    NewCaseField {
        target: FieldTarget::Property,
        section: "Intake",
        visibility: Visibility::VolunteerOnly,
        label: "Intake completed on",
        description: "Date the intake was finished.",
    },
    NewCaseField {
        target: FieldTarget::Property,
        section: "Intake",
        visibility: Visibility::VolunteerOnly,
        label: "Intake volunteer",
        description: "Who ran the intake.",
    },
    // ── Outtake ──────────────────────────────────────────────────────────────
    NewCaseField {
        target: FieldTarget::File,
        section: "Outtake",
        visibility: Visibility::VolunteerOnly,
        label: "Exit interview",
        description: "Recording or written summary of the closing interview.",
    },
    NewCaseField {
        target: FieldTarget::File,
        section: "Outtake",
        visibility: Visibility::VolunteerOnly,
        label: "Case closure summary",
        description: "The signed summary handed over when the case is closed.",
    },
    NewCaseField {
        target: FieldTarget::Property,
        section: "Outtake",
        visibility: Visibility::VolunteerOnly,
        label: "Outcome",
        description: "How the case ended.",
    },
    NewCaseField {
        target: FieldTarget::Property,
        section: "Outtake",
        visibility: Visibility::VolunteerOnly,
        label: "Case closed on",
        description: "Date the case was closed out.",
    },
];

/// The properties a new case starts with, in order.
pub fn properties() -> impl Iterator<Item = &'static NewCaseField> {
    NEW_CASE_FIELDS
        .iter()
        .filter(|f| f.target == FieldTarget::Property)
}

/// The files a new case starts with, in order.
pub fn files() -> impl Iterator<Item = &'static NewCaseField> {
    NEW_CASE_FIELDS
        .iter()
        .filter(|f| f.target == FieldTarget::File)
}
