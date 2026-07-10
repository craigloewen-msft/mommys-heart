//! Per-case chat messages.

use crate::server::db::{ids, now_stamp, pool};
use crate::types::Message;

#[derive(sqlx::FromRow)]
struct MessageRow {
    id: String,
    case_id: String,
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
            author_id: r.author_id,
            author: r.author,
            body: r.body,
            sent_at: r.sent_at,
        }
    }
}

/// All messages for a case, in send order.
pub async fn for_case(case_id: &str) -> Result<Vec<Message>, sqlx::Error> {
    let rows = sqlx::query_as::<_, MessageRow>(
        "SELECT id, case_id, author_id, author, body, sent_at
         FROM messages WHERE case_id = $1 ORDER BY seq ASC",
    )
    .bind(case_id)
    .fetch_all(pool())
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// Post a message to a case's chat.
pub async fn create(
    case_id: &str,
    author_id: &str,
    author: &str,
    body: &str,
) -> Result<Message, sqlx::Error> {
    let id = ids::next(pool(), "m").await?;
    let sent_at = now_stamp();
    sqlx::query(
        "INSERT INTO messages (id, case_id, author_id, author, body, sent_at)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(&id)
    .bind(case_id)
    .bind(author_id)
    .bind(author)
    .bind(body)
    .bind(&sent_at)
    .execute(pool())
    .await?;
    Ok(Message {
        id,
        case_id: case_id.to_string(),
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
    author_id: &str,
    author: &str,
    body: &str,
    sent_at: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO messages (id, case_id, author_id, author, body, sent_at)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(case_id)
    .bind(author_id)
    .bind(author)
    .bind(body)
    .bind(sent_at)
    .execute(pool())
    .await?;
    Ok(())
}
