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

fn env(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default.to_string())
}
