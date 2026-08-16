//! Shared types and authenticated operations for the outreach contact directory.
//!
//! The directory is the volunteer-facing view of the CRM `contacts` records: the
//! same people, presented as a searchable address book with a reusable category
//! taxonomy and a log of every communication. The person record itself stays
//! owned by [`crate::server_fns::contacts`].

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
const MAX_SHORT: usize = 300;
#[cfg(feature = "ssr")]
const MAX_COMMUNICATION: usize = 10_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContactCategory {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub parent_name: String,
}

impl ContactCategory {
    pub fn label(&self) -> String {
        if self.parent_name.is_empty() {
            self.name.clone()
        } else {
            format!("{} / {}", self.parent_name, self.name)
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ContactInput {
    pub full_name: String,
    pub title: String,
    pub organization: String,
    pub email: String,
    pub phone: String,
    pub address: String,
    pub website: String,
    pub category_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    pub full_name: String,
    pub title: String,
    pub organization: String,
    pub email: String,
    pub phone: String,
    pub address: String,
    pub website: String,
    pub categories: Vec<ContactCategory>,
    pub updated_at: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommunicationKind {
    Outreach,
    Conversation,
    Referral,
    FollowUp,
    Relationship,
    Note,
}

impl CommunicationKind {
    pub const ALL: [Self; 6] = [
        Self::Outreach,
        Self::Conversation,
        Self::Referral,
        Self::FollowUp,
        Self::Relationship,
        Self::Note,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            Self::Outreach => "outreach",
            Self::Conversation => "conversation",
            Self::Referral => "referral",
            Self::FollowUp => "follow_up",
            Self::Relationship => "relationship",
            Self::Note => "note",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Outreach => "Outreach attempt",
            Self::Conversation => "Conversation",
            Self::Referral => "Referral",
            Self::FollowUp => "Follow-up",
            Self::Relationship => "Relationship update",
            Self::Note => "General note",
        }
    }

    pub fn from_slug(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.slug() == value)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContactCommunication {
    pub id: String,
    pub kind: CommunicationKind,
    pub body: String,
    pub author_name: String,
    pub occurred_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContactDetails {
    pub contact: Contact,
    pub communications: Vec<ContactCommunication>,
}

#[cfg(feature = "ssr")]
fn require_directory_access(user: &crate::server_fns::users::User) -> Result<(), ServerFnError> {
    if user.role.has_volunteer_privileges() {
        Ok(())
    } else {
        Err(ServerFnError::new(
            "The contact directory is available to volunteers and administrators.",
        ))
    }
}

#[cfg(feature = "ssr")]
fn validate_input(mut input: ContactInput) -> Result<ContactInput, ServerFnError> {
    input.full_name = input.full_name.trim().to_string();
    input.title = input.title.trim().to_string();
    input.organization = input.organization.trim().to_string();
    input.email = input.email.trim().to_string();
    input.phone = input.phone.trim().to_string();
    input.address = input.address.trim().to_string();
    input.website = input.website.trim().to_string();
    input.category_ids = input
        .category_ids
        .into_iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect();
    input.category_ids.sort();
    input.category_ids.dedup();

    if input.full_name.is_empty() {
        return Err(ServerFnError::new("Enter the contact's full name."));
    }
    for (label, value) in [
        ("Full name", &input.full_name),
        ("Title", &input.title),
        ("Organization", &input.organization),
        ("Email", &input.email),
        ("Phone", &input.phone),
        ("Address", &input.address),
        ("Website", &input.website),
    ] {
        if value.chars().count() > MAX_SHORT {
            return Err(ServerFnError::new(format!(
                "{label} must be {MAX_SHORT} characters or fewer."
            )));
        }
    }
    if !input.email.is_empty() && !input.email.contains('@') {
        return Err(ServerFnError::new("Enter a valid email address."));
    }
    Ok(input)
}

#[server(prefix = "/api")]
pub async fn list_contact_categories() -> Result<Vec<ContactCategory>, ServerFnError> {
    use crate::server::db::contact_directory as directory;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    require_directory_access(&user)?;
    directory::list_categories()
        .await
        .map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn add_contact_category(
    name: String,
    parent_id: Option<String>,
) -> Result<Vec<ContactCategory>, ServerFnError> {
    use crate::server::db::contact_directory as directory;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    require_directory_access(&user)?;
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 100 {
        return Err(ServerFnError::new(
            "Category names must be between 1 and 100 characters.",
        ));
    }
    let parent_id = parent_id
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty());
    if let Some(parent_id) = parent_id.as_deref() {
        if !directory::category_is_root(parent_id)
            .await
            .map_err(ServerFnError::new)?
        {
            return Err(ServerFnError::new(
                "Subcategories can only be added beneath a top-level category.",
            ));
        }
    }
    directory::create_category(name, parent_id.as_deref())
        .await
        .map_err(|error| {
            if error
                .as_database_error()
                .is_some_and(|db| db.is_unique_violation())
            {
                ServerFnError::new("That category already exists.")
            } else {
                ServerFnError::new(error.to_string())
            }
        })?;
    directory::list_categories()
        .await
        .map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn search_contacts(
    query: String,
    category_ids: Vec<String>,
) -> Result<Vec<Contact>, ServerFnError> {
    use crate::server::db::contact_directory as directory;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    require_directory_access(&user)?;
    let mut category_ids: Vec<String> = category_ids
        .into_iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect();
    category_ids.sort();
    category_ids.dedup();
    directory::search(query.trim(), &category_ids)
        .await
        .map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn get_contact(contact_id: String) -> Result<ContactDetails, ServerFnError> {
    use crate::server::db::contact_directory as directory;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    require_directory_access(&user)?;
    directory::get(contact_id.trim())
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("Contact not found."))
}

#[server(prefix = "/api")]
pub async fn save_contact(
    contact_id: Option<String>,
    input: ContactInput,
) -> Result<ContactDetails, ServerFnError> {
    use crate::server::db::contact_directory as directory;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    require_directory_access(&user)?;
    let input = validate_input(input)?;
    let contact_id = contact_id
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty());
    let id = directory::save(contact_id.as_deref(), &input, &user.id, &user.full_name())
        .await
        .map_err(ServerFnError::new)?;
    directory::get(&id)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("Contact could not be loaded after saving."))
}

#[server(prefix = "/api")]
pub async fn add_contact_communication(
    contact_id: String,
    kind: CommunicationKind,
    body: String,
) -> Result<ContactDetails, ServerFnError> {
    use crate::server::db::contact_directory as directory;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    require_directory_access(&user)?;
    let contact_id = contact_id.trim();
    let body = body.trim();
    if body.is_empty() {
        return Err(ServerFnError::new("Enter a communication note."));
    }
    if body.chars().count() > MAX_COMMUNICATION {
        return Err(ServerFnError::new(
            "Communication notes must be 10,000 characters or fewer.",
        ));
    }
    directory::add_communication(contact_id, kind, body, &user.id, &user.full_name())
        .await
        .map_err(ServerFnError::new)?;
    directory::get(contact_id)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("Contact not found."))
}
