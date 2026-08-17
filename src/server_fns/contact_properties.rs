//! Contact properties: the named key/value facts recorded on a person, such as
//! "Preferred language", "Availability", or "Board term ends".
//!
//! The same shape as [`crate::server_fns::case_properties`] — an ordered list
//! grouped under a free-text section heading, rewritten in place — because it
//! answers the same question about a different subject.
//!
//! Deliberately without a visibility axis. On a case, shared vs volunteer-only
//! decides whether the *client* sees a row; clients cannot see contacts at all,
//! so the same words would mean something weaker here. Field-level sensitivity is
//! future work, not a reused column.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// How many properties one contact may carry.
pub const MAX_PROPERTIES: usize = 60;
pub const MAX_KEY_CHARS: usize = 80;
pub const MAX_VALUE_CHARS: usize = 500;

/// A named key/value fact about a person.
///
/// An empty `value` is meaningful: it is a field that has been named but not
/// filled in yet. An empty `key` is not, and is dropped on save.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ContactProperty {
    pub key: String,
    pub value: String,
    /// Display grouping heading. Empty groups the row under
    /// [`sections::DEFAULT_LABEL`](crate::helpers::sections::DEFAULT_LABEL).
    #[serde(default)]
    pub section: String,
}

/// The default blank properties every new person starts with.
pub fn default_properties() -> Vec<ContactProperty> {
    crate::helpers::new_crm_fields::person_properties()
        .map(|field| ContactProperty {
            key: field.label.to_string(),
            value: String::new(),
            section: field.section.to_string(),
        })
        .collect()
}

/// Drop the properties that cannot be stored and trim the rest.
pub fn clean(properties: Vec<ContactProperty>) -> Vec<ContactProperty> {
    properties
        .into_iter()
        .filter_map(|p| {
            let key = p.key.trim().to_string();
            if key.is_empty() {
                return None;
            }
            Some(ContactProperty {
                key,
                value: p.value.trim().to_string(),
                section: p.section.trim().to_string(),
            })
        })
        .collect()
}

/// Check a cleaned list against the stored limits.
pub fn validate(properties: &[ContactProperty]) -> Result<(), String> {
    if properties.len() > MAX_PROPERTIES {
        return Err(format!(
            "A contact may have at most {MAX_PROPERTIES} properties."
        ));
    }
    for property in properties {
        if property.key.chars().count() > MAX_KEY_CHARS {
            return Err(format!(
                "A property name must be {MAX_KEY_CHARS} characters or fewer."
            ));
        }
        if property.value.chars().count() > MAX_VALUE_CHARS {
            return Err(format!(
                "A property value must be {MAX_VALUE_CHARS} characters or fewer."
            ));
        }
    }
    Ok(())
}

#[server(prefix = "/api")]
pub async fn list_contact_properties(
    contact_id: String,
) -> Result<Vec<ContactProperty>, ServerFnError> {
    use crate::server::db::contact_properties;
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    crate::server::permissions::require_information_management_access(&user)?;
    require_operations_admin(&user)?;
    contact_properties::list(&contact_id)
        .await
        .map_err(ServerFnError::new)
}

/// Replace a contact's whole property list. Unlike case properties there is only
/// one list per contact, so no visibility scoping is needed to keep a second one
/// safe.
#[server(prefix = "/api")]
pub async fn set_contact_properties(
    contact_id: String,
    properties: Vec<ContactProperty>,
) -> Result<(), ServerFnError> {
    use crate::server::db::contact_properties;
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    crate::server::permissions::require_information_management_access(&user)?;
    require_operations_admin(&user)?;
    let cleaned = clean(properties);
    validate(&cleaned).map_err(ServerFnError::new)?;
    contact_properties::replace(&contact_id, cleaned, &user.id, &user.full_name())
        .await
        .map_err(ServerFnError::new)
}

/// Append any missing code-owned defaults without disturbing existing rows.
#[server(prefix = "/api")]
pub async fn add_missing_contact_property_defaults(
    contact_id: String,
) -> Result<(), ServerFnError> {
    use crate::server::db::contact_properties;
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    crate::server::permissions::require_information_management_access(&user)?;
    require_operations_admin(&user)?;
    contact_properties::add_missing_defaults(&contact_id, &user.id, &user.full_name())
        .await
        .map_err(ServerFnError::new)
}
