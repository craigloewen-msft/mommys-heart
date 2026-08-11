//! Organization persistence (SSR only): the outside bodies the foundation deals
//! with.
//!
//! Organizations are archived, never deleted: contacts and grants reference them
//! with `ON DELETE RESTRICT`, so retiring one must not erase the history that
//! points at it.

use crate::server::db::{audit, ids, pool};
use crate::server_fns::organizations::{
    Organization, OrganizationFilters, OrganizationInput, OrganizationKind,
};
use crate::server_fns::pagination::Page;

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
        }
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

    let where_sql = "WHERE ($1 OR NOT o.archived)
           AND ($2 = '' OR o.kind = $2)
           AND ($3 = '' OR o.name ILIKE '%' || $3 || '%' OR o.email ILIKE '%' || $3 || '%')";

    let total: i64 =
        sqlx::query_scalar(&format!("SELECT count(*) FROM organizations o {where_sql}"))
            .bind(filters.include_archived)
            .bind(kind)
            .bind(keyword)
            .fetch_one(pool())
            .await?;

    let rows = sqlx::query_as::<_, OrganizationRow>(&format!(
        "SELECT {SELECT_COLUMNS} FROM organizations o {where_sql}
         ORDER BY o.archived ASC, lower(o.name) ASC
         OFFSET $4 LIMIT $5"
    ))
    .bind(filters.include_archived)
    .bind(kind)
    .bind(keyword)
    .bind(offset.max(0))
    .bind(limit.clamp(1, 200))
    .fetch_all(pool())
    .await?;

    Ok(Page {
        items: rows.into_iter().map(Into::into).collect(),
        total,
    })
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

/// Every organization that may still be chosen, for pickers.
pub async fn active_options() -> Result<Vec<(String, String)>, sqlx::Error> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, name FROM organizations WHERE NOT archived ORDER BY lower(name) ASC",
    )
    .fetch_all(pool())
    .await?;
    Ok(rows)
}

/// Create an organization and audit it in one transaction, so an unaudited row
/// can never exist.
pub async fn create(input: &OrganizationInput, actor: &str) -> Result<String, sqlx::Error> {
    let id = ids::next(pool(), "org").await?;
    let mut tx = pool().begin().await?;
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

pub async fn update(id: &str, input: &OrganizationInput, actor: &str) -> Result<(), sqlx::Error> {
    let mut tx = pool().begin().await?;
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

pub async fn set_archived(id: &str, archived: bool, actor: &str) -> Result<(), sqlx::Error> {
    let mut tx = pool().begin().await?;
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
