//! `GET /api/version` — build + model info.

use axum::{routing::get, Json, Router};

use crate::server::service;
use crate::types::VersionResponse;

pub fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().route("/api/version", get(version))
}

async fn version() -> Json<VersionResponse> {
    Json(service::version())
}
