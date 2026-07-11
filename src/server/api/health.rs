//! `GET /api/health` — liveness probe.

use axum::{routing::get, Json, Router};
use serde::{Deserialize, Serialize};

/// `GET /api/health` response body.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
}

pub fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().route("/api/health", get(health))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
    })
}
