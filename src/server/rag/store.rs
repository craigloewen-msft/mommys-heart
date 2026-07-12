//! In-memory vector store (SSR only).
//!
//! Replaces ChromaDB from the legacy Python app with a simple in-process store:
//! a `Vec` of embedded chunks searched by cosine similarity. Populated once at
//! startup (see `super::ingest`). For the current corpus size this is more than
//! fast enough and removes the persistent-DB dependency.

/// An embedded document chunk held in the store.
#[derive(Clone, Debug)]
pub struct StoredChunk {
    pub text: String,
    pub heading: String,
    pub filename: String,
    pub embedding: Vec<f32>,
}

/// A retrieval result with its cosine similarity (relevance) score.
#[derive(Clone, Debug)]
pub struct Retrieved {
    pub text: String,
    pub heading: String,
    pub filename: String,
    pub relevance: f32,
}

/// The vector store: a flat list of embedded chunks.
#[derive(Clone, Debug, Default)]
pub struct VectorStore {
    pub chunks: Vec<StoredChunk>,
}

impl VectorStore {
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// Return the `top_k` chunks most similar to `query`, highest first.
    pub fn search(&self, query: &[f32], top_k: usize) -> Vec<Retrieved> {
        let mut scored: Vec<Retrieved> = self
            .chunks
            .iter()
            .map(|c| Retrieved {
                text: c.text.clone(),
                heading: c.heading.clone(),
                filename: c.filename.clone(),
                relevance: cosine_similarity(query, &c.embedding),
            })
            .collect();

        scored.sort_by(|a, b| {
            b.relevance
                .partial_cmp(&a.relevance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(top_k);
        scored
    }
}

/// Cosine similarity between two vectors; 0.0 if either is degenerate.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}
