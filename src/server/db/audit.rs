//! The unified append-only audit log shared by users and cases.

use crate::server::db::{ids, now_stamp, pool};
use crate::server_fns::audit::ChangeLogEntry;
use crate::server_fns::pagination::Page;

/// How long audit entries are retained before the background task prunes them.
const RETENTION_YEARS: i64 = 10;

/// How often the background retention task runs.
const RETENTION_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30 * 24 * 60 * 60);

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

pub async fn page(
    entity: Entity,
    entity_id: &str,
    start: &str,
    end: &str,
    offset: i64,
    limit: i64,
) -> Result<Page<ChangeLogEntry>, sqlx::Error> {
    let pool = pool();

    let total: i64 = sqlx::query_scalar(
        "SELECT count(*)
         FROM audit_log
         WHERE entity_type = $1 AND entity_id = $2
           AND ($3 = '' OR left(at, 10) >= $3)
           AND ($4 = '' OR left(at, 10) <= $4)",
    )
    .bind(entity.as_str())
    .bind(entity_id)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await?;

    let rows = sqlx::query_as::<_, AuditRow>(
        "SELECT id, actor, field, old_value, new_value, at
         FROM audit_log
         WHERE entity_type = $1 AND entity_id = $2
           AND ($3 = '' OR left(at, 10) >= $3)
           AND ($4 = '' OR left(at, 10) <= $4)
         ORDER BY seq DESC
         LIMIT $5 OFFSET $6",
    )
    .bind(entity.as_str())
    .bind(entity_id)
    .bind(start)
    .bind(end)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(Page {
        items: rows.into_iter().map(Into::into).collect(),
        total,
    })
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
    let mut connection = pool.acquire().await?;
    record_with_connection(
        &mut connection,
        entity,
        entity_id,
        actor,
        field,
        old_value,
        new_value,
    )
    .await
}

/// Append an audit entry as part of an existing transaction, so the audited
/// mutation and its audit record commit or roll back together.
pub async fn record_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    entity: Entity,
    entity_id: &str,
    actor: &str,
    field: &str,
    old_value: &str,
    new_value: &str,
) -> Result<(), sqlx::Error> {
    record_with_connection(
        &mut **transaction,
        entity,
        entity_id,
        actor,
        field,
        old_value,
        new_value,
    )
    .await
}

async fn record_with_connection(
    connection: &mut sqlx::PgConnection,
    entity: Entity,
    entity_id: &str,
    actor: &str,
    field: &str,
    old_value: &str,
    new_value: &str,
) -> Result<(), sqlx::Error> {
    let id = ids::next(&mut *connection, "cl").await?;
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
    .execute(connection)
    .await?;
    Ok(())
}

/// Delete audit entries older than the retention window ([`RETENTION_YEARS`]),
/// returning the number of rows removed. Retention is computed against the
/// machine-readable `created_at` timestamp (not the display-only `at` text).
pub async fn purge_expired(pool: &sqlx::PgPool) -> Result<u64, sqlx::Error> {
    let result =
        sqlx::query("DELETE FROM audit_log WHERE created_at < now() - make_interval(years => $1)")
            .bind(RETENTION_YEARS as i32)
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
