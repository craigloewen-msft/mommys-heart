//! Case contacts: who is involved in a case, and in what role.
//!
//! These links are readable through case access or the admin read path. Every
//! mutation remains anchored to the stored case's `EditCase` capability.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::server_fns::crm::coded_enum;

coded_enum!(CaseContactRole {
    Client => ("client", "Client"),
    HouseholdMember => ("household_member", "Household member"),
    EmergencyContact => ("emergency_contact", "Emergency contact"),
    Attorney => ("attorney", "Attorney"),
    OpposingParty => ("opposing_party", "Opposing party"),
    Caseworker => ("caseworker", "Caseworker"),
    ProviderContact => ("provider_contact", "Provider contact"),
    CourtProfessional => ("court_professional", "Court professional"),
    Other => ("other", "Other"),
});

impl CaseContactRole {
    pub fn badge_classes(self) -> &'static str {
        match self {
            Self::Client => "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30",
            Self::EmergencyContact => "bg-rose-500/15 text-rose-300 ring-1 ring-rose-500/30",
            Self::Attorney | Self::CourtProfessional => {
                "bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/30"
            }
            Self::OpposingParty => "bg-slate-700/40 text-slate-300 ring-1 ring-slate-600",
            _ => "bg-sky-500/15 text-sky-300 ring-1 ring-sky-500/30",
        }
    }
}

impl Default for CaseContactRole {
    fn default() -> Self {
        Self::Other
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CaseContact {
    pub id: String,
    pub case_id: String,
    pub contact_id: String,
    pub contact_name: String,
    pub organization_name: String,
    pub email: String,
    pub phone: String,
    pub role: CaseContactRole,
    pub note: String,
    pub is_primary: bool,
    pub contact_archived: bool,
}

/// One case link shown from a person's record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContactCaseLink {
    pub id: String,
    pub case_id: String,
    pub case_name: String,
    pub case_status: crate::server_fns::cases::CaseStatus,
    pub role: CaseContactRole,
    pub note: String,
    pub is_primary: bool,
    pub can_edit: bool,
}

/// One writable case returned by the person-side typeahead.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EditableCaseSummary {
    pub id: String,
    pub name: String,
    pub status: crate::server_fns::cases::CaseStatus,
}

#[cfg(feature = "ssr")]
fn clean_note(note: &str) -> Result<String, ServerFnError> {
    crate::server_fns::crm::clean_text(note, "note", crate::server_fns::crm::MAX_SHORT_TEXT)
        .map_err(ServerFnError::new)
}

#[cfg(feature = "ssr")]
fn relationship_error(error: sqlx::Error) -> ServerFnError {
    let message = error.to_string();
    if message.contains("case_contacts_unique") || message.contains("duplicate key") {
        ServerFnError::new("This person already has that role on this case.")
    } else if message.contains("archived contact") {
        ServerFnError::new("Restore this person before linking them to a case.")
    } else {
        ServerFnError::new(error)
    }
}

#[server(prefix = "/api")]
pub async fn list_case_contacts(case_id: String) -> Result<Vec<CaseContact>, ServerFnError> {
    use crate::server::db::case_contacts;
    use crate::server::permissions::{require_case_view_or_admin_read, require_user};

    let user = require_user().await?;
    crate::server::permissions::require_information_management_access(&user)?;
    crate::server_fns::crm::require_staff(&user)?;
    require_case_view_or_admin_read(&user, &case_id).await?;
    case_contacts::list(&case_id)
        .await
        .map_err(ServerFnError::new)
}

/// List every case-contact row for one person through the admin read path.
#[server(prefix = "/api")]
pub async fn list_contact_cases(contact_id: String) -> Result<Vec<ContactCaseLink>, ServerFnError> {
    use crate::server::db::case_contacts;
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    crate::server::permissions::require_information_management_access(&user)?;
    require_operations_admin(&user)?;
    case_contacts::list_for_contact(&contact_id, &user.id)
        .await
        .map_err(ServerFnError::new)
}

/// Search only cases on which the caller can edit case information.
#[server(prefix = "/api")]
pub async fn search_editable_cases(
    query: String,
) -> Result<Vec<EditableCaseSummary>, ServerFnError> {
    use crate::server::db::cases;
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    crate::server::permissions::require_information_management_access(&user)?;
    require_operations_admin(&user)?;
    cases::search_editable(&query, &user.id, 10)
        .await
        .map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn add_case_contact(
    case_id: String,
    contact_id: String,
    role: CaseContactRole,
    note: String,
    is_primary: bool,
) -> Result<(), ServerFnError> {
    use crate::server::db::case_contacts;
    use crate::server::permissions::{require_cap, require_user};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    crate::server::permissions::require_information_management_access(&user)?;
    crate::server_fns::crm::require_staff(&user)?;
    require_cap(&user, &case_id, CaseCapability::EditCase).await?;
    case_contacts::add(
        &case_id,
        contact_id.trim(),
        role,
        &clean_note(&note)?,
        is_primary,
        &user.id,
        &user.full_name(),
    )
    .await
    .map_err(relationship_error)
}

/// Update role, note, and primary state while authorizing against the stored case.
#[server(prefix = "/api")]
pub async fn update_case_contact(
    id: String,
    role: CaseContactRole,
    note: String,
    is_primary: bool,
) -> Result<(), ServerFnError> {
    use crate::server::db::case_contacts;
    use crate::server::permissions::{require_cap, require_user};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    crate::server::permissions::require_information_management_access(&user)?;
    crate::server_fns::crm::require_staff(&user)?;
    let case_id = case_contacts::case_id(&id)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("That case contact no longer exists."))?;
    require_cap(&user, &case_id, CaseCapability::EditCase).await?;
    case_contacts::update(
        &id,
        &case_id,
        role,
        &clean_note(&note)?,
        is_primary,
        &user.id,
        &user.full_name(),
    )
    .await
    .map_err(relationship_error)
}

#[server(prefix = "/api")]
pub async fn remove_case_contact(id: String) -> Result<(), ServerFnError> {
    use crate::server::db::case_contacts;
    use crate::server::permissions::{require_cap, require_user};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    crate::server::permissions::require_information_management_access(&user)?;
    crate::server_fns::crm::require_staff(&user)?;
    let case_id = case_contacts::case_id(&id)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("That case contact no longer exists."))?;
    require_cap(&user, &case_id, CaseCapability::EditCase).await?;
    case_contacts::remove(&id, &case_id, &user.id, &user.full_name())
        .await
        .map_err(ServerFnError::new)
}
