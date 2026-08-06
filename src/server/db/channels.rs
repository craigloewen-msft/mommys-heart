//! Per-case chat **channels**: the named threads a case's chat is split into.
//!
//! Every case owns exactly one `volunteer_only` channel (the private staff
//! back-channel, created with the case and never deletable) plus one or more
//! `standard` channels, starting with "General".
//!
//! Visibility is enforced here as well as in the server-function layer: the
//! read helpers take an explicit `sees_restricted` flag and filter the
//! volunteer-only channel out for client accounts, so a repository call can
//! never accidentally hand a client the private thread.

use crate::server::db::{audit, ids, pool};
use crate::server_fns::channels::{
    Channel, ChannelKind, DEFAULT_CHANNEL_NAME, VOLUNTEER_CHANNEL_NAME,
};

#[derive(sqlx::FromRow)]
struct ChannelRow {
    id: String,
    case_id: String,
    name: String,
    kind: String,
    message_count: i64,
}

impl From<ChannelRow> for Channel {
    fn from(r: ChannelRow) -> Self {
        Channel {
            id: r.id,
            case_id: r.case_id,
            name: r.name,
            kind: ChannelKind::from_slug(&r.kind).unwrap_or(ChannelKind::Standard),
            message_count: r.message_count.max(0) as usize,
        }
    }
}

/// The `SELECT` list projecting a `case_channels` row (aliased `ch`) with its
/// message count. Callers append their own `WHERE`/`ORDER`.
const CHANNEL_SELECT: &str = "SELECT ch.id, ch.case_id, ch.name, ch.kind,
        (SELECT COUNT(*) FROM messages m WHERE m.channel_id = ch.id) AS message_count
 FROM case_channels ch";

/// A single channel by id, or `None` when it does not exist. Deliberately
/// *unfiltered* — callers use it to resolve the owning case before running an
/// authorization check, so it must not depend on one.
pub async fn get(channel_id: &str) -> Result<Option<Channel>, sqlx::Error> {
    let row = sqlx::query_as::<_, ChannelRow>(&format!("{CHANNEL_SELECT} WHERE ch.id = $1"))
        .bind(channel_id)
        .fetch_optional(pool())
        .await?;
    Ok(row.map(Into::into))
}

/// The channels of a case that a caller may read, ordered with the
/// volunteer-only channel first (when visible) and the rest in creation order.
/// When `sees_restricted` is false — i.e. the caller is a client — the
/// volunteer-only channel is omitted entirely.
pub async fn list_visible(
    case_id: &str,
    sees_restricted: bool,
) -> Result<Vec<Channel>, sqlx::Error> {
    let rows = sqlx::query_as::<_, ChannelRow>(&format!(
        "{CHANNEL_SELECT}
         WHERE ch.case_id = $1 AND ($2 OR ch.kind <> '{restricted}')
         ORDER BY ch.ord ASC, ch.seq ASC",
        restricted = ChannelKind::VolunteerOnly.slug()
    ))
    .bind(case_id)
    .bind(sees_restricted)
    .fetch_all(pool())
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// Whether a database error is the "channel name already used on this case"
/// unique-index violation, so callers can turn it into a friendly message
/// instead of leaking a Postgres error string.
pub fn is_duplicate_name(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(db) if db.constraint() == Some("case_channels_case_name_idx"))
}

/// Create a standard channel on a case, auditing the change. Fails with a
/// unique-index violation when the name is already taken on this case — see
/// [`is_duplicate_name`].
pub async fn create(case_id: &str, name: &str, actor: &str) -> Result<Channel, sqlx::Error> {
    let id = ids::next(pool(), "ch").await?;
    // `ord` 1 groups every standard channel after the pinned volunteer-only
    // channel (`ord` 0); ties break on `seq`, i.e. creation order.
    sqlx::query(
        "INSERT INTO case_channels (id, case_id, name, kind, ord) VALUES ($1, $2, $3, $4, 1)",
    )
    .bind(&id)
    .bind(case_id)
    .bind(name)
    .bind(ChannelKind::Standard.slug())
    .execute(pool())
    .await?;
    audit::record(
        pool(),
        audit::Entity::Case,
        case_id,
        actor,
        "message channel",
        "",
        &format!("created \"{name}\""),
    )
    .await?;
    Ok(Channel {
        id,
        case_id: case_id.to_string(),
        name: name.to_string(),
        kind: ChannelKind::Standard,
        message_count: 0,
    })
}

/// Delete a channel (and, via `ON DELETE CASCADE`, its messages), auditing the
/// change. The volunteer-only channel is protected by the `WHERE kind <> ...`
/// clause as a second line of defense behind the server-function check.
pub async fn delete(channel_id: &str, case_id: &str, actor: &str) -> Result<(), sqlx::Error> {
    let deleted: Option<String> = sqlx::query_scalar(&format!(
        "DELETE FROM case_channels WHERE id = $1 AND kind <> '{restricted}' RETURNING name",
        restricted = ChannelKind::VolunteerOnly.slug()
    ))
    .bind(channel_id)
    .fetch_optional(pool())
    .await?;
    if let Some(name) = deleted {
        audit::record(
            pool(),
            audit::Entity::Case,
            case_id,
            actor,
            "message channel",
            &format!("\"{name}\""),
            "deleted",
        )
        .await?;
    }
    Ok(())
}

/// Create the two channels every new case starts with: the permanent
/// volunteer-only back-channel and a "General" channel. Takes a connection so
/// it can run inside the case-creation transaction — a case can never exist
/// without somewhere to chat.
pub async fn create_defaults(
    conn: &mut sqlx::PgConnection,
    case_id: &str,
) -> Result<(), sqlx::Error> {
    for (name, kind, ord) in [
        (VOLUNTEER_CHANNEL_NAME, ChannelKind::VolunteerOnly, 0i32),
        (DEFAULT_CHANNEL_NAME, ChannelKind::Standard, 1i32),
    ] {
        let id = ids::next(&mut *conn, "ch").await?;
        sqlx::query(
            "INSERT INTO case_channels (id, case_id, name, kind, ord) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&id)
        .bind(case_id)
        .bind(name)
        .bind(kind.slug())
        .bind(ord)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// The id of a case's channel with the given kind and name — used by the seed to
/// route fixture messages into "General".
pub async fn id_for(case_id: &str, name: &str) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT id FROM case_channels WHERE case_id = $1 AND name = $2")
        .bind(case_id)
        .bind(name)
        .fetch_optional(pool())
        .await
}
