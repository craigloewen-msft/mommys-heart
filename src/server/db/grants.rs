//! Funding grants (minimal: id + name).

use crate::server::db::{ids, pool};
use crate::server_fns::grants::GrantSummary;

/// All grants, ordered by id — the grants screen's list-view shape.
pub async fn summaries() -> Result<Vec<GrantSummary>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String)>("SELECT id, name FROM grants ORDER BY id")
        .fetch_all(pool())
        .await?;
    Ok(rows
        .into_iter()
        .map(|(id, name)| GrantSummary { id, name })
        .collect())
}

/// Create a grant, returning its new id.
pub async fn create(name: &str) -> Result<String, sqlx::Error> {
    let id = ids::next(pool(), "g").await?;
    sqlx::query("INSERT INTO grants (id, name) VALUES ($1, $2)")
        .bind(&id)
        .bind(name)
        .execute(pool())
        .await?;
    Ok(id)
}

/// Rename a grant.
pub async fn rename(grant_id: &str, name: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE grants SET name = $1 WHERE id = $2")
        .bind(name)
        .bind(grant_id)
        .execute(pool())
        .await?;
    Ok(())
}

/// Delete a grant.
pub async fn delete(grant_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM grants WHERE id = $1")
        .bind(grant_id)
        .execute(pool())
        .await?;
    Ok(())
}

/// Insert a grant with a caller-supplied id (used by the seed).
pub async fn insert(id: &str, name: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO grants (id, name) VALUES ($1, $2)")
        .bind(id)
        .bind(name)
        .execute(pool())
        .await?;
    Ok(())
}
