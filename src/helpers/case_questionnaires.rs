//! Volunteer-only case questionnaires and the simple rules that connect them.
//!
//! These definitions are intentionally code-backed for the first iteration.
//! They can later be replaced by administrator-managed templates without
//! changing how answers are stored on a case.

use std::collections::BTreeMap;

use crate::helpers::visibility::Visibility;
use crate::server_fns::case_properties::CaseProperty;

pub const COMPLETED_KEY: &str = "Questionnaire completed";
pub const COMPLETED_VALUE: &str = "Yes";

#[derive(Clone, Copy, Debug)]
pub enum QuestionnaireInput {
    Select(&'static [&'static str]),
    Text,
    TextArea,
}

#[derive(Clone, Copy, Debug)]
pub struct QuestionnaireQuestion {
    pub key: &'static str,
    pub label: &'static str,
    pub input: QuestionnaireInput,
    pub required: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct CaseQuestionnaire {
    pub slug: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub section: &'static str,
    pub questions: &'static [QuestionnaireQuestion],
}

#[derive(Clone, Copy, Debug)]
pub struct RequiredQuestionnaire {
    pub questionnaire: &'static CaseQuestionnaire,
    pub reason: &'static str,
}

pub const GENERAL_QUESTIONNAIRE: CaseQuestionnaire = CaseQuestionnaire {
    slug: "general-intake",
    title: "General intake questionnaire",
    description: "Start here so the case team knows which additional intake forms are needed.",
    section: "Questionnaire: General intake",
    questions: &[
        QuestionnaireQuestion {
            key: "Primary legal matter",
            label: "What is the primary legal matter?",
            input: QuestionnaireInput::Select(&["Divorce", "Custody", "Both", "Other"]),
            required: true,
        },
        QuestionnaireQuestion {
            key: "Children under 18 involved",
            label: "Are children under 18 involved?",
            input: QuestionnaireInput::Select(&["Yes", "No"]),
            required: true,
        },
        QuestionnaireQuestion {
            key: "Immediate safety concerns",
            label: "Are there immediate safety concerns?",
            input: QuestionnaireInput::Select(&["Yes", "No"]),
            required: true,
        },
        QuestionnaireQuestion {
            key: "Needs housing assistance",
            label: "Does the client need housing assistance?",
            input: QuestionnaireInput::Select(&["Yes", "No"]),
            required: true,
        },
        QuestionnaireQuestion {
            key: "Situation summary",
            label: "Briefly summarize the client's situation.",
            input: QuestionnaireInput::TextArea,
            required: true,
        },
    ],
};

pub const CHILD_CUSTODY_QUESTIONNAIRE: CaseQuestionnaire = CaseQuestionnaire {
    slug: "child-custody",
    title: "Child and custody intake",
    description: "Record the basic family and custody details needed by the case team.",
    section: "Questionnaire: Child and custody intake",
    questions: &[
        QuestionnaireQuestion {
            key: "Children and ages",
            label: "List the children and their ages.",
            input: QuestionnaireInput::TextArea,
            required: true,
        },
        QuestionnaireQuestion {
            key: "Current parenting arrangement",
            label: "What is the current parenting arrangement?",
            input: QuestionnaireInput::TextArea,
            required: true,
        },
        QuestionnaireQuestion {
            key: "Next family court date",
            label: "When is the next family court date, if known?",
            input: QuestionnaireInput::Text,
            required: false,
        },
    ],
};

pub const SAFETY_QUESTIONNAIRE: CaseQuestionnaire = CaseQuestionnaire {
    slug: "safety-planning",
    title: "Safety planning intake",
    description: "Capture the immediate contact and safety information needed for follow-up.",
    section: "Questionnaire: Safety planning intake",
    questions: &[
        QuestionnaireQuestion {
            key: "Safe to contact",
            label: "Is it safe to contact the client?",
            input: QuestionnaireInput::Select(&["Yes", "No", "Only at certain times"]),
            required: true,
        },
        QuestionnaireQuestion {
            key: "Safest contact method",
            label: "What is the safest contact method and time?",
            input: QuestionnaireInput::Text,
            required: true,
        },
        QuestionnaireQuestion {
            key: "Immediate safety needs",
            label: "Describe any immediate safety needs.",
            input: QuestionnaireInput::TextArea,
            required: true,
        },
    ],
};

pub const HOUSING_QUESTIONNAIRE: CaseQuestionnaire = CaseQuestionnaire {
    slug: "housing-support",
    title: "Housing support intake",
    description: "Collect a few details so the team can identify appropriate housing resources.",
    section: "Questionnaire: Housing support intake",
    questions: &[
        QuestionnaireQuestion {
            key: "Current living situation",
            label: "What is the client's current living situation?",
            input: QuestionnaireInput::TextArea,
            required: true,
        },
        QuestionnaireQuestion {
            key: "Housing deadline",
            label: "Is there a date by which housing is needed?",
            input: QuestionnaireInput::Text,
            required: false,
        },
        QuestionnaireQuestion {
            key: "Preferred area",
            label: "What borough or area is preferred?",
            input: QuestionnaireInput::Text,
            required: false,
        },
    ],
};

pub const QUESTIONNAIRES: &[&CaseQuestionnaire] = &[
    &GENERAL_QUESTIONNAIRE,
    &CHILD_CUSTODY_QUESTIONNAIRE,
    &SAFETY_QUESTIONNAIRE,
    &HOUSING_QUESTIONNAIRE,
];

pub fn questionnaire(slug: &str) -> Option<&'static CaseQuestionnaire> {
    QUESTIONNAIRES
        .iter()
        .copied()
        .find(|questionnaire| questionnaire.slug == slug)
}

pub fn is_questionnaire_section(section: &str) -> bool {
    QUESTIONNAIRES
        .iter()
        .any(|questionnaire| questionnaire.section == section)
}

pub fn answers_for(
    properties: &[CaseProperty],
    questionnaire: &CaseQuestionnaire,
) -> BTreeMap<String, String> {
    properties
        .iter()
        .filter(|property| {
            property.visibility == Visibility::VolunteerOnly
                && property.section == questionnaire.section
                && property.key != COMPLETED_KEY
        })
        .map(|property| (property.key.clone(), property.value.clone()))
        .collect()
}

pub fn is_complete(properties: &[CaseProperty], questionnaire: &CaseQuestionnaire) -> bool {
    properties.iter().any(|property| {
        property.visibility == Visibility::VolunteerOnly
            && property.section == questionnaire.section
            && property.key == COMPLETED_KEY
            && property.value == COMPLETED_VALUE
    })
}

pub fn required_followups(properties: &[CaseProperty]) -> Vec<RequiredQuestionnaire> {
    if !is_complete(properties, &GENERAL_QUESTIONNAIRE) {
        return Vec::new();
    }

    let answers = answers_for(properties, &GENERAL_QUESTIONNAIRE);
    followups_for_answers(&answers)
}

pub fn followups_for_answers(answers: &BTreeMap<String, String>) -> Vec<RequiredQuestionnaire> {
    let answer_is = |key: &str, expected: &[&str]| {
        answers
            .get(key)
            .is_some_and(|answer| expected.iter().any(|value| answer == value))
    };

    let mut required = Vec::new();
    if answer_is("Children under 18 involved", &["Yes"])
        || answer_is("Primary legal matter", &["Custody", "Both"])
    {
        required.push(RequiredQuestionnaire {
            questionnaire: &CHILD_CUSTODY_QUESTIONNAIRE,
            reason: "Required because children or custody are part of this case.",
        });
    }
    if answer_is("Immediate safety concerns", &["Yes"]) {
        required.push(RequiredQuestionnaire {
            questionnaire: &SAFETY_QUESTIONNAIRE,
            reason: "Required because an immediate safety concern was reported.",
        });
    }
    if answer_is("Needs housing assistance", &["Yes"]) {
        required.push(RequiredQuestionnaire {
            questionnaire: &HOUSING_QUESTIONNAIRE,
            reason: "Required because housing assistance was requested.",
        });
    }
    required
}

pub fn validate_answers(
    questionnaire: &CaseQuestionnaire,
    answers: &BTreeMap<String, String>,
) -> Result<(), String> {
    for question in questionnaire.questions {
        let answer = answers
            .get(question.key)
            .map(String::as_str)
            .unwrap_or_default()
            .trim();
        if question.required && answer.is_empty() {
            return Err(format!("Please answer \"{}\".", question.label));
        }
        if let QuestionnaireInput::Select(options) = question.input {
            if !answer.is_empty() && !options.contains(&answer) {
                return Err(format!("Choose a valid answer for \"{}\".", question.label));
            }
        }
    }
    Ok(())
}

pub fn answer_properties(
    questionnaire: &CaseQuestionnaire,
    answers: &BTreeMap<String, String>,
) -> Vec<CaseProperty> {
    let mut properties = questionnaire
        .questions
        .iter()
        .map(|question| CaseProperty {
            key: question.key.to_string(),
            value: answers
                .get(question.key)
                .map(|answer| answer.trim().to_string())
                .unwrap_or_default(),
            section: questionnaire.section.to_string(),
            visibility: Visibility::VolunteerOnly,
        })
        .collect::<Vec<_>>();
    properties.push(CaseProperty {
        key: COMPLETED_KEY.to_string(),
        value: COMPLETED_VALUE.to_string(),
        section: questionnaire.section.to_string(),
        visibility: Visibility::VolunteerOnly,
    });
    properties
}
