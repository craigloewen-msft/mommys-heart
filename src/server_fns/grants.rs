//! Grants: an award sought or received from a funder.
//!
//! `migrations/0001_init.sql` created a two-column `grants` stub that nothing
//! ever used; migration 0019 grows it into a real record in place. Money is
//! integer minor units (cents) end to end — never a float, which cannot hold a
//! currency amount exactly.
//!
//! Grants and [`crate::server_fns::funding`] are back-office records: operations
//! admins and above only, not volunteers.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::server_fns::crm::{
    clean_date, clean_text, coded_enum, parse_cents, MAX_LONG_TEXT, MAX_NAME,
};
use crate::server_fns::pagination::Page;

coded_enum!(GrantStatus {
    Prospect => ("prospect", "Prospect"),
    Applied => ("applied", "Applied"),
    Awarded => ("awarded", "Awarded"),
    Active => ("active", "Active"),
    Reporting => ("reporting", "Reporting"),
    Closed => ("closed", "Closed"),
    Declined => ("declined", "Declined"),
});

impl GrantStatus {
    /// Whether this status means the award exists, and so requires an amount and
    /// a period. Mirrors `grants_awarded_requires_terms_check`.
    pub fn is_awarded(self) -> bool {
        matches!(
            self,
            Self::Awarded | Self::Active | Self::Reporting | Self::Closed
        )
    }

    /// Whether a decision has been made, and so requires a decision date.
    /// Mirrors `grants_decided_requires_date_check`.
    pub fn is_decided(self) -> bool {
        matches!(self, Self::Closed | Self::Declined)
    }

    pub fn badge_classes(self) -> &'static str {
        match self {
            Self::Prospect => "bg-slate-700/40 text-slate-300 ring-1 ring-slate-600",
            Self::Applied => "bg-sky-500/15 text-sky-300 ring-1 ring-sky-500/30",
            Self::Awarded | Self::Active => {
                "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30"
            }
            Self::Reporting => "bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/30",
            Self::Closed => "bg-primary-500/15 text-primary-300 ring-1 ring-primary-500/30",
            Self::Declined => "bg-rose-500/15 text-rose-300 ring-1 ring-rose-500/30",
        }
    }
}

impl Default for GrantStatus {
    fn default() -> Self {
        Self::Prospect
    }
}

coded_enum!(ReportingCadence {
    None => ("none", "No reporting"),
    Monthly => ("monthly", "Monthly"),
    Quarterly => ("quarterly", "Quarterly"),
    Semiannual => ("semiannual", "Twice a year"),
    Annual => ("annual", "Annual"),
    FinalOnly => ("final_only", "Final report only"),
});

impl Default for ReportingCadence {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Grant {
    pub id: String,
    pub name: String,
    pub status: GrantStatus,
    pub funder_organization_id: String,
    pub funder_name: String,
    pub program_officer_contact_id: String,
    pub program_officer_name: String,
    pub amount_requested_cents: Option<i64>,
    pub amount_awarded_cents: Option<i64>,
    pub application_date: String,
    pub decision_date: String,
    pub period_start: String,
    pub period_end: String,
    pub purpose: String,
    pub reporting_cadence: ReportingCadence,
    pub notes: String,
    /// Sum of non-voided funding recorded against this grant, in cents.
    pub received_cents: i64,
}

impl Grant {
    /// Award minus what has come in. `None` when nothing has been awarded yet,
    /// because there is no total to measure against.
    pub fn remaining_cents(&self) -> Option<i64> {
        self.amount_awarded_cents
            .map(|awarded| awarded - self.received_cents)
    }

    /// Whether receipts have exceeded the award — worth flagging rather than
    /// showing a negative balance without comment.
    pub fn is_overfunded(&self) -> bool {
        self.remaining_cents()
            .is_some_and(|remaining| remaining < 0)
    }
}

/// The editable fields of a grant. Amounts arrive as typed text so a person can
/// enter `$2,500.00`; the server parses them to cents.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GrantInput {
    pub name: String,
    pub status: GrantStatus,
    pub funder_organization_id: String,
    pub program_officer_contact_id: String,
    pub amount_requested: String,
    pub amount_awarded: String,
    pub application_date: String,
    pub decision_date: String,
    pub period_start: String,
    pub period_end: String,
    pub purpose: String,
    pub reporting_cadence: ReportingCadence,
    pub notes: String,
}

/// A grant input with its amounts and dates already parsed and checked.
pub struct ValidatedGrant {
    pub name: String,
    pub status: GrantStatus,
    pub funder_organization_id: Option<String>,
    pub program_officer_contact_id: Option<String>,
    pub amount_requested_cents: Option<i64>,
    pub amount_awarded_cents: Option<i64>,
    pub application_date: Option<String>,
    pub decision_date: Option<String>,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    pub purpose: String,
    pub reporting_cadence: ReportingCadence,
    pub notes: String,
}

impl GrantInput {
    /// Parse and check every field, enforcing the same rules the database does so
    /// a mistake reads as a sentence instead of a constraint violation.
    pub fn validate(&self) -> Result<ValidatedGrant, String> {
        let name = clean_text(&self.name, "grant name", MAX_NAME)?;
        if name.is_empty() {
            return Err("Enter the grant's name.".into());
        }

        let optional_amount = |raw: &str, label: &str| -> Result<Option<i64>, String> {
            if raw.trim().is_empty() {
                Ok(None)
            } else {
                parse_cents(raw)
                    .map(Some)
                    .map_err(|e| format!("{label}: {e}"))
            }
        };
        let amount_requested_cents = optional_amount(&self.amount_requested, "Amount requested")?;
        let amount_awarded_cents = optional_amount(&self.amount_awarded, "Amount awarded")?;

        let application_date = clean_date(&self.application_date, "application date")?;
        let decision_date = clean_date(&self.decision_date, "decision date")?;
        let period_start = clean_date(&self.period_start, "period start")?;
        let period_end = clean_date(&self.period_end, "period end")?;

        if !period_start.is_empty() && !period_end.is_empty() && period_end < period_start {
            return Err("The period must end on or after it starts.".into());
        }
        if self.status.is_awarded() {
            if amount_awarded_cents.is_none() {
                return Err(format!(
                    "A grant that is {} needs an awarded amount.",
                    self.status.label().to_lowercase()
                ));
            }
            if period_start.is_empty() || period_end.is_empty() {
                return Err(format!(
                    "A grant that is {} needs a start and end date.",
                    self.status.label().to_lowercase()
                ));
            }
        }
        if self.status.is_decided() && decision_date.is_empty() {
            return Err(format!(
                "A grant that is {} needs a decision date.",
                self.status.label().to_lowercase()
            ));
        }

        let optional_id = |raw: &str| {
            let trimmed = raw.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        };
        Ok(ValidatedGrant {
            name,
            status: self.status,
            funder_organization_id: optional_id(&self.funder_organization_id),
            program_officer_contact_id: optional_id(&self.program_officer_contact_id),
            amount_requested_cents,
            amount_awarded_cents,
            application_date: optional_id(&application_date),
            decision_date: optional_id(&decision_date),
            period_start: optional_id(&period_start),
            period_end: optional_id(&period_end),
            purpose: clean_text(&self.purpose, "purpose", MAX_LONG_TEXT)?,
            reporting_cadence: self.reporting_cadence,
            notes: clean_text(&self.notes, "notes", MAX_LONG_TEXT)?,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GrantFilters {
    pub keyword: String,
    pub status: Option<GrantStatus>,
    pub funder_organization_id: String,
}

/// Totals for the grant list header, so the money picture is visible without
/// paging through every row.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GrantTotals {
    pub awarded_cents: i64,
    pub received_cents: i64,
    pub active_count: i64,
    pub prospect_count: i64,
}

#[server(prefix = "/api")]
pub async fn list_grants(
    filters: GrantFilters,
    offset: i64,
    limit: i64,
) -> Result<Page<Grant>, ServerFnError> {
    use crate::server::db::grants;
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    require_operations_admin(&user)?;
    grants::page(&filters, offset, limit)
        .await
        .map_err(ServerFnError::new)
}

/// Named `load_grant_totals` because `#[server]` generates a request struct from
/// the function name, and `grant_totals` would collide with [`GrantTotals`].
#[server(prefix = "/api")]
pub async fn load_grant_totals() -> Result<GrantTotals, ServerFnError> {
    use crate::server::db::grants;
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    require_operations_admin(&user)?;
    grants::totals().await.map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn load_grant(id: String) -> Result<Option<Grant>, ServerFnError> {
    use crate::server::db::grants;
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    require_operations_admin(&user)?;
    grants::get(&id).await.map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn create_grant(input: GrantInput) -> Result<String, ServerFnError> {
    use crate::server::db::grants;
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    require_operations_admin(&user)?;
    let validated = input.validate().map_err(ServerFnError::new)?;
    grants::create(&validated, &user.full_name())
        .await
        .map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn update_grant(id: String, input: GrantInput) -> Result<(), ServerFnError> {
    use crate::server::db::grants;
    use crate::server::permissions::{require_operations_admin, require_user};

    let user = require_user().await?;
    require_operations_admin(&user)?;
    let validated = input.validate().map_err(ServerFnError::new)?;
    grants::update(&id, &validated, &user.full_name())
        .await
        .map_err(ServerFnError::new)
}
