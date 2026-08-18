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

/// Name of the short-lived cookie that carries a pending MFA challenge token
/// between the password step and the code-verification step.
pub const MFA_COOKIE_NAME: &str = "mfa";

/// Name of the long-lived "remember this device" cookie that lets a trusted
/// browser skip the OTP step on future logins.
pub const TRUSTED_DEVICE_COOKIE_NAME: &str = "trusted_device";

/// Name of the short-lived cookie that carries a pending *registration*
/// email-verification challenge between the sign-up form and the code step.
pub const REGISTER_COOKIE_NAME: &str = "register";

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

/// Whether authentication cookies should carry the `Secure` flag.
fn cookie_secure() -> bool {
    crate::server::config::is_production()
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

/// Build the short-lived `Set-Cookie` holding a pending MFA challenge token. It
/// only needs to outlive the code entry, so its lifetime matches the challenge
/// TTL rather than the session.
pub fn build_mfa_cookie(raw_token: String) -> Cookie<'static> {
    Cookie::build((MFA_COOKIE_NAME, raw_token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(cookie_secure())
        .max_age(time::Duration::minutes(10))
        .build()
}

/// Build a cookie that clears the pending MFA challenge on the client.
pub fn clear_mfa_cookie() -> Cookie<'static> {
    Cookie::build((MFA_COOKIE_NAME, ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(cookie_secure())
        .max_age(time::Duration::seconds(0))
        .build()
}

/// Build the long-lived "remember this device" cookie carrying a trusted-device
/// token. Valid for 30 days, matching the stored grant.
pub fn build_trusted_device_cookie(raw_token: String) -> Cookie<'static> {
    Cookie::build((TRUSTED_DEVICE_COOKIE_NAME, raw_token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(cookie_secure())
        .max_age(time::Duration::days(30))
        .build()
}

/// Build the short-lived `Set-Cookie` holding a pending registration
/// email-verification challenge token. Like the MFA cookie, it only needs to
/// outlive the code entry, so its lifetime matches the challenge TTL.
pub fn build_register_cookie(raw_token: String) -> Cookie<'static> {
    Cookie::build((REGISTER_COOKIE_NAME, raw_token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(cookie_secure())
        .max_age(time::Duration::minutes(10))
        .build()
}

/// Build a cookie that clears the pending registration challenge on the client.
pub fn clear_register_cookie() -> Cookie<'static> {
    Cookie::build((REGISTER_COOKIE_NAME, ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(cookie_secure())
        .max_age(time::Duration::seconds(0))
        .build()
}

/// Generate a fresh opaque token (256 bits, hex-encoded) for MFA challenges,
/// trusted devices, and password-reset links.
pub fn generate_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Generate a random 6-digit numeric one-time code (zero-padded, e.g. `"048213"`).
pub fn generate_code() -> String {
    use rand::Rng;
    let n: u32 = rand::thread_rng().gen_range(0..1_000_000);
    format!("{n:06}")
}

/// Whether an explicitly non-production process may bypass emailed MFA.
pub fn skip_mfa() -> bool {
    if crate::server::config::is_production() {
        return false;
    }
    cfg!(debug_assertions)
        || std::env::var("ALLOW_MFA_BYPASS")
            .is_ok_and(|value| value.eq_ignore_ascii_case("true") || value == "1")
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
