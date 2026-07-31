//! Monotonic id generation backed by the `app_id_seq` Postgres sequence. This
//! replaces the in-memory counter that produced ids like `c-5001`, `n-5002`.
//!
//! Sequence-derived ids are fine for records that never appear in a URL, but
//! they are guessable and they leak how many records exist and in what order
//! they were created. Anything addressable by a URL should use [`opaque`]
//! instead — see [`crate::server::db::users::next_id`].

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

/// Mint an unguessable id as `<prefix>-<32 hex chars>` (128 bits from the OS
/// CSPRNG, e.g. `opaque("u")` -> `"u-3f9a...c1"`).
///
/// Used for records whose id is exposed in a URL, so that holding one id tells
/// you nothing about any other: the id space is far too sparse to enumerate,
/// and it encodes neither a position nor a creation time. Needs no round-trip
/// to the database, since it does not draw from the shared sequence.
pub fn opaque(prefix: &str) -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!("{prefix}-{}", hex::encode(bytes))
}
