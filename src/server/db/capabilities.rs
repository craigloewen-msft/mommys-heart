//! Per-case capability lookups against the `case_assignments` table.
//!
//! Resolves the capabilities a user holds on one or more cases so that case
//! data can carry the caller's rights, instead of relying on a cached copy of
//! the user's assignments.

use crate::server::db::pool;
use crate::server_fns::capabilities::CaseCapability;
use std::collections::HashMap;

/// The capabilities `user_id` holds on a single `case_id`, read from
/// `case_assignments` (empty when the user is unassigned).
pub async fn get_single_case(user_id: &str, case_id: &str) -> Result<Vec<CaseCapability>, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT capability FROM case_assignments WHERE user_id = $1 AND case_id = $2",
    )
    .bind(user_id)
    .bind(case_id)
    .fetch_all(pool())
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|(cap,)| CaseCapability::from_slug(&cap))
        .collect())
}

/// The capabilities `user_id` holds on each of `case_ids`, resolved in a single
/// query and grouped by case id. Cases the user is unassigned to are absent from
/// the map. Used to attach per-case rights to a page of summaries without an
/// N+1 query.
pub async fn get_multi_case(
    user_id: &str,
    case_ids: &[String],
) -> Result<HashMap<String, Vec<CaseCapability>>, sqlx::Error> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT case_id, capability FROM case_assignments \
         WHERE user_id = $1 AND case_id = ANY($2)",
    )
    .bind(user_id)
    .bind(case_ids)
    .fetch_all(pool())
    .await?;
    let mut map: HashMap<String, Vec<CaseCapability>> = HashMap::new();
    for (case_id, cap) in rows {
        if let Some(c) = CaseCapability::from_slug(&cap) {
            map.entry(case_id).or_default().push(c);
        }
    }
    Ok(map)
}
