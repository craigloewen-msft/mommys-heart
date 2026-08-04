use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::helpers::sections;
use crate::helpers::visibility::Visibility;
use crate::server_fns::case_properties::CaseProperty;

/// How a single scalar intake answer is entered on the form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntakeInput {
    /// A dropdown limited to the listed answers.
    Select(&'static [&'static str]),
    /// A free-text line.
    Text,
    /// A free-text line that hints a decimal amount (an hourly rate).
    Rate,
}

/// One entry of the intake questionnaire, in the order it is shown on the form
/// and stored on the case.
///
/// [`INTAKE_ITEMS`] is the single source of truth for the questionnaire: the
/// signup form renders its inputs from it and a new case's intake properties are
/// produced from it. Adding, renaming, or reordering a question is a one-line
/// edit here that both the form and the stored properties pick up.
#[derive(Clone, Copy, Debug)]
pub enum IntakeItem {
    /// A single scalar question. `key` is the stable property name the answer is
    /// stored under; `label` is what the form shows the person filling it in,
    /// which may carry hints or punctuation the stored key should not.
    Field {
        key: &'static str,
        label: &'static str,
        input: IntakeInput,
    },
    /// The repeatable list of judges. `heading` labels the form section; each
    /// judge is stored under its own numbered key ("Judge 1", ...).
    Judges { heading: &'static str },
    /// The repeatable list of courts and their docket numbers. `heading` labels
    /// the form section; each is stored under numbered keys ("Court 1", ...).
    Courts { heading: &'static str },
}

/// The intake questionnaire, in display and storage order.
pub const INTAKE_ITEMS: &[IntakeItem] = &[
    IntakeItem::Field {
        key: "Is the case in litigation?",
        label: "Is the case in litigation?",
        input: IntakeInput::Select(&["Yes", "No"]),
    },
    IntakeItem::Field {
        key: "What is the action sought?",
        label: "What is the action sought? (divorce, custody, both, other)",
        input: IntakeInput::Select(&["Divorce", "Custody", "Both", "Other"]),
    },
    IntakeItem::Field {
        key: "Client's attorney",
        label: "Client's attorney (if any):",
        input: IntakeInput::Text,
    },
    IntakeItem::Field {
        key: "Hourly rate of client's attorney",
        label: "Hourly rate of client's attorney:",
        input: IntakeInput::Rate,
    },
    IntakeItem::Field {
        key: "Opponent's attorney",
        label: "Opponent's attorney:",
        input: IntakeInput::Text,
    },
    IntakeItem::Field {
        key: "Hourly rate of opponent's attorney",
        label: "Hourly rate of opponent's attorney:",
        input: IntakeInput::Rate,
    },
    IntakeItem::Field {
        key: "Occupation of client",
        label: "Occupation of client:",
        input: IntakeInput::Text,
    },
    IntakeItem::Field {
        key: "Occupation of opponent",
        label: "Occupation of opponent:",
        input: IntakeInput::Text,
    },
    IntakeItem::Judges {
        heading: "Judges",
    },
    IntakeItem::Courts {
        heading: "Courts and docket numbers",
    },
    IntakeItem::Field {
        key: "Attorney for the Child",
        label: "Attorney for the Child:",
        input: IntakeInput::Text,
    },
    IntakeItem::Field {
        key: "Hourly rate of Attorney for the Child",
        label: "Hourly rate of Attorney for the Child (if any):",
        input: IntakeInput::Rate,
    },
];

/// The scalar-question storage keys, in order — one per [`IntakeItem::Field`].
pub fn field_keys() -> impl Iterator<Item = &'static str> {
    INTAKE_ITEMS.iter().filter_map(|item| match item {
        IntakeItem::Field { key, .. } => Some(*key),
        IntakeItem::Judges { .. } | IntakeItem::Courts { .. } => None,
    })
}

pub fn judge_label(index: usize) -> String {
    format!("Judge {}", index + 1)
}

pub fn court_label(index: usize) -> String {
    format!("Court {}", index + 1)
}

pub fn docket_number_label(index: usize) -> String {
    format!("Docket number {}", index + 1)
}

/// The stable property key the client's optional free-text notes are stored under.
pub const EXTRA_NOTES_KEY: &str = "Additional notes";

/// The form label shown above the optional free-text notes field.
pub const EXTRA_NOTES_LABEL: &str = "Is there anything else you'd like to share? (optional)";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CourtDocket {
    pub court: String,
    pub docket_number: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CaseIntake {
    /// Scalar answers keyed by their [`IntakeItem::Field`] key.
    pub fields: BTreeMap<String, String>,
    pub judges: Vec<String>,
    pub courts: Vec<CourtDocket>,
    /// Optional free-text notes the client may add; stored under [`EXTRA_NOTES_KEY`].
    #[serde(default)]
    pub extra_notes: String,
}

impl CaseIntake {
    /// The case properties this intake becomes, in [`INTAKE_ITEMS`] order.
    pub fn properties(&self) -> Vec<CaseProperty> {
        let property = |key: &str, value: &str| CaseProperty {
            key: key.to_string(),
            value: value.trim().to_string(),
            section: sections::INTAKE.to_string(),
            visibility: Visibility::Shared,
        };
        let mut properties = Vec::new();
        for item in INTAKE_ITEMS {
            match item {
                IntakeItem::Field { key, .. } => {
                    let value = self.fields.get(*key).map(String::as_str).unwrap_or_default();
                    properties.push(property(key, value));
                }
                IntakeItem::Judges { .. } => {
                    for (index, judge) in self.judges.iter().enumerate() {
                        properties.push(property(&judge_label(index), judge));
                    }
                }
                IntakeItem::Courts { .. } => {
                    for (index, entry) in self.courts.iter().enumerate() {
                        properties.push(property(&court_label(index), &entry.court));
                        properties
                            .push(property(&docket_number_label(index), &entry.docket_number));
                    }
                }
            }
        }
        properties.push(property(EXTRA_NOTES_KEY, &self.extra_notes));
        properties
    }
}
