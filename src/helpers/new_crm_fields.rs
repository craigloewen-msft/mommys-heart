//! The code-owned default CRM properties every new person or organization starts with.

/// The section used for communication preferences.
pub const COMMUNICATION: &str = "Communication";
/// The section used for relationship-tracking fields.
pub const RELATIONSHIP: &str = "Relationship";

/// One named blank property a new CRM record starts with.
#[derive(Clone, Copy, Debug)]
pub struct DefaultPropertyField {
    pub section: &'static str,
    pub label: &'static str,
}

/// The default fields every new person gets, in display order.
pub const PERSON_FIELDS: &[DefaultPropertyField] = &[
    DefaultPropertyField {
        section: COMMUNICATION,
        label: "Preferred contact method",
    },
    DefaultPropertyField {
        section: COMMUNICATION,
        label: "Best time to reach",
    },
    DefaultPropertyField {
        section: COMMUNICATION,
        label: "Preferred language",
    },
    DefaultPropertyField {
        section: RELATIONSHIP,
        label: "Relationship status",
    },
    DefaultPropertyField {
        section: RELATIONSHIP,
        label: "Last contacted on",
    },
    DefaultPropertyField {
        section: RELATIONSHIP,
        label: "Next follow-up on",
    },
];

/// The default fields every new organization gets, in display order.
pub const ORGANIZATION_FIELDS: &[DefaultPropertyField] = &[
    DefaultPropertyField {
        section: RELATIONSHIP,
        label: "Relationship status",
    },
    DefaultPropertyField {
        section: RELATIONSHIP,
        label: "Preferred contact method",
    },
    DefaultPropertyField {
        section: RELATIONSHIP,
        label: "Last contacted on",
    },
    DefaultPropertyField {
        section: RELATIONSHIP,
        label: "Next follow-up on",
    },
];

pub fn person_properties() -> impl Iterator<Item = &'static DefaultPropertyField> {
    PERSON_FIELDS.iter()
}

pub fn organization_properties() -> impl Iterator<Item = &'static DefaultPropertyField> {
    ORGANIZATION_FIELDS.iter()
}

/// Normalize a section or property label for idempotent matching.
pub fn normalize_property_part(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Normalize the section+label pair a stored row is matched by.
pub fn normalized_property_key(section: &str, label: &str) -> (String, String) {
    (
        normalize_property_part(section),
        normalize_property_part(label),
    )
}
