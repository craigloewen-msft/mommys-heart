//! Filtering records by their custom properties, shared by contacts and
//! organizations.
//!
//! Properties are free text: two people recording the same fact may type
//! "Location", "location", or " Location ". Everything here matches on the
//! normalized form (trimmed and lowercased) and carries the most common raw
//! spelling alongside it purely for display, so the two records land in one
//! facet without rewriting what anyone typed.
//!
//! A cross-cutting module rather than one per subject, because the filter is the
//! same question asked of two different top-level objects.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// How many properties one filter set may name.
pub const MAX_FILTERS: usize = 10;
/// How many values one property filter may select.
pub const MAX_FILTER_VALUES: usize = 50;
/// How many property names a facet listing returns.
pub const MAX_FACET_KEYS: i64 = 200;
/// How many values one facet returns before reporting itself truncated.
pub const MAX_FACET_VALUES: i64 = 50;

/// Which table a property filter applies to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropertySubject {
    #[default]
    Contact,
    Organization,
}

impl PropertySubject {
    pub const ALL: &'static [PropertySubject] =
        &[PropertySubject::Contact, PropertySubject::Organization];

    /// The stable slug used in query strings.
    pub fn slug(self) -> &'static str {
        match self {
            Self::Contact => "contact",
            Self::Organization => "organization",
        }
    }

    pub fn from_slug(value: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|s| s.slug() == value)
    }

    /// The plural noun for UI copy, e.g. "17 of 17 people".
    pub fn noun_plural(self) -> &'static str {
        match self {
            Self::Contact => "people",
            Self::Organization => "organizations",
        }
    }

    /// The label for a subject picker.
    pub fn label(self) -> &'static str {
        match self {
            Self::Contact => "People",
            Self::Organization => "Organizations",
        }
    }
}

/// One property facet the reader is filtering by.
///
/// `key` and `values` are normalized. An empty string in `values` is meaningful:
/// it selects records where the property is named but not filled in, which is a
/// different answer from not having the property at all.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PropertyFilter {
    pub key: String,
    pub values: Vec<String>,
}

/// One selectable value of a facet, with how many records carry it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PropertyFacetValue {
    /// Normalized, for matching.
    pub value: String,
    /// The most common raw spelling, for display.
    pub display_value: String,
    pub count: i64,
}

impl PropertyFacetValue {
    /// The label to show for a value, naming the blank case explicitly.
    pub fn label(&self) -> String {
        if self.value.is_empty() {
            "\u{2014} not filled in \u{2014}".to_string()
        } else {
            self.display_value.clone()
        }
    }
}

/// One property name in use, with the values recorded under it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PropertyFacet {
    /// Normalized, for matching.
    pub key: String,
    /// The most common raw spelling, for display.
    pub display_key: String,
    /// How many records carry this property at all.
    pub record_count: i64,
    pub values: Vec<PropertyFacetValue>,
    /// How many distinct values exist, including any beyond `values`.
    pub distinct_values: i64,
    /// Whether `values` was cut short by [`MAX_FACET_VALUES`].
    pub truncated: bool,
}

/// The filters already in play when facets are counted, so the counts describe
/// what is actually on screen rather than the whole database.
///
/// One struct for both subjects: each caller fills the fields its own directory
/// has and leaves the rest at their defaults.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PropertyFacetScope {
    pub keyword: String,
    pub include_archived: bool,
    /// Contacts only: required categories, contact type, and organization.
    pub category_ids: Vec<String>,
    pub contact_type: String,
    pub organization_id: String,
    /// Organizations only.
    pub organization_kind: String,
    /// Contacts only: restrict to addressable people, as the mail picker does.
    pub mailable_only: bool,
    /// The property chips already applied.
    pub property_filters: Vec<PropertyFilter>,
}

/// Normalize one property name or value for matching.
pub fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
}

/// Drop the filters that cannot be applied and bound the rest.
///
/// Applied on the server for every entry point, so a hand-written URL or a
/// crafted request cannot ask for an unbounded number of clauses.
pub fn clean(filters: Vec<PropertyFilter>) -> Vec<PropertyFilter> {
    let mut cleaned = Vec::<PropertyFilter>::new();
    for filter in filters {
        let key = normalize(&filter.key);
        if key.is_empty() {
            continue;
        }
        let mut values: Vec<String> = filter.values.iter().map(|value| normalize(value)).collect();
        values.sort();
        values.dedup();
        values.truncate(MAX_FILTER_VALUES);
        if values.is_empty() {
            continue;
        }
        // A repeated key would mean two AND clauses on one property, which can
        // only ever match nothing; merge them into the OR the reader meant.
        if let Some(existing) = cleaned.iter_mut().find(|existing| existing.key == key) {
            existing.values.extend(values);
            existing.values.sort();
            existing.values.dedup();
            existing.values.truncate(MAX_FILTER_VALUES);
        } else {
            cleaned.push(PropertyFilter { key, values });
        }
        if cleaned.len() >= MAX_FILTERS {
            break;
        }
    }
    cleaned
}

/// Escape the characters the URL form uses as separators.
fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        if matches!(ch, '\\' | ':' | '|' | ',') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// Render filters for the query string as `key:value|value,key:value`.
///
/// Readable enough to see what a shared link filters by, and escaped so a value
/// containing a separator survives the round trip.
pub fn encode(filters: &[PropertyFilter]) -> String {
    filters
        .iter()
        .filter(|filter| !filter.key.is_empty() && !filter.values.is_empty())
        .map(|filter| {
            let values = filter
                .values
                .iter()
                .map(|value| escape(value))
                .collect::<Vec<_>>()
                .join("|");
            format!("{}:{}", escape(&filter.key), values)
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// Read back what [`encode`] wrote.
///
/// One pass over the whole string rather than nested splits: escapes must be
/// consumed exactly once, and splitting level by level would strip them before
/// the inner separators had been read. Malformed input is salvaged rather than
/// rejected, since it arrives from a pasted URL.
pub fn decode(encoded: &str) -> Vec<PropertyFilter> {
    let mut filters = Vec::<PropertyFilter>::new();
    let mut key = String::new();
    let mut values = Vec::<String>::new();
    let mut current = String::new();
    // Before the first unescaped ':' every character belongs to the key.
    let mut in_values = false;
    let mut escaped = false;

    let finish_filter =
        |filters: &mut Vec<PropertyFilter>, key: &mut String, values: &mut Vec<String>| {
            if !key.is_empty() {
                filters.push(PropertyFilter {
                    key: std::mem::take(key),
                    values: std::mem::take(values),
                });
            } else {
                key.clear();
                values.clear();
            }
        };

    for ch in encoded.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            ':' if !in_values => {
                key = std::mem::take(&mut current);
                in_values = true;
            }
            '|' if in_values => values.push(std::mem::take(&mut current)),
            ',' => {
                if in_values {
                    values.push(std::mem::take(&mut current));
                } else {
                    // A chunk with no ':' at all is malformed; drop it.
                    current.clear();
                }
                finish_filter(&mut filters, &mut key, &mut values);
                in_values = false;
            }
            _ => current.push(ch),
        }
    }
    if in_values {
        values.push(current);
        finish_filter(&mut filters, &mut key, &mut values);
    }
    clean(filters)
}

/// The property names and values in use, counted against everything else the
/// reader has already filtered by.
#[server(prefix = "/api", input = leptos::server_fn::codec::Json)]
pub async fn list_property_facets(
    subject: PropertySubject,
    scope: PropertyFacetScope,
) -> Result<Vec<PropertyFacet>, ServerFnError> {
    use crate::server::db::property_filters;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    crate::server::permissions::require_information_management_access(&user)?;
    crate::server_fns::crm::require_staff(&user)?;
    let scope = PropertyFacetScope {
        property_filters: clean(scope.property_filters),
        ..scope
    };
    property_filters::facets(
        subject,
        &scope,
        user.role.has_operations_admin_permissions(),
    )
    .await
    .map_err(ServerFnError::new)
}
