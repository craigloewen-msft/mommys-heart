//! Retrieval-Augmented Generation pipeline (SSR only).
//!
//! Documents are parsed, chunked, embedded (Azure OpenAI) and held in an
//! in-memory [`store::VectorStore`]. `/api/chat` embeds the question, retrieves
//! the top matches, and asks the chat model to answer — preferring the
//! documents but falling back to general knowledge.
//!
//! Everything degrades gracefully: if Azure OpenAI is not configured (no
//! credentials) the endpoint still responds, just without document grounding.

pub mod azure;
pub mod documents;
pub mod store;

use std::path::PathBuf;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;

use crate::server::config::AzureConfig;
use store::{StoredChunk, VectorStore};

/// A single cited source returned by the RAG chat endpoint.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceInfo {
    pub filename: String,
    pub heading: String,
    pub snippet: String,
    pub relevance: f64,
    pub anchor: String,
}

/// Where a chat answer came from.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Documents,
    GeneralKnowledge,
    Mixed,
}

/// `POST /api/chat` response body — the RAG engine's public answer shape,
/// serialized directly by the chat endpoint.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub answer: String,
    pub sources: Vec<SourceInfo>,
    pub source_type: SourceType,
    /// The conversation this turn belongs to. Echo it back on the next request
    /// to keep the thread continuous.
    #[serde(default)]
    pub conversation_id: Option<String>,
}

const TOP_K: usize = 5;

const SYSTEM_PROMPT: &str = r#"You are the Mommy's Heart AI Assistant. You are given numbered context excerpts retrieved from Mommy's Heart's own documents. They may or may not be relevant to the user's question.

Answer the user's question following these rules:
1. PREFER the provided document context. If one or more excerpts actually contain the answer, base your answer on them.
2. If the excerpts do NOT contain the answer (even though they were retrieved), DO still answer the question directly using your own general knowledge. Do not refuse or say "the documents don't contain this" — just answer helpfully from what you know.
3. Only say you cannot help if you genuinely do not know the answer and it is not in the excerpts.
4. Be concise but thorough. Do NOT include any source list, citations, or "Sources" section in your answer text — sources are handled separately by the application.

Respond with a JSON object containing exactly these fields:
- "answer": your answer as plain text, with no source list.
- "used_sources": an array of the integer numbers of the excerpts you ACTUALLY used to write the answer (e.g. [1, 3]). Use an empty array [] if you did not use any excerpt (for example when answering from general knowledge or when the documents are irrelevant). Never list an excerpt you did not actually rely on.
- "source_type": one of:
    - "documents"          → your answer came entirely from the listed excerpts.
    - "general_knowledge"  → you used your own general knowledge (used_sources must be []).
    - "mixed"              → you combined listed excerpts with your own general knowledge.
"#;

/// Global vector store, populated at startup and by `/api/reingest`.
static STORE: OnceLock<RwLock<VectorStore>> = OnceLock::new();

fn store() -> &'static RwLock<VectorStore> {
    STORE.get_or_init(|| RwLock::new(VectorStore::default()))
}

/// Stats returned by an ingestion run.
#[derive(Clone, Debug, serde::Serialize)]
pub struct IngestStats {
    pub status: String,
    pub files_processed: usize,
    pub chunks_created: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Spawn a background ingestion task so the server starts serving immediately
/// while documents are parsed/embedded. An unavailable Azure leaves the
/// endpoint answering without document grounding.
pub fn start_background_ingest() {
    tokio::spawn(async {
        match ingest().await {
            Ok(stats) => tracing::info!(
                "RAG ingest: {} ({} files, {} chunks)",
                stats.status,
                stats.files_processed,
                stats.chunks_created
            ),
            Err(e) => tracing::warn!("RAG ingest failed: {e}"),
        }
    });
}

/// Parse, chunk, embed and load all `.docx` files into the vector store.
pub async fn ingest() -> Result<IngestStats, String> {
    let cfg = AzureConfig::from_env();

    if !cfg.is_configured() {
        return Ok(IngestStats {
            status: "skipped".into(),
            files_processed: 0,
            chunks_created: 0,
            reason: Some("Azure OpenAI is not configured".into()),
        });
    }

    let files = docx_files(&cfg.docs_dir);
    if files.is_empty() {
        return Ok(IngestStats {
            status: "skipped".into(),
            files_processed: 0,
            chunks_created: 0,
            reason: Some(format!("no .docx files in '{}'", cfg.docs_dir)),
        });
    }

    // Extract + chunk every document.
    let mut all_chunks = Vec::new();
    for path in &files {
        let sections = documents::extract_sections(path)?;
        if !sections.is_empty() {
            all_chunks.extend(documents::chunk_sections(&sections));
        }
    }

    if all_chunks.is_empty() {
        return Ok(IngestStats {
            status: "error".into(),
            files_processed: files.len(),
            chunks_created: 0,
            reason: Some("no content found in documents".into()),
        });
    }

    // Embed everything BEFORE swapping the store (safe replacement).
    let texts: Vec<String> = all_chunks.iter().map(|c| c.text.clone()).collect();
    let embeddings = azure::embed_texts(&cfg, &texts).await?;
    if embeddings.len() != all_chunks.len() {
        return Err(format!(
            "embedding count {} != chunk count {}",
            embeddings.len(),
            all_chunks.len()
        ));
    }

    let stored: Vec<StoredChunk> = all_chunks
        .into_iter()
        .zip(embeddings)
        .map(|(c, embedding)| StoredChunk {
            text: c.text,
            heading: c.heading,
            filename: c.filename,
            embedding,
        })
        .collect();

    let chunks_created = stored.len();
    {
        let mut guard = store().write().await;
        guard.chunks = stored;
    }

    Ok(IngestStats {
        status: "success".into(),
        files_processed: files.len(),
        chunks_created,
        reason: None,
    })
}

/// Force a re-ingest (exposed via `POST /api/reingest`).
pub async fn reingest() -> Result<IngestStats, String> {
    ingest().await
}

/// Run the full RAG pipeline for a question, always returning a `ChatResponse`.
///
/// On any failure (Azure unconfigured, network error, empty store) it returns a
/// graceful `general_knowledge` fallback instead of erroring, so `/api/chat`
/// never hard-fails for the widget or the app UI.
pub async fn answer(question: &str) -> ChatResponse {
    let cfg = AzureConfig::from_env();

    if !cfg.is_configured() {
        return fallback(
            "The assistant isn't fully configured yet (Azure OpenAI credentials \
             are missing), so I can't answer from Mommy's Heart's documents right \
             now. Please try again later.",
        );
    }

    match run_pipeline(&cfg, question).await {
        Ok(resp) => resp,
        Err(e) => {
            leptos::logging::warn!("RAG query failed: {e}");
            fallback(
                "Sorry — I ran into a problem answering that just now. Please try \
                 again in a moment.",
            )
        }
    }
}

async fn run_pipeline(cfg: &AzureConfig, question: &str) -> Result<ChatResponse, String> {
    // 1. Embed the question and retrieve the top matches.
    let query_embedding = azure::embed_query(cfg, question).await?;
    let contexts = {
        let guard = store().read().await;
        guard.search(&query_embedding, TOP_K)
    };

    // 2. Build the numbered context block for the model.
    let mut context_block = String::new();
    for (i, ctx) in contexts.iter().enumerate() {
        context_block.push_str(&format!(
            "\n--- Source {}: {} | Section: {} ---\n{}\n",
            i + 1,
            ctx.filename,
            ctx.heading,
            ctx.text
        ));
    }

    // 3. Ask the chat model.
    let user = format!("Context from documents:\n{context_block}\n\nQuestion: {question}");
    let raw = azure::chat_completion(cfg, SYSTEM_PROMPT, &user).await?;

    // 4. Parse the JSON answer, reconcile the source type, and build sources.
    Ok(build_response(&raw, &contexts))
}

#[derive(Deserialize, Default)]
struct ModelAnswer {
    answer: Option<String>,
    #[serde(default)]
    used_sources: Vec<Value>,
    source_type: Option<String>,
}

fn build_response(raw: &str, contexts: &[store::Retrieved]) -> ChatResponse {
    // Parse the model's JSON; fall back to treating the whole thing as the answer.
    let (answer, mut source_type, used_indices) = match serde_json::from_str::<ModelAnswer>(raw) {
        Ok(parsed) => {
            let answer = parsed
                .answer
                .filter(|a| !a.is_empty())
                .unwrap_or_else(|| "(no answer)".to_string());
            let source_type = parsed
                .source_type
                .unwrap_or_else(|| "documents".to_string());
            let used: Vec<usize> = parsed
                .used_sources
                .iter()
                .filter_map(coerce_index)
                .collect();
            (answer, source_type, used)
        }
        Err(_) => (raw.to_string(), "general_knowledge".to_string(), Vec::new()),
    };

    if !matches!(
        source_type.as_str(),
        "documents" | "general_knowledge" | "mixed"
    ) {
        source_type = "documents".to_string();
    }

    // Map 1-based indices back to retrieved contexts, keeping valid ones.
    let used_contexts: Vec<&store::Retrieved> = used_indices
        .iter()
        .filter(|&&i| i >= 1 && i <= contexts.len())
        .map(|&i| &contexts[i - 1])
        .collect();

    // Reconcile the declared mode with what was actually cited.
    if used_contexts.is_empty() {
        if source_type == "documents" || source_type == "mixed" {
            source_type = "general_knowledge".to_string();
        }
    } else if source_type == "general_knowledge" {
        source_type = "mixed".to_string();
    }

    // Build the deduplicated list of used sources.
    let mut sources = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for ctx in used_contexts {
        let key = (ctx.filename.clone(), ctx.heading.clone());
        if !seen.insert(key) {
            continue;
        }
        let first_heading = ctx.heading.split(',').next().unwrap_or("").trim();
        let snippet = if ctx.text.chars().count() > 200 {
            let truncated: String = ctx.text.chars().take(200).collect();
            format!("{truncated}...")
        } else {
            ctx.text.clone()
        };
        sources.push(SourceInfo {
            filename: ctx.filename.clone(),
            heading: ctx.heading.clone(),
            snippet,
            relevance: round3(ctx.relevance),
            anchor: documents::slugify(first_heading),
        });
    }

    ChatResponse {
        answer,
        sources,
        source_type: parse_source_type(&source_type),
        conversation_id: None,
    }
}

fn parse_source_type(s: &str) -> SourceType {
    match s {
        "documents" => SourceType::Documents,
        "mixed" => SourceType::Mixed,
        _ => SourceType::GeneralKnowledge,
    }
}

/// Coerce a JSON value (int, float, or numeric string) into a source index.
fn coerce_index(v: &Value) -> Option<usize> {
    match v {
        Value::Number(n) => n.as_i64().and_then(|i| usize::try_from(i).ok()),
        Value::String(s) => s.trim().parse::<usize>().ok(),
        _ => None,
    }
}

fn round3(v: f32) -> f64 {
    ((v as f64) * 1000.0).round() / 1000.0
}

fn fallback(message: &str) -> ChatResponse {
    ChatResponse {
        answer: message.to_string(),
        sources: Vec::new(),
        source_type: SourceType::GeneralKnowledge,
        conversation_id: None,
    }
}

fn docx_files(docs_dir: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = match std::fs::read_dir(docs_dir) {
        Ok(rd) => rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("docx"))
            .collect(),
        Err(_) => Vec::new(),
    };
    files.sort();
    files
}
