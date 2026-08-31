//! CORS policy for the dedicated `/api/*` router.

use axum::http::{header, Method};
use tower_http::cors::{Any, CorsLayer};

/// CORS policy for the API, driven by `ALLOWED_ORIGINS` (comma-separated, or
/// `*` for any origin).
pub fn cors_layer() -> CorsLayer {
    let origins = std::env::var("ALLOWED_ORIGINS").unwrap_or_else(|_| "*".into());
    let layer = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

    if origins.trim() == "*" {
        layer.allow_origin(Any)
    } else {
        let list: Vec<_> = origins
            .split(',')
            .filter_map(|o| o.trim().parse().ok())
            .collect();
        layer.allow_origin(list)
    }
}
