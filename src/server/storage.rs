//! Azure Blob Storage access for case **evidence** files (SSR only).
//!
//! Evidence file *bytes* live in a private Blob Storage container; the database
//! keeps only metadata plus the [`blob_path`] that points here. All access to
//! the Azure SDK is funnelled through this one module so the rest of the server
//! never touches a blob client directly — a deliberate seam that keeps the
//! authentication details (and any future switch to SAS/direct-upload) local.

use std::collections::HashMap;
use std::sync::OnceLock;

use azure_core::http::{RequestContent, Url};
use azure_identity::ManagedIdentityCredential;
use azure_storage_blob::models::BlockBlobClientUploadOptions;
use azure_storage_blob::BlobContainerClient;

use hmac::{Hmac, Mac};
use sha2::Sha256;

/// Default container name; override with `AZURE_STORAGE_CONTAINER`.
const DEFAULT_CONTAINER: &str = "evidence";

/// Process-wide client scoped to the evidence container. Initialized once by
/// [`init`]; mirrors the [`crate::server::db`] pool pattern.
static CONTAINER: OnceLock<BlobContainerClient> = OnceLock::new();

/// Whether evidence storage is configured at all. When `false`, uploads are
/// rejected with a clear error and the rest of the app keeps working — matching
/// how the RAG pipeline degrades when Azure OpenAI is absent.
pub fn is_configured() -> bool {
    CONTAINER.get().is_some()
}

/// The initialized evidence container client. Errors (rather than panics) when
/// storage was not configured, so request handlers can return a clean 503.
pub fn container() -> Result<&'static BlobContainerClient, String> {
    CONTAINER
        .get()
        .ok_or_else(|| "evidence storage is not configured (see .env / .env.example)".to_string())
}

/// The blob name for a piece of evidence: `cases/{case_id}/{evidence_id}`. Both
/// components are server-issued opaque ids, so the name is collision-free and
/// carries no user-controlled path segments.
///
/// It deliberately does *not* mirror the case's folder tree. Folders are what
/// people organize, and they live in the database; the container is just where
/// the bytes sit, which is what lets a file move between folders without
/// touching storage at all.
pub fn blob_path(case_id: &str, evidence_id: &str) -> String {
    format!("cases/{case_id}/{evidence_id}")
}

/// Configure the evidence container from the environment. Safe to call once at
/// startup. Missing configuration is *not* an error: the server still boots and
/// uploads simply report that storage is unavailable, so local development and
/// the chat-only deployments keep working.
pub async fn init() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let container_name = std::env::var("AZURE_STORAGE_CONTAINER")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_CONTAINER.to_string());

    let client = match build_client(&container_name)? {
        Some(client) => client,
        None => {
            tracing::info!(
                "evidence storage not configured (set AZURE_STORAGE_ACCOUNT_URL for managed \
                 identity or AZURE_STORAGE_CONNECTION_STRING for Azurite); uploads disabled"
            );
            return Ok(());
        }
    };

    ensure_container(&client).await?;

    CONTAINER
        .set(client)
        .map_err(|_| "evidence storage already initialized")?;
    tracing::info!("evidence storage ready (container '{container_name}')");
    Ok(())
}

/// Upload (overwriting) the bytes of a piece of evidence, tagging the blob with
/// its content type and a little metadata for traceability.
pub async fn put(
    blob_path: &str,
    bytes: Vec<u8>,
    content_type: &str,
    metadata: HashMap<String, String>,
) -> Result<(), String> {
    let blob = container()?.blob_client(blob_path);
    let options = BlockBlobClientUploadOptions {
        blob_content_type: Some(content_type.to_string()),
        metadata: Some(metadata),
        ..Default::default()
    };
    blob.upload(RequestContent::from(bytes), Some(options))
        .await
        .map_err(|e| format!("blob upload failed: {e}"))?;
    Ok(())
}

/// Download the full bytes of a piece of evidence.
pub async fn get(blob_path: &str) -> Result<Vec<u8>, String> {
    let blob = container()?.blob_client(blob_path);
    let result = blob
        .download(None)
        .await
        .map_err(|e| format!("blob download failed: {e}"))?;
    let bytes = result
        .body
        .collect()
        .await
        .map_err(|e| format!("reading blob body failed: {e}"))?;
    Ok(bytes.to_vec())
}

/// Delete the blob backing a piece of evidence. A missing blob is treated as
/// success so evidence removal is idempotent even if the bytes are already gone.
pub async fn delete(blob_path: &str) -> Result<(), String> {
    let blob = container()?.blob_client(blob_path);
    match blob.delete(None).await {
        Ok(_) => Ok(()),
        Err(e) if e.http_status() == Some(azure_core::http::StatusCode::NotFound) => Ok(()),
        Err(e) => Err(format!("blob delete failed: {e}")),
    }
}

// ---------------------------------------------------------------------------
// Client construction
// ---------------------------------------------------------------------------

/// Build the container client for whichever auth mode the environment selects,
/// or `None` when storage is not configured.
fn build_client(
    container: &str,
) -> Result<Option<BlobContainerClient>, Box<dyn std::error::Error + Send + Sync>> {
    // Prefer the emulator/shared-key path when a connection string is present so
    // local development never needs a real Azure account or Entra token.
    if let Some(conn) = env_nonempty("AZURE_STORAGE_CONNECTION_STRING") {
        let parts = ConnectionString::parse(&conn)?;
        let sas = account_sas(&parts.account_name, &parts.account_key)?;
        let url = Url::parse(&format!("{}/{container}?{sas}", parts.blob_endpoint))?;
        let client = BlobContainerClient::new(url, None, None)?;
        return Ok(Some(client));
    }

    if let Some(account_url) = env_nonempty("AZURE_STORAGE_ACCOUNT_URL") {
        let account_url = account_url.trim_end_matches('/');
        let url = Url::parse(&format!("{account_url}/{container}"))?;
        let credential = ManagedIdentityCredential::new(None)?;
        let client = BlobContainerClient::new(url, Some(credential), None)?;
        return Ok(Some(client));
    }

    Ok(None)
}

/// Create the container if it does not already exist. A 409 (already exists) is
/// benign and ignored.
async fn ensure_container(
    client: &BlobContainerClient,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match client.exists().await {
        Ok(true) => Ok(()),
        Ok(false) => match client.create(None).await {
            Ok(_) => Ok(()),
            Err(e) if e.http_status() == Some(azure_core::http::StatusCode::Conflict) => Ok(()),
            Err(e) => Err(Box::new(e)),
        },
        Err(e) => Err(Box::new(e)),
    }
}

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|s| !s.is_empty())
}

// ---------------------------------------------------------------------------
// Azure Storage connection string + account SAS (shared-key / Azurite path)
// ---------------------------------------------------------------------------

/// The pieces of an Azure Storage connection string we need to mint a SAS.
struct ConnectionString {
    account_name: String,
    account_key: String,
    blob_endpoint: String,
}

impl ConnectionString {
    /// Parse the `Key=Value;Key=Value;…` connection-string form, deriving the
    /// blob endpoint when it is not given explicitly (as in a real account's
    /// short connection string).
    fn parse(s: &str) -> Result<Self, String> {
        let mut account_name = None;
        let mut account_key = None;
        let mut blob_endpoint = None;
        let mut protocol = "https".to_string();
        let mut suffix = "core.windows.net".to_string();

        for part in s.split(';') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let (key, value) = part.split_once('=').ok_or_else(|| {
                "malformed AZURE_STORAGE_CONNECTION_STRING (expected Key=Value pairs)".to_string()
            })?;
            match key.trim() {
                "AccountName" => account_name = Some(value.trim().to_string()),
                "AccountKey" => account_key = Some(value.trim().to_string()),
                "BlobEndpoint" => {
                    blob_endpoint = Some(value.trim().trim_end_matches('/').to_string())
                }
                "DefaultEndpointsProtocol" => protocol = value.trim().to_string(),
                "EndpointSuffix" => suffix = value.trim().to_string(),
                _ => {}
            }
        }

        let account_name = account_name
            .ok_or_else(|| "AZURE_STORAGE_CONNECTION_STRING is missing AccountName".to_string())?;
        let account_key = account_key
            .ok_or_else(|| "AZURE_STORAGE_CONNECTION_STRING is missing AccountKey".to_string())?;
        let blob_endpoint =
            blob_endpoint.unwrap_or_else(|| format!("{protocol}://{account_name}.blob.{suffix}"));

        Ok(Self {
            account_name,
            account_key,
            blob_endpoint,
        })
    }
}

/// Mint an **account SAS** granting blob read/write/delete/list/add/create over
/// service+container+object resources, valid for a year. Used only for the
/// Azurite/shared-key path; production uses managed identity instead.
///
/// Follows the string-to-sign layout in
/// <https://learn.microsoft.com/rest/api/storageservices/create-account-sas>.
fn account_sas(account_name: &str, account_key: &str) -> Result<String, String> {
    const SERVICE_VERSION: &str = "2022-11-02";
    const SIGNED_SERVICES: &str = "b"; // blob
    const SIGNED_RESOURCE_TYPES: &str = "sco"; // service + container + object
    const SIGNED_PERMISSIONS: &str = "rwdlac"; // read/write/delete/list/add/create
    const SIGNED_PROTOCOL: &str = "https,http"; // http allowed for the emulator

    let start = "";
    let expiry = (chrono::Utc::now() + chrono::Duration::days(365))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let signed_ip = "";
    let signed_encryption_scope = ""; // required (empty) for version >= 2020-12-06

    let string_to_sign = format!(
        "{account_name}\n{SIGNED_PERMISSIONS}\n{SIGNED_SERVICES}\n{SIGNED_RESOURCE_TYPES}\n\
         {start}\n{expiry}\n{signed_ip}\n{SIGNED_PROTOCOL}\n{SERVICE_VERSION}\n\
         {signed_encryption_scope}\n"
    );

    let key = base64_decode(account_key)?;
    let mut mac = Hmac::<Sha256>::new_from_slice(&key)
        .map_err(|e| format!("invalid storage account key: {e}"))?;
    mac.update(string_to_sign.as_bytes());
    let signature = base64_encode(&mac.finalize().into_bytes());

    Ok(format!(
        "sv={sv}&ss={ss}&srt={srt}&sp={sp}&se={se}&spr={spr}&sig={sig}",
        sv = SERVICE_VERSION,
        ss = SIGNED_SERVICES,
        srt = SIGNED_RESOURCE_TYPES,
        sp = SIGNED_PERMISSIONS,
        se = percent_encode(&expiry),
        spr = percent_encode(SIGNED_PROTOCOL),
        sig = percent_encode(&signature),
    ))
}

fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|e| format!("invalid base64 account key: {e}"))
}

fn base64_encode(bytes: &[u8]) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// RFC 3986 percent-encoding for a SAS query-parameter value (encodes anything
/// outside the unreserved set, so `+`, `/`, `=` and `:` are escaped).
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
