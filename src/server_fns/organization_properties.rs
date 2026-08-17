//! Organization properties: named key/value facts recorded on an organization.
//!
//! The shape mirrors contact properties: one ordered list, optionally grouped by
//! free-text section headings, with blank values allowed as placeholders.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::server_fns::contact_properties::{MAX_KEY_CHARS, MAX_PROPERTIES, MAX_VALUE_CHARS};

/// A named key/value fact about an organization.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OrganizationProperty {
    pub key: String,
    pub value: String,
    /// Display grouping heading. Empty groups the row under
    /// [`sections::DEFAULT_LABEL`](crate::helpers::sections::DEFAULT_LABEL).
    #[serde(default)]
    pub section: String,
}

/// The default blank properties every new organization starts with.
pub fn default_properties() -> Vec<OrganizationProperty> {
    crate::helpers::new_crm_fields::organization_properties()
        .map(|field| OrganizationProperty {
            key: field.label.to_string(),
            value: String::new(),
            section: field.section.to_string(),
        })
        .collect()
}

/// Drop the properties that cannot be stored and trim the rest.
pub fn clean(properties: Vec<OrganizationProperty>) -> Vec<OrganizationProperty> {
    properties
        .into_iter()
        .filter_map(|p| {
            let key = p.key.trim().to_string();
            if key.is_empty() {
                return None;
            }
            Some(OrganizationProperty {
                key,
                value: p.value.trim().to_string(),
                section: p.section.trim().to_string(),
            })
        })
        .collect()
}

/// Check a cleaned list against the stored limits.
pub fn validate(properties: &[OrganizationProperty]) -> Result<(), String> {
    if properties.len() > MAX_PROPERTIES {
        return Err(format!(
            "An organization may have at most {MAX_PROPERTIES} properties."
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
pub async fn list_organization_properties(
    organization_id: String,
) -> Result<Vec<OrganizationProperty>, ServerFnError> {
    use crate::server::db::organization_properties;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    crate::server::permissions::require_information_management_access(&user)?;
    organization_properties::list(&organization_id)
        .await
        .map_err(ServerFnError::new)
}

/// Replace an organization's whole property list.
#[server(prefix = "/api")]
pub async fn set_organization_properties(
    organization_id: String,
    properties: Vec<OrganizationProperty>,
) -> Result<(), ServerFnError> {
    use crate::server::db::organization_properties;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    crate::server::permissions::require_information_management_access(&user)?;
    let cleaned = clean(properties);
    validate(&cleaned).map_err(ServerFnError::new)?;
    organization_properties::replace(&organization_id, cleaned, &user.id, &user.full_name())
        .await
        .map_err(ServerFnError::new)
}

/// Append any missing code-owned defaults without disturbing existing rows.
#[server(prefix = "/api")]
pub async fn add_missing_organization_property_defaults(
    organization_id: String,
) -> Result<(), ServerFnError> {
    use crate::server::db::organization_properties;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    crate::server::permissions::require_information_management_access(&user)?;
    organization_properties::add_missing_defaults(&organization_id, &user.id, &user.full_name())
        .await
        .map_err(ServerFnError::new)
}
