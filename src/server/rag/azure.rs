//! Azure OpenAI REST client (SSR only).
//!
//! Direct `reqwest` calls to the Azure OpenAI data-plane API — no SDK. Only the
//! embedding and chat-completion endpoints are used.

use serde::{Deserialize, Serialize};
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
    chat_json(
        cfg,
        &cfg.chat_deployment,
        &[ChatTurn::system(system), ChatTurn::user(user)],
        1500,
    )
    .await
}

/// Whether a deployment is one of the reasoning models, which take
/// `max_completion_tokens` and reject `temperature`.
///
/// Matched on the deployment name because that is all the data plane gives us
/// before the first call; deployments here are named after their model.
fn is_reasoning_model(deployment: &str) -> bool {
    let name = deployment.to_ascii_lowercase();
    ["gpt-5", "o1", "o3", "o4"]
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

/// One turn of a chat conversation sent to the model.
#[derive(Clone, Debug, Serialize)]
pub struct ChatTurn {
    pub role: String,
    pub content: String,
}

impl ChatTurn {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
        }
    }
}

/// Run a multi-turn chat completion constrained to a JSON object reply.
///
/// This is what makes an agent loop possible: the caller keeps appending the
/// model's own replies plus the results of whatever it asked for, and calls
/// again with the whole conversation.
///
/// `deployment` selects which model answers, so a caller that needs stronger
/// reasoning (the reporting agent) can use a different deployment from the
/// document chatbot.
pub async fn chat_json(
    cfg: &AzureConfig,
    deployment: &str,
    messages: &[ChatTurn],
    max_tokens: u32,
) -> Result<String, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/openai/deployments/{}/chat/completions?api-version={}",
        base(cfg),
        deployment,
        cfg.api_version,
    );

    let mut body = json!({
        "messages": messages,
        "response_format": { "type": "json_object" },
    });
    // Reasoning models renamed the output budget and reject `temperature`
    // outright, so the shape of the request follows the deployment.
    if is_reasoning_model(deployment) {
        body["max_completion_tokens"] = json!(max_tokens);
    } else {
        body["max_tokens"] = json!(max_tokens);
        body["temperature"] = json!(0.3);
    }

    let resp = client
        .post(&url)
        .header("api-key", &cfg.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("chat request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        // A content-filter refusal is something the person asking can act on;
        // the raw JSON envelope around it is not.
        if let Some(message) = filtered_message(&body) {
            return Err(message);
        }
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

/// Turn Azure's content-filter and policy rejections into a plain sentence.
///
/// These arrive as an HTTP 400 whose body is a JSON envelope; surfacing that
/// verbatim tells the user nothing they can act on.
fn filtered_message(body: &str) -> Option<String> {
    let error = serde_json::from_str::<serde_json::Value>(body)
        .ok()?
        .get("error")?
        .clone();
    let code = error.get("code").and_then(|c| c.as_str()).unwrap_or("");
    let filtered = code.contains("policy")
        || code.contains("content_filter")
        || error
            .get("message")
            .and_then(|m| m.as_str())
            .is_some_and(|m| m.contains("flagged") || m.contains("content management policy"));

    filtered.then(|| {
        "The AI service declined that request under its content policy. Try rephrasing it."
            .to_string()
    })
}
