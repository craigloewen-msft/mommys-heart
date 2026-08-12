//! Contact property persistence (SSR only): the `contact_properties` table.
//!
//! The same replace-in-place write as [`crate::server::db::case_properties`],
//! but simpler: a contact has exactly one property list, so there is no
//! visibility to scope the delete to and no second list an edit could destroy.

use std::collections::HashSet;

use crate::helpers::new_crm_fields;
use crate::server::db::{audit, pool};
use crate::server_fns::contact_properties::{default_properties, ContactProperty};

#[derive(sqlx::FromRow)]
struct PropertyRow {
    ord: i32,
    key: String,
    value: String,
    section: String,
}

fn to_property(row: PropertyRow) -> ContactProperty {
    ContactProperty {
        key: row.key,
        value: row.value,
        section: row.section,
    }
}

async fn list_rows(contact_id: &str) -> Result<Vec<PropertyRow>, sqlx::Error> {
    sqlx::query_as::<_, PropertyRow>(
        "SELECT ord, key, value, section FROM contact_properties
         WHERE contact_id = $1 ORDER BY ord ASC",
    )
    .bind(contact_id)
    .fetch_all(pool())
    .await
}

async fn list_rows_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    contact_id: &str,
) -> Result<Vec<PropertyRow>, sqlx::Error> {
    sqlx::query_as::<_, PropertyRow>(
        "SELECT ord, key, value, section FROM contact_properties
         WHERE contact_id = $1 ORDER BY ord ASC",
    )
    .bind(contact_id)
    .fetch_all(&mut **tx)
    .await
}

async fn insert_at(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    contact_id: &str,
    ord: i32,
    property: &ContactProperty,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO contact_properties (contact_id, ord, key, value, section)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(contact_id)
    .bind(ord)
    .bind(&property.key)
    .bind(&property.value)
    .bind(&property.section)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// A contact's properties in display order.
pub async fn list(contact_id: &str) -> Result<Vec<ContactProperty>, sqlx::Error> {
    Ok(list_rows(contact_id)
        .await?
        .into_iter()
        .map(to_property)
        .collect())
}

/// Append any missing code-owned defaults, preserving every existing row and its order.
pub async fn ensure_defaults_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    contact_id: &str,
) -> Result<bool, sqlx::Error> {
    let existing = list_rows_in_transaction(tx, contact_id).await?;
    let mut present: HashSet<(String, String)> = existing
        .iter()
        .map(|row| new_crm_fields::normalized_property_key(&row.section, &row.key))
        .collect();
    let mut next_ord = existing.last().map(|row| row.ord + 1).unwrap_or(0);
    let mut changed = false;

    for property in default_properties() {
        let normalized = new_crm_fields::normalized_property_key(&property.section, &property.key);
        if present.insert(normalized) {
            insert_at(tx, contact_id, next_ord, &property).await?;
            next_ord += 1;
            changed = true;
        }
    }
    Ok(changed)
}

/// Insert the standard starting properties inside the caller's transaction.
pub async fn add_defaults_for_new_contact(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    contact_id: &str,
) -> Result<(), sqlx::Error> {
    let _ = ensure_defaults_in_transaction(tx, contact_id).await?;
    Ok(())
}

/// Replace a contact's whole property list, auditing once when it actually
/// changes so a no-op save does not add noise to the Change Log.
pub async fn replace(
    contact_id: &str,
    properties: Vec<ContactProperty>,
    actor_user_id: &str,
    actor: &str,
) -> Result<(), sqlx::Error> {
    let existing = list(contact_id).await?;
    let changed = existing != properties;

    let mut tx = pool().begin().await?;
    audit::set_actor_in_transaction(&mut tx, actor_user_id).await?;
    sqlx::query("DELETE FROM contact_properties WHERE contact_id = $1")
        .bind(contact_id)
        .execute(&mut *tx)
        .await?;
    for (ord, property) in properties.iter().enumerate() {
        insert_at(&mut tx, contact_id, ord as i32, property).await?;
    }
    if changed {
        audit::record_in_transaction(
            &mut tx,
            audit::Entity::Contact,
            contact_id,
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
pub async fn add_missing_defaults(
    contact_id: &str,
    actor_user_id: &str,
    actor: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = pool().begin().await?;
    audit::set_actor_in_transaction(&mut tx, actor_user_id).await?;
    let changed = ensure_defaults_in_transaction(&mut tx, contact_id).await?;
    if changed {
        audit::record_in_transaction(
            &mut tx,
            audit::Entity::Contact,
            contact_id,
            actor,
            "properties",
            "",
            "added missing defaults",
        )
        .await?;
    }
    tx.commit().await
}
