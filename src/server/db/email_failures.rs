//! Durable, append-only log of outbound email delivery failures (SSR only).
//!
//! The email flows ([`crate::server::notifications`] and
//! [`crate::server::email::auth_notifications`]) call [`record`] whenever a send
//! ultimately fails, so the failure is browsable in the admin dashboard instead
//! of living only in the server logs. The admin viewer reads it back through
//! [`page`]. Mirrors [`crate::server::db::audit`]: newest-first via a monotonic
//! `seq`, with a background retention task pruning old rows.

use crate::server::db::{ids, now_stamp, pool};
use crate::server_fns::email_failures::EmailFailure;
use crate::server_fns::pagination::Page;

/// How long failure rows are retained before the background task prunes them.
const RETENTION_YEARS: i64 = 10;

/// How often the background retention task runs.
const RETENTION_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30 * 24 * 60 * 60);

#[derive(sqlx::FromRow)]
struct FailureRow {
    id: String,
    recipient: String,
    subject: String,
    context: String,
    error: String,
    at: String,
}

impl From<FailureRow> for EmailFailure {
    fn from(r: FailureRow) -> Self {
        EmailFailure {
            id: r.id,
            recipient: r.recipient,
            subject: r.subject,
            context: r.context,
            error: r.error,
            at: r.at,
        }
    }
}

/// One page of recorded failures, newest first.
pub async fn page(offset: i64, limit: i64) -> Result<Page<EmailFailure>, sqlx::Error> {
    let pool = pool();

    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM email_failures")
        .fetch_one(pool)
        .await?;

    let rows = sqlx::query_as::<_, FailureRow>(
        "SELECT id, recipient, subject, context, error, at
         FROM email_failures
         ORDER BY seq DESC
         LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(Page {
        items: rows.into_iter().map(Into::into).collect(),
        total,
    })
}

/// Everything recorded since `last_seq`, oldest first, for the admin activity
/// digest. Returns at most `limit` failures, a count of how many further ones
/// were left for the next run, and the new watermark to store once sent.
pub async fn since(
    last_seq: i64,
    limit: i64,
) -> Result<(Vec<EmailFailure>, i64, i64), sqlx::Error> {
    let pool = pool();

    let rows = sqlx::query_as::<_, FailureRow>(
        "SELECT id, recipient, subject, context, error, at
         FROM email_failures
         WHERE seq > $1
         ORDER BY seq
         LIMIT $2",
    )
    .bind(last_seq)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    // The watermark advances only over what is actually reported, so a capped
    // batch leaves the remainder for the next run.
    let next: i64 = sqlx::query_scalar(
        "SELECT COALESCE(max(seq), $1) FROM (
             SELECT seq FROM email_failures WHERE seq > $1 ORDER BY seq LIMIT $2
         ) reported",
    )
    .bind(last_seq)
    .bind(limit)
    .fetch_one(pool)
    .await?;

    let extra: i64 = sqlx::query_scalar("SELECT count(*) FROM email_failures WHERE seq > $1")
        .bind(next)
        .fetch_one(pool)
        .await?;

    Ok((rows.into_iter().map(Into::into).collect(), extra, next))
}

/// Append one failure. Best-effort and self-contained: this is called from
/// already-failing, best-effort paths, so it never returns an error — if the
/// insert itself fails it is logged and swallowed rather than masking the
/// original send failure.
pub async fn record(recipient: &str, subject: &str, context: &str, error: &str) {
    let pool = pool();
    let id = match ids::next(pool, "ef").await {
        Ok(id) => id,
        Err(e) => {
            tracing::warn!("email_failures: could not allocate id for {recipient}: {e}");
            return;
        }
    };
    if let Err(e) = sqlx::query(
        "INSERT INTO email_failures (id, recipient, subject, context, error, at)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(&id)
    .bind(recipient)
    .bind(subject)
    .bind(context)
    .bind(error)
    .bind(now_stamp())
    .execute(pool)
    .await
    {
        tracing::warn!("email_failures: could not persist failure for {recipient}: {e}");
    }
}

/// Delete failure rows older than the retention window ([`RETENTION_YEARS`]),
/// returning the number removed. Retention is computed against the
/// machine-readable `created_at` timestamp (not the display-only `at` text).
pub async fn purge_expired(pool: &sqlx::PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM email_failures WHERE created_at < now() - make_interval(years => $1)",
    )
    .bind(RETENTION_YEARS as i32)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// Spawn a background task that prunes expired failure rows at startup and then
/// once every [`RETENTION_INTERVAL`]. Mirrors
/// [`crate::server::db::audit::start_retention_task`]: failures are logged and
/// never crash the server.
pub fn start_retention_task() {
    tokio::spawn(async {
        loop {
            match purge_expired(pool()).await {
                Ok(n) => tracing::info!("email-failure retention: deleted {n} expired entries"),
                Err(e) => tracing::warn!("email-failure retention failed: {e}"),
            }
            tokio::time::sleep(RETENTION_INTERVAL).await;
        }
    });
}
