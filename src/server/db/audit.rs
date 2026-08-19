//! The unified append-only audit log shared by users and cases.

use crate::helpers::visibility::Visibility;
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
    Contact,
    Organization,
    Grant,
    Funding,
}

impl Entity {
    fn as_str(self) -> &'static str {
        match self {
            Entity::User => "user",
            Entity::Case => "case",
            Entity::Contact => "contact",
            Entity::Organization => "organization",
            Entity::Grant => "grant",
            Entity::Funding => "funding",
        }
    }
}

#[derive(sqlx::FromRow)]
struct AuditRow {
    id: String,
    actor_user_id: Option<String>,
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
            actor_user_id: r.actor_user_id.unwrap_or_default(),
            actor: r.actor,
            field: r.field,
            old_value: r.old_value,
            new_value: r.new_value,
            at: r.at,
        }
    }
}

// REQ-AUD-001..004: callers choose whether restricted metadata is visible;
// substantive message and note content never belongs in this table.
pub async fn page(
    entity: Entity,
    entity_id: &str,
    start: &str,
    end: &str,
    offset: i64,
    limit: i64,
    include_restricted: bool,
    restrict_contact_history: bool,
) -> Result<Page<ChangeLogEntry>, sqlx::Error> {
    let pool = pool();

    let total: i64 = sqlx::query_scalar(
        "SELECT count(*)
         FROM audit_log
         WHERE entity_type = $1 AND entity_id = $2
           AND ($3 = '' OR left(at, 10) >= $3)
           AND ($4 = '' OR left(at, 10) <= $4)
           AND ($5 OR COALESCE(visibility, 'shared') <> $6)
           AND (NOT $7 OR field NOT IN ('case link', 'account link'))",
    )
    .bind(entity.as_str())
    .bind(entity_id)
    .bind(start)
    .bind(end)
    .bind(include_restricted)
    .bind(Visibility::VolunteerOnly.slug())
    .bind(restrict_contact_history)
    .fetch_one(pool)
    .await?;

    let rows = sqlx::query_as::<_, AuditRow>(
        "SELECT id, actor_user_id, actor, field, old_value, new_value, at
         FROM audit_log
         WHERE entity_type = $1 AND entity_id = $2
           AND ($3 = '' OR left(at, 10) >= $3)
           AND ($4 = '' OR left(at, 10) <= $4)
           AND ($5 OR COALESCE(visibility, 'shared') <> $6)
           AND (NOT $7 OR field NOT IN ('case link', 'account link'))
         ORDER BY seq DESC
         LIMIT $8 OFFSET $9",
    )
    .bind(entity.as_str())
    .bind(entity_id)
    .bind(start)
    .bind(end)
    .bind(include_restricted)
    .bind(Visibility::VolunteerOnly.slug())
    .bind(restrict_contact_history)
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
    record_with_visibility(
        pool,
        entity,
        entity_id,
        actor,
        field,
        old_value,
        new_value,
        Visibility::Shared,
    )
    .await
}

/// Append an audit entry with explicit audience visibility.
pub async fn record_with_visibility(
    pool: &sqlx::PgPool,
    entity: Entity,
    entity_id: &str,
    actor: &str,
    field: &str,
    old_value: &str,
    new_value: &str,
    visibility: Visibility,
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
        visibility,
    )
    .await
}

/// Bind the authenticated account to this transaction. CRM audit writes inherit
/// the stable id without duplicating it at every repository call.
pub async fn set_actor_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor_user_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT set_config('app.actor_user_id', $1, true)")
        .bind(actor_user_id)
        .execute(&mut **transaction)
        .await?;
    Ok(())
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
    record_in_transaction_with_visibility(
        transaction,
        entity,
        entity_id,
        actor,
        field,
        old_value,
        new_value,
        Visibility::Shared,
    )
    .await
}

/// Append one audit entry per `(entity_id, old_value)` pair in a single
/// statement, all sharing the same field and new value.
///
/// A bulk edit touches many records at once; recording them one row at a time
/// would issue thousands of round trips inside a single transaction, holding
/// locks for the whole run.
pub async fn record_many_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    entity: Entity,
    entries: &[(String, String)],
    actor_user_id: &str,
    actor: &str,
    field: &str,
    new_value: &str,
) -> Result<(), sqlx::Error> {
    if entries.is_empty() {
        return Ok(());
    }
    let entity_ids: Vec<&str> = entries.iter().map(|(id, _)| id.as_str()).collect();
    let old_values: Vec<&str> = entries.iter().map(|(_, old)| old.as_str()).collect();
    sqlx::query(
        "INSERT INTO audit_log
            (id, entity_type, entity_id, actor_user_id, actor, field, old_value,
             new_value, at, visibility)
         SELECT 'cl-' || nextval('app_id_seq'), $1, entry.entity_id, $2, $3, $4,
                entry.old_value, $5, $6, $7
         FROM unnest($8::text[], $9::text[]) AS entry(entity_id, old_value)",
    )
    .bind(entity.as_str())
    .bind(actor_user_id)
    .bind(actor)
    .bind(field)
    .bind(new_value)
    .bind(now_stamp())
    .bind(Visibility::Shared.slug())
    .bind(&entity_ids)
    .bind(&old_values)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

/// Record a mutation with both stable actor identity and its historical display
/// snapshot. Legacy callers may continue using [`record_in_transaction`].
pub async fn record_in_transaction_by(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    entity: Entity,
    entity_id: &str,
    actor_user_id: &str,
    actor: &str,
    field: &str,
    old_value: &str,
    new_value: &str,
) -> Result<(), sqlx::Error> {
    record_with_connection_by(
        &mut **transaction,
        entity,
        entity_id,
        Some(actor_user_id),
        actor,
        field,
        old_value,
        new_value,
        Visibility::Shared,
    )
    .await
}

/// Stable-actor variant for restricted audit metadata.
pub async fn record_in_transaction_by_with_visibility(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    entity: Entity,
    entity_id: &str,
    actor_user_id: &str,
    actor: &str,
    field: &str,
    old_value: &str,
    new_value: &str,
    visibility: Visibility,
) -> Result<(), sqlx::Error> {
    record_with_connection_by(
        &mut **transaction,
        entity,
        entity_id,
        Some(actor_user_id),
        actor,
        field,
        old_value,
        new_value,
        visibility,
    )
    .await
}

/// Append an explicitly visible audit entry inside an existing transaction.
pub async fn record_in_transaction_with_visibility(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    entity: Entity,
    entity_id: &str,
    actor: &str,
    field: &str,
    old_value: &str,
    new_value: &str,
    visibility: Visibility,
) -> Result<(), sqlx::Error> {
    record_with_connection(
        &mut **transaction,
        entity,
        entity_id,
        actor,
        field,
        old_value,
        new_value,
        visibility,
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
    visibility: Visibility,
) -> Result<(), sqlx::Error> {
    record_with_connection_by(
        connection, entity, entity_id, None, actor, field, old_value, new_value, visibility,
    )
    .await
}

async fn record_with_connection_by(
    connection: &mut sqlx::PgConnection,
    entity: Entity,
    entity_id: &str,
    actor_user_id: Option<&str>,
    actor: &str,
    field: &str,
    old_value: &str,
    new_value: &str,
    visibility: Visibility,
) -> Result<(), sqlx::Error> {
    let id = ids::next(&mut *connection, "cl").await?;
    sqlx::query(
        "INSERT INTO audit_log
            (id, entity_type, entity_id, actor_user_id, actor, field, old_value,
             new_value, at, visibility)
         VALUES ($1, $2, $3,
                 COALESCE($4, NULLIF(current_setting('app.actor_user_id', true), '')),
                 $5, $6, $7, $8, $9, $10)",
    )
    .bind(&id)
    .bind(entity.as_str())
    .bind(entity_id)
    .bind(actor_user_id)
    .bind(actor)
    .bind(field)
    .bind(old_value)
    .bind(new_value)
    .bind(now_stamp())
    .bind(visibility.slug())
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
