//! Authentication server functions: register, login, MFA verification, logout,
//! and self-service password reset.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::server_fns::users::User;

/// The result of a successful password check
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LoginOutcome {
    /// Password *and* second factor satisfied 
    Authenticated(User),
    /// Password correct, but an emailed one-time code is still required. The
    /// client should route to the MFA screen and call [`verify_mfa`].
    MfaRequired,
}

/// Append a `Set-Cookie` header to the outgoing response. Server-only.
#[cfg(feature = "ssr")]
fn append_cookie(
    cookie: axum_extra::extract::cookie::Cookie<'static>,
) -> Result<(), ServerFnError> {
    use axum::http::{header::SET_COOKIE, HeaderValue};
    let response = expect_context::<leptos_axum::ResponseOptions>();
    let value = HeaderValue::from_str(&cookie.to_string()).map_err(ServerFnError::new)?;
    response.append_header(SET_COOKIE, value);
    Ok(())
}

/// Read a cookie value off the incoming request, if present. Server-only.
#[cfg(feature = "ssr")]
async fn request_cookie(name: &str) -> Option<String> {
    use axum_extra::extract::cookie::CookieJar;
    leptos_axum::extract::<CookieJar>()
        .await
        .ok()
        .and_then(|jar| jar.get(name).map(|c| c.value().to_string()))
}

/// Sign in with email + password. On a correct password this either mints a
/// session immediately (when this browser is a live trusted device for the
/// account) or starts an email one-time-code challenge — see [`LoginOutcome`].
#[server(prefix = "/api")]
pub async fn login(email: String, password: String) -> Result<LoginOutcome, ServerFnError> {
    use crate::server::auth::{
        build_mfa_cookie, build_session_cookie, generate_code, generate_token, verify_password,
        TRUSTED_DEVICE_COOKIE_NAME,
    };
    use crate::server::db::{mfa, sessions, trusted_devices, users};
    use crate::server::email::auth_notifications as auth_email;

    let email = email.trim();
    let (user, hash) = users::authenticate(email)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("Invalid email or password."))?;
    if !verify_password(&password, &hash) {
        return Err(ServerFnError::new("Invalid email or password."));
    }

    // Skip the second factor when this browser is a live trusted device for
    // *this* user (the "remember this device" grant).
    if let Some(token) = request_cookie(TRUSTED_DEVICE_COOKIE_NAME).await {
        if trusted_devices::is_trusted_for_user(&user.id, &token)
            .await
            .unwrap_or(false)
        {
            let raw = sessions::create(&user.id)
                .await
                .map_err(ServerFnError::new)?;
            append_cookie(build_session_cookie(raw))?;
            return Ok(LoginOutcome::Authenticated(user));
        }
    }

    // Otherwise start an email one-time-code challenge.
    let challenge = generate_token();
    let code = generate_code();
    mfa::create(&user.id, &challenge, &code)
        .await
        .map_err(ServerFnError::new)?;
    auth_email::send_mfa_code(&user.email, &user.full_name(), &code)
        .await
        .map_err(|e| {
            tracing::warn!("failed to send MFA code to {}: {e}", user.email);
            ServerFnError::new("We couldn't send your verification code. Please try again.")
        })?;
    append_cookie(build_mfa_cookie(challenge))?;
    Ok(LoginOutcome::MfaRequired)
}

/// Complete a login by verifying the emailed one-time `code`. On success a
/// session is minted; when `remember_device` is set, a 30-day trusted-device
/// cookie is issued so this browser can skip MFA on future logins.
#[server(prefix = "/api")]
pub async fn verify_mfa(code: String, remember_device: bool) -> Result<User, ServerFnError> {
    use crate::server::auth::{
        build_session_cookie, build_trusted_device_cookie, clear_mfa_cookie, generate_token,
        MFA_COOKIE_NAME,
    };
    use crate::server::db::{mfa, sessions, trusted_devices, users};

    let code = code.trim();
    let challenge = request_cookie(MFA_COOKIE_NAME).await.ok_or_else(|| {
        ServerFnError::new("Your verification session expired. Please sign in again.")
    })?;

    let user_id = match mfa::verify(&challenge, code).await.map_err(ServerFnError::new)? {
        mfa::Verify::Ok(uid) => uid,
        mfa::Verify::WrongCode => {
            return Err(ServerFnError::new("That code is incorrect. Please try again."));
        }
        mfa::Verify::Expired => {
            append_cookie(clear_mfa_cookie())?;
            return Err(ServerFnError::new(
                "Your code has expired. Please sign in again.",
            ));
        }
    };

    let user = users::get(&user_id)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("Account not found."))?;

    let raw = sessions::create(&user_id)
        .await
        .map_err(ServerFnError::new)?;
    append_cookie(build_session_cookie(raw))?;
    append_cookie(clear_mfa_cookie())?;

    if remember_device {
        // Best-effort: if recording the trust fails, the user simply re-verifies
        // next time — it must not block a successful sign-in.
        let token = generate_token();
        match trusted_devices::create(&user_id, &token).await {
            Ok(()) => append_cookie(build_trusted_device_cookie(token))?,
            Err(e) => tracing::warn!("failed to record trusted device for {user_id}: {e}"),
        }
    }

    Ok(user)
}

/// Re-send a fresh one-time code for the in-progress MFA challenge (same
/// challenge cookie, new code, reset attempt counter and expiry).
#[server(prefix = "/api")]
pub async fn resend_mfa() -> Result<(), ServerFnError> {
    use crate::server::auth::{generate_code, MFA_COOKIE_NAME};
    use crate::server::db::{mfa, users};
    use crate::server::email::auth_notifications as auth_email;

    let challenge = request_cookie(MFA_COOKIE_NAME).await.ok_or_else(|| {
        ServerFnError::new("Your verification session expired. Please sign in again.")
    })?;
    let user_id = mfa::user_for(&challenge)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| {
            ServerFnError::new("Your verification session expired. Please sign in again.")
        })?;
    let user = users::get(&user_id)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("Account not found."))?;

    let code = generate_code();
    mfa::create(&user_id, &challenge, &code)
        .await
        .map_err(ServerFnError::new)?;
    auth_email::send_mfa_code(&user.email, &user.full_name(), &code)
        .await
        .map_err(|e| {
            tracing::warn!("failed to resend MFA code to {}: {e}", user.email);
            ServerFnError::new("We couldn't resend your code. Please try again.")
        })?;
    Ok(())
}

/// Register a new client account, immediately sign it in, and return the user.
#[server(prefix = "/api")]
pub async fn register(
    first_name: String,
    last_name: String,
    email: String,
    password: String,
) -> Result<User, ServerFnError> {
    use crate::server::auth::{build_session_cookie, hash_password};
    use crate::server::db::{sessions, users};
    use crate::server_fns::users::AccountRole;

    let first_name = first_name.trim().to_string();
    let last_name = last_name.trim().to_string();
    let email = email.trim().to_string();
    if first_name.is_empty() || email.is_empty() || password.is_empty() {
        return Err(ServerFnError::new(
            "Please fill in first name, email, and password.",
        ));
    }
    if users::email_exists(&email)
        .await
        .map_err(ServerFnError::new)?
    {
        return Err(ServerFnError::new(
            "An account with that email already exists.",
        ));
    }

    let id = users::next_id().await.map_err(ServerFnError::new)?;
    let password_hash = hash_password(&password).map_err(ServerFnError::new)?;
    users::insert(
        &id,
        &first_name,
        &last_name,
        &email,
        "",
        "",
        &password_hash,
        AccountRole::Client,
    )
    .await
    .map_err(ServerFnError::new)?;

    let user = users::get(&id)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("User disappeared after insert."))?;
    let raw = sessions::create(&id).await.map_err(ServerFnError::new)?;
    append_cookie(build_session_cookie(raw))?;
    Ok(user)
}

/// Request a password-reset link by email
#[server(prefix = "/api")]
pub async fn request_password_reset(email: String) -> Result<(), ServerFnError> {
    use crate::server::auth::generate_token;
    use crate::server::config::Brand;
    use crate::server::db::{password_reset, users};
    use crate::server::email::auth_notifications as auth_email;

    let email = email.trim();
    if email.is_empty() {
        return Err(ServerFnError::new("Please enter your email address."));
    }

    // Look the account up, but never surface whether it exists.
    if let Ok(Some((user, _))) = users::authenticate(email).await {
        let token = generate_token();
        if let Err(e) = password_reset::create(&user.id, &token).await {
            tracing::warn!("failed to create reset token for {}: {e}", user.email);
        } else {
            let base = Brand::from_env().app_url;
            let base = if base.is_empty() {
                "https://crm.example.org".to_string()
            } else {
                base
            };
            let reset_url = format!("{base}/reset-password?token={token}");
            if let Err(e) =
                auth_email::send_password_reset(&user.email, &user.full_name(), &reset_url).await
            {
                tracing::warn!("failed to send reset email to {}: {e}", user.email);
            }
        }
    }
    Ok(())
}

/// Complete a password reset: validate the (single-use) `token`
#[server(prefix = "/api")]
pub async fn reset_password(token: String, new_password: String) -> Result<(), ServerFnError> {
    use crate::server::auth::hash_password;
    use crate::server::db::{password_reset, sessions, users};

    if new_password.trim().is_empty() {
        return Err(ServerFnError::new("Please choose a new password."));
    }
    let user_id = password_reset::consume(&token)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| {
            ServerFnError::new(
                "This reset link is invalid or has expired. Please request a new one.",
            )
        })?;

    let password_hash = hash_password(&new_password).map_err(ServerFnError::new)?;
    users::set_password_hash(&user_id, &password_hash)
        .await
        .map_err(ServerFnError::new)?;
    // Best effort: sign out any other sessions after a credential change.
    if let Err(e) = sessions::delete_all_for_user(&user_id).await {
        tracing::warn!("failed to clear sessions after reset for {user_id}: {e}");
    }
    Ok(())
}

/// Sign out: delete the current session (best effort) and clear the cookie.
#[server(prefix = "/api")]
pub async fn logout() -> Result<(), ServerFnError> {
    use crate::server::auth::{clear_session_cookie, COOKIE_NAME};
    use crate::server::db::sessions;
    use axum_extra::extract::cookie::CookieJar;

    if let Ok(jar) = leptos_axum::extract::<CookieJar>().await {
        if let Some(cookie) = jar.get(COOKIE_NAME) {
            let _ = sessions::delete(cookie.value()).await;
        }
    }
    append_cookie(clear_session_cookie())?;
    Ok(())
}
