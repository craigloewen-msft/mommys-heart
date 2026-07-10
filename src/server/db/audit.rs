//! The unified append-only audit log shared by users and cases.

use crate::server::db::{ids, now_stamp};
use crate::types::ChangeLogEntry;
use sqlx::PgExecutor;

/// Which kind of entity an audit entry is attached to.
#[derive(Clone, Copy, Debug)]
pub enum Entity {
    User,
    Case,
}

impl Entity {
    fn as_str(self) -> &'static str {
        match self {
            Entity::User => "user",
            Entity::Case => "case",
        }
    }
}

#[derive(sqlx::FromRow)]
struct AuditRow {
    id: String,
    actor: String,
    field: String,
    old_value: String,
    new_value: String,
    at: String,
}

impl From<AuditRow> for ChangeLogEntry {
    fn from(r: AuditRow) -> Self {
        ChangeLogEntry {
            id: r.id,
            actor: r.actor,
            field: r.field,
            old_value: r.old_value,
            new_value: r.new_value,
            at: r.at,
        }
    }
}

/// Load an entity's audit log, newest first.
pub async fn for_entity<'e, E>(
    executor: E,
    entity: Entity,
    entity_id: &str,
) -> Result<Vec<ChangeLogEntry>, sqlx::Error>
where
    E: PgExecutor<'e>,
{
    let rows = sqlx::query_as::<_, AuditRow>(
        "SELECT id, actor, field, old_value, new_value, at
         FROM audit_log
         WHERE entity_type = $1 AND entity_id = $2
         ORDER BY seq DESC",
    )
    .bind(entity.as_str())
    .bind(entity_id)
    .fetch_all(executor)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// Append a new audit entry, allocating its id. `actor` is the display name of
/// whoever made the change.
pub async fn record(
    pool: &sqlx::PgPool,
    entity: Entity,
    entity_id: &str,
    actor: &str,
    field: &str,
    old_value: &str,
    new_value: &str,
) -> Result<(), sqlx::Error> {
    let id = ids::next(pool, "cl").await?;
    sqlx::query(
        "INSERT INTO audit_log (id, entity_type, entity_id, actor, field, old_value, new_value, at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(&id)
    .bind(entity.as_str())
    .bind(entity_id)
    .bind(actor)
    .bind(field)
    .bind(old_value)
    .bind(new_value)
    .bind(now_stamp())
    .execute(pool)
    .await?;
    Ok(())
}
