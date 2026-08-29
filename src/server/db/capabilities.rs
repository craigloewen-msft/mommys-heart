//! Per-case capability lookups against the `case_assignments` table.
//!
//! Resolves the capabilities a user holds on one or more cases so that case
//! data can carry the caller's rights, instead of relying on a cached copy of
//! the user's assignments. A role with full case access (site admin) holds every
//! capability without any stored row, so those lookups skip the query.

use crate::server::db::pool;
use crate::server_fns::capabilities::CaseCapability;
use crate::server_fns::users::User;
use std::collections::HashMap;

/// The capabilities `user` holds on a single `case_id`: all of them for a role
/// with full case access, else the stored `case_assignments` rows (empty when
/// the user is unassigned).
pub async fn get_single_case(
    user: &User,
    case_id: &str,
) -> Result<Vec<CaseCapability>, sqlx::Error> {
    if user.role.has_full_case_access() {
        return Ok(CaseCapability::ALL.to_vec());
    }
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT capability FROM case_assignments WHERE user_id = $1 AND case_id = $2",
    )
    .bind(&user.id)
    .bind(case_id)
    .fetch_all(pool())
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|(cap,)| CaseCapability::from_slug(&cap))
        .collect())
}

/// The capabilities `user` holds on each of `case_ids`, resolved in a single
/// query and grouped by case id. Cases the user is unassigned to are absent from
/// the map. Used to attach per-case rights to a page of summaries without an
/// N+1 query.
pub async fn get_multi_case(
    user: &User,
    case_ids: &[String],
) -> Result<HashMap<String, Vec<CaseCapability>>, sqlx::Error> {
    if user.role.has_full_case_access() {
        return Ok(case_ids
            .iter()
            .map(|id| (id.clone(), CaseCapability::ALL.to_vec()))
            .collect());
    }
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT case_id, capability FROM case_assignments \
         WHERE user_id = $1 AND case_id = ANY($2)",
    )
    .bind(&user.id)
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
