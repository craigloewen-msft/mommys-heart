//! Browser-facing security headers and same-origin mutation checks.

use axum::body::Body;
use axum::http::{header, HeaderName, HeaderValue, Method, Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;

/// Protect cookie-authenticated mutations and apply reviewed browser headers.
pub async fn protect(request: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    if is_cookie_authenticated_mutation(&request) && !origin_allowed(&request) {
        return Err(StatusCode::FORBIDDEN);
    }

    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("camera=(), microphone=(), geolocation=(), payment=()"),
    );
    headers.insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static(
            "default-src 'self'; base-uri 'self'; frame-ancestors 'none'; form-action 'self'; object-src 'none'; img-src 'self' data:; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; connect-src 'self' ws: wss:",
        ),
    );
    if crate::server::config::is_production() {
        headers.insert(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }
    Ok(response)
}

fn is_cookie_authenticated_mutation(request: &Request<Body>) -> bool {
    !matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    ) && request
        .headers()
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|cookies| {
            cookies
                .split(';')
                .any(|cookie| cookie.trim().starts_with("session="))
        })
}

fn origin_allowed(request: &Request<Body>) -> bool {
    let origin = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok());
    let Some(origin) = origin else {
        return !crate::server::config::is_production();
    };

    if crate::server::config::is_production() {
        return std::env::var("APP_URL").ok().is_some_and(|expected| {
            origin.trim_end_matches('/') == expected.trim_end_matches('/')
        });
    }

    request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|host| {
            origin == format!("http://{host}") || origin == format!("https://{host}")
        })
}
