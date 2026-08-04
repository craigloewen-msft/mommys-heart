//! The volunteer-only paperwork every new case is created with — the team's own
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
//! Each entry becomes an ordinary, *empty* row on the case: a case property with
//! no value yet, or a case file with no file in it yet. Because they are
//! ordinary rows, everything that already works on properties and files —
//! editing, uploading, deleting, auditing — works on them with no special cases.

use crate::helpers::sections;

pub const SIGNED_SERVICE_AGREEMENT_LABEL: &str = "Signed service agreement";

/// Which of a case's two kinds of information an entry becomes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldTarget {
    /// A case property, created with an empty value.
    Property,
    /// A case file, created with no file in it yet.
    File,
}

/// One volunteer-only field a new case starts with.
#[derive(Clone, Copy, Debug)]
pub struct VolunteerOnlyField {
    /// Whether this becomes a property or a file.
    pub target: FieldTarget,
    /// Display grouping heading, e.g. "Intake". Free text; empty means the
    /// catch-all group.
    pub section: &'static str,
    /// The property key / file name shown to the user.
    pub label: &'static str,
    /// Optional help text. Stored as the file's description; ignored for
    /// properties, which have no description column.
    pub description: &'static str,
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
        target: FieldTarget::File,
        section: sections::INTAKE,
        label: "Intake interview",
        description: "Recording or written summary of the first interview with the client.",
    },
    VolunteerOnlyField {
        target: FieldTarget::File,
        section: sections::INTAKE,
        label: "Intake letter",
        description: "The letter sent to the client when the case was opened.",
    },
    VolunteerOnlyField {
        target: FieldTarget::File,
        section: sections::INTAKE,
        label: SIGNED_SERVICE_AGREEMENT_LABEL,
        description: "The service agreement signed by the client.",
    },
    VolunteerOnlyField {
        target: FieldTarget::Property,
        section: sections::INTAKE,
        label: "Referral source",
        description: "How the client found us.",
    },
    VolunteerOnlyField {
        target: FieldTarget::Property,
        section: sections::INTAKE,
        label: "Intake completed on",
        description: "Date the intake was finished.",
    },
    VolunteerOnlyField {
        target: FieldTarget::Property,
        section: sections::INTAKE,
        label: "Intake volunteer",
        description: "Who ran the intake.",
    },
    // ── Outtake ──────────────────────────────────────────────────────────────
    VolunteerOnlyField {
        target: FieldTarget::File,
        section: sections::OUTTAKE,
        label: "Exit interview",
        description: "Recording or written summary of the closing interview.",
    },
    VolunteerOnlyField {
        target: FieldTarget::File,
        section: sections::OUTTAKE,
        label: "Case closure summary",
        description: "The signed summary handed over when the case is closed.",
    },
    VolunteerOnlyField {
        target: FieldTarget::Property,
        section: sections::OUTTAKE,
        label: "Outcome",
        description: "How the case ended.",
    },
    VolunteerOnlyField {
        target: FieldTarget::Property,
        section: sections::OUTTAKE,
        label: "Case closed on",
        description: "Date the case was closed out.",
    },
];

/// The volunteer-only properties a new case starts with, in order.
pub fn volunteer_only_properties() -> impl Iterator<Item = &'static VolunteerOnlyField> {
    VOLUNTEER_ONLY_FIELDS
        .iter()
        .filter(|f| f.target == FieldTarget::Property)
}

/// The volunteer-only files a new case starts with, in order.
pub fn volunteer_only_files() -> impl Iterator<Item = &'static VolunteerOnlyField> {
    VOLUNTEER_ONLY_FIELDS
        .iter()
        .filter(|f| f.target == FieldTarget::File)
}
