//! Azure Communication Services (ACS) Email REST transport (SSR only).
//!
//! Like [`crate::server::rag::azure`], this talks to Azure directly over REST
//! with `reqwest` — there is no official ACS Email SDK for Rust. Requests are
//! signed with the ACS HMAC-SHA256 shared-key scheme and POSTed to the
//! resource's `emails:send` endpoint.

use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::server::config::EmailConfig;

type HmacSha256 = Hmac<Sha256>;

/// The GA data-plane API version for ACS Email.
const API_VERSION: &str = "2023-03-31";

/// How many times to attempt a send before giving up (1 initial try + retries).
const MAX_ATTEMPTS: u32 = 3;

/// How long to wait between attempts after a transient failure.
const RETRY_DELAY: Duration = Duration::from_secs(25);

/// Maximum logical email sends allowed in a rolling hour.
const MAX_EMAILS_PER_HOUR: usize = 100;
const EMAIL_LIMIT_WINDOW: Duration = Duration::from_secs(60 * 60);

static EMAIL_SENDS: OnceLock<Mutex<VecDeque<Instant>>> = OnceLock::new();

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

/// A single outbound email.
#[derive(Clone, Debug)]
pub struct EmailMessage {
    pub recipients: EmailRecipients,
    pub subject: String,
    /// HTML body.
    pub html: String,
    /// Plain-text fallback body.
    pub plain_text: String,
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

    reserve_email_send()?;

    // Retry transient failures a few times. Each attempt is freshly signed
    // because the `x-ms-date` header (and thus the signature) must be current.
    let mut last_err = String::new();
    for attempt in 1..=MAX_ATTEMPTS {
        match try_send(cfg, &url, &path_and_query, &host, &serialized).await {
            Ok(()) => {
                tracing::info!("email \"{}\" sent to {recipient_summary}", msg.subject);
                return Ok(());
            }
            Err(SendError::Permanent(e)) => return Err(e),
            Err(SendError::Retryable(e)) => {
                last_err = e;
                if attempt < MAX_ATTEMPTS {
                    tracing::warn!(
                        "email to {recipient_summary} failed (attempt {attempt}/{MAX_ATTEMPTS}): \
                         {last_err}; retrying"
                    );
                    tokio::time::sleep(RETRY_DELAY).await;
                }
            }
        }
    }
    Err(format!(
        "email failed after {MAX_ATTEMPTS} attempts: {last_err}"
    ))
}

/// Reserve one slot in the process-local rolling hourly send limit.
fn reserve_email_send() -> Result<(), String> {
    let now = Instant::now();
    let sends = EMAIL_SENDS.get_or_init(|| Mutex::new(VecDeque::new()));
    let mut sends = sends
        .lock()
        .map_err(|_| "email hourly limit guard unavailable".to_string())?;

    while sends
        .front()
        .is_some_and(|sent_at| now.duration_since(*sent_at) >= EMAIL_LIMIT_WINDOW)
    {
        sends.pop_front();
    }

    if sends.len() >= MAX_EMAILS_PER_HOUR {
        return Err(format!(
            "email hourly limit reached ({MAX_EMAILS_PER_HOUR} sends in the last hour)"
        ));
    }

    sends.push_back(now);
    Ok(())
}

fn recipients_json(recipients: &EmailRecipients) -> Result<(serde_json::Value, String), String> {
    match recipients {
        EmailRecipients::To(recipient) => Ok((
            json!({ "to": [recipient_json(recipient)] }),
            recipient.address.clone(),
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
