//! Azure Communication Services (ACS) Email REST transport (SSR only).
//!
//! Like [`crate::server::rag::azure`], this talks to Azure directly over REST
//! with `reqwest` — there is no official ACS Email SDK for Rust. Requests are
//! signed with the ACS HMAC-SHA256 shared-key scheme and POSTed to the
//! resource's `emails:send` endpoint.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::sync::{Mutex, Notify};

use crate::server::config::{Brand, EmailConfig};
use crate::server::email::templates::{self, RenderedEmail};

type HmacSha256 = Hmac<Sha256>;

/// The GA data-plane API version for ACS Email.
const API_VERSION: &str = "2023-03-31";

/// How many times to attempt a send before giving up (1 initial try + retries).
const MAX_ATTEMPTS: u32 = 3;

/// How long to wait between attempts after a transient failure.
const RETRY_DELAY: Duration = Duration::from_secs(25);

/// Number of recent sends at which standard emails must wait.
const STANDARD_EMAIL_LIMIT: usize = 90;
const EMAIL_LIMIT_WINDOW: Duration = Duration::from_secs(60 * 60);

static EMAIL_SENDS_HISTORY: OnceLock<Mutex<VecDeque<Instant>>> = OnceLock::new();
static NEXT_EMAIL_SEND_ID: AtomicU64 = AtomicU64::new(0);
static EMAILS_WAITING_QUEUE: OnceLock<Mutex<VecDeque<u64>>> = OnceLock::new();
static EMAIL_QUEUE_CHANGED: OnceLock<Notify> = OnceLock::new();

/// ACS Email's maximum total recipients in one message.
pub const MAX_RECIPIENTS_PER_MESSAGE: usize = 50;

/// Outcome of a single send attempt: `Retryable` failures (network errors, HTTP
/// 429/5xx) are worth another try; `Permanent` ones (bad request, auth, a bad
/// recipient) are not.
enum SendError {
    Retryable(String),
    Permanent(String),
}

/// One email recipient.
#[derive(Clone, Debug)]
pub struct EmailRecipient {
    pub address: String,
    /// Recipient display name (may be empty).
    pub name: String,
}

/// The visible-recipient policy for an outbound email.
#[derive(Clone, Debug)]
pub enum EmailRecipients {
    /// A direct, single-recipient message.
    To(EmailRecipient),
    /// A shared message whose recipients must remain hidden from one another.
    Bcc(Vec<EmailRecipient>),
}

/// Whether an email uses the standard limit or the reserved OTP capacity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmailKind {
    Standard,
    Otp,
}

/// A single outbound email.
#[derive(Clone, Debug)]
pub struct EmailMessage {
    pub kind: EmailKind,
    pub recipients: EmailRecipients,
    pub subject: String,
    /// HTML body.
    pub html: String,
    /// Plain-text fallback body.
    pub plain_text: String,
}

/// One recipient submitted to the high-level batch delivery path.
#[derive(Clone, Debug)]
pub struct EmailBatchRecipient {
    /// Caller-owned stable key returned unchanged in the outcome.
    pub key: i32,
    pub address: String,
    pub name: String,
}

/// Provider acceptance or final failure for one submitted batch recipient.
#[derive(Clone, Debug)]
pub struct EmailBatchOutcome {
    pub recipient: EmailBatchRecipient,
    pub result: Result<(), String>,
}

/// Render and deliver one administrator-authored contact message through the
/// shared batch path.
pub async fn send_contact_batch(
    task_id: &str,
    recipients: &[EmailBatchRecipient],
    subject: &str,
    body: &str,
) -> Vec<EmailBatchOutcome> {
    let email = templates::contact_mail(&Brand::from_env(), subject, body);
    let context = format!("Contact mail task {task_id}");
    send_standard_batch(recipients, &email, &context).await
}

/// Deliver shared content to hidden-recipient ACS batches and return one outcome
/// for every submitted recipient.
async fn send_standard_batch(
    recipients: &[EmailBatchRecipient],
    email: &RenderedEmail,
    context: &str,
) -> Vec<EmailBatchOutcome> {
    let cfg = EmailConfig::from_env();
    let mut outcomes = Vec::with_capacity(recipients.len());

    for chunk in recipients.chunks(MAX_RECIPIENTS_PER_MESSAGE) {
        if cfg.dry_run {
            tracing::warn!(
                subject = %email.subject,
                recipient_count = chunk.len(),
                "[email dry-run] batch accepted without delivery"
            );
            outcomes.extend(chunk.iter().cloned().map(|recipient| EmailBatchOutcome {
                recipient,
                result: Ok(()),
            }));
            continue;
        }

        let result = if cfg.is_configured() {
            let recipients = if let [recipient] = chunk {
                EmailRecipients::To(batch_email_recipient(recipient))
            } else {
                EmailRecipients::Bcc(chunk.iter().map(batch_email_recipient).collect())
            };
            send_email(
                &cfg,
                &EmailMessage {
                    kind: EmailKind::Standard,
                    recipients,
                    subject: email.subject.clone(),
                    html: email.html.clone(),
                    plain_text: email.plain_text.clone(),
                },
            )
            .await
        } else {
            Err("Email delivery is not configured.".to_string())
        };

        if let Err(error) = &result {
            tracing::warn!(recipient_count = chunk.len(), "email batch failed: {error}");
            for recipient in chunk {
                crate::server::db::email_failures::record(
                    &recipient.address,
                    &email.subject,
                    context,
                    error,
                )
                .await;
            }
        }
        outcomes.extend(chunk.iter().cloned().map(|recipient| EmailBatchOutcome {
            recipient,
            result: result.clone(),
        }));
    }

    outcomes
}

fn batch_email_recipient(recipient: &EmailBatchRecipient) -> EmailRecipient {
    EmailRecipient {
        address: recipient.address.clone(),
        name: recipient.name.clone(),
    }
}

/// Send one email through ACS. Returns `Ok(())` when ACS accepts the request
/// (HTTP 2xx — delivery itself is asynchronous on Azure's side). Never panics;
/// all failures are surfaced as `Err(String)` for the caller to log.
///
/// The request is authenticated with the ACS shared-key HMAC-SHA256 scheme:
/// see <https://learn.microsoft.com/azure/communication-services/tutorials/hmac-header-tutorial>.
pub async fn send_email(cfg: &EmailConfig, msg: &EmailMessage) -> Result<(), String> {
    let base = cfg.endpoint.trim_end_matches('/');
    let host = host_of(base)?;
    let path_and_query = format!("/emails:send?api-version={API_VERSION}");
    let url = format!("{base}{path_and_query}");

    let (recipients, recipient_summary) = recipients_json(&msg.recipients)?;
    let body = json!({
        "senderAddress": cfg.sender_address,
        "content": {
            "subject": msg.subject,
            "plainText": msg.plain_text,
            "html": msg.html,
        },
        "recipients": recipients,
    });
    // Serialize once: the exact bytes we hash must be the exact bytes we send.
    let serialized =
        serde_json::to_string(&body).map_err(|e| format!("email encode failed: {e}"))?;

    let send_id = register_waiting_email(msg).await;

    // Retry transient failures a few times. Each attempt is freshly signed
    // because the `x-ms-date` header (and thus the signature) must be current.
    let mut attempt = 1;
    let result = loop {
        match try_send(cfg, &url, &path_and_query, &host, &serialized).await {
            Ok(()) => {
                tracing::info!("email \"{}\" sent to {recipient_summary}", msg.subject);
                break Ok(());
            }
            Err(SendError::Permanent(e)) => break Err(e),
            Err(SendError::Retryable(e)) => {
                if attempt < MAX_ATTEMPTS {
                    tracing::warn!(
                        "email to {recipient_summary} failed (attempt {attempt}/{MAX_ATTEMPTS}): \
                         {e}; retrying"
                    );
                    tokio::time::sleep(RETRY_DELAY).await;
                    attempt += 1;
                } else {
                    break Err(format!("email failed after {MAX_ATTEMPTS} attempts: {e}"));
                }
            }
        }
    };
    remove_waiting_email(send_id).await;
    result
}

/// The mechanism we use to ensure we aren't hitting the rate limit.
/// And also ensure that OTP messages can be sent out always with priority.
async fn register_waiting_email(msg: &EmailMessage) -> u64 {
    // OTP emails bypass the standard limit and start immediately.
    if msg.kind == EmailKind::Otp {
        let send_id = NEXT_EMAIL_SEND_ID.fetch_add(1, Ordering::Relaxed);
        let waiting_queue_mutex = EMAILS_WAITING_QUEUE.get_or_init(|| Mutex::new(VecDeque::new()));
        let mut waiting_queue = waiting_queue_mutex.lock().await;
        waiting_queue.push_back(send_id);

        let history_mutex = EMAIL_SENDS_HISTORY.get_or_init(|| Mutex::new(VecDeque::new()));
        history_mutex.lock().await.push_back(Instant::now());
        return send_id;
    }

    loop {
        // Register for notification before inspecting the queue so an OTP
        // completion cannot be missed between the check and the await.
        let queue_changed = EMAIL_QUEUE_CHANGED.get_or_init(Notify::new).notified();
        let waiting_queue_mutex = EMAILS_WAITING_QUEUE.get_or_init(|| Mutex::new(VecDeque::new()));
        let mut waiting_queue = waiting_queue_mutex.lock().await;

        let history_mutex = EMAIL_SENDS_HISTORY.get_or_init(|| Mutex::new(VecDeque::new()));
        let mut history = history_mutex.lock().await;

        if waiting_queue.len() > 0 {
            tracing::warn!(
                "email \"{}\" waiting for OTP emails to be sent first ({} in queue)",
                msg.subject,
                waiting_queue.len()
            );
            drop(waiting_queue);
            drop(history);
            queue_changed.await;
            continue;
        }

        let now = Instant::now();
        history.retain(|&sent_at| now.duration_since(sent_at) < EMAIL_LIMIT_WINDOW);

        if history.len() < STANDARD_EMAIL_LIMIT {
            history.push_back(Instant::now());
            let send_id = NEXT_EMAIL_SEND_ID.fetch_add(1, Ordering::Relaxed);
            if msg.kind == EmailKind::Otp {
                waiting_queue.push_back(send_id);
            }
            return send_id;
        }

        let wait = history
            .front()
            .map(|sent_at| EMAIL_LIMIT_WINDOW.saturating_sub(now.duration_since(*sent_at)))
            .unwrap_or_default();
        tracing::warn!(
            "email \"{}\" waiting for {wait:?} due to rate limit ({} sent in the last hour)",
            msg.subject,
            history.len()
        );
        drop(waiting_queue);
        drop(history);
        tokio::time::sleep(wait).await;
    }
}

async fn remove_waiting_email(send_id: u64) {
    let waiting_queue_mutex = EMAILS_WAITING_QUEUE.get_or_init(|| Mutex::new(VecDeque::new()));
    let mut waiting_queue = waiting_queue_mutex.lock().await;

    let previous_len = waiting_queue.len();
    waiting_queue.retain(|&queued_id| queued_id != send_id);
    let removed = waiting_queue.len() < previous_len;
    drop(waiting_queue);
    if removed {
        EMAIL_QUEUE_CHANGED
            .get_or_init(Notify::new)
            .notify_waiters();
    }
}

fn recipients_json(recipients: &EmailRecipients) -> Result<(serde_json::Value, String), String> {
    match recipients {
        EmailRecipients::To(recipient) => Ok((
            json!({ "to": [recipient_json(recipient)] }),
            "1 direct recipient".to_string(),
        )),
        EmailRecipients::Bcc(recipients) => {
            if recipients.is_empty() {
                return Err("email requires at least one recipient".to_string());
            }
            if recipients.len() > MAX_RECIPIENTS_PER_MESSAGE {
                return Err(format!(
                    "email has {} recipients; ACS permits at most {MAX_RECIPIENTS_PER_MESSAGE}",
                    recipients.len()
                ));
            }
            Ok((
                json!({
                    "bcc": recipients.iter().map(recipient_json).collect::<Vec<_>>()
                }),
                format!("{} BCC recipients", recipients.len()),
            ))
        }
    }
}

fn recipient_json(recipient: &EmailRecipient) -> serde_json::Value {
    if recipient.name.trim().is_empty() {
        json!({ "address": recipient.address })
    } else {
        json!({ "address": recipient.address, "displayName": recipient.name })
    }
}

/// Make one signed POST to ACS. On failure, classify it as retryable (network
/// error, HTTP 429 or 5xx) or permanent (everything else) so the caller knows
/// whether another attempt is worthwhile.
async fn try_send(
    cfg: &EmailConfig,
    url: &str,
    path_and_query: &str,
    host: &str,
    serialized: &str,
) -> Result<(), SendError> {
    // RFC1123 UTC timestamp for the `x-ms-date` header, e.g.
    // "Mon, 02 Jan 2006 15:04:05 GMT".
    let date = chrono::Utc::now()
        .format("%a, %d %b %Y %H:%M:%S GMT")
        .to_string();
    let content_hash = content_hash(serialized);
    let string_to_sign = format!("POST\n{path_and_query}\n{date};{host};{content_hash}");
    let signature = sign(&cfg.access_key, &string_to_sign).map_err(SendError::Permanent)?;
    let authorization = format!(
        "HMAC-SHA256 SignedHeaders=x-ms-date;host;x-ms-content-sha256&Signature={signature}"
    );

    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .header("x-ms-date", &date)
        .header("x-ms-content-sha256", &content_hash)
        .header("Authorization", &authorization)
        .header("Content-Type", "application/json")
        .body(serialized.to_owned())
        .send()
        .await
        .map_err(|e| SendError::Retryable(format!("email request failed: {e}")))?;

    let status = resp.status();
    if status.is_success() {
        return Ok(());
    }
    let detail = resp.text().await.unwrap_or_default();
    let message = format!("email HTTP {status}: {detail}");
    // Rate-limiting and server errors are transient; anything else (bad request,
    // auth, unknown recipient) will fail identically on a retry.
    if status.as_u16() == 429 || status.is_server_error() {
        Err(SendError::Retryable(message))
    } else {
        Err(SendError::Permanent(message))
    }
}

/// The `host` component (authority) of an endpoint URL, used both in the
/// request `host` header and the string-to-sign.
fn host_of(endpoint: &str) -> Result<String, String> {
    let after_scheme = endpoint
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(endpoint);
    let host = after_scheme.split('/').next().unwrap_or("");
    if host.is_empty() {
        return Err(format!("invalid ACS_EMAIL_ENDPOINT: {endpoint}"));
    }
    Ok(host.to_string())
}

/// Base64(SHA-256(body)) for the `x-ms-content-sha256` header.
fn content_hash(body: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(body.as_bytes());
    STANDARD.encode(hasher.finalize())
}

/// Base64(HMAC-SHA256(base64decode(access_key), string_to_sign)).
fn sign(access_key: &str, string_to_sign: &str) -> Result<String, String> {
    let key = STANDARD
        .decode(access_key)
        .map_err(|e| format!("invalid ACS_EMAIL_ACCESS_KEY (not base64): {e}"))?;
    let mut mac = HmacSha256::new_from_slice(&key).map_err(|e| format!("HMAC key error: {e}"))?;
    mac.update(string_to_sign.as_bytes());
    Ok(STANDARD.encode(mac.finalize().into_bytes()))
}
