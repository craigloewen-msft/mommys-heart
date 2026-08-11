//! Case contacts: who is involved in a case, and in what role.
//!
//! The bridge between the CRM and case work. Unlike the rest of the CRM these
//! rows are per-case, so they use the existing per-case capability gate: reading
//! needs `ViewCase`, editing needs `EditCase`. Clients never see them, and their
//! audit entries are content-free and volunteer-only.

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

/// One person's involvement in a case. The contact fields are joined in for
/// display and remain owned by `contacts`.
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

#[server(prefix = "/api")]
pub async fn list_case_contacts(case_id: String) -> Result<Vec<CaseContact>, ServerFnError> {
    use crate::server::db::case_contacts;
    use crate::server::permissions::{require_case_view_or_admin_read, require_user};

    let user = require_user().await?;
    crate::server_fns::crm::require_staff(&user)?;
    require_case_view_or_admin_read(&user, &case_id).await?;
    case_contacts::list(&case_id)
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
    crate::server_fns::crm::require_staff(&user)?;
    require_cap(&user, &case_id, CaseCapability::EditCase).await?;
    let note =
        crate::server_fns::crm::clean_text(&note, "note", crate::server_fns::crm::MAX_SHORT_TEXT)
            .map_err(ServerFnError::new)?;
    case_contacts::add(
        &case_id,
        contact_id.trim(),
        role,
        &note,
        is_primary,
        &user.full_name(),
    )
    .await
    .map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn remove_case_contact(id: String) -> Result<(), ServerFnError> {
    use crate::server::db::case_contacts;
    use crate::server::permissions::{require_cap, require_user};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    crate::server_fns::crm::require_staff(&user)?;
    // Resolve the owning case from the stored row, never from the caller, so the
    // capability is always checked against the case the link actually belongs to.
    let case_id = case_contacts::case_id(&id)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("That case contact no longer exists."))?;
    require_cap(&user, &case_id, CaseCapability::EditCase).await?;
    case_contacts::remove(&id, &case_id, &user.full_name())
        .await
        .map_err(ServerFnError::new)
}
