//! Organization persistence (SSR only): the outside bodies the foundation deals
//! with.
//!
//! Organizations are archived, never deleted: contacts and grants reference them
//! with `ON DELETE RESTRICT`, so retiring one must not erase the history that
//! points at it.

use crate::server::db::{audit, ids, organization_properties, pool, property_filters};
use crate::server_fns::organizations::{
    ActiveOrganizationSummary, Organization, OrganizationFilters, OrganizationInput,
    OrganizationKind,
};
use crate::server_fns::pagination::Page;
use crate::server_fns::property_filters::PropertySubject;

#[derive(sqlx::FromRow)]
struct OrganizationRow {
    id: String,
    name: String,
    kind: String,
    website: String,
    phone: String,
    email: String,
    address: String,
    description: String,
    archived: bool,
    contact_count: i64,
}

impl From<OrganizationRow> for Organization {
    fn from(row: OrganizationRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            kind: OrganizationKind::from_slug(&row.kind).unwrap_or_default(),
            website: row.website,
            phone: row.phone,
            email: row.email,
            address: row.address,
            description: row.description,
            archived: row.archived,
            contact_count: row.contact_count,
            filtered_properties: Vec::new(),
        }
    }
}

#[derive(sqlx::FromRow)]
struct ActiveOrganizationRow {
    id: String,
    name: String,
    kind: String,
}

impl From<ActiveOrganizationRow> for ActiveOrganizationSummary {
    fn from(row: ActiveOrganizationRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            kind: OrganizationKind::from_slug(&row.kind).unwrap_or_default(),
        }
    }
}

fn escaped_like_pattern(query: &str) -> Option<String> {
    let term = query.trim();
    if term.is_empty() {
        None
    } else {
        Some(format!(
            "%{}%",
            term.replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        ))
    }
}

/// The `SELECT` list every read shares, including the joined contact count.
const SELECT_COLUMNS: &str = "o.id, o.name, o.kind, o.website, o.phone, o.email, o.address,
     o.description, o.archived,
     (SELECT count(*) FROM contacts c WHERE c.organization_id = o.id) AS contact_count";

/// One page of the directory. Archived rows are excluded unless asked for, so
/// pickers get only organizations that can still be chosen.
pub async fn page(
    filters: &OrganizationFilters,
    offset: i64,
    limit: i64,
) -> Result<Page<Organization>, sqlx::Error> {
    let keyword = filters.keyword.trim();
    let kind = filters.kind.map(|k| k.slug()).unwrap_or_default();
    // A typed word also matches a property value, so "Boston" finds an
    // organization whose Location says so without building a filter first.
    let property_search_sql =
        property_filters::keyword_match_sql(PropertySubject::Organization, "o.id", 3);
    let property_filter_sql =
        property_filters::predicate_sql(PropertySubject::Organization, "o.id", 4);
    let filters_json = property_filters::to_json(&filters.property_filters);

    let where_sql = format!(
        "WHERE ($1 OR NOT o.archived)
           AND ($2 = '' OR o.kind = $2)
           AND ($3 = '' OR o.name ILIKE '%' || $3 || '%' OR o.email ILIKE '%' || $3 || '%'
                {property_search_sql})
           {property_filter_sql}"
    );

    let total: i64 =
        sqlx::query_scalar(&format!("SELECT count(*) FROM organizations o {where_sql}"))
            .bind(filters.include_archived)
            .bind(kind)
            .bind(keyword)
            .bind(&filters_json)
            .fetch_one(pool())
            .await?;

    let rows = sqlx::query_as::<_, OrganizationRow>(&format!(
        "SELECT {SELECT_COLUMNS} FROM organizations o {where_sql}
         ORDER BY o.archived ASC, lower(o.name) ASC
         OFFSET $5 LIMIT $6"
    ))
    .bind(filters.include_archived)
    .bind(kind)
    .bind(keyword)
    .bind(&filters_json)
    .bind(offset.max(0))
    .bind(limit.clamp(1, 200))
    .fetch_all(pool())
    .await?;

    let mut items: Vec<Organization> = rows.into_iter().map(Into::into).collect();
    // The values behind the active chips, so each card can name why it matched.
    let ids: Vec<String> = items.iter().map(|item| item.id.clone()).collect();
    let keys: Vec<String> = filters
        .property_filters
        .iter()
        .map(|filter| filter.key.clone())
        .collect();
    let matched = property_filters::values_for(PropertySubject::Organization, &ids, &keys).await?;
    for item in &mut items {
        item.filtered_properties = matched.get(&item.id).cloned().unwrap_or_default();
    }

    Ok(Page { items, total })
}

pub async fn get(id: &str) -> Result<Option<Organization>, sqlx::Error> {
    let row = sqlx::query_as::<_, OrganizationRow>(&format!(
        "SELECT {SELECT_COLUMNS} FROM organizations o WHERE o.id = $1"
    ))
    .bind(id)
    .fetch_optional(pool())
    .await?;
    Ok(row.map(Into::into))
}

/// Every organization that may still be chosen, for legacy pickers.
pub async fn active_options() -> Result<Vec<(String, String)>, sqlx::Error> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, name FROM organizations WHERE NOT archived ORDER BY lower(name) ASC",
    )
    .fetch_all(pool())
    .await?;
    Ok(rows)
}

/// Active organizations for typeahead pickers, with only the fields the picker needs.
pub async fn search_active(
    query: &str,
    limit: i64,
) -> Result<Vec<ActiveOrganizationSummary>, sqlx::Error> {
    let limit = limit.clamp(1, 50);
    let pattern = escaped_like_pattern(query);

    let rows = sqlx::query_as::<_, ActiveOrganizationRow>(
        "SELECT id, name, kind FROM organizations
         WHERE NOT archived
           AND ($1::text IS NULL OR name ILIKE $1 OR email ILIKE $1)
         ORDER BY lower(name) ASC, seq DESC
         LIMIT $2",
    )
    .bind(&pattern)
    .bind(limit)
    .fetch_all(pool())
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

/// Create an organization and audit it in one transaction, so an unaudited row
/// can never exist.
pub async fn create(
    input: &OrganizationInput,
    actor_user_id: &str,
    actor: &str,
) -> Result<String, sqlx::Error> {
    let mut tx = pool().begin().await?;
    audit::set_actor_in_transaction(&mut tx, actor_user_id).await?;
    let id = ids::next(&mut *tx, "org").await?;
    sqlx::query(
        "INSERT INTO organizations
             (id, name, kind, website, phone, email, address, description)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(&id)
    .bind(&input.name)
    .bind(input.kind.slug())
    .bind(&input.website)
    .bind(&input.phone)
    .bind(&input.email)
    .bind(&input.address)
    .bind(&input.description)
    .execute(&mut *tx)
    .await?;
    organization_properties::add_defaults_for_new_organization(&mut tx, &id).await?;
    audit::record_in_transaction(
        &mut tx,
        audit::Entity::Organization,
        &id,
        actor,
        "organization",
        "",
        "created",
    )
    .await?;
    tx.commit().await?;
    Ok(id)
}

pub async fn update(
    id: &str,
    input: &OrganizationInput,
    actor_user_id: &str,
    actor: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = pool().begin().await?;
    audit::set_actor_in_transaction(&mut tx, actor_user_id).await?;
    let updated = sqlx::query(
        "UPDATE organizations
         SET name = $2, kind = $3, website = $4, phone = $5, email = $6,
             address = $7, description = $8, updated_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .bind(&input.name)
    .bind(input.kind.slug())
    .bind(&input.website)
    .bind(&input.phone)
    .bind(&input.email)
    .bind(&input.address)
    .bind(&input.description)
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(sqlx::Error::RowNotFound);
    }
    audit::record_in_transaction(
        &mut tx,
        audit::Entity::Organization,
        id,
        actor,
        "organization",
        "",
        "updated",
    )
    .await?;
    tx.commit().await
}

pub async fn set_archived(
    id: &str,
    archived: bool,
    actor_user_id: &str,
    actor: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = pool().begin().await?;
    audit::set_actor_in_transaction(&mut tx, actor_user_id).await?;
    let updated =
        sqlx::query("UPDATE organizations SET archived = $2, updated_at = now() WHERE id = $1")
            .bind(id)
            .bind(archived)
            .execute(&mut *tx)
            .await?;
    if updated.rows_affected() == 0 {
        return Err(sqlx::Error::RowNotFound);
    }
    audit::record_in_transaction(
        &mut tx,
        audit::Entity::Organization,
        id,
        actor,
        "organization",
        "",
        if archived { "archived" } else { "restored" },
    )
    .await?;
    tx.commit().await
}
