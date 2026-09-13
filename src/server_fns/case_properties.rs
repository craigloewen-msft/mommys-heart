//! Case properties: the named key/value facts recorded on a case ("Attorney",
//! "Court", "Referral source").
//!
//! Its own module, and its own `case_properties` table, rather than a field of
//! the case: properties are a list with their own display order, their own
//! visibility, and their own edit operation, all of which are independent of the
//! case's name, status, and owner.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::helpers::visibility::Visibility;

/// A named key/value property on a case (e.g. attorney names, court, docket).
///
/// An empty `value` is meaningful: it is a property that has been named but not
/// filled in yet, which is how a new case can list the facts it is still waiting
/// on. An empty `key` is not — an unlabelled property has nothing to show — and
/// is dropped on save.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CaseProperty {
    pub key: String,
    pub value: String,
    /// Display grouping heading, e.g. "Intake". Free text; empty groups the row
    /// under [`sections::DEFAULT_LABEL`](crate::helpers::sections::DEFAULT_LABEL).
    #[serde(default)]
    pub section: String,
    /// Who may see this property.
    #[serde(default)]
    pub visibility: Visibility,
}

impl CaseProperty {
    /// A shared, unsectioned property — the shape every property had before
    /// visibility and sections existed.
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            section: String::new(),
            visibility: Visibility::Shared,
        }
    }
}

/// Replace the properties a case holds at one visibility (requires the
/// `EditCase` capability).
///
/// Scoped to a single visibility on purpose. The write deletes before it
/// reinserts, and a case holds shared and volunteer-only properties side by
/// side; replacing everything at once would let an edit made from the shared
/// list — which a client can make, and which never even loads the volunteer-only
/// properties — silently wipe the team's intake record.
#[server(prefix = "/api")]
pub async fn set_case_properties(
    case_id: String,
    visibility: Visibility,
    properties: Vec<CaseProperty>,
) -> Result<(), ServerFnError> {
    use crate::server::db::case_properties;
    use crate::server::permissions::{require_cap, require_user, require_visibility};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    require_cap(&user, &case_id, CaseCapability::AddNotes).await?;
    require_visibility(&user, visibility)?;
    case_properties::replace(&case_id, visibility, properties, &user.full_name())
        .await
        .map_err(ServerFnError::new)?;
    crate::server::notifications::notify_case(
        case_id,
        user.id.clone(),
        user.full_name(),
        crate::server_fns::settings::NotificationKind::CaseData,
        "updated the case properties".to_string(),
        crate::server::notifications::audience_for(visibility),
    );
    Ok(())
}

/// Save one volunteer-only intake questionnaire without replacing any other
/// case information.
#[server(prefix = "/api")]
pub async fn save_case_questionnaire(
    case_id: String,
    questionnaire_slug: String,
    answers: BTreeMap<String, String>,
) -> Result<(), ServerFnError> {
    use crate::helpers::case_questionnaires::{
        answer_properties, questionnaire, required_followups, validate_answers,
        GENERAL_QUESTIONNAIRE,
    };
    use crate::server::db::case_properties;
    use crate::server::permissions::{require_cap, require_user, require_visibility};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    require_cap(&user, &case_id, CaseCapability::EditCase).await?;
    require_visibility(&user, Visibility::VolunteerOnly)?;

    let questionnaire = questionnaire(&questionnaire_slug)
        .ok_or_else(|| ServerFnError::new("Questionnaire not found."))?;
    validate_answers(questionnaire, &answers).map_err(ServerFnError::new)?;

    if questionnaire.slug != GENERAL_QUESTIONNAIRE.slug {
        let properties = case_properties::get_case_properties(&case_id, true)
            .await
            .map_err(ServerFnError::new)?;
        let is_required = required_followups(&properties)
            .iter()
            .any(|required| required.questionnaire.slug == questionnaire.slug);
        if !is_required {
            return Err(ServerFnError::new(
                "This follow-up is not required by the current general intake.",
            ));
        }
    }

    case_properties::replace_section(
        &case_id,
        Visibility::VolunteerOnly,
        questionnaire.section,
        answer_properties(questionnaire, &answers),
        &user.full_name(),
    )
    .await
    .map_err(ServerFnError::new)?;
    crate::server::notifications::notify_case(
        case_id,
        user.id.clone(),
        user.full_name(),
        crate::server_fns::settings::NotificationKind::CaseData,
        format!("completed the {}", questionnaire.title.to_lowercase()),
        crate::server::notifications::audience_for(Visibility::VolunteerOnly),
    );
    Ok(())
}
