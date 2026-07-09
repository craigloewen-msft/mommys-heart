//! Typed client the CRM website uses to talk to the dedicated API.
//!
//! Compiled for both targets. During SSR the functions call the server service
//! layer directly (no self-HTTP round-trip); in the browser they issue real
//! HTTP requests to `/api/*` — the same endpoints external consumers (the
//! Squarespace widget) use.

use crate::types::{ChatRequest, ChatResponse, VersionResponse};

// ---------------------------------------------------------------------------
// SSR branch — call the service layer directly.
// ---------------------------------------------------------------------------
#[cfg(feature = "ssr")]
pub async fn get_version() -> Result<VersionResponse, String> {
    Ok(crate::server::service::version())
}

#[cfg(feature = "ssr")]
pub async fn send_chat(req: ChatRequest) -> Result<ChatResponse, String> {
    Ok(crate::server::service::chat(req.message.trim(), req.conversation_id).await)
}

// ---------------------------------------------------------------------------
// Browser branch — fetch the dedicated API over HTTP.
// ---------------------------------------------------------------------------
#[cfg(not(feature = "ssr"))]
async fn get_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, String> {
    let resp = gloo_net::http::Request::get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.json::<T>().await.map_err(|e| e.to_string())
}

#[cfg(not(feature = "ssr"))]
async fn post_json<B: serde::Serialize, T: serde::de::DeserializeOwned>(
    url: &str,
    body: &B,
) -> Result<T, String> {
    let resp = gloo_net::http::Request::post(url)
        .json(body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.json::<T>().await.map_err(|e| e.to_string())
}

#[cfg(not(feature = "ssr"))]
pub async fn get_version() -> Result<VersionResponse, String> {
    send_wrapper::SendWrapper::new(async move { get_json("/api/version").await }).await
}

#[cfg(not(feature = "ssr"))]
pub async fn send_chat(req: ChatRequest) -> Result<ChatResponse, String> {
    send_wrapper::SendWrapper::new(async move { post_json("/api/chat", &req).await }).await
}
