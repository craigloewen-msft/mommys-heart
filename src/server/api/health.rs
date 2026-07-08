//! `GET /api/health` — liveness probe.

use axum::{routing::get, Json, Router};

use crate::types::HealthResponse;

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
