//! Contact rule persistence (SSR only): the tables behind "when a contact is
//! X, offer these grouped categories and these property fields".
//!
//! A rule stores nothing on a contact. It decides what the editor *offers* and
//! which blank properties get appended; what is saved is still ordinary
//! `contact_category_assignments` and `contact_properties` rows, so every
//! reader of those (search, filters, bulk edits, mail campaigns) is unaffected.

use std::collections::HashSet;

use crate::helpers::new_crm_fields;
use crate::server::db::{audit, ids, pool};
use crate::server_fns::contact_properties::ContactProperty;
use crate::server_fns::contact_rules::{
    ContactRule, RuleField, RuleGroup, RuleGroupOption, RuleInput, RuleTrigger,
};

#[derive(sqlx::FromRow)]
struct RuleRow {
    id: String,
    name: String,
    description: String,
    trigger_kind: String,
    trigger_value: String,
    active: bool,
}

#[derive(sqlx::FromRow)]
struct GroupRow {
    id: String,
    rule_id: String,
    label: String,
    help_text: String,
    min_choices: i32,
    max_choices: Option<i32>,
}

#[derive(sqlx::FromRow)]
struct OptionRow {
    group_id: String,
    category_id: String,
    category_name: String,
    parent_name: String,
}

#[derive(sqlx::FromRow)]
struct FieldRow {
    rule_id: String,
    section: String,
    key: String,
    default_value: String,
    suggestions: Vec<String>,
    required: bool,
}

/// Every rule with its groups, options and fields, in display order.
pub async fn list() -> Result<Vec<ContactRule>, sqlx::Error> {
    let rules = sqlx::query_as::<_, RuleRow>(
        "SELECT id, name, description, trigger_kind, trigger_value, active
         FROM contact_rules ORDER BY ord, name",
    )
    .fetch_all(pool())
    .await?;

    let groups = sqlx::query_as::<_, GroupRow>(
        "SELECT id, rule_id, label, help_text, min_choices, max_choices
         FROM contact_rule_groups ORDER BY rule_id, ord, label",
    )
    .fetch_all(pool())
    .await?;

    let options = sqlx::query_as::<_, OptionRow>(
        "SELECT o.group_id, o.category_id, c.name AS category_name,
                COALESCE(p.name, '') AS parent_name
         FROM contact_rule_group_options o
         JOIN contact_categories c ON c.id = o.category_id
         LEFT JOIN contact_categories p ON p.id = c.parent_id
         ORDER BY o.group_id, o.ord, c.name",
    )
    .fetch_all(pool())
    .await?;

    let fields = sqlx::query_as::<_, FieldRow>(
        "SELECT rule_id, section, key, default_value, suggestions, required
         FROM contact_rule_fields ORDER BY rule_id, ord, key",
    )
    .fetch_all(pool())
    .await?;

    Ok(rules
        .into_iter()
        .map(|rule| {
            let rule_groups = groups
                .iter()
                .filter(|group| group.rule_id == rule.id)
                .map(|group| RuleGroup {
                    id: group.id.clone(),
                    label: group.label.clone(),
                    help_text: group.help_text.clone(),
                    min_choices: group.min_choices,
                    max_choices: group.max_choices,
                    options: options
                        .iter()
                        .filter(|option| option.group_id == group.id)
                        .map(|option| RuleGroupOption {
                            category_id: option.category_id.clone(),
                            category_name: option.category_name.clone(),
                            parent_name: option.parent_name.clone(),
                        })
                        .collect(),
                })
                .collect();
            let rule_fields = fields
                .iter()
                .filter(|field| field.rule_id == rule.id)
                .map(|field| RuleField {
                    section: field.section.clone(),
                    key: field.key.clone(),
                    default_value: field.default_value.clone(),
                    suggestions: field.suggestions.clone(),
                    required: field.required,
                })
                .collect();
            ContactRule {
                id: rule.id,
                name: rule.name,
                description: rule.description,
                trigger: RuleTrigger::from_parts(&rule.trigger_kind, &rule.trigger_value),
                active: rule.active,
                groups: rule_groups,
                fields: rule_fields,
            }
        })
        .collect())
}

/// Replace one rule and all of its groups, options and fields. Creating and
/// editing are the same write: the editor always sends the whole rule, so a
/// removed group leaves no orphan behind.
pub async fn save(rule_id: Option<&str>, input: &RuleInput) -> Result<String, sqlx::Error> {
    let mut tx = pool().begin().await?;
    // Validated by the server fn before it gets here.
    let trigger = input
        .trigger
        .clone()
        .unwrap_or_else(|| RuleTrigger::Category(String::new()));
    let (kind, value) = trigger.as_parts();

    let id = match rule_id {
        Some(id) => {
            let result = sqlx::query(
                "UPDATE contact_rules
                 SET name = $2, description = $3, trigger_kind = $4, trigger_value = $5,
                     active = $6, updated_at = now()
                 WHERE id = $1",
            )
            .bind(id)
            .bind(&input.name)
            .bind(&input.description)
            .bind(kind)
            .bind(&value)
            .bind(input.active)
            .execute(&mut *tx)
            .await?;
            if result.rows_affected() == 0 {
                return Err(sqlx::Error::RowNotFound);
            }
            // Children are rewritten wholesale below; CASCADE clears them here.
            sqlx::query("DELETE FROM contact_rule_groups WHERE rule_id = $1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM contact_rule_fields WHERE rule_id = $1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            id.to_string()
        }
        None => {
            let id = ids::opaque("cr");
            sqlx::query(
                "INSERT INTO contact_rules
                     (id, name, description, trigger_kind, trigger_value, active, ord)
                 VALUES ($1, $2, $3, $4, $5, $6,
                         COALESCE((SELECT max(ord) + 1 FROM contact_rules), 0))",
            )
            .bind(&id)
            .bind(&input.name)
            .bind(&input.description)
            .bind(kind)
            .bind(&value)
            .bind(input.active)
            .execute(&mut *tx)
            .await?;
            id
        }
    };

    for (group_ord, group) in input.groups.iter().enumerate() {
        let group_id = ids::opaque("crg");
        sqlx::query(
            "INSERT INTO contact_rule_groups
                 (id, rule_id, label, help_text, min_choices, max_choices, ord)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&group_id)
        .bind(&id)
        .bind(&group.label)
        .bind(&group.help_text)
        .bind(group.min_choices)
        .bind(group.max_choices)
        .bind(group_ord as i32)
        .execute(&mut *tx)
        .await?;

        for (option_ord, category_id) in group.category_ids.iter().enumerate() {
            sqlx::query(
                "INSERT INTO contact_rule_group_options (group_id, category_id, ord)
                 VALUES ($1, $2, $3)
                 ON CONFLICT DO NOTHING",
            )
            .bind(&group_id)
            .bind(category_id)
            .bind(option_ord as i32)
            .execute(&mut *tx)
            .await?;
        }
    }

    for (field_ord, field) in input.fields.iter().enumerate() {
        sqlx::query(
            "INSERT INTO contact_rule_fields
                 (id, rule_id, section, key, default_value, suggestions, ord, required)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(ids::opaque("crf"))
        .bind(&id)
        .bind(&field.section)
        .bind(&field.key)
        .bind(&field.default_value)
        .bind(&field.suggestions)
        .bind(field_ord as i32)
        .bind(field.required)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(id)
}

pub async fn delete(rule_id: &str) -> Result<(), sqlx::Error> {
    let result = sqlx::query("DELETE FROM contact_rules WHERE id = $1")
        .bind(rule_id)
        .execute(pool())
        .await?;
    if result.rows_affected() == 0 {
        return Err(sqlx::Error::RowNotFound);
    }
    Ok(())
}

/// The active rules that a contact carrying these types and categories fires.
///
/// Evaluated from the selection in hand rather than from what is stored, so the
/// editor can show a rule's groups the moment its trigger is ticked.
pub async fn matching(
    type_slugs: &[String],
    category_ids: &[String],
) -> Result<Vec<ContactRule>, sqlx::Error> {
    Ok(list()
        .await?
        .into_iter()
        .filter(|rule| rule.active)
        .filter(|rule| match &rule.trigger {
            RuleTrigger::ContactType(slug) => type_slugs.iter().any(|value| value == slug),
            RuleTrigger::Category(id) => category_ids.iter().any(|value| value == id),
        })
        .collect())
}

/// Check a selection against the rules it fires, returning one message per
/// group that has too many choices.
///
/// `min_choices` is deliberately *not* enforced: an incomplete record must stay
/// editable, so a shortfall is reported to the reader as advice instead.
pub fn over_limit(rules: &[ContactRule], category_ids: &[String]) -> Vec<String> {
    let mut problems = Vec::new();
    for rule in rules {
        for group in &rule.groups {
            let Some(max) = group.max_choices else {
                continue;
            };
            let chosen = group
                .options
                .iter()
                .filter(|option| category_ids.iter().any(|id| id == &option.category_id))
                .count();
            if chosen as i32 > max {
                problems.push(if max == 1 {
                    format!(
                        "{} \u{2014} {}: choose one only ({chosen} selected).",
                        rule.name, group.label
                    )
                } else {
                    format!(
                        "{} \u{2014} {}: choose at most {max} ({chosen} selected).",
                        rule.name, group.label
                    )
                });
            }
        }
    }
    problems
}

/// Which required groups a selection has not answered, for the advisory note.
pub fn unmet(rules: &[ContactRule], category_ids: &[String]) -> Vec<String> {
    let mut missing = Vec::new();
    for rule in rules {
        for group in &rule.groups {
            if group.min_choices <= 0 {
                continue;
            }
            let chosen = group
                .options
                .iter()
                .filter(|option| category_ids.iter().any(|id| id == &option.category_id))
                .count();
            if (chosen as i32) < group.min_choices {
                missing.push(group.label.clone());
            }
        }
    }
    missing
}

/// Which required rule fields a submitted property list leaves blank.
///
/// Enforced where a contact is created, because that is where the form asks
/// for the values. An existing contact is not held to a field added after it
/// was filed: it is shown as required and unanswered instead, so classifying
/// someone can never become impossible.
pub fn missing_required(
    rules: &[crate::server_fns::contact_rules::ContactRule],
    properties: &[ContactProperty],
) -> Vec<String> {
    let mut missing = Vec::new();
    for rule in rules {
        for field in &rule.fields {
            if !field.required {
                continue;
            }
            let answered = properties.iter().any(|property| {
                field.matches(&property.section, &property.key)
                    && !property.value.trim().is_empty()
            });
            if !answered {
                missing.push(field.label());
            }
        }
    }
    missing
}

/// Append the fields of the given rules that the contact does not already
/// carry, matched on the normalized `(section, key)` pair every other default
/// is matched on. Returns whether anything was added.
pub async fn apply_fields_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    contact_id: &str,
    rules: &[ContactRule],
) -> Result<bool, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct Existing {
        ord: i32,
        key: String,
        section: String,
    }

    let existing = sqlx::query_as::<_, Existing>(
        "SELECT ord, key, section FROM contact_properties
         WHERE contact_id = $1 ORDER BY ord ASC",
    )
    .bind(contact_id)
    .fetch_all(&mut **tx)
    .await?;

    let mut present: HashSet<(String, String)> = existing
        .iter()
        .map(|row| new_crm_fields::normalized_property_key(&row.section, &row.key))
        .collect();
    let mut next_ord = existing.last().map(|row| row.ord + 1).unwrap_or(0);
    let mut changed = false;

    for rule in rules {
        for field in &rule.fields {
            let normalized = new_crm_fields::normalized_property_key(&field.section, &field.key);
            if !present.insert(normalized) {
                continue;
            }
            let property = ContactProperty {
                key: field.key.clone(),
                value: field.default_value.clone(),
                section: field.section.clone(),
            };
            sqlx::query(
                "INSERT INTO contact_properties (contact_id, ord, key, value, section)
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(contact_id)
            .bind(next_ord)
            .bind(&property.key)
            .bind(&property.value)
            .bind(&property.section)
            .execute(&mut **tx)
            .await?;
            next_ord += 1;
            changed = true;
        }
    }
    Ok(changed)
}

/// Apply the fields of every rule a contact currently fires, on demand.
pub async fn apply_fields_for_contact(
    contact_id: &str,
    actor_user_id: &str,
    actor: &str,
) -> Result<bool, sqlx::Error> {
    let type_slugs: Vec<String> = sqlx::query_scalar("SELECT unnest(types) FROM contacts WHERE id = $1")
        .bind(contact_id)
        .fetch_all(pool())
        .await?;
    let category_ids: Vec<String> = sqlx::query_scalar(
        "SELECT category_id FROM contact_category_assignments WHERE contact_id = $1",
    )
    .bind(contact_id)
    .fetch_all(pool())
    .await?;

    let rules = matching(&type_slugs, &category_ids).await?;
    if rules.is_empty() {
        return Ok(false);
    }

    let mut tx = pool().begin().await?;
    audit::set_actor_in_transaction(&mut tx, actor_user_id).await?;
    let changed = apply_fields_in_transaction(&mut tx, contact_id, &rules).await?;
    if changed {
        audit::record_in_transaction(
            &mut tx,
            audit::Entity::Contact,
            contact_id,
            actor,
            "properties",
            "",
            "added suggested fields",
        )
        .await?;
    }
    tx.commit().await?;
    Ok(changed)
}
