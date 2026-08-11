//! Contact property persistence (SSR only): the `contact_properties` table.
//!
//! The same replace-in-place write as [`crate::server::db::case_properties`],
//! but simpler: a contact has exactly one property list, so there is no
//! visibility to scope the delete to and no second list an edit could destroy.

use crate::server::db::{audit, pool};
use crate::server_fns::contact_properties::ContactProperty;

/// A contact's properties in display order.
pub async fn list(contact_id: &str) -> Result<Vec<ContactProperty>, sqlx::Error> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT key, value, section FROM contact_properties
         WHERE contact_id = $1 ORDER BY ord ASC",
    )
    .bind(contact_id)
    .fetch_all(pool())
    .await?;

    Ok(rows
        .into_iter()
        .map(|(key, value, section)| ContactProperty {
            key,
            value,
            section,
        })
        .collect())
}

/// Replace a contact's whole property list, auditing once when it actually
/// changes so a no-op save does not add noise to the Change Log.
pub async fn replace(
    contact_id: &str,
    properties: Vec<ContactProperty>,
    actor: &str,
) -> Result<(), sqlx::Error> {
    let existing = list(contact_id).await?;
    let changed = existing != properties;

    let mut tx = pool().begin().await?;
    sqlx::query("DELETE FROM contact_properties WHERE contact_id = $1")
        .bind(contact_id)
        .execute(&mut *tx)
        .await?;
    for (ord, property) in properties.iter().enumerate() {
        sqlx::query(
            "INSERT INTO contact_properties (contact_id, ord, key, value, section)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(contact_id)
        .bind(ord as i32)
        .bind(&property.key)
        .bind(&property.value)
        .bind(&property.section)
        .execute(&mut *tx)
        .await?;
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
