//! Server-side authentication: password hashing (argon2), session cookies, and
//! the [`AuthUser`] extractor that identifies the signed-in user.
//!
//! This is the *authentication* half of access control ("who are you?"). The
//! *authorization* half ("what may you do?") — admin/capability/visibility
//! checks — lives in [`crate::server::permissions`].

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};

use crate::server::db::{sessions, users};
use crate::server_fns::users::User;

/// Name of the session cookie.
pub const COOKIE_NAME: &str = "session";

/// Hash a plaintext password with argon2 (PHC string form).
pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}

/// Verify a plaintext password against a stored argon2 hash.
pub fn verify_password(password: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

/// Whether session cookies should carry the `Secure` flag (production/HTTPS).
fn cookie_secure() -> bool {
    std::env::var("COOKIE_SECURE")
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(false)
}

/// Build the `Set-Cookie` for a freshly minted session token.
pub fn build_session_cookie(raw_token: String) -> Cookie<'static> {
    Cookie::build((COOKIE_NAME, raw_token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(cookie_secure())
        .max_age(time::Duration::days(30))
        .build()
}

/// Build a cookie that clears the session on the client.
pub fn clear_session_cookie() -> Cookie<'static> {
    Cookie::build((COOKIE_NAME, ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(cookie_secure())
        .max_age(time::Duration::seconds(0))
        .build()
}

/// Extractor that resolves the session cookie to the signed-in [`User`],
/// rejecting with `401 Unauthorized` when there is no valid session.
pub struct AuthUser(pub User);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar
            .get(COOKIE_NAME)
            .map(|c| c.value().to_string())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        // Resolve session → user → capabilities in a single round-trip. On a
        // networked database this halves the per-request auth cost versus the
        // old session-lookup-then-user-lookup chain.
        let token_hash = sessions::hash_token(&token);
        let user = users::resolve_by_session_token(&token_hash)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::UNAUTHORIZED)?;

        Ok(AuthUser(user))
    }
}
