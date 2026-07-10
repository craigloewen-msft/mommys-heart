//! Monotonic id generation backed by the `app_id_seq` Postgres sequence. This
//! replaces the in-memory counter that produced ids like `c-5001`, `n-5002`.

use sqlx::PgExecutor;

/// Fetch the next id as `<prefix>-<n>` (e.g. `next("c")` -> `"c-5001"`).
pub async fn next<'e, E>(executor: E, prefix: &str) -> Result<String, sqlx::Error>
where
    E: PgExecutor<'e>,
{
    let n: i64 = sqlx::query_scalar("SELECT nextval('app_id_seq')")
        .fetch_one(executor)
        .await?;
    Ok(format!("{prefix}-{n}"))
}
