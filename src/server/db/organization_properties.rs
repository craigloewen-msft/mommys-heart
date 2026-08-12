//! Organization property persistence (SSR only): the `organization_properties` table.

use std::collections::HashSet;

use crate::helpers::new_crm_fields;
use crate::server::db::{audit, pool};
use crate::server_fns::organization_properties::{default_properties, OrganizationProperty};

#[derive(sqlx::FromRow)]
struct PropertyRow {
    ord: i32,
    key: String,
    value: String,
    section: String,
}

fn to_property(row: PropertyRow) -> OrganizationProperty {
    OrganizationProperty {
        key: row.key,
        value: row.value,
        section: row.section,
    }
}

async fn list_rows(organization_id: &str) -> Result<Vec<PropertyRow>, sqlx::Error> {
    sqlx::query_as::<_, PropertyRow>(
        "SELECT ord, key, value, section FROM organization_properties
         WHERE organization_id = $1 ORDER BY ord ASC",
    )
    .bind(organization_id)
    .fetch_all(pool())
    .await
}

async fn list_rows_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    organization_id: &str,
) -> Result<Vec<PropertyRow>, sqlx::Error> {
    sqlx::query_as::<_, PropertyRow>(
        "SELECT ord, key, value, section FROM organization_properties
         WHERE organization_id = $1 ORDER BY ord ASC",
    )
    .bind(organization_id)
    .fetch_all(&mut **tx)
    .await
}

async fn insert_at(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    organization_id: &str,
    ord: i32,
    property: &OrganizationProperty,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO organization_properties (organization_id, ord, key, value, section)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(organization_id)
    .bind(ord)
    .bind(&property.key)
    .bind(&property.value)
    .bind(&property.section)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// An organization's properties in display order.
pub async fn list(organization_id: &str) -> Result<Vec<OrganizationProperty>, sqlx::Error> {
    Ok(list_rows(organization_id)
        .await?
        .into_iter()
        .map(to_property)
        .collect())
}

/// Append any missing code-owned defaults, preserving every existing row and its order.
pub async fn ensure_defaults_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    organization_id: &str,
) -> Result<bool, sqlx::Error> {
    let existing = list_rows_in_transaction(tx, organization_id).await?;
    let mut present: HashSet<(String, String)> = existing
        .iter()
        .map(|row| new_crm_fields::normalized_property_key(&row.section, &row.key))
        .collect();
    let mut next_ord = existing.last().map(|row| row.ord + 1).unwrap_or(0);
    let mut changed = false;

    for property in default_properties() {
        let normalized = new_crm_fields::normalized_property_key(&property.section, &property.key);
        if present.insert(normalized) {
            insert_at(tx, organization_id, next_ord, &property).await?;
            next_ord += 1;
            changed = true;
        }
    }
    Ok(changed)
}

/// Insert the standard starting properties inside the caller's transaction.
pub async fn add_defaults_for_new_organization(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    organization_id: &str,
) -> Result<(), sqlx::Error> {
    let _ = ensure_defaults_in_transaction(tx, organization_id).await?;
    Ok(())
}

/// Replace an organization's whole property list, auditing once when it actually changes.
pub async fn replace(
    organization_id: &str,
    properties: Vec<OrganizationProperty>,
    actor: &str,
) -> Result<(), sqlx::Error> {
    let existing = list(organization_id).await?;
    let changed = existing != properties;

    let mut tx = pool().begin().await?;
    sqlx::query("DELETE FROM organization_properties WHERE organization_id = $1")
        .bind(organization_id)
        .execute(&mut *tx)
        .await?;
    for (ord, property) in properties.iter().enumerate() {
        insert_at(&mut tx, organization_id, ord as i32, property).await?;
    }
    if changed {
        audit::record_in_transaction(
            &mut tx,
            audit::Entity::Organization,
            organization_id,
            actor,
            "properties",
            "",
            "updated",
        )
        .await?;
    }
    tx.commit().await
}

/// Add the missing defaults once, auditing only when rows were appended.
pub async fn add_missing_defaults(organization_id: &str, actor: &str) -> Result<(), sqlx::Error> {
    let mut tx = pool().begin().await?;
    let changed = ensure_defaults_in_transaction(&mut tx, organization_id).await?;
    if changed {
        audit::record_in_transaction(
            &mut tx,
            audit::Entity::Organization,
            organization_id,
            actor,
            "properties",
            "",
            "added missing defaults",
        )
        .await?;
    }
    tx.commit().await
}
