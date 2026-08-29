//! Per-case chat messages. Every message belongs to a channel (see
//! [`crate::server::db::channels`]); a case's chat is the union of its channels,
//! so all reads and writes here are channel-scoped.

use crate::helpers::visibility::Visibility;
use crate::server::db::{audit, ids, now_stamp, pool};
use crate::server_fns::channels::{Channel, ChannelKind};
use crate::server_fns::message::{Message, MessageTranscriptExport, MAX_MESSAGE_BODY_CHARS};
use crate::server_fns::pagination::Page;

const SEND_WINDOW_SECONDS: i32 = 60;
const SEND_WINDOW_MAX: i32 = 20;
const SEND_MIN_INTERVAL_SECONDS: i64 = 2;

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

#[derive(sqlx::FromRow)]
struct ReceiptExportRow {
    message_id: String,
    reader_user_id: String,
    reader_name: String,
    first_read_at: Option<chrono::DateTime<chrono::Utc>>,
}

fn visibility_for(kind: ChannelKind) -> Visibility {
    if kind == ChannelKind::VolunteerOnly {
        Visibility::VolunteerOnly
    } else {
        Visibility::Shared
    }
}

fn first_read_display(at: Option<chrono::DateTime<chrono::Utc>>) -> String {
    at.map(|ts| {
        ts.with_timezone(&chrono::Local)
            .format("%Y-%m-%d %H:%M")
            .to_string()
    })
    .unwrap_or_default()
}

/// Validate and normalize a message body.
pub fn normalize_body(body: &str) -> Result<String, String> {
    let body = body.trim().to_string();
    if body.is_empty() {
        return Err("Message cannot be empty.".to_string());
    }
    if body.chars().count() > MAX_MESSAGE_BODY_CHARS {
        return Err(format!(
            "Message must be {MAX_MESSAGE_BODY_CHARS} characters or fewer."
        ));
    }
    Ok(body)
}

/// One page of a channel's chat: the most recent `limit` messages (returned in
/// send order, oldest-first) plus the total number of messages in the channel.
pub async fn page(
    channel_id: &str,
    limit: i64,
    reader_id: &str,
) -> Result<Page<Message>, sqlx::Error> {
    let limit = limit.clamp(1, 200);
    let mut tx = pool().begin().await?;

    let total = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM messages WHERE channel_id = $1")
        .bind(channel_id)
        .fetch_one(&mut *tx)
        .await?;

    // Take the newest `limit` rows, then re-sort ascending for chat display.
    let rows = sqlx::query_as::<_, MessageRow>(
        "SELECT id, case_id, channel_id, author_id, author, body, sent_at FROM (
             SELECT id, case_id, channel_id, author_id, author, body, sent_at, seq
             FROM messages WHERE channel_id = $1 ORDER BY seq DESC LIMIT $2
         ) recent ORDER BY seq ASC",
    )
    .bind(channel_id)
    .bind(limit)
    .fetch_all(&mut *tx)
    .await?;

    let message_ids = rows.iter().map(|row| row.id.as_str()).collect::<Vec<_>>();
    record_first_reads_for_ids_in(&mut tx, &message_ids, reader_id).await?;
    clear_notifications_for_ids_in(&mut tx, &message_ids, reader_id).await?;
    tx.commit().await?;

    Ok(Page {
        items: rows.into_iter().map(Into::into).collect(),
        total,
    })
}

async fn record_first_reads_for_ids_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    message_ids: &[&str],
    reader_id: &str,
) -> Result<(), sqlx::Error> {
    if message_ids.is_empty() {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO message_read_receipts
             (message_id, channel_id, case_id, user_id, first_read_at)
         SELECT m.id, m.channel_id, m.case_id, $2, now()
         FROM messages m
         WHERE m.id = ANY($1) AND m.author_id <> $2
         ON CONFLICT (message_id, user_id) DO UPDATE SET
             first_read_at = COALESCE(message_read_receipts.first_read_at, EXCLUDED.first_read_at)",
    )
    .bind(message_ids)
    .bind(reader_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn clear_notifications_for_ids_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    message_ids: &[&str],
    reader_id: &str,
) -> Result<(), sqlx::Error> {
    if message_ids.is_empty() {
        return Ok(());
    }
    sqlx::query("DELETE FROM channel_notifications WHERE user_id = $1 AND message_id = ANY($2)")
        .bind(reader_id)
        .bind(message_ids)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn check_send_throttle(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: &str,
) -> Result<(), sqlx::Error> {
    let allowed: Option<bool> = sqlx::query_scalar(
        "INSERT INTO message_send_throttle (user_id, window_start, send_count, last_sent_at)
         VALUES ($1, now(), 1, now())
         ON CONFLICT (user_id) DO UPDATE SET
             window_start = CASE
                 WHEN message_send_throttle.window_start <= now() - ($3 * interval '1 second')
                 THEN now()
                 ELSE message_send_throttle.window_start
             END,
             send_count = CASE
                 WHEN message_send_throttle.window_start <= now() - ($3 * interval '1 second')
                 THEN 1
                 ELSE message_send_throttle.send_count + 1
             END,
             last_sent_at = now()
         WHERE message_send_throttle.last_sent_at <= now() - ($2 * interval '1 second')
           AND (message_send_throttle.window_start <= now() - ($3 * interval '1 second')
                OR message_send_throttle.send_count < $4)
         RETURNING true",
    )
    .bind(user_id)
    .bind(SEND_MIN_INTERVAL_SECONDS as i32)
    .bind(SEND_WINDOW_SECONDS)
    .bind(SEND_WINDOW_MAX)
    .fetch_optional(&mut **tx)
    .await?;
    if allowed.is_none() {
        return Err(sqlx::Error::Protocol(
            "Please wait before sending another message.".to_string(),
        ));
    }
    Ok(())
}

/// Post a message and its official recipient/read state in one transaction.
pub async fn create(
    channel: &Channel,
    author_id: &str,
    author: &str,
    body: &str,
) -> Result<Message, sqlx::Error> {
    let id = ids::next(pool(), "m").await?;
    let sent_at = now_stamp();
    let mut tx = pool().begin().await?;

    check_send_throttle(&mut tx, author_id).await?;

    let channel_state: Option<String> = sqlx::query_scalar(
        "SELECT state FROM case_channels WHERE id = $1 AND case_id = $2 FOR UPDATE",
    )
    .bind(&channel.id)
    .bind(&channel.case_id)
    .fetch_optional(&mut *tx)
    .await?;
    if channel_state.as_deref() != Some("active") {
        return Err(sqlx::Error::RowNotFound);
    }

    sqlx::query(
        "INSERT INTO messages (id, case_id, channel_id, author_id, author, body, sent_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(&id)
    .bind(&channel.case_id)
    .bind(&channel.id)
    .bind(author_id)
    .bind(author)
    .bind(body)
    .bind(&sent_at)
    .execute(&mut *tx)
    .await?;

    sqlx::query(&format!(
        "INSERT INTO message_read_receipts (message_id, channel_id, case_id, user_id)
         SELECT $3, $2, $1, a.user_id
         FROM case_assignments a
         JOIN users u ON u.id = a.user_id
         WHERE a.case_id = $1
           AND a.capability = 'view_case'
           AND a.user_id <> $4
           AND (NOT $5 OR u.role IN {staff})
         ON CONFLICT DO NOTHING",
        staff = crate::server_fns::users::AccountRole::STAFF_ROLES_SQL
    ))
    .bind(&channel.case_id)
    .bind(&channel.id)
    .bind(&id)
    .bind(author_id)
    .bind(channel.kind == ChannelKind::VolunteerOnly)
    .execute(&mut *tx)
    .await?;

    crate::server::db::channel_notifications::record_for_channel_message_in(
        &mut tx,
        &channel.case_id,
        &channel.id,
        &id,
        author_id,
        channel.kind,
    )
    .await?;

    audit::record_in_transaction_with_visibility(
        &mut tx,
        audit::Entity::Case,
        &channel.case_id,
        author,
        "message",
        "",
        &format!("sent {}", id),
        visibility_for(channel.kind),
    )
    .await?;

    tx.commit().await?;
    Ok(Message {
        id,
        case_id: channel.case_id.clone(),
        channel_id: channel.id.clone(),
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

/// Build an audited CSV export for an authorized channel.
pub async fn export_channel(
    channel: &Channel,
    actor: &str,
) -> Result<MessageTranscriptExport, sqlx::Error> {
    let mut tx = pool().begin().await?;
    let rows = sqlx::query_as::<_, MessageRow>(
        "SELECT id, case_id, channel_id, author_id, author, body, sent_at
         FROM messages WHERE channel_id = $1 ORDER BY seq ASC",
    )
    .bind(&channel.id)
    .fetch_all(&mut *tx)
    .await?;

    let receipts = sqlx::query_as::<_, ReceiptExportRow>(
        "SELECT r.message_id,
                r.user_id AS reader_user_id,
                COALESCE(NULLIF(trim(u.first_name || ' ' || u.last_name), ''), r.user_id) AS reader_name,
                r.first_read_at
         FROM message_read_receipts r
         LEFT JOIN users u ON u.id = r.user_id
         WHERE r.channel_id = $1
         ORDER BY r.message_id, r.user_id",
    )
    .bind(&channel.id)
    .fetch_all(&mut *tx)
    .await?;

    audit::record_in_transaction_with_visibility(
        &mut tx,
        audit::Entity::Case,
        &channel.case_id,
        actor,
        "message transcript export",
        "",
        &format!("exported {}", channel.id),
        visibility_for(channel.kind),
    )
    .await?;
    tx.commit().await?;

    let mut by_message: std::collections::BTreeMap<String, Vec<ReceiptExportRow>> =
        std::collections::BTreeMap::new();
    for receipt in receipts {
        by_message
            .entry(receipt.message_id.clone())
            .or_default()
            .push(receipt);
    }

    let mut csv = String::from(
        "case_id,channel_id,channel_name,channel_audience,channel_archived,message_id,author_id,author_name,sent_at,body,reader_user_id,reader_name,first_read_at\n",
    );
    for row in rows {
        if let Some(receipts) = by_message.get(&row.id) {
            for receipt in receipts {
                append_export_line(&mut csv, channel, &row, Some(receipt));
            }
        } else {
            append_export_line(&mut csv, channel, &row, None);
        }
    }

    Ok(MessageTranscriptExport {
        filename: format!(
            "case-{}-channel-{}-transcript.csv",
            channel.case_id, channel.id
        ),
        csv,
    })
}

fn append_export_line(
    csv: &mut String,
    channel: &Channel,
    row: &MessageRow,
    receipt: Option<&ReceiptExportRow>,
) {
    let (reader_id, reader_name, first_read_at) = receipt
        .map(|r| {
            (
                r.reader_user_id.as_str(),
                r.reader_name.as_str(),
                first_read_display(r.first_read_at),
            )
        })
        .unwrap_or(("", "", String::new()));
    let fields = [
        row.case_id.as_str(),
        row.channel_id.as_str(),
        channel.name.as_str(),
        channel.kind.slug(),
        if channel.archived { "true" } else { "false" },
        row.id.as_str(),
        row.author_id.as_str(),
        row.author.as_str(),
        row.sent_at.as_str(),
        row.body.as_str(),
        reader_id,
        reader_name,
        first_read_at.as_str(),
    ];
    csv.push_str(
        &fields
            .into_iter()
            .map(csv_escape)
            .collect::<Vec<_>>()
            .join(","),
    );
    csv.push('\n');
}

// REQ-MSG-001..009: exports preserve the authorized immutable transcript while
// neutralizing spreadsheet formulas in user-controlled cells.
fn csv_escape(value: &str) -> String {
    let safe = if value.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{value}")
    } else {
        value.to_string()
    };
    let escaped = safe.replace('"', "\"\"");
    format!("\"{escaped}\"")
}
