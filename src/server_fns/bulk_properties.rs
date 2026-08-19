//! Bulk property editing: name one property once, choose many people or
//! organizations, and set the same value on all of them.
//!
//! Both subjects live here rather than in two mirrored modules because
//! `contact_properties` and `organization_properties` are the same table shape
//! with a different owner column. Everything that differs is reduced to a
//! [`PropertySubject`] the persistence layer maps to a compile-time descriptor.
//!
//! The selection model is the one the contact-mail campaign tool already uses:
//! the browser sends either an explicit id list or "everything matching these
//! filters, minus these exceptions", and the server re-resolves the latter
//! itself, so the record set is never taken on trust.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::helpers::new_crm_fields::normalize_property_part;
use crate::server_fns::contact_properties::{MAX_KEY_CHARS, MAX_VALUE_CHARS};
use crate::server_fns::contacts::ContactType;
use crate::server_fns::organizations::OrganizationKind;
use crate::server_fns::pagination::Page;
use crate::server_fns::property_filters::{self, PropertyFilter, PropertySubject};

/// How many records one bulk save may change. The write happens inline in a
/// single transaction rather than as a background task, so the batch is capped
/// instead of queued.
pub const MAX_BULK_TARGETS: usize = 1000;

/// How many candidates one page of the picker holds.
pub const CANDIDATE_PAGE_SIZE: i64 = 50;

/// A property named by its section heading and its name, as typed by the user.
///
/// Stored rows are always matched on the normalized pair, so "Relationship
/// Status" finds a row stored as "relationship status".
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PropertyRef {
    #[serde(default)]
    pub section: String,
    pub key: String,
}

impl PropertyRef {
    pub fn trimmed(&self) -> Self {
        Self {
            section: self.section.trim().to_string(),
            key: self.key.trim().to_string(),
        }
    }

    pub fn is_named(&self) -> bool {
        !self.key.trim().is_empty()
    }

    /// The normalized `(section, key)` pair stored rows are matched on.
    pub fn normalized(&self) -> (String, String) {
        (
            normalize_property_part(&self.section),
            normalize_property_part(&self.key),
        )
    }

    /// "Section / Name", for confirmations and change-log entries.
    pub fn label(&self) -> String {
        format!(
            "{} / {}",
            crate::helpers::sections::label(&self.section),
            self.key.trim()
        )
    }
}

/// Which records the picker offers. Fields that do not apply to the chosen
/// subject are ignored rather than rejected, so switching subjects never leaves
/// a stale filter that silently returns nothing.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BulkPropertyFilters {
    pub keyword: String,
    /// People only.
    pub contact_type: Option<ContactType>,
    /// People only.
    pub organization_id: String,
    /// Organizations only.
    pub organization_kind: Option<OrganizationKind>,
    pub include_archived: bool,
    /// The shared property chips: AND across properties, OR within one. Exactly
    /// the filter the Contacts and Organizations directories use, so "the records
    /// I was just looking at" means the same thing here.
    #[serde(default)]
    pub property_filters: Vec<PropertyFilter>,
    /// Bulk-edit only: restrict to records that do not carry the property being
    /// set. The shared chips select *values*, so they cannot express "has no row
    /// for this property at all" — which is the backfill case this tool exists
    /// for.
    #[serde(default)]
    pub only_missing_target: bool,
}

/// One record offered for selection, with its current value for the property
/// being edited so the user sees what a save would overwrite.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BulkPropertyCandidate {
    pub id: String,
    pub name: String,
    /// Organization name for a person; kind label for an organization.
    pub subtitle: String,
    pub archived: bool,
    /// `None` means the record has no row for the target property.
    pub current_value: Option<String>,
}

/// Either an explicit set of records, or every record matching `filters` minus
/// the listed exceptions.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BulkPropertySelection {
    pub all_matching: bool,
    pub filters: BulkPropertyFilters,
    /// Explicit selections when `all_matching` is false.
    pub ids: Vec<String>,
    /// Explicit exceptions when `all_matching` is true.
    pub excluded_ids: Vec<String>,
}

/// The single supported operation: set this property to this value.
///
/// A struct rather than a bare pair so clear/remove/add-only can later join it
/// as an operation field without changing any caller's shape.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BulkPropertyEdit {
    pub property: PropertyRef,
    /// An empty value is allowed and means "named but not answered yet", the
    /// same as a blank row in the per-record editor.
    pub value: String,
}

/// What a save would do, computed without writing anything.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BulkPropertyPreview {
    pub total: i64,
    pub will_add: i64,
    pub will_overwrite: i64,
    pub unchanged: i64,
    /// Records that would need a new row but already hold the maximum.
    pub at_property_limit: i64,
}

/// What a save actually did.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BulkPropertyOutcome {
    pub added: i64,
    pub overwritten: i64,
    pub unchanged: i64,
    /// Display names of records skipped because they are at the property limit.
    pub skipped: Vec<String>,
}

impl BulkPropertyOutcome {
    pub fn changed(&self) -> i64 {
        self.added + self.overwritten
    }
}

/// One `(section, name)` pair offered by the property picker.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PropertyKeyOption {
    pub section: String,
    pub key: String,
    /// How many records already carry it; 0 for an unused code-owned default.
    pub usage_count: i64,
}

impl PropertyKeyOption {
    pub fn label(&self) -> String {
        format!(
            "{} / {}",
            crate::helpers::sections::label(&self.section),
            self.key
        )
    }
}

/// Trim an edit and reject what cannot be stored, mirroring the per-record
/// editor's limits so bulk and single saves accept the same input.
pub fn clean_edit(edit: &BulkPropertyEdit) -> Result<BulkPropertyEdit, String> {
    let property = edit.property.trimmed();
    if property.key.is_empty() {
        return Err("Name the property to set.".into());
    }
    if property.key.chars().count() > MAX_KEY_CHARS {
        return Err(format!(
            "A property name must be {MAX_KEY_CHARS} characters or fewer."
        ));
    }
    if property.section.chars().count() > MAX_KEY_CHARS {
        return Err(format!(
            "A section name must be {MAX_KEY_CHARS} characters or fewer."
        ));
    }
    let value = edit.value.trim().to_string();
    if value.chars().count() > MAX_VALUE_CHARS {
        return Err(format!(
            "A property value must be {MAX_VALUE_CHARS} characters or fewer."
        ));
    }
    Ok(BulkPropertyEdit { property, value })
}

/// Normalize the property chips, applying the shared caps on how many
/// properties and values one request may name.
pub fn clean_filters(filters: BulkPropertyFilters) -> BulkPropertyFilters {
    BulkPropertyFilters {
        property_filters: property_filters::clean(filters.property_filters),
        ..filters
    }
}

/// Every entry point here is the same bar as editing one record's properties:
/// bulk editing is a faster way to do what the per-record panel already allows,
/// not a wider power, so it does not additionally require operations admin the
/// way the contact-mail tool does (that gate is about sending outside email).
///
/// Enforced here rather than relying on the route guard, which is UI-only.
#[cfg(feature = "ssr")]
async fn authorize() -> Result<crate::server_fns::users::User, ServerFnError> {
    let user = crate::server::permissions::require_user().await?;
    crate::server::permissions::require_information_management_access(&user)?;
    crate::server_fns::crm::require_staff(&user)?;
    Ok(user)
}

#[cfg(feature = "ssr")]
fn guard_target_count(ids: &[String]) -> Result<(), ServerFnError> {
    if ids.len() > MAX_BULK_TARGETS {
        return Err(ServerFnError::new(format!(
            "Narrow the filters - a single bulk save can change at most {MAX_BULK_TARGETS} records."
        )));
    }
    Ok(())
}

/// One page of records to choose from, each carrying its current value for
/// `target` so the list doubles as a preview of what would be overwritten.
#[server(prefix = "/api", input = leptos::server_fn::codec::Json)]
pub async fn list_bulk_property_candidates(
    subject: PropertySubject,
    filters: BulkPropertyFilters,
    target: PropertyRef,
    offset: i64,
    limit: i64,
) -> Result<Page<BulkPropertyCandidate>, ServerFnError> {
    authorize().await?;
    let filters = clean_filters(filters);
    crate::server::db::bulk_properties::candidate_page(subject, &filters, &target, offset, limit)
        .await
        .map_err(ServerFnError::new)
}

/// The `(section, name)` pairs already in use for this subject, plus the
/// code-owned defaults, so the picker suggests existing properties before
/// inviting a new one.
#[server(prefix = "/api")]
pub async fn list_property_key_options(
    subject: PropertySubject,
) -> Result<Vec<PropertyKeyOption>, ServerFnError> {
    authorize().await?;
    crate::server::db::bulk_properties::key_options(subject)
        .await
        .map_err(ServerFnError::new)
}

/// Count what a save would do. Resolves the selection exactly as
/// [`apply_bulk_property_edit`] does, so the numbers shown are the numbers that
/// will happen barring a concurrent edit.
#[server(prefix = "/api", input = leptos::server_fn::codec::Json)]
pub async fn preview_bulk_property_edit(
    subject: PropertySubject,
    selection: BulkPropertySelection,
    edit: BulkPropertyEdit,
) -> Result<BulkPropertyPreview, ServerFnError> {
    authorize().await?;
    let edit = clean_edit(&edit).map_err(ServerFnError::new)?;
    let selection = BulkPropertySelection {
        filters: clean_filters(selection.filters),
        ..selection
    };
    let ids =
        crate::server::db::bulk_properties::resolve_target_ids(subject, &selection, &edit.property)
            .await
            .map_err(ServerFnError::new)?;
    guard_target_count(&ids)?;
    crate::server::db::bulk_properties::preview(subject, &ids, &edit)
        .await
        .map_err(ServerFnError::new)
}

/// Set the property on every selected record in one transaction.
#[server(prefix = "/api", input = leptos::server_fn::codec::Json)]
pub async fn apply_bulk_property_edit(
    subject: PropertySubject,
    selection: BulkPropertySelection,
    edit: BulkPropertyEdit,
) -> Result<BulkPropertyOutcome, ServerFnError> {
    let user = authorize().await?;
    let edit = clean_edit(&edit).map_err(ServerFnError::new)?;
    let selection = BulkPropertySelection {
        filters: clean_filters(selection.filters),
        ..selection
    };
    let ids =
        crate::server::db::bulk_properties::resolve_target_ids(subject, &selection, &edit.property)
            .await
            .map_err(ServerFnError::new)?;
    guard_target_count(&ids)?;
    if ids.is_empty() {
        return Err(ServerFnError::new("Choose at least one record."));
    }
    crate::server::db::bulk_properties::apply(subject, &ids, &edit, &user.id, &user.full_name())
        .await
        .map_err(ServerFnError::new)
}
