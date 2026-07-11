//! Authentication server functions: register, login, and logout.
//!
//! Password hashing, session-token minting, and the session cookie itself live
//! in [`crate::server::auth`]; these functions orchestrate them and attach the
//! resulting `Set-Cookie` header to the response.

use leptos::prelude::*;

use crate::server_fns::users::User;

/// Attach a `Set-Cookie` header to the outgoing response. Server-only.
#[cfg(feature = "ssr")]
fn set_session_cookie(
    cookie: axum_extra::extract::cookie::Cookie<'static>,
) -> Result<(), ServerFnError> {
    use axum::http::{header::SET_COOKIE, HeaderValue};
    let response = expect_context::<leptos_axum::ResponseOptions>();
    let value = HeaderValue::from_str(&cookie.to_string()).map_err(ServerFnError::new)?;
    response.append_header(SET_COOKIE, value);
    Ok(())
}

/// Sign in with email + password. On success a session is created and its cookie
/// is set on the response; the signed-in [`User`] is returned.
#[server(prefix = "/api")]
pub async fn login(email: String, password: String) -> Result<User, ServerFnError> {
    use crate::server::auth::{build_session_cookie, verify_password};
    use crate::server::db::{sessions, users};

    let email = email.trim();
    let (user, hash) = users::authenticate(email)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("Invalid email or password."))?;
    if !verify_password(&password, &hash) {
        return Err(ServerFnError::new("Invalid email or password."));
    }

    let raw = sessions::create(&user.id)
        .await
        .map_err(ServerFnError::new)?;
    set_session_cookie(build_session_cookie(raw))?;
    Ok(user)
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
    set_session_cookie(build_session_cookie(raw))?;
    Ok(user)
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
    set_session_cookie(clear_session_cookie())?;
    Ok(())
}
