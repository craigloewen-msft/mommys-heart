use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::helpers::dates;
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
    /// A date, or an explicit "never": the event may genuinely never have
    /// happened, and that is a different answer from leaving the question blank.
    DateOrNever,
}

/// The stored answer meaning the event has never happened. Written as the
/// literal word rather than an empty value, which would be indistinguishable
/// from an unanswered question in the case property list.
pub const NEVER_ANSWER: &str = "Never";

/// Whether a raw [`IntakeInput::DateOrNever`] answer is the "never" sentinel.
pub fn is_never(raw: &str) -> bool {
    raw.trim().eq_ignore_ascii_case(NEVER_ANSWER)
}

/// The earliest date accepted for a "most recent ..." answer. Anything before
/// this is a typo rather than an event.
const EARLIEST_YEAR: i32 = 1900;

/// Whether an intake field must have an answer before a case can be created.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntakeRequirement {
    Required,
    Optional,
}

impl IntakeRequirement {
    pub const fn is_required(self) -> bool {
        matches!(self, Self::Required)
    }
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
        requirement: IntakeRequirement,
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
        requirement: IntakeRequirement::Required,
    },
    IntakeItem::Field {
        key: "What is the action sought?",
        label: "What is the action sought? (divorce, custody, both, other)",
        input: IntakeInput::Select(&["Divorce", "Custody", "Both", "Other"]),
        requirement: IntakeRequirement::Required,
    },
    IntakeItem::Field {
        key: "Client's attorney",
        label: "Client's attorney:",
        input: IntakeInput::Text,
        requirement: IntakeRequirement::Optional,
    },
    IntakeItem::Field {
        key: "Hourly rate of client's attorney",
        label: "Hourly rate of client's attorney:",
        input: IntakeInput::Rate,
        requirement: IntakeRequirement::Optional,
    },
    IntakeItem::Field {
        key: "Opponent's attorney",
        label: "Opponent's attorney:",
        input: IntakeInput::Text,
        requirement: IntakeRequirement::Optional,
    },
    IntakeItem::Field {
        key: "Hourly rate of opponent's attorney",
        label: "Hourly rate of opponent's attorney:",
        input: IntakeInput::Rate,
        requirement: IntakeRequirement::Optional,
    },
    IntakeItem::Field {
        key: "Occupation of client",
        label: "Occupation of client:",
        input: IntakeInput::Text,
        requirement: IntakeRequirement::Required,
    },
    IntakeItem::Field {
        key: "Occupation of opponent",
        label: "Occupation of opponent:",
        input: IntakeInput::Text,
        requirement: IntakeRequirement::Optional,
    },
    IntakeItem::Judges { heading: "Judges" },
    IntakeItem::Courts {
        heading: "Courts and docket numbers",
    },
    IntakeItem::Field {
        key: "Attorney for the Child",
        label: "Attorney for the Child:",
        input: IntakeInput::Text,
        requirement: IntakeRequirement::Optional,
    },
    IntakeItem::Field {
        key: "Hourly rate of Attorney for the Child",
        label: "Hourly rate of Attorney for the Child:",
        input: IntakeInput::Rate,
        requirement: IntakeRequirement::Optional,
    },
    IntakeItem::Field {
        key: "Date of the most recent domestic-abuse incident",
        label: "Date of the most recent domestic-abuse incident:",
        input: IntakeInput::DateOrNever,
        requirement: IntakeRequirement::Required,
    },
    IntakeItem::Field {
        key: "Date of the client's most recent therapy appointment",
        label: "Date of the client's most recent therapy appointment:",
        input: IntakeInput::DateOrNever,
        requirement: IntakeRequirement::Required,
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
    pub fn validate(&self) -> Result<(), String> {
        for item in INTAKE_ITEMS {
            let IntakeItem::Field {
                key,
                input,
                requirement,
                ..
            } = item
            else {
                continue;
            };
            let raw = self.fields.get(*key).map(String::as_str).unwrap_or_default();
            if matches!(input, IntakeInput::DateOrNever) {
                validate_date_or_never(key, raw, *requirement)?;
                continue;
            }
            if requirement.is_required() && raw.trim().is_empty() {
                return Err(format!("Please complete the required field \"{key}\"."));
            }
        }
        Ok(())
    }

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
                IntakeItem::Field { key, input, .. } => {
                    let raw = self.fields.get(*key).map(String::as_str).unwrap_or_default();
                    let value = match input {
                        IntakeInput::DateOrNever => stored_date_or_never(raw),
                        _ => raw.trim().to_string(),
                    };
                    properties.push(property(key, &value));
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

/// A [`IntakeInput::DateOrNever`] answer must be either the "never" sentinel or
/// a real, non-future date. Blank fails only when the question is required.
fn validate_date_or_never(
    key: &str,
    raw: &str,
    requirement: IntakeRequirement,
) -> Result<(), String> {
    let raw = raw.trim();
    if raw.is_empty() {
        if requirement.is_required() {
            return Err(format!(
                "Please answer \"{key}\" with a date, or tick \"Never\"."
            ));
        }
        return Ok(());
    }
    if is_never(raw) {
        return Ok(());
    }
    let malformed = || format!("Enter \"{key}\" as a valid date.");
    let (year, _, _) = dates::parse_iso(raw).ok_or_else(malformed)?;
    if year < EARLIEST_YEAR {
        return Err(malformed());
    }
    // These questions ask for a most recent past event, so a later date is
    // always a mistake. Today itself is a legitimate answer.
    if raw > dates::today().as_str() {
        return Err(format!("\"{key}\" cannot be in the future."));
    }
    Ok(())
}

/// A [`IntakeInput::DateOrNever`] answer as it is stored: the canonical "never"
/// spelling, or the date in the `MM-DD-YYYY` form the case view displays.
fn stored_date_or_never(raw: &str) -> String {
    let raw = raw.trim();
    if is_never(raw) {
        NEVER_ANSWER.to_string()
    } else {
        dates::to_us(raw)
    }
}
