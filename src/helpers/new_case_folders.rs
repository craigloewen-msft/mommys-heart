//! The folder tree every new case starts with — the filing scheme a volunteer
//! opens a case expecting to see.
//!
//! Like [`new_case_fields`](crate::helpers::new_case_fields), this list lives in
//! code on purpose: it is small, applies org-wide, and changes by deploy rather
//! than at runtime. Adding, renaming, or reordering a folder is a one-line edit
//! here — never a schema change, because a case's folders are ordinary rows.
//!
//! Only the top-level folders name an audience. Everything inside one inherits
//! it, which is the same rule the database enforces: a file's folder is what
//! decides who can see it.

use crate::helpers::visibility::Visibility;

/// One folder a new case starts with, and the folders inside it.
pub struct NewCaseFolder {
    pub name: &'static str,
    /// Who may see what goes in here. The children inherit it.
    pub visibility: Visibility,
    pub children: &'static [&'static str],
}

/// The folders every new case is created with, in display order.
pub const NEW_CASE_FOLDERS: &[NewCaseFolder] = &[
    NewCaseFolder {
        name: "Case Notes",
        // Volunteer-only, because this is where a finalized Case Note's filed
        // document lands. Structured Case Notes are staff-only records (the
        // server refuses to show a client one at all), and audience is decided
        // by the top-level folder — so a shared folder here would hand the
        // client the team's working record.
        visibility: Visibility::VolunteerOnly,
        children: &[],
    },
    NewCaseFolder {
        name: "Closing",
        visibility: Visibility::VolunteerOnly,
        children: &[],
    },
    NewCaseFolder {
        name: "Intake",
        visibility: Visibility::VolunteerOnly,
        children: &[
            "Service Agreement",
            "Prospective Client Application",
            "Intake Summary",
            "Zoom Video",
            "Legal Memo",
        ],
    },
    NewCaseFolder {
        name: "Social Services",
        visibility: Visibility::Shared,
        children: &[],
    },
    NewCaseFolder {
        name: "Strategy Notes",
        visibility: Visibility::Shared,
        children: &[],
    },
    NewCaseFolder {
        name: "Client Emails and Text Messages",
        visibility: Visibility::Shared,
        children: &[],
    },
    NewCaseFolder {
        name: "Supporting Documents",
        visibility: Visibility::Shared,
        children: &[
            "Court Orders",
            "Petitions and Motions",
            "Orders of Protection",
            "Police Reports",
            "Other",
        ],
    },
];
