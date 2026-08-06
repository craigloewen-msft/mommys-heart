//! Per-user unread chat notifications (SSR only).
//!
//! A notification row exists exactly while a user has an unread message in a
//! channel: [`record_for_message`] fans one out to every reader of a channel
//! when a message is posted (skipping the author), and [`mark_channel_read`]
//! deletes a user's rows for a channel the moment they open it. The nav badge
//! and the per-channel dots in the case chat are both driven by
//! [`unread_for_user`], which groups the surviving rows by channel.
//!
//! Rows normally clear themselves when the user reads the channel; the only ones
//! left for the retention task to prune are those a user never read within the
//! window (see [`purge_expired`] / [`start_retention_task`]).

use crate::server::db::pool;
use crate::server_fns::channel_notifications::ChannelUnread;
use crate::server_fns::channels::ChannelKind;
use crate::server_fns::users::AccountRole;

/// How long an unread notification is retained before the background task prunes
/// it. Notifications a user actually reads are deleted immediately; this only
/// bounds ones that are never read.
const RETENTION_YEARS: i64 = 10;

/// How often the background retention task runs.
const RETENTION_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30 * 24 * 60 * 60);

#[derive(sqlx::FromRow)]
struct UnreadRow {
    case_id: String,
    channel_id: String,
    count: i64,
}

impl From<UnreadRow> for ChannelUnread {
    fn from(r: UnreadRow) -> Self {
        ChannelUnread {
            case_id: r.case_id,
            channel_id: r.channel_id,
            count: r.count.max(0),
        }
    }
}

/// Fan a new message out into an unread notification for every user who can read
/// the channel, except its author. Mirrors the recipient rule the email
/// notifications use ([`crate::server::db::settings::recipients_for_case`]):
/// everyone assigned to the case with `view_case`, and — for the volunteer-only
/// channel (`staff_only`) — never a client account.
///
/// The insert is idempotent on `(user_id, message_id)`, so a retried send never
/// double-counts.
pub async fn record_for_channel_message(
    case_id: &str,
    channel_id: &str,
    message_id: &str,
    author_id: &str,
    kind: ChannelKind,
) -> Result<(), sqlx::Error> {
    sqlx::query(&format!(
        "INSERT INTO channel_notifications (user_id, channel_id, case_id, message_id)
         SELECT a.user_id, $2, $1, $3
         FROM case_assignments a
         JOIN users u ON u.id = a.user_id
         WHERE a.case_id = $1
           AND a.capability = 'view_case'
           AND a.user_id <> $4
           AND (NOT $5 OR u.role <> '{client}')
         ON CONFLICT DO NOTHING",
        client = AccountRole::Client.slug()
    ))
    .bind(case_id)
    .bind(channel_id)
    .bind(message_id)
    .bind(author_id)
    .bind(kind == ChannelKind::VolunteerOnly)
    .execute(pool())
    .await?;
    Ok(())
}

/// Clear a user's unread notifications for a single channel — called when they
/// open it. Returns the number of rows removed.
pub async fn mark_channel_read(user_id: &str, channel_id: &str) -> Result<u64, sqlx::Error> {
    let result =
        sqlx::query("DELETE FROM channel_notifications WHERE user_id = $1 AND channel_id = $2")
            .bind(user_id)
            .bind(channel_id)
            .execute(pool())
            .await?;
    Ok(result.rows_affected())
}

/// Every channel this user has unread messages in, with the unread count per
/// channel. Drives both the "Case Chat" nav badge (any rows at all) and the
/// per-channel dots in the case list.
pub async fn unread_for_user(user_id: &str) -> Result<Vec<ChannelUnread>, sqlx::Error> {
    let rows = sqlx::query_as::<_, UnreadRow>(
        "SELECT case_id, channel_id, COUNT(*) AS count
         FROM channel_notifications
         WHERE user_id = $1
         GROUP BY case_id, channel_id",
    )
    .bind(user_id)
    .fetch_all(pool())
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// Delete notifications left unread past the retention window
/// ([`RETENTION_YEARS`]), returning the number of rows removed.
pub async fn purge_expired(pool: &sqlx::PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM channel_notifications WHERE created_at < now() - make_interval(years => $1)",
    )
    .bind(RETENTION_YEARS as i32)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// Spawn a background task that prunes expired notifications at startup and then
/// once every [`RETENTION_INTERVAL`]. Mirrors
/// [`crate::server::db::audit::start_retention_task`]: failures are logged and
/// never crash the server.
pub fn start_retention_task() {
    tokio::spawn(async {
        loop {
            match purge_expired(pool()).await {
                Ok(n) => tracing::info!("notification retention: deleted {n} expired entries"),
                Err(e) => tracing::warn!("notification retention failed: {e}"),
            }
            tokio::time::sleep(RETENTION_INTERVAL).await;
        }
    });
}
