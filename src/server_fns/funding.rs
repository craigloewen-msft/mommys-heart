//! Funding: money received, against a grant or as a standalone gift.
//!
//! Amounts are integer minor units (cents) throughout. A mistaken record is
//! *voided* with a reason rather than deleted — it leaves the rollups but stays
//! visible and audited, and a database trigger refuses `DELETE` outright
//! (REQ-AUD-004).

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::server_fns::crm::{
    clean_date, clean_text, coded_enum, parse_cents, MAX_LONG_TEXT, MAX_SHORT_TEXT,
};
use crate::server_fns::pagination::Page;

coded_enum!(FundingKind {
    GrantPayment => ("grant_payment", "Grant payment"),
    Donation => ("donation", "Donation"),
    InKind => ("in_kind", "In-kind"),
    Other => ("other", "Other"),
});

impl FundingKind {
    pub fn badge_classes(self) -> &'static str {
        match self {
            Self::GrantPayment => "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30",
            Self::Donation => "bg-sky-500/15 text-sky-300 ring-1 ring-sky-500/30",
            Self::InKind => "bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/30",
            Self::Other => "bg-slate-700/40 text-slate-300 ring-1 ring-slate-600",
        }
    }
}

impl Default for FundingKind {
    fn default() -> Self {
        Self::Donation
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FundingRecord {
    pub id: String,
    pub kind: FundingKind,
    pub amount_cents: i64,
    pub received_on: String,
    pub grant_id: String,
    pub grant_name: String,
    pub source_organization_id: String,
    pub source_organization_name: String,
    pub source_contact_id: String,
    pub source_contact_name: String,
    pub reference: String,
    pub notes: String,
    pub voided: bool,
    pub void_reason: String,
    pub recorded_by: String,
    pub created_at: String,
}

impl FundingRecord {
    /// Where the money came from, for a single display column.
    pub fn source_label(&self) -> String {
        if !self.source_organization_name.is_empty() {
            self.source_organization_name.clone()
        } else if !self.source_contact_name.is_empty() {
            self.source_contact_name.clone()
        } else if !self.grant_name.is_empty() {
            self.grant_name.clone()
        } else {
            "Unattributed".to_string()
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FundingInput {
    pub kind: FundingKind,
    /// Typed by a person, e.g. `$2,500.00`; parsed to cents on the server.
    pub amount: String,
    pub received_on: String,
    pub grant_id: String,
    pub source_organization_id: String,
    pub source_contact_id: String,
    pub reference: String,
    pub notes: String,
}

pub struct ValidatedFunding {
    pub kind: FundingKind,
    pub amount_cents: i64,
    pub received_on: String,
    pub grant_id: Option<String>,
    pub source_organization_id: Option<String>,
    pub source_contact_id: Option<String>,
    pub reference: String,
    pub notes: String,
}

impl FundingInput {
    /// Enforce the same rules as the table's `CHECK`s: a positive amount, a real
    /// date, a grant payment that names its grant, and a donation that says
    /// where it came from.
    pub fn validate(&self) -> Result<ValidatedFunding, String> {
        let amount_cents = parse_cents(&self.amount)?;
        if amount_cents <= 0 {
            return Err("An amount must be greater than zero.".into());
        }

        let received_on = clean_date(&self.received_on, "received date")?;
        if received_on.is_empty() {
            return Err("Enter the date the funding was received.".into());
        }

        let optional_id = |raw: &str| {
            let trimmed = raw.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        };
        let grant_id = optional_id(&self.grant_id);
        let source_organization_id = optional_id(&self.source_organization_id);
        let source_contact_id = optional_id(&self.source_contact_id);

        if self.kind == FundingKind::GrantPayment && grant_id.is_none() {
            return Err("A grant payment must say which grant it is against.".into());
        }
        if self.kind == FundingKind::Donation
            && source_organization_id.is_none()
            && source_contact_id.is_none()
        {
            return Err("A donation must name the organization or person it came from.".into());
        }

        Ok(ValidatedFunding {
            kind: self.kind,
            amount_cents,
            received_on,
            grant_id,
            source_organization_id,
            source_contact_id,
            reference: clean_text(&self.reference, "reference", MAX_SHORT_TEXT)?,
            notes: clean_text(&self.notes, "notes", MAX_LONG_TEXT)?,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FundingFilters {
    pub kind: Option<FundingKind>,
    pub grant_id: String,
    pub include_voided: bool,
}

#[server(prefix = "/api")]
pub async fn list_funding(
    filters: FundingFilters,
    offset: i64,
    limit: i64,
) -> Result<Page<FundingRecord>, ServerFnError> {
    use crate::server::db::funding;
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    require_operations_admin(&user)?;
    funding::page(&filters, offset, limit)
        .await
        .map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn record_funding(input: FundingInput) -> Result<String, ServerFnError> {
    use crate::server::db::funding;
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    require_operations_admin(&user)?;
    let validated = input.validate().map_err(ServerFnError::new)?;
    funding::create(&validated, &user.full_name())
        .await
        .map_err(ServerFnError::new)
}

/// Void a funding record with a reason. It leaves the rollups but stays on the
/// ledger: corrections are visible, not silent.
#[server(prefix = "/api")]
pub async fn void_funding(id: String, reason: String) -> Result<(), ServerFnError> {
    use crate::server::db::funding;
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    require_operations_admin(&user)?;
    let reason = clean_text(&reason, "reason", MAX_SHORT_TEXT).map_err(ServerFnError::new)?;
    if reason.is_empty() {
        return Err(ServerFnError::new(
            "Give a reason so the correction explains itself later.",
        ));
    }
    funding::void(&id, &reason, &user.full_name())
        .await
        .map_err(ServerFnError::new)
}
