//! Organizations: the outside bodies the foundation deals with — funders,
//! partner agencies, service providers, courts, and employers.
//!
//! An organization is reference data shared across every case and contact, so it
//! lives at the top level rather than under a case. Contacts are filed under one
//! (see [`crate::server_fns::contacts`]), and grants are awarded by one.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::server_fns::crm::{coded_enum, MAX_LONG_TEXT, MAX_NAME, MAX_SHORT_TEXT};
use crate::server_fns::pagination::Page;

coded_enum!(OrganizationKind {
    Funder => ("funder", "Funder"),
    Partner => ("partner", "Partner agency"),
    ServiceProvider => ("service_provider", "Service provider"),
    Government => ("government", "Government agency"),
    Court => ("court", "Court"),
    Employer => ("employer", "Employer"),
    Other => ("other", "Other"),
});

impl OrganizationKind {
    pub fn badge_classes(self) -> &'static str {
        match self {
            Self::Funder => "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30",
            Self::Partner => "bg-sky-500/15 text-sky-300 ring-1 ring-sky-500/30",
            Self::ServiceProvider => {
                "bg-primary-500/15 text-primary-300 ring-1 ring-primary-500/30"
            }
            Self::Government | Self::Court => {
                "bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/30"
            }
            Self::Employer | Self::Other => "bg-slate-700/40 text-slate-300 ring-1 ring-slate-600",
        }
    }
}

impl Default for OrganizationKind {
    fn default() -> Self {
        Self::Partner
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Organization {
    pub id: String,
    pub name: String,
    pub kind: OrganizationKind,
    pub website: String,
    pub phone: String,
    pub email: String,
    pub address: String,
    pub description: String,
    pub archived: bool,
    /// How many contacts are filed under this organization.
    pub contact_count: i64,
}

/// The editable fields of an organization. Separate from [`Organization`] so the
/// server decides id, counts, and timestamps rather than trusting the browser.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OrganizationInput {
    pub name: String,
    pub kind: OrganizationKind,
    pub website: String,
    pub phone: String,
    pub email: String,
    pub address: String,
    pub description: String,
}

impl OrganizationInput {
    /// Trim every field and reject the ones that cannot be stored. Mirrors the
    /// database `CHECK`s so a bad value fails with a readable message instead of
    /// a constraint violation.
    pub fn validate(&self) -> Result<Self, String> {
        let cleaned = Self {
            name: self.name.trim().to_string(),
            kind: self.kind,
            website: self.website.trim().to_string(),
            phone: self.phone.trim().to_string(),
            email: self.email.trim().to_string(),
            address: self.address.trim().to_string(),
            description: self.description.trim().to_string(),
        };
        if cleaned.name.is_empty() {
            return Err("Enter the organization's name.".into());
        }
        for (label, value, max) in [
            ("name", &cleaned.name, MAX_NAME),
            ("website", &cleaned.website, MAX_SHORT_TEXT),
            ("phone", &cleaned.phone, MAX_SHORT_TEXT),
            ("email", &cleaned.email, MAX_SHORT_TEXT),
            ("address", &cleaned.address, MAX_SHORT_TEXT),
            ("description", &cleaned.description, MAX_LONG_TEXT),
        ] {
            if value.chars().count() > max {
                return Err(format!("The {label} must be {max} characters or fewer."));
            }
        }
        Ok(cleaned)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OrganizationFilters {
    pub keyword: String,
    pub kind: Option<OrganizationKind>,
    pub include_archived: bool,
}

/// The organization directory. Readable by any staff account so a volunteer can
/// file a contact under an organization; clients are refused.
#[server(prefix = "/api")]
pub async fn list_organizations(
    filters: OrganizationFilters,
    offset: i64,
    limit: i64,
) -> Result<Page<Organization>, ServerFnError> {
    use crate::server::db::organizations;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    crate::server_fns::crm::require_staff(&user)?;
    organizations::page(&filters, offset, limit)
        .await
        .map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn load_organization(id: String) -> Result<Option<Organization>, ServerFnError> {
    use crate::server::db::organizations;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    crate::server_fns::crm::require_staff(&user)?;
    organizations::get(&id).await.map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn create_organization(input: OrganizationInput) -> Result<String, ServerFnError> {
    use crate::server::db::organizations;
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    require_operations_admin(&user)?;
    let input = input.validate().map_err(ServerFnError::new)?;
    organizations::create(&input, &user.full_name())
        .await
        .map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn update_organization(
    id: String,
    input: OrganizationInput,
) -> Result<(), ServerFnError> {
    use crate::server::db::organizations;
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    require_operations_admin(&user)?;
    let input = input.validate().map_err(ServerFnError::new)?;
    organizations::update(&id, &input, &user.full_name())
        .await
        .map_err(ServerFnError::new)
}

/// Archive or restore an organization. Archiving hides it from pickers while
/// leaving every existing contact and grant reference intact.
#[server(prefix = "/api")]
pub async fn set_organization_archived(id: String, archived: bool) -> Result<(), ServerFnError> {
    use crate::server::db::organizations;
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    require_operations_admin(&user)?;
    organizations::set_archived(&id, archived, &user.full_name())
        .await
        .map_err(ServerFnError::new)
}
