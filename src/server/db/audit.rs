//! The unified append-only audit log shared by users and cases.

use crate::server::db::{ids, now_stamp, pool};
use crate::server_fns::audit::ChangeLogEntry;
use sqlx::PgExecutor;
use std::collections::HashMap;

/// How long audit entries are retained before the background task prunes them.
const RETENTION_MONTHS: i64 = 12;

/// How often the background retention task runs.
const RETENTION_INTERVAL: std::time::Duration = std::time::Duration::from_secs(24 * 60 * 60);

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

/// Load the audit logs for many entities of the same type in a single query,
/// grouped by entity id (each list newest-first). Used by the paged list views
/// to avoid running one audit query per row (an N+1 pattern).
pub async fn for_entities<'e, E>(
    executor: E,
    entity: Entity,
    entity_ids: &[String],
) -> Result<HashMap<String, Vec<ChangeLogEntry>>, sqlx::Error>
where
    E: PgExecutor<'e>,
{
    if entity_ids.is_empty() {
        return Ok(HashMap::new());
    }

    #[derive(sqlx::FromRow)]
    struct Row {
        entity_id: String,
        id: String,
        actor: String,
        field: String,
        old_value: String,
        new_value: String,
        at: String,
    }

    let rows = sqlx::query_as::<_, Row>(
        "SELECT entity_id, id, actor, field, old_value, new_value, at
         FROM audit_log
         WHERE entity_type = $1 AND entity_id = ANY($2)
         ORDER BY entity_id, seq DESC",
    )
    .bind(entity.as_str())
    .bind(entity_ids)
    .fetch_all(executor)
    .await?;

    let mut map: HashMap<String, Vec<ChangeLogEntry>> = HashMap::new();
    for r in rows {
        map.entry(r.entity_id).or_default().push(ChangeLogEntry {
            id: r.id,
            actor: r.actor,
            field: r.field,
            old_value: r.old_value,
            new_value: r.new_value,
            at: r.at,
        });
    }
    Ok(map)
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

/// Delete audit entries older than the retention window ([`RETENTION_MONTHS`]),
/// returning the number of rows removed. Retention is computed against the
/// machine-readable `created_at` timestamp (not the display-only `at` text).
pub async fn purge_expired(pool: &sqlx::PgPool) -> Result<u64, sqlx::Error> {
    let result =
        sqlx::query("DELETE FROM audit_log WHERE created_at < now() - make_interval(months => $1)")
            .bind(RETENTION_MONTHS as i32)
            .execute(pool)
            .await?;
    Ok(result.rows_affected())
}

/// Spawn a background task that prunes expired audit entries at startup and then
/// once every [`RETENTION_INTERVAL`]. Mirrors [`crate::server::rag::start_background_ingest`]:
/// failures are logged and never crash the server.
pub fn start_retention_task() {
    tokio::spawn(async {
        loop {
            match purge_expired(pool()).await {
                Ok(n) => tracing::info!("audit retention: deleted {n} expired entries"),
                Err(e) => tracing::warn!("audit retention failed: {e}"),
            }
            tokio::time::sleep(RETENTION_INTERVAL).await;
        }
    });
}
