//! `GET /api/version` — build + model info.

use axum::{routing::get, Json, Router};
use serde::{Deserialize, Serialize};

use crate::server::service;

/// `GET /api/version` response body.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VersionResponse {
    pub version: String,
    pub chat_model: String,
    pub embedding_model: String,
    pub captcha_enabled: bool,
}

pub fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().route("/api/version", get(version))
}

async fn version() -> Json<VersionResponse> {
    Json(service::version())
}
