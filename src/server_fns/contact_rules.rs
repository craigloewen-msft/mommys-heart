//! Contact rules: the organization's own answer to "what should someone fill in
//! for a contact like this?".
//!
//! A rule pairs a trigger (a contact type, or a category) with what to offer
//! when it fires: labelled groups of categories with a selection limit, and
//! property fields to start blank. It is configuration, not schema \u2014 the
//! vendor taxonomy the foundation asked for is one rule's worth of rows, and a
//! second one needs no code.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// What makes a rule apply to a contact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum RuleTrigger {
    /// A [`crate::server_fns::contacts::ContactType`] slug.
    ContactType(String),
    /// A `contact_categories` id.
    Category(String),
}

impl RuleTrigger {
    /// The stored `(trigger_kind, trigger_value)` pair.
    pub fn as_parts(&self) -> (&'static str, String) {
        match self {
            Self::ContactType(slug) => ("contact_type", slug.clone()),
            Self::Category(id) => ("category", id.clone()),
        }
    }

    /// Rebuild from storage, defaulting to a category so an unknown kind cannot
    /// masquerade as a contact type.
    pub fn from_parts(kind: &str, value: &str) -> Self {
        match kind {
            "contact_type" => Self::ContactType(value.to_string()),
            _ => Self::Category(value.to_string()),
        }
    }
}

/// One category a group offers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RuleGroupOption {
    pub category_id: String,
    pub category_name: String,
    pub parent_name: String,
}

/// A labelled block of category choices with a selection limit.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RuleGroup {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub help_text: String,
    /// Advisory: a shortfall is reported, never blocked.
    pub min_choices: i32,
    /// `None` is unlimited; `Some(1)` is a single-select.
    pub max_choices: Option<i32>,
    pub options: Vec<RuleGroupOption>,
}

impl RuleGroup {
    pub fn is_single_select(&self) -> bool {
        self.max_choices == Some(1)
    }

    /// How the limit reads to someone filling the form in.
    pub fn limit_label(&self) -> String {
        match (self.min_choices, self.max_choices) {
            (min, Some(1)) if min >= 1 => "Choose one".to_string(),
            (_, Some(1)) => "Choose one at most".to_string(),
            (min, None) if min >= 1 => "Choose one or more".to_string(),
            (_, None) => "Choose any".to_string(),
            (_, Some(max)) => format!("Choose up to {max}"),
        }
    }
}

/// A property field a matching contact starts with.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RuleField {
    #[serde(default)]
    pub section: String,
    pub key: String,
    #[serde(default)]
    pub default_value: String,
    /// Offered as a datalist, so a value stays free text but is easy to reuse.
    #[serde(default)]
    pub suggestions: Vec<String>,
    /// Whether a value must be supplied when the contact is created. Adding a
    /// field to a rule normally means it should be answered, so this defaults
    /// to true; untick it for a field that is only a prompt.
    #[serde(default = "default_true")]
    pub required: bool,
}

fn default_true() -> bool {
    true
}

impl Default for RuleField {
    fn default() -> Self {
        Self {
            section: String::new(),
            key: String::new(),
            default_value: String::new(),
            suggestions: Vec::new(),
            required: true,
        }
    }
}

impl RuleField {
    /// Whether a stored property row is this field.
    pub fn matches(&self, section: &str, key: &str) -> bool {
        normalize_part(&self.section) == normalize_part(section)
            && normalize_part(&self.key) == normalize_part(key)
    }

    /// "Section / Name", for messages naming the field.
    pub fn label(&self) -> String {
        if self.section.trim().is_empty() {
            self.key.trim().to_string()
        } else {
            format!("{} / {}", self.section.trim(), self.key.trim())
        }
    }
}

/// Normalize a section or key the way stored rows are matched.
fn normalize_part(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContactRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub trigger: RuleTrigger,
    pub active: bool,
    pub groups: Vec<RuleGroup>,
    pub fields: Vec<RuleField>,
}

impl ContactRule {
    /// Every category id this rule offers, so the general picker can leave the
    /// grouped ones to the groups instead of listing them twice.
    pub fn grouped_category_ids(&self) -> Vec<String> {
        self.groups
            .iter()
            .flat_map(|group| group.options.iter().map(|o| o.category_id.clone()))
            .collect()
    }
}

/// The editable shape of a rule. The editor always submits the whole thing.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RuleInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub trigger: Option<RuleTrigger>,
    pub active: bool,
    #[serde(default)]
    pub groups: Vec<RuleGroupInput>,
    #[serde(default)]
    pub fields: Vec<RuleField>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RuleGroupInput {
    pub label: String,
    #[serde(default)]
    pub help_text: String,
    pub min_choices: i32,
    pub max_choices: Option<i32>,
    #[serde(default)]
    pub category_ids: Vec<String>,
}

/// What the editor needs to show for a selection: the groups to offer, the
/// categories they cover, and any advisory shortfall.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RuleGuidance {
    pub groups: Vec<RuleGroup>,
    /// Category ids covered by a group, to hide from the general picker.
    pub grouped_category_ids: Vec<String>,
    /// Required groups left unanswered, phrased for display.
    pub unmet: Vec<String>,
    /// The property fields these rules contribute, so a contact being created
    /// can collect them before it exists, and an existing one can mark them.
    #[serde(default)]
    pub fields: Vec<RuleField>,
}

#[cfg(feature = "ssr")]
const MAX_RULE_TEXT: usize = 300;

/// Clean and check a submitted rule.
#[cfg(feature = "ssr")]
fn validate_rule(mut input: RuleInput) -> Result<RuleInput, ServerFnError> {
    use crate::server_fns::contact_properties::{MAX_KEY_CHARS, MAX_VALUE_CHARS};

    input.name = input.name.trim().to_string();
    input.description = input.description.trim().to_string();
    if input.name.is_empty() || input.name.chars().count() > MAX_RULE_TEXT {
        return Err(ServerFnError::new("Enter a name for the rule."));
    }
    if input.description.chars().count() > MAX_RULE_TEXT {
        return Err(ServerFnError::new(
            "The rule description is too long.",
        ));
    }

    let trigger = input
        .trigger
        .clone()
        .ok_or_else(|| ServerFnError::new("Choose what makes this rule apply."))?;
    match &trigger {
        RuleTrigger::ContactType(slug) => {
            if crate::server_fns::contacts::ContactType::from_slug(slug).is_none() {
                return Err(ServerFnError::new("That contact type does not exist."));
            }
        }
        RuleTrigger::Category(id) => {
            if id.trim().is_empty() {
                return Err(ServerFnError::new("Choose a category for the trigger."));
            }
        }
    }
    input.trigger = Some(trigger);

    if input.groups.len() > 20 {
        return Err(ServerFnError::new("A rule may have at most 20 groups."));
    }
    for group in &mut input.groups {
        group.label = group.label.trim().to_string();
        group.help_text = group.help_text.trim().to_string();
        if group.label.is_empty() || group.label.chars().count() > MAX_RULE_TEXT {
            return Err(ServerFnError::new("Every group needs a label."));
        }
        if group.min_choices < 0 {
            group.min_choices = 0;
        }
        if let Some(max) = group.max_choices {
            if max < 1 {
                return Err(ServerFnError::new(
                    "A group's maximum must be at least 1, or blank for no limit.",
                ));
            }
            if group.min_choices > max {
                return Err(ServerFnError::new(format!(
                    "\u{201C}{}\u{201D} asks for at least {} choices but allows at most {max}.",
                    group.label, group.min_choices
                )));
            }
        }
        group.category_ids = group
            .category_ids
            .iter()
            .map(|id| id.trim().to_string())
            .filter(|id| !id.is_empty())
            .collect();
        group.category_ids.dedup();
        if group.category_ids.is_empty() {
            return Err(ServerFnError::new(format!(
                "\u{201C}{}\u{201D} offers no categories.",
                group.label
            )));
        }
    }

    if input.fields.len() > 20 {
        return Err(ServerFnError::new("A rule may add at most 20 fields."));
    }
    input.fields.retain(|field| !field.key.trim().is_empty());
    for field in &mut input.fields {
        field.section = field.section.trim().to_string();
        field.key = field.key.trim().to_string();
        field.default_value = field.default_value.trim().to_string();
        field.suggestions = field
            .suggestions
            .iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect();
        if field.key.chars().count() > MAX_KEY_CHARS {
            return Err(ServerFnError::new(format!(
                "A field name must be {MAX_KEY_CHARS} characters or fewer."
            )));
        }
        if field.default_value.chars().count() > MAX_VALUE_CHARS {
            return Err(ServerFnError::new(format!(
                "A field value must be {MAX_VALUE_CHARS} characters or fewer."
            )));
        }
    }

    Ok(input)
}

#[cfg(feature = "ssr")]
fn require_rule_editor(user: &crate::server_fns::users::User) -> Result<(), ServerFnError> {
    if user.role.has_operations_admin_permissions() {
        Ok(())
    } else {
        Err(ServerFnError::new(
            "Categories and rules are managed by administrators.",
        ))
    }
}

#[server(prefix = "/api")]
pub async fn list_contact_rules() -> Result<Vec<ContactRule>, ServerFnError> {
    use crate::server::db::contact_rules;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    crate::server::permissions::require_information_management_access(&user)?;
    contact_rules::list().await.map_err(ServerFnError::new)
}

/// The groups a contact with this selection should be offered.
#[server(prefix = "/api", input = leptos::server_fn::codec::Json)]
pub async fn contact_rule_guidance(
    type_slugs: Vec<String>,
    category_ids: Vec<String>,
) -> Result<RuleGuidance, ServerFnError> {
    use crate::server::db::contact_rules;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    crate::server::permissions::require_information_management_access(&user)?;
    let rules = contact_rules::matching(&type_slugs, &category_ids)
        .await
        .map_err(ServerFnError::new)?;

    let mut groups = Vec::new();
    let mut grouped_category_ids = Vec::new();
    let mut fields = Vec::new();
    for rule in &rules {
        grouped_category_ids.extend(rule.grouped_category_ids());
        groups.extend(rule.groups.iter().cloned());
        fields.extend(rule.fields.iter().cloned());
    }
    Ok(RuleGuidance {
        groups,
        grouped_category_ids,
        unmet: contact_rules::unmet(&rules, &category_ids),
        fields,
    })
}

#[server(prefix = "/api", input = leptos::server_fn::codec::Json)]
pub async fn save_contact_rule(
    rule_id: Option<String>,
    input: RuleInput,
) -> Result<Vec<ContactRule>, ServerFnError> {
    use crate::server::db::contact_rules;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    crate::server::permissions::require_information_management_access(&user)?;
    require_rule_editor(&user)?;
    let input = validate_rule(input)?;
    let rule_id = rule_id
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty());
    contact_rules::save(rule_id.as_deref(), &input)
        .await
        .map_err(|error| match error {
            sqlx::Error::RowNotFound => ServerFnError::new("That rule no longer exists."),
            other => ServerFnError::new(other.to_string()),
        })?;
    contact_rules::list().await.map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn delete_contact_rule(rule_id: String) -> Result<Vec<ContactRule>, ServerFnError> {
    use crate::server::db::contact_rules;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    crate::server::permissions::require_information_management_access(&user)?;
    require_rule_editor(&user)?;
    contact_rules::delete(rule_id.trim())
        .await
        .map_err(|error| match error {
            sqlx::Error::RowNotFound => ServerFnError::new("That rule no longer exists."),
            other => ServerFnError::new(other.to_string()),
        })?;
    contact_rules::list().await.map_err(ServerFnError::new)
}

/// Add the suggested property fields for the rules a contact already fires.
#[server(prefix = "/api")]
pub async fn apply_contact_rule_fields(contact_id: String) -> Result<bool, ServerFnError> {
    use crate::server::db::contact_rules;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    crate::server::permissions::require_information_management_access(&user)?;
    contact_rules::apply_fields_for_contact(contact_id.trim(), &user.id, &user.full_name())
        .await
        .map_err(ServerFnError::new)
}
