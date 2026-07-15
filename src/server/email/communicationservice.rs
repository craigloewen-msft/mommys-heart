//! Azure Communication Services (ACS) Email REST transport (SSR only).
//!
//! Like [`crate::server::rag::azure`], this talks to Azure directly over REST
//! with `reqwest` — there is no official ACS Email SDK for Rust. Requests are
//! signed with the ACS HMAC-SHA256 shared-key scheme and POSTed to the
//! resource's `emails:send` endpoint.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::server::config::EmailConfig;

type HmacSha256 = Hmac<Sha256>;

/// The GA data-plane API version for ACS Email.
const API_VERSION: &str = "2023-03-31";

/// A single outbound email addressed to one recipient.
#[derive(Clone, Debug)]
pub struct EmailMessage {
    /// Recipient email address.
    pub to_address: String,
    /// Recipient display name (may be empty).
    pub to_name: String,
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

    let recipient = if msg.to_name.trim().is_empty() {
        json!({ "address": msg.to_address })
    } else {
        json!({ "address": msg.to_address, "displayName": msg.to_name })
    };
    let body = json!({
        "senderAddress": cfg.sender_address,
        "content": {
            "subject": msg.subject,
            "plainText": msg.plain_text,
            "html": msg.html,
        },
        "recipients": { "to": [recipient] },
    });
    // Serialize once: the exact bytes we hash must be the exact bytes we send.
    let serialized = serde_json::to_string(&body).map_err(|e| format!("email encode failed: {e}"))?;

    // RFC1123 UTC timestamp for the `x-ms-date` header, e.g.
    // "Mon, 02 Jan 2006 15:04:05 GMT".
    let date = chrono::Utc::now()
        .format("%a, %d %b %Y %H:%M:%S GMT")
        .to_string();
    let content_hash = content_hash(&serialized);
    let string_to_sign = format!("POST\n{path_and_query}\n{date};{host};{content_hash}");
    let signature = sign(&cfg.access_key, &string_to_sign)?;
    let authorization =
        format!("HMAC-SHA256 SignedHeaders=x-ms-date;host;x-ms-content-sha256&Signature={signature}");

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("x-ms-date", &date)
        .header("x-ms-content-sha256", &content_hash)
        .header("Authorization", &authorization)
        .header("Content-Type", "application/json")
        .body(serialized)
        .send()
        .await
        .map_err(|e| format!("email request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let detail = resp.text().await.unwrap_or_default();
        return Err(format!("email HTTP {status}: {detail}"));
    }
    tracing::info!(
        "email \"{}\" sent to {}",
        msg.subject,
        msg.to_address
    );
    Ok(())
}

/// The `host` component (authority) of an endpoint URL, used both in the
/// request `host` header and the string-to-sign.
fn host_of(endpoint: &str) -> Result<String, String> {
    let after_scheme = endpoint.split_once("://").map(|(_, rest)| rest).unwrap_or(endpoint);
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
    let mut mac =
        HmacSha256::new_from_slice(&key).map_err(|e| format!("HMAC key error: {e}"))?;
    mac.update(string_to_sign.as_bytes());
    Ok(STANDARD.encode(mac.finalize().into_bytes()))
}
