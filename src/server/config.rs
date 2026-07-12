//! Azure OpenAI + RAG configuration, read from the environment (SSR only).
//!
//! Mirrors the legacy Python `app/config.py` settings. Everything is optional so
//! the server boots and degrades gracefully when Azure OpenAI is not configured
//! (e.g. local development without credentials).

/// Azure OpenAI connection + deployment settings.
#[derive(Clone, Debug)]
pub struct AzureConfig {
    pub endpoint: String,
    pub api_key: String,
    pub chat_deployment: String,
    pub embedding_deployment: String,
    pub api_version: String,
    pub docs_dir: String,
}

impl AzureConfig {
    /// Read configuration from the environment, applying the same defaults as
    /// the legacy Python app.
    pub fn from_env() -> Self {
        Self {
            endpoint: env("AZURE_OPENAI_ENDPOINT", ""),
            api_key: env("AZURE_OPENAI_API_KEY", ""),
            chat_deployment: env("AZURE_OPENAI_CHAT_DEPLOYMENT", "gpt-4o"),
            embedding_deployment: env(
                "AZURE_OPENAI_EMBEDDING_DEPLOYMENT",
                "text-embedding-ada-002",
            ),
            api_version: env("AZURE_OPENAI_API_VERSION", "2024-12-01-preview"),
            docs_dir: env("DOCS_DIR", "docs"),
        }
    }

    /// Whether Azure OpenAI credentials are present. When `false`, the RAG
    /// pipeline is disabled and `/api/chat` returns a graceful fallback.
    pub fn is_configured(&self) -> bool {
        !self.endpoint.is_empty() && !self.api_key.is_empty()
    }
}

/// Azure Communication Services (ACS) Email connection settings. Read from the
/// environment, all optional so the server boots and email delivery degrades to
/// a graceful no-op when ACS is not configured (e.g. local development).
#[derive(Clone, Debug)]
pub struct EmailConfig {
    /// Resource endpoint, e.g. `https://your-resource.communication.azure.com`.
    pub endpoint: String,
    /// The resource's base64 access key (the `accesskey=` value of the ACS
    /// connection string), used to sign requests with HMAC-SHA256.
    pub access_key: String,
    /// A verified sender address, e.g. `DoNotReply@your-domain.azurecomm.net`.
    pub sender_address: String,
    /// When `true`, notification emails are *not* actually sent: the dispatcher
    /// logs what it would have sent instead. Used to keep local/test runs from
    /// emailing real recipients. Controlled by `EMAIL_DRY_RUN`.
    pub dry_run: bool,
}

impl EmailConfig {
    /// Read ACS Email configuration from the environment.
    pub fn from_env() -> Self {
        Self {
            endpoint: env("ACS_EMAIL_ENDPOINT", ""),
            access_key: env("ACS_EMAIL_ACCESS_KEY", ""),
            sender_address: env("ACS_EMAIL_SENDER_ADDRESS", ""),
            dry_run: env_bool("EMAIL_DRY_RUN", false),
        }
    }

    /// Whether all ACS Email settings are present. When `false`, email delivery
    /// is skipped (a graceful no-op) rather than erroring.
    pub fn is_configured(&self) -> bool {
        !self.endpoint.is_empty()
            && !self.access_key.is_empty()
            && !self.sender_address.is_empty()
    }
}

fn env(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// Read a boolean environment flag. Truthy values (case-insensitive) are
/// `true`, `1`, `yes`, and `on`; anything else falls back to `default`.
fn env_bool(key: &str, default: bool) -> bool {
    match std::env::var(key) {
        Ok(v) => matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "true" | "1" | "yes" | "on"
        ),
        Err(_) => default,
    }
}
