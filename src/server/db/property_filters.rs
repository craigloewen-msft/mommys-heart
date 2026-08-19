//! Property filtering persistence (SSR only): facet counts and the shared
//! `WHERE` fragment every property-filtered search pastes in.
//!
//! Contacts and organizations keep their properties in separate tables with the
//! same shape, so everything here is written once and switched on
//! [`PropertySubject`].
//!
//! Matching is on `lower(btrim(...))` of both the key and the value, so the same
//! fact typed as "Location" and "location" is one facet. The raw spelling is
//! never rewritten; the most common one is reported for display.

use std::collections::HashMap;

use crate::server::db::pool;
use crate::server_fns::property_filters::{
    PropertyFacet, PropertyFacetScope, PropertyFacetValue, PropertyFilter, PropertySubject,
    MAX_FACET_KEYS, MAX_FACET_VALUES,
};

/// The property table and the column joining it back to its record.
fn table(subject: PropertySubject) -> (&'static str, &'static str) {
    match subject {
        PropertySubject::Contact => ("contact_properties", "contact_id"),
        PropertySubject::Organization => ("organization_properties", "organization_id"),
    }
}

/// Encode filters for the single `jsonb` bind the predicate reads.
///
/// One parameter however many chips are active, so the `$n` placeholders around
/// it stay fixed and each call site keeps its existing hand-numbered SQL.
pub fn to_json(filters: &[PropertyFilter]) -> String {
    serde_json::to_string(filters).unwrap_or_else(|_| "[]".to_string())
}

/// The `AND` fragment restricting `record_id_sql` to records matching every
/// active filter, where a filter is satisfied by any one of its values.
///
/// Read as "no filter fails to match": AND across filters, OR within one. An
/// empty array matches everything, so unfiltered callers pass `[]`.
pub fn predicate_sql(subject: PropertySubject, record_id_sql: &str, placeholder: usize) -> String {
    let (table, id_column) = table(subject);
    format!(
        "AND NOT EXISTS (
             SELECT 1 FROM jsonb_array_elements(${placeholder}::jsonb) AS mh_filter
             WHERE NOT EXISTS (
                 SELECT 1 FROM {table} mh_prop
                 WHERE mh_prop.{id_column} = {record_id_sql}
                   AND lower(btrim(mh_prop.key)) = mh_filter->>'key'
                   AND lower(btrim(mh_prop.value)) IN (
                       SELECT jsonb_array_elements_text(mh_filter->'values'))))"
    )
}

/// The `OR` fragment letting a plain keyword match a property value.
///
/// Values only, never keys: matching the key would make the word "location"
/// return every record that has a Location property.
pub fn keyword_match_sql(
    subject: PropertySubject,
    record_id_sql: &str,
    placeholder: usize,
) -> String {
    let (table, id_column) = table(subject);
    format!(
        "OR EXISTS (
             SELECT 1 FROM {table} mh_search
             WHERE mh_search.{id_column} = {record_id_sql}
               AND mh_search.value <> '' AND mh_search.value ILIKE ${placeholder})"
    )
}

/// The keyword clause contacts search by, shared with the facet scope so the
/// counts describe the same rows the directory lists.
fn contact_keyword_sql(include_account_projection: bool) -> String {
    let (account_clause, display_name) = if include_account_projection {
        (
            "OR u.first_name ILIKE $2 OR u.last_name ILIKE $2 OR u.email ILIKE $2",
            "btrim(
                 coalesce(nullif(c.preferred_name, ''),
                          CASE WHEN u.id IS NULL THEN c.first_name ELSE u.first_name END)
                 || ' '
                 || CASE WHEN u.id IS NULL THEN c.last_name ELSE u.last_name END)",
        )
    } else {
        (
            "",
            "btrim(coalesce(nullif(c.preferred_name, ''), c.first_name) || ' ' || c.last_name)",
        )
    };
    let property_match = keyword_match_sql(PropertySubject::Contact, "c.id", 2);
    format!(
        "($1 = '' OR c.first_name ILIKE $2 OR c.last_name ILIKE $2
          OR c.preferred_name ILIKE $2 {account_clause}
          OR {display_name} ILIKE $2
          OR c.job_title ILIKE $2 OR o.name ILIKE $2
          OR c.email ILIKE $2 OR c.phone ILIKE $2
          OR c.mobile ILIKE $2 OR c.address ILIKE $2 OR c.website ILIKE $2
          {property_match})"
    )
}

/// One flattened facet row: a key with one of its values.
#[derive(sqlx::FromRow)]
struct FacetRow {
    key: String,
    display_key: String,
    record_count: i64,
    value: String,
    display_value: String,
    value_count: i64,
    distinct_values: i64,
}

/// Assemble the facet query around a subject-specific `scope` CTE.
///
/// A record counts toward a facet when it matches every active filter, or when
/// the *only* filter it fails is the one being counted. That is what lets the
/// Location chip keep showing the other Location values you could switch to,
/// while every other facet narrows to the current result set.
fn facet_sql(subject: PropertySubject, scope_sql: &str, filters_placeholder: usize) -> String {
    let (table, id_column) = table(subject);
    let keys_placeholder = filters_placeholder + 1;
    let values_placeholder = filters_placeholder + 2;
    format!(
        "WITH scope AS ({scope_sql}),
         qualified AS (
             SELECT s.id,
                    ARRAY(
                        SELECT mh_filter->>'key'
                        FROM jsonb_array_elements(${filters_placeholder}::jsonb) AS mh_filter
                        WHERE NOT EXISTS (
                            SELECT 1 FROM {table} mh_prop
                            WHERE mh_prop.{id_column} = s.id
                              AND lower(btrim(mh_prop.key)) = mh_filter->>'key'
                              AND lower(btrim(mh_prop.value)) IN (
                                  SELECT jsonb_array_elements_text(mh_filter->'values')))
                    ) AS unmatched
             FROM scope s
         ),
         counted AS (
             SELECT q.id, lower(btrim(p.key)) AS key, p.key AS raw_key,
                    lower(btrim(p.value)) AS value, p.value AS raw_value
             FROM qualified q
             JOIN {table} p ON p.{id_column} = q.id
             WHERE cardinality(q.unmatched) = 0
                OR (cardinality(q.unmatched) = 1
                    AND q.unmatched[1] = lower(btrim(p.key)))
         ),
         key_counts AS (
             SELECT key, mode() WITHIN GROUP (ORDER BY raw_key) AS display_key,
                    count(DISTINCT id) AS record_count
             FROM counted GROUP BY key
         ),
         value_counts AS (
             SELECT key, value, mode() WITHIN GROUP (ORDER BY raw_value) AS display_value,
                    count(DISTINCT id) AS value_count
             FROM counted GROUP BY key, value
         ),
         ranked AS (
             SELECT key, value, display_value, value_count,
                    row_number() OVER (
                        PARTITION BY key ORDER BY value_count DESC, value ASC) AS rank,
                    count(*) OVER (PARTITION BY key) AS distinct_values
             FROM value_counts
         ),
         top_keys AS (
             SELECT key FROM key_counts
             WHERE key IN (SELECT mh_filter->>'key'
                           FROM jsonb_array_elements(${filters_placeholder}::jsonb) AS mh_filter)
             UNION
             (SELECT key FROM key_counts ORDER BY record_count DESC, key ASC
              LIMIT ${keys_placeholder})
         )
         SELECT k.key, k.display_key, k.record_count,
                r.value, r.display_value, r.value_count, r.distinct_values
         FROM key_counts k
         JOIN top_keys t ON t.key = k.key
         JOIN ranked r ON r.key = k.key
         WHERE r.rank <= ${values_placeholder}
         ORDER BY k.record_count DESC, k.key ASC, r.value_count DESC, r.value ASC"
    )
}

/// Fold the flat key/value rows into one facet per key, preserving the SQL order.
fn fold_facets(rows: Vec<FacetRow>) -> Vec<PropertyFacet> {
    let mut facets = Vec::<PropertyFacet>::new();
    let mut indexes = HashMap::<String, usize>::new();
    for row in rows {
        let index = *indexes.entry(row.key.clone()).or_insert_with(|| {
            facets.push(PropertyFacet {
                key: row.key.clone(),
                display_key: row.display_key.clone(),
                record_count: row.record_count,
                values: Vec::new(),
                distinct_values: row.distinct_values,
                truncated: row.distinct_values > MAX_FACET_VALUES,
            });
            facets.len() - 1
        });
        facets[index].values.push(PropertyFacetValue {
            value: row.value,
            display_value: row.display_value,
            count: row.value_count,
        });
    }
    facets
}

/// The property names and values in use across the records `scope` describes.
pub async fn facets(
    subject: PropertySubject,
    scope: &PropertyFacetScope,
    include_account_projection: bool,
) -> Result<Vec<PropertyFacet>, sqlx::Error> {
    let keyword = scope.keyword.trim();
    let pattern = format!("%{keyword}%");
    let filters_json = to_json(&scope.property_filters);

    let rows = match subject {
        PropertySubject::Contact => {
            let keyword_sql = contact_keyword_sql(include_account_projection);
            // The mail picker escapes its keyword pattern; a query containing a
            // literal % or _ can therefore count marginally differently there.
            let scope_sql = format!(
                "SELECT c.id
                 FROM contacts c
                 LEFT JOIN organizations o ON o.id = c.organization_id
                 LEFT JOIN users u ON u.id = c.user_id
                 WHERE {keyword_sql}
                   AND (cardinality($3::text[]) = 0 OR c.id IN (
                           SELECT selected.contact_id
                           FROM contact_category_assignments selected
                           WHERE selected.category_id = ANY($3::text[])
                           GROUP BY selected.contact_id
                           HAVING count(DISTINCT selected.category_id)
                                  = cardinality($3::text[])))
                   AND ($4 = '' OR $4 = ANY(c.types))
                   AND ($5 = '' OR c.organization_id = $5)
                   AND ($6 OR NOT c.archived)
                   AND (NOT $7 OR (
                           NOT c.archived AND NOT c.do_not_contact
                           AND btrim(CASE WHEN u.id IS NULL THEN c.email ELSE u.email END) <> ''
                           AND CASE WHEN u.id IS NULL THEN c.email ELSE u.email END LIKE '%@%'))"
            );
            sqlx::query_as::<_, FacetRow>(&facet_sql(subject, &scope_sql, 8))
                .bind(keyword)
                .bind(&pattern)
                .bind(&scope.category_ids)
                .bind(scope.contact_type.trim())
                .bind(scope.organization_id.trim())
                .bind(scope.include_archived)
                .bind(scope.mailable_only)
                .bind(&filters_json)
                .bind(MAX_FACET_KEYS)
                .bind(MAX_FACET_VALUES)
                .fetch_all(pool())
                .await?
        }
        PropertySubject::Organization => {
            let property_match = keyword_match_sql(subject, "o.id", 2);
            let scope_sql = format!(
                "SELECT o.id FROM organizations o
                 WHERE ($1 = '' OR o.name ILIKE $2 OR o.email ILIKE $2 {property_match})
                   AND ($3 = '' OR o.kind = $3)
                   AND ($4 OR NOT o.archived)"
            );
            sqlx::query_as::<_, FacetRow>(&facet_sql(subject, &scope_sql, 5))
                .bind(keyword)
                .bind(&pattern)
                .bind(scope.organization_kind.trim())
                .bind(scope.include_archived)
                .bind(&filters_json)
                .bind(MAX_FACET_KEYS)
                .bind(MAX_FACET_VALUES)
                .fetch_all(pool())
                .await?
        }
    };
    Ok(fold_facets(rows))
}

/// One record's value for a filtered property, for showing why a row matched.
#[derive(sqlx::FromRow)]
struct MatchedRow {
    record_id: String,
    display_key: String,
    value: String,
}

/// The values the listed records carry for the filtered keys, keyed by record id.
///
/// One bounded query for a whole page of results, so the cards can name the
/// reason each row matched without a request per row.
pub async fn values_for(
    subject: PropertySubject,
    record_ids: &[String],
    keys: &[String],
) -> Result<HashMap<String, Vec<(String, String)>>, sqlx::Error> {
    if record_ids.is_empty() || keys.is_empty() {
        return Ok(HashMap::new());
    }
    let (table, id_column) = table(subject);
    let rows = sqlx::query_as::<_, MatchedRow>(&format!(
        "SELECT {id_column} AS record_id, key AS display_key, value
         FROM {table}
         WHERE {id_column} = ANY($1::text[])
           AND lower(btrim(key)) = ANY($2::text[])
         ORDER BY {id_column}, ord"
    ))
    .bind(record_ids)
    .bind(keys)
    .fetch_all(pool())
    .await?;

    let mut matched = HashMap::<String, Vec<(String, String)>>::new();
    for row in rows {
        matched
            .entry(row.record_id)
            .or_default()
            .push((row.display_key, row.value));
    }
    Ok(matched)
}
