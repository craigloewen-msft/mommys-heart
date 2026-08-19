//! Bulk property persistence (SSR only), shared by contacts and organizations.
//!
//! `contact_properties` and `organization_properties` are the same table with a
//! different owner column, so one set of queries serves both. Everything that
//! differs is a [`SubjectTables`] descriptor of `&'static str`s chosen by
//! matching on [`PropertySubject`]: no table or column name ever comes from
//! client input, and every value is bound, so the `format!`-assembled SQL is as
//! safe as the hand-written statements elsewhere in this module tree.

use std::collections::HashSet;

use crate::helpers::new_crm_fields;
use crate::server::db::{audit, pool};
use crate::server_fns::bulk_properties::{
    BulkPropertyCandidate, BulkPropertyEdit, BulkPropertyFilters, BulkPropertyOutcome,
    BulkPropertyPreview, BulkPropertySelection, PropertyCondition, PropertyKeyOption, PropertyRef,
    PropertySubject, PropertyValueMatch,
};
use crate::server_fns::contact_properties::MAX_PROPERTIES;
use crate::server_fns::contacts::ContactType;
use crate::server_fns::organizations::OrganizationKind;
use crate::server_fns::pagination::Page;

/// Everything that differs between the two subjects.
struct SubjectTables {
    /// Where the properties live.
    properties_table: &'static str,
    /// The column in that table pointing back at the record.
    owner_column: &'static str,
    /// The record table and the alias every expression below uses.
    owner_table: &'static str,
    alias: &'static str,
    from_joins: &'static str,
    /// `id`, `name`, `subtitle`, `archived`, in that order.
    select_columns: &'static str,
    /// Uses `$1` (include archived), `$2` (type/kind slug), `$3` (organization
    /// id), `$4` (keyword).
    base_where: &'static str,
    entity: audit::Entity,
}

const PEOPLE: SubjectTables = SubjectTables {
    properties_table: "contact_properties",
    owner_column: "contact_id",
    owner_table: "contacts",
    alias: "c",
    from_joins: "FROM contacts c LEFT JOIN organizations o ON o.id = c.organization_id",
    select_columns: "c.id AS id,
     coalesce(
         nullif(btrim(coalesce(nullif(c.preferred_name, ''), c.first_name) || ' ' || c.last_name), ''),
         nullif(o.name, ''),
         c.id
     ) AS name,
     coalesce(o.name, '') AS subtitle,
     c.archived AS archived",
    base_where: "WHERE ($1 OR NOT c.archived)
       AND ($2 = '' OR $2 = ANY(c.types))
       AND ($3 = '' OR c.organization_id = $3)
       AND ($4 = '' OR c.first_name ILIKE '%' || $4 || '%'
                    OR c.last_name ILIKE '%' || $4 || '%'
                    OR c.preferred_name ILIKE '%' || $4 || '%'
                    OR c.email ILIKE '%' || $4 || '%'
                    OR c.phone ILIKE '%' || $4 || '%'
                    OR c.mobile ILIKE '%' || $4 || '%'
                    OR o.name ILIKE '%' || $4 || '%')",
    entity: audit::Entity::Contact,
};

const ORGANIZATIONS: SubjectTables = SubjectTables {
    properties_table: "organization_properties",
    owner_column: "organization_id",
    owner_table: "organizations",
    alias: "o",
    from_joins: "FROM organizations o",
    select_columns: "o.id AS id, o.name AS name, o.kind AS subtitle, o.archived AS archived",
    // `$3` narrows to a single organization; the picker leaves it empty, but the
    // parameter stays in the same position as the people query so one binding
    // order serves both subjects.
    base_where: "WHERE ($1 OR NOT o.archived)
       AND ($2 = '' OR o.kind = $2)
       AND ($3 = '' OR o.id = $3)
       AND ($4 = '' OR o.name ILIKE '%' || $4 || '%'
                    OR o.email ILIKE '%' || $4 || '%')",
    entity: audit::Entity::Organization,
};

fn tables(subject: PropertySubject) -> &'static SubjectTables {
    match subject {
        PropertySubject::People => &PEOPLE,
        PropertySubject::Organizations => &ORGANIZATIONS,
    }
}

/// The SQL counterpart of [`new_crm_fields::normalize_property_part`]: lowercase
/// and collapse internal whitespace, so matching agrees with the Rust side.
fn normalized(column: &str) -> String {
    format!("lower(regexp_replace(btrim({column}), '\\s+', ' ', 'g'))")
}

#[derive(sqlx::FromRow)]
struct CandidateRow {
    id: String,
    name: String,
    subtitle: String,
    archived: bool,
    current_value: Option<String>,
}

fn to_candidate(subject: PropertySubject, row: CandidateRow) -> BulkPropertyCandidate {
    // Organizations carry their kind slug in `subtitle`; show its label instead.
    let subtitle = match subject {
        PropertySubject::People => row.subtitle,
        PropertySubject::Organizations => OrganizationKind::from_slug(&row.subtitle)
            .map(|kind| kind.label().to_string())
            .unwrap_or_default(),
    };
    BulkPropertyCandidate {
        id: row.id,
        name: row.name,
        subtitle,
        archived: row.archived,
        current_value: row.current_value,
    }
}

/// The three always-present text parameters, `$2` through `$4`.
fn base_params(subject: PropertySubject, filters: &BulkPropertyFilters) -> Vec<String> {
    let type_or_kind = match subject {
        PropertySubject::People => filters
            .contact_type
            .map(ContactType::slug)
            .unwrap_or_default()
            .to_string(),
        PropertySubject::Organizations => filters
            .organization_kind
            .map(OrganizationKind::slug)
            .unwrap_or_default()
            .to_string(),
    };
    let organization_id = match subject {
        PropertySubject::People => filters.organization_id.trim().to_string(),
        // Not offered for organizations; kept empty so the clause is inert.
        PropertySubject::Organizations => String::new(),
    };
    vec![
        type_or_kind,
        organization_id,
        filters.keyword.trim().to_string(),
    ]
}

/// The `EXISTS` / `NOT EXISTS` clause implementing "filter by a property the
/// record already carries". Returns an empty string when the condition is inert.
fn condition_sql(
    subject: PropertySubject,
    condition: &PropertyCondition,
    first_param: usize,
) -> String {
    if !condition.is_active() {
        return String::new();
    }
    let t = tables(subject);
    let key_param = first_param;
    let section_param = first_param + 1;
    let value_param = first_param + 2;
    let key = normalized("p.key");
    let section = normalized("p.section");

    let (negated, value_predicate) = match condition.match_kind {
        PropertyValueMatch::Missing => (true, String::new()),
        PropertyValueMatch::Blank => (false, " AND btrim(p.value) = ''".to_string()),
        PropertyValueMatch::Filled => (false, " AND btrim(p.value) <> ''".to_string()),
        PropertyValueMatch::Equals => (
            false,
            format!(" AND {} = ${value_param}", normalized("p.value")),
        ),
        PropertyValueMatch::Contains => (
            false,
            format!(" AND p.value ILIKE '%' || ${value_param} || '%'"),
        ),
        PropertyValueMatch::Any => (false, String::new()),
    };
    let exists = if negated { "NOT EXISTS" } else { "EXISTS" };
    format!(
        " AND {exists} (SELECT 1 FROM {table} p
             WHERE p.{owner} = {alias}.id
               AND {key} = ${key_param}
               AND {section} = ${section_param}{value_predicate})",
        table = t.properties_table,
        owner = t.owner_column,
        alias = t.alias,
    )
}

/// The parameters [`condition_sql`] expects, in order.
fn condition_params(condition: &PropertyCondition) -> Vec<String> {
    if !condition.is_active() {
        return Vec::new();
    }
    let (section, key) = condition.property.normalized();
    let mut params = vec![key, section];
    if condition.match_kind.needs_value() {
        params.push(match condition.match_kind {
            // `Equals` compares against the normalized value; `Contains` is a
            // raw substring match handled by ILIKE.
            PropertyValueMatch::Equals => {
                new_crm_fields::normalize_property_part(&condition.value)
            }
            _ => condition.value.trim().to_string(),
        });
    }
    params
}

/// A lateral join exposing the target property's current value, matched on the
/// same normalized pair the write path uses.
fn target_join(subject: PropertySubject, first_param: usize) -> String {
    let t = tables(subject);
    format!(
        "LEFT JOIN LATERAL (
             SELECT p.value FROM {table} p
             WHERE p.{owner} = {alias}.id
               AND {key} = ${key_param}
               AND {section} = ${section_param}
             ORDER BY p.ord ASC LIMIT 1
         ) target ON TRUE",
        table = t.properties_table,
        owner = t.owner_column,
        alias = t.alias,
        key = normalized("p.key"),
        section = normalized("p.section"),
        key_param = first_param,
        section_param = first_param + 1,
    )
}

fn target_params(target: &PropertyRef) -> Vec<String> {
    let (section, key) = target.normalized();
    vec![key, section]
}

/// One page of records to choose from, each with its current value for `target`.
pub async fn candidate_page(
    subject: PropertySubject,
    filters: &BulkPropertyFilters,
    target: &PropertyRef,
    offset: i64,
    limit: i64,
) -> Result<Page<BulkPropertyCandidate>, sqlx::Error> {
    let t = tables(subject);
    let inert = PropertyCondition::default();
    let condition = filters.condition.as_ref().unwrap_or(&inert);

    let mut params = base_params(subject, filters);
    let condition_clause = condition_sql(subject, condition, params.len() + 2);
    params.extend(condition_params(condition));

    let total_sql = format!(
        "SELECT count(*) {joins} {base}{condition_clause}",
        joins = t.from_joins,
        base = t.base_where,
    );
    let mut total_query = sqlx::query_scalar::<_, i64>(&total_sql).bind(filters.include_archived);
    for param in &params {
        total_query = total_query.bind(param);
    }
    let total = total_query.fetch_one(pool()).await?;

    let join = target_join(subject, params.len() + 2);
    params.extend(target_params(target));
    let limit_param = params.len() + 2;
    let offset_param = limit_param + 1;
    let rows_sql = format!(
        "SELECT {columns}, target.value AS current_value
         {joins} {join} {base}{condition_clause}
         ORDER BY {alias}.archived ASC, lower(name) ASC, {alias}.id ASC
         LIMIT ${limit_param} OFFSET ${offset_param}",
        columns = t.select_columns,
        joins = t.from_joins,
        base = t.base_where,
        alias = t.alias,
    );
    let mut rows_query =
        sqlx::query_as::<_, CandidateRow>(&rows_sql).bind(filters.include_archived);
    for param in &params {
        rows_query = rows_query.bind(param);
    }
    let rows = rows_query
        .bind(limit.clamp(1, 200))
        .bind(offset.max(0))
        .fetch_all(pool())
        .await?;

    Ok(Page {
        items: rows
            .into_iter()
            .map(|row| to_candidate(subject, row))
            .collect(),
        total,
    })
}

/// Turn a selection into the exact set of record ids that will be written.
///
/// "Everything matching" is re-resolved here rather than trusted from the
/// browser, and an explicit list is intersected with the subject's own table so
/// a stale or foreign id can never reach the write path.
pub async fn resolve_target_ids(
    subject: PropertySubject,
    selection: &BulkPropertySelection,
) -> Result<Vec<String>, sqlx::Error> {
    let t = tables(subject);
    if !selection.all_matching {
        let ids = clean_ids(&selection.ids);
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        return sqlx::query_scalar::<_, String>(&format!(
            "SELECT id FROM {table} WHERE id = ANY($1::text[]) ORDER BY id",
            table = t.owner_table,
        ))
        .bind(&ids)
        .fetch_all(pool())
        .await;
    }

    let filters = &selection.filters;
    let inert = PropertyCondition::default();
    let condition = filters.condition.as_ref().unwrap_or(&inert);
    let mut params = base_params(subject, filters);
    let condition_clause = condition_sql(subject, condition, params.len() + 2);
    params.extend(condition_params(condition));

    let sql = format!(
        "SELECT {alias}.id {joins} {base}{condition_clause} ORDER BY {alias}.id",
        alias = t.alias,
        joins = t.from_joins,
        base = t.base_where,
    );
    let mut query = sqlx::query_scalar::<_, String>(&sql).bind(filters.include_archived);
    for param in &params {
        query = query.bind(param);
    }
    let matching = query.fetch_all(pool()).await?;

    let excluded: HashSet<String> = clean_ids(&selection.excluded_ids).into_iter().collect();
    Ok(matching
        .into_iter()
        .filter(|id| !excluded.contains(id))
        .collect())
}

fn clean_ids(ids: &[String]) -> Vec<String> {
    let mut cleaned: Vec<String> = ids
        .iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect();
    cleaned.sort();
    cleaned.dedup();
    cleaned
}

#[derive(sqlx::FromRow)]
struct TargetRow {
    id: String,
    name: String,
    current_value: Option<String>,
    property_count: i64,
}

/// Load each target's current value for the property and how many properties it
/// already holds, so both preview and apply classify from the same data.
async fn target_rows(
    subject: PropertySubject,
    ids: &[String],
    target: &PropertyRef,
) -> Result<Vec<TargetRow>, sqlx::Error> {
    let t = tables(subject);
    let sql = format!(
        "SELECT {columns}, target.value AS current_value,
                (SELECT count(*) FROM {table} pc WHERE pc.{owner} = {alias}.id) AS property_count
         {joins} {join}
         WHERE {alias}.id = ANY($3::text[])
         ORDER BY lower(name) ASC, {alias}.id ASC",
        columns = t.select_columns,
        table = t.properties_table,
        owner = t.owner_column,
        alias = t.alias,
        joins = t.from_joins,
        join = target_join(subject, 1),
    );
    let params = target_params(target);
    sqlx::query_as::<_, TargetRow>(&sql)
        .bind(&params[0])
        .bind(&params[1])
        .bind(ids)
        .fetch_all(pool())
        .await
}

/// Count what a save would do, without writing anything.
pub async fn preview(
    subject: PropertySubject,
    ids: &[String],
    edit: &BulkPropertyEdit,
) -> Result<BulkPropertyPreview, sqlx::Error> {
    if ids.is_empty() {
        return Ok(BulkPropertyPreview::default());
    }
    let rows = target_rows(subject, ids, &edit.property).await?;
    let mut preview = BulkPropertyPreview {
        total: rows.len() as i64,
        ..Default::default()
    };
    for row in &rows {
        match row.current_value.as_deref() {
            Some(existing) if existing == edit.value => preview.unchanged += 1,
            Some(_) => preview.will_overwrite += 1,
            None if row.property_count >= MAX_PROPERTIES as i64 => preview.at_property_limit += 1,
            None => preview.will_add += 1,
        }
    }
    Ok(preview)
}

/// Set the property on every target in one transaction.
///
/// An existing row keeps its `ord` and its stored spelling of the section and
/// name: only the value changes, so a bulk save never reorders or re-cases a
/// record's list. A record that does not have the property gets one appended.
pub async fn apply(
    subject: PropertySubject,
    ids: &[String],
    edit: &BulkPropertyEdit,
    actor_user_id: &str,
    actor: &str,
) -> Result<BulkPropertyOutcome, sqlx::Error> {
    let t = tables(subject);
    let rows = target_rows(subject, ids, &edit.property).await?;
    let (section_norm, key_norm) = edit.property.normalized();
    let mut outcome = BulkPropertyOutcome::default();

    let mut tx = pool().begin().await?;
    audit::set_actor_in_transaction(&mut tx, actor_user_id).await?;

    for row in rows {
        let previous = match row.current_value.as_deref() {
            Some(existing) if existing == edit.value => {
                outcome.unchanged += 1;
                continue;
            }
            Some(existing) => {
                sqlx::query(&format!(
                    "UPDATE {table} SET value = $1
                     WHERE {owner} = $2 AND {key} = $3 AND {section} = $4",
                    table = t.properties_table,
                    owner = t.owner_column,
                    key = normalized("key"),
                    section = normalized("section"),
                ))
                .bind(&edit.value)
                .bind(&row.id)
                .bind(&key_norm)
                .bind(&section_norm)
                .execute(&mut *tx)
                .await?;
                outcome.overwritten += 1;
                existing.to_string()
            }
            None => {
                if row.property_count >= MAX_PROPERTIES as i64 {
                    outcome.skipped.push(row.name);
                    continue;
                }
                sqlx::query(&format!(
                    "INSERT INTO {table} ({owner}, ord, key, value, section)
                     SELECT $1, coalesce(max(ord) + 1, 0), $2, $3, $4
                     FROM {table} WHERE {owner} = $1",
                    table = t.properties_table,
                    owner = t.owner_column,
                ))
                .bind(&row.id)
                .bind(&edit.property.key)
                .bind(&edit.value)
                .bind(&edit.property.section)
                .execute(&mut *tx)
                .await?;
                outcome.added += 1;
                String::new()
            }
        };

        audit::record_in_transaction(
            &mut tx,
            t.entity,
            &row.id,
            actor,
            &format!("property: {}", edit.property.label()),
            &previous,
            &edit.value,
        )
        .await?;
    }

    tx.commit().await?;
    Ok(outcome)
}

/// The `(section, name)` pairs the picker offers: everything already in use for
/// this subject, plus the code-owned defaults so a fresh database still suggests
/// the standard fields.
pub async fn key_options(
    subject: PropertySubject,
) -> Result<Vec<PropertyKeyOption>, sqlx::Error> {
    let t = tables(subject);
    let rows = sqlx::query_as::<_, (String, String, i64)>(&format!(
        "SELECT btrim(section) AS section, btrim(key) AS key, count(DISTINCT {owner}) AS usage_count
         FROM {table}
         GROUP BY btrim(section), btrim(key)
         ORDER BY usage_count DESC, lower(btrim(section)) ASC, lower(btrim(key)) ASC",
        owner = t.owner_column,
        table = t.properties_table,
    ))
    .fetch_all(pool())
    .await?;

    let mut options: Vec<PropertyKeyOption> = rows
        .into_iter()
        .map(|(section, key, usage_count)| PropertyKeyOption {
            section,
            key,
            usage_count,
        })
        .collect();

    let mut seen: HashSet<(String, String)> = options
        .iter()
        .map(|option| new_crm_fields::normalized_property_key(&option.section, &option.key))
        .collect();
    let defaults: Box<dyn Iterator<Item = &'static new_crm_fields::DefaultPropertyField>> =
        match subject {
            PropertySubject::People => Box::new(new_crm_fields::person_properties()),
            PropertySubject::Organizations => Box::new(new_crm_fields::organization_properties()),
        };
    for field in defaults {
        if seen.insert(new_crm_fields::normalized_property_key(
            field.section,
            field.label,
        )) {
            options.push(PropertyKeyOption {
                section: field.section.to_string(),
                key: field.label.to_string(),
                usage_count: 0,
            });
        }
    }
    Ok(options)
}
