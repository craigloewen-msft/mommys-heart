//! Shared API contract types used by both the Axum JSON API (server) and the
//! Leptos UI (client). Keeping them in one place guarantees the website and the
//! dedicated API never drift apart.

use serde::{Deserialize, Serialize};

/// A single cited source returned by the RAG chat endpoint.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceInfo {
    pub filename: String,
    pub heading: String,
    pub snippet: String,
    pub relevance: f64,
    pub anchor: String,
}

/// Where a chat answer came from — mirrors the legacy Python contract.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Documents,
    GeneralKnowledge,
    Mixed,
}

/// `POST /api/chat` request body.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    #[serde(default)]
    pub captcha_token: Option<String>,
}

/// `POST /api/chat` response body.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub answer: String,
    pub sources: Vec<SourceInfo>,
    pub source_type: SourceType,
}

/// `GET /api/version` response body.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VersionResponse {
    pub version: String,
    pub chat_model: String,
    pub embedding_model: String,
    pub captcha_enabled: bool,
}

/// `GET /api/health` response body.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
}

/// CRM contact lifecycle stage.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContactStatus {
    Lead,
    Active,
    Inactive,
}

impl ContactStatus {
    /// Human label for display.
    pub fn label(self) -> &'static str {
        match self {
            ContactStatus::Lead => "Lead",
            ContactStatus::Active => "Active",
            ContactStatus::Inactive => "Inactive",
        }
    }

    /// Tailwind badge classes for this status.
    pub fn badge_classes(self) -> &'static str {
        match self {
            ContactStatus::Active => "bg-green-100 text-green-700",
            ContactStatus::Lead => "bg-blue-100 text-blue-700",
            ContactStatus::Inactive => "bg-gray-100 text-gray-600",
        }
    }
}

/// A CRM contact.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub company: String,
    pub status: ContactStatus,
    pub notes: String,
    pub created_at: String,
}
