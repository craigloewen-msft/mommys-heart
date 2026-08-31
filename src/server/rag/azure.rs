//! Azure OpenAI REST client (SSR only).
//!
//! Direct `reqwest` calls to the Azure OpenAI data-plane API — no SDK. Only the
//! embedding and chat-completion endpoints are used.

use serde::Deserialize;
use serde_json::json;

use crate::server::config::AzureConfig;

const EMBED_BATCH: usize = 16;

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingItem>,
}

#[derive(Deserialize)]
struct EmbeddingItem {
    embedding: Vec<f32>,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct ChatMessage {
    content: Option<String>,
}

fn base(cfg: &AzureConfig) -> String {
    cfg.endpoint.trim_end_matches('/').to_string()
}

/// Embed a batch of texts, in groups of [`EMBED_BATCH`].
pub async fn embed_texts(cfg: &AzureConfig, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/openai/deployments/{}/embeddings?api-version={}",
        base(cfg),
        cfg.embedding_deployment,
        cfg.api_version,
    );

    let mut all = Vec::with_capacity(texts.len());
    for batch in texts.chunks(EMBED_BATCH) {
        let resp = client
            .post(&url)
            .header("api-key", &cfg.api_key)
            .json(&json!({ "input": batch }))
            .send()
            .await
            .map_err(|e| format!("embeddings request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("embeddings HTTP {status}: {body}"));
        }

        let parsed: EmbeddingResponse = resp
            .json()
            .await
            .map_err(|e| format!("embeddings decode failed: {e}"))?;
        all.extend(parsed.data.into_iter().map(|i| i.embedding));
    }

    Ok(all)
}

/// Embed a single query string.
pub async fn embed_query(cfg: &AzureConfig, text: &str) -> Result<Vec<f32>, String> {
    let mut out = embed_texts(cfg, std::slice::from_ref(&text.to_string())).await?;
    out.pop()
        .ok_or_else(|| "empty embedding response".to_string())
}

/// Run a chat completion, returning the raw message content (expected JSON).
pub async fn chat_completion(
    cfg: &AzureConfig,
    system: &str,
    user: &str,
) -> Result<String, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/openai/deployments/{}/chat/completions?api-version={}",
        base(cfg),
        cfg.chat_deployment,
        cfg.api_version,
    );

    let resp = client
        .post(&url)
        .header("api-key", &cfg.api_key)
        .json(&json!({
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user },
            ],
            "temperature": 0.3,
            "max_tokens": 1500,
            "response_format": { "type": "json_object" },
        }))
        .send()
        .await
        .map_err(|e| format!("chat request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("chat HTTP {status}: {body}"));
    }

    let parsed: ChatResponse = resp
        .json()
        .await
        .map_err(|e| format!("chat decode failed: {e}"))?;

    Ok(parsed
        .choices
        .into_iter()
        .next()
        .and_then(|c| c.message.content)
        .unwrap_or_else(|| "{}".to_string()))
}
