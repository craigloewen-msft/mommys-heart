//! Per-case chat messages. Every message belongs to a channel (see
//! [`crate::server::db::channels`]); a case's chat is the union of its channels,
//! so all reads and writes here are channel-scoped.

use crate::server::db::{ids, now_stamp, pool};
use crate::server_fns::message::Message;
use crate::server_fns::pagination::Page;

#[derive(sqlx::FromRow)]
struct MessageRow {
    id: String,
    case_id: String,
    channel_id: String,
    author_id: String,
    author: String,
    body: String,
    sent_at: String,
}

impl From<MessageRow> for Message {
    fn from(r: MessageRow) -> Self {
        Message {
            id: r.id,
            case_id: r.case_id,
            channel_id: r.channel_id,
            author_id: r.author_id,
            author: r.author,
            body: r.body,
            sent_at: r.sent_at,
        }
    }
}

/// One page of a channel's chat: the most recent `limit` messages (returned in
/// send order, oldest-first) plus the total number of messages in the channel.
///
/// Backs the chat's "Load more" pagination — the newest messages load first and
/// older ones page in on demand — so the browser never has to pull an entire
/// long-running conversation at once.
///
/// Scoped to a single channel, which is what keeps the volunteer-only thread
/// private: a caller who cannot reach that channel's id can never read its
/// messages through this query.
pub async fn page(channel_id: &str, limit: i64) -> Result<Page<Message>, sqlx::Error> {
    let limit = limit.clamp(1, 200);

    let total = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM messages WHERE channel_id = $1")
        .bind(channel_id)
        .fetch_one(pool())
        .await?;

    // Take the newest `limit` rows (seq DESC), then re-sort ascending so the UI
    // renders them oldest-first with the latest message at the bottom.
    let rows = sqlx::query_as::<_, MessageRow>(
        "SELECT id, case_id, channel_id, author_id, author, body, sent_at FROM (
             SELECT id, case_id, channel_id, author_id, author, body, sent_at, seq
             FROM messages WHERE channel_id = $1 ORDER BY seq DESC LIMIT $2
         ) recent ORDER BY seq ASC",
    )
    .bind(channel_id)
    .bind(limit)
    .fetch_all(pool())
    .await?;

    Ok(Page {
        items: rows.into_iter().map(Into::into).collect(),
        total,
    })
}

/// Post a message to a channel of a case's chat.
pub async fn create(
    case_id: &str,
    channel_id: &str,
    author_id: &str,
    author: &str,
    body: &str,
) -> Result<Message, sqlx::Error> {
    let id = ids::next(pool(), "m").await?;
    let sent_at = now_stamp();
    sqlx::query(
        "INSERT INTO messages (id, case_id, channel_id, author_id, author, body, sent_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(&id)
    .bind(case_id)
    .bind(channel_id)
    .bind(author_id)
    .bind(author)
    .bind(body)
    .bind(&sent_at)
    .execute(pool())
    .await?;
    Ok(Message {
        id,
        case_id: case_id.to_string(),
        channel_id: channel_id.to_string(),
        author_id: author_id.to_string(),
        author: author.to_string(),
        body: body.to_string(),
        sent_at,
    })
}

/// Insert a message with a caller-supplied id + timestamp (used by the seed).
pub async fn insert(
    id: &str,
    case_id: &str,
    channel_id: &str,
    author_id: &str,
    author: &str,
    body: &str,
    sent_at: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO messages (id, case_id, channel_id, author_id, author, body, sent_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(id)
    .bind(case_id)
    .bind(channel_id)
    .bind(author_id)
    .bind(author)
    .bind(body)
    .bind(sent_at)
    .execute(pool())
    .await?;
    Ok(())
}
