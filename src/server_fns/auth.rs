//! Authentication server functions: register (with email-OTP verification),
//! login, MFA verification, logout, and self-service password reset.

use leptos::prelude::*;
use leptos::server_fn::codec::{MultipartData, MultipartFormData};
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
use crate::helpers::case_intake::CaseIntake;
use crate::server_fns::users::{User, UserSummary};

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

/// Opportunistically remove abandoned registration rows and their staged
/// agreement blobs whenever somebody starts a fresh signup.
#[cfg(feature = "ssr")]
async fn cleanup_expired_registrations() {
    use crate::server::db::pending_registrations;
    use crate::server::storage;

    match pending_registrations::delete_expired().await {
        Ok(paths) => {
            for path in paths {
                if let Err(error) = storage::delete(&path).await {
                    tracing::warn!("failed to clean abandoned signup blob '{path}': {error}");
                }
            }
        }
        Err(error) => tracing::warn!("failed to prune expired registrations: {error}"),
    }
}

/// Sign in with email + password. On a correct password this either mints a
/// session immediately (when this browser is a live trusted device for the
/// account) or starts an email one-time-code challenge — see [`LoginOutcome`].
#[server(prefix = "/api")]
pub async fn login(email: String, password: String) -> Result<LoginOutcome, ServerFnError> {
    use crate::server::auth::{
        build_mfa_cookie, build_session_cookie, generate_code, generate_token, skip_mfa,
        verify_password, TRUSTED_DEVICE_COOKIE_NAME,
    };
    use crate::server::db::{mfa, sessions, throttle, trusted_devices, users};
    use crate::server::email::auth_notifications as auth_email;

    let email = email.trim();

    // Brute-force gate: reject up front while the account is locked, *before*
    // hitting the (deliberately expensive) argon2 verify or sending any OTP mail.
    if let Some(secs) = throttle::seconds_locked(throttle::Action::Login, email)
        .await
        .map_err(ServerFnError::new)?
    {
        let minutes = throttle::minutes_remaining(secs);
        return Err(ServerFnError::new(format!(
            "Too many failed attempts. Please try again in about {minutes} minute{}.",
            if minutes == 1 { "" } else { "s" }
        )));
    }

    let auth = users::authenticate(email)
        .await
        .map_err(ServerFnError::new)?;
    let (user, hash) = match auth {
        Some(pair) => pair,
        None => {
            // Record failures for unknown emails too, so the throttle behaves
            // identically whether or not the account exists (no enumeration).
            let _ = throttle::record_failure(throttle::Action::Login, email).await;
            return Err(ServerFnError::new("Invalid email or password."));
        }
    };
    if !verify_password(&password, &hash) {
        let _ = throttle::record_failure(throttle::Action::Login, email).await;
        return Err(ServerFnError::new("Invalid email or password."));
    }

    // Correct password: reset the failure counter for this account.
    let _ = throttle::clear(throttle::Action::Login, email).await;

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

    if skip_mfa() {
        tracing::warn!(
            "WARN: skipping MFA for {} — no deliverable email or debug build. \
             Signed in on the password alone; never run production this way.",
            user.email
        );
        let raw = sessions::create(&user.id)
            .await
            .map_err(ServerFnError::new)?;
        append_cookie(build_session_cookie(raw))?;
        return Ok(LoginOutcome::Authenticated(user));
    }

    // Otherwise start an email one-time-code challenge
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

/// Begin registering a new client account. Rather than creating the account
/// immediately, this validates the input, emails a 6-digit verification code to
/// the address, and stashes the pending signup behind a short-lived `register`
/// cookie. The account is only created once the code is confirmed via
/// [`verify_registration`], so an unverified email never becomes a real account.
#[server(prefix = "/api")]
pub async fn register(
    first_name: String,
    last_name: String,
    email: String,
    password: String,
) -> Result<(), ServerFnError> {
    use crate::server::auth::{build_register_cookie, generate_code, generate_token, hash_password, REGISTER_COOKIE_NAME};
    use crate::server::db::pending_registrations::{self, PendingAccount};
    use crate::server::db::{throttle, users};
    use crate::server::email::auth_notifications as auth_email;
    use crate::server::storage;

    cleanup_expired_registrations().await;

    let first_name = first_name.trim().to_string();
    let last_name = last_name.trim().to_string();
    let email = email.trim().to_lowercase();
    if first_name.is_empty() || email.is_empty() || password.is_empty() {
        return Err(ServerFnError::new(
            "Please fill in first name, email, and password.",
        ));
    }

    // Rate-limit sign-up attempts per email so this endpoint cannot be used to
    // email-bomb a victim with verification codes.
    if let Some(secs) = throttle::seconds_locked(throttle::Action::Register, &email)
        .await
        .map_err(ServerFnError::new)?
    {
        let minutes = throttle::minutes_remaining(secs);
        return Err(ServerFnError::new(format!(
            "Too many sign-up attempts. Please try again in about {minutes} minute{}.",
            if minutes == 1 { "" } else { "s" }
        )));
    }
    let _ = throttle::record_failure(throttle::Action::Register, &email).await;

    if users::email_exists(&email)
        .await
        .map_err(ServerFnError::new)?
    {
        return Err(ServerFnError::new(
            "An account with that email already exists.",
        ));
    }

    let password_hash = hash_password(&password).map_err(ServerFnError::new)?;
    let challenge = generate_token();
    let code = generate_code();
    let account = PendingAccount {
        first_name,
        last_name,
        email: email.clone(),
        password_hash,
    };
    pending_registrations::create(&challenge, &account, &code)
        .await
        .map_err(ServerFnError::new)?;
    if let Err(error) = auth_email::send_email_verification(&email, &account.full_name(), &code).await {
        let _ = pending_registrations::delete(&challenge).await;
        tracing::warn!("failed to send verification code to {email}: {error}");
        return Err(ServerFnError::new(
            "We couldn't send your verification code. Please try again.",
        ));
    }
    if let Some(previous_challenge) = request_cookie(REGISTER_COOKIE_NAME).await {
        if let Ok(Some(previous_blob)) = pending_registrations::delete(&previous_challenge).await {
            let _ = storage::delete(&previous_blob).await;
        }
    }
    append_cookie(build_register_cookie(challenge))?;
    Ok(())
}

/// Begin a customer signup that will create both an account and a case after
/// email verification. The signed agreement is staged at its final blob path;
/// its database row remains pending until [`verify_registration`] commits the
/// complete account and case transaction.
#[server(prefix = "/api", input = MultipartFormData)]
pub async fn register_case_signup(data: MultipartData) -> Result<(), ServerFnError> {
    use crate::server::auth::{build_register_cookie, generate_code, generate_token, hash_password, REGISTER_COOKIE_NAME};
    use crate::server::db::pending_registrations::{self, PendingAccount, PendingCaseSignup};
    use crate::server::db::{evidence, ids, pool, throttle, users};
    use crate::server::email::auth_notifications as auth_email;
    use crate::server::storage;
    use sha2::{Digest, Sha256};
    use std::collections::HashMap;

    cleanup_expired_registrations().await;

    let mut multipart = data
        .into_inner()
        .ok_or_else(|| ServerFnError::new("Malformed signup form."))?;
    let mut first_name = String::new();
    let mut last_name = String::new();
    let mut email = String::new();
    let mut password = String::new();
    let mut password_confirmation = String::new();
    let mut intake_json = String::new();
    let mut agreement_filename = None;
    let mut agreement_bytes = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| ServerFnError::new(format!("Malformed signup form: {error}")))?
    {
        let field_name = field.name().map(str::to_owned);
        let file_name = field.file_name().map(str::to_owned);
        match field_name.as_deref() {
            Some("agreement") => {
                agreement_filename = Some(file_name.unwrap_or_else(|| "agreement.docx".to_string()));
                agreement_bytes = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|error| {
                            ServerFnError::new(format!("Could not read signed agreement: {error}"))
                        })?
                        .to_vec(),
                );
            }
            Some("first_name") => first_name = field.text().await.unwrap_or_default(),
            Some("last_name") => last_name = field.text().await.unwrap_or_default(),
            Some("email") => email = field.text().await.unwrap_or_default(),
            Some("password") => password = field.text().await.unwrap_or_default(),
            Some("password_confirmation") => {
                password_confirmation = field.text().await.unwrap_or_default()
            }
            Some("intake_json") => intake_json = field.text().await.unwrap_or_default(),
            _ => {
                let _ = field.bytes().await;
            }
        }
    }

    let first_name = first_name.trim().to_string();
    let last_name = last_name.trim().to_string();
    let email = email.trim().to_lowercase();
    if first_name.is_empty() || last_name.is_empty() || email.is_empty() || password.is_empty() {
        return Err(ServerFnError::new(
            "Please fill in first name, last name, email, and password.",
        ));
    }
    if password != password_confirmation {
        return Err(ServerFnError::new("The passwords do not match."));
    }
    let intake: CaseIntake = serde_json::from_str(&intake_json)
        .map_err(|_| ServerFnError::new("The case information could not be read."))?;
    let intake_json = serde_json::to_string(&intake).map_err(ServerFnError::new)?;

    let (agreement_filename, agreement_bytes) = match (agreement_filename, agreement_bytes) {
        (Some(filename), Some(bytes)) => (filename, bytes),
        _ => return Err(ServerFnError::new("Please upload the signed agreement.")),
    };
    let agreement_filename = crate::server_fns::evidence::sanitize_filename(&agreement_filename);
    if !agreement_filename.to_ascii_lowercase().ends_with(".docx") {
        return Err(ServerFnError::new(
            "The signed agreement must be uploaded as a .docx file.",
        ));
    }
    let content_type = crate::server_fns::evidence::validate_docx(&agreement_bytes)
        .map_err(ServerFnError::new)?;
    if !storage::is_configured() {
        return Err(ServerFnError::new(
            "Agreement storage is not configured on this server.",
        ));
    }

    if let Some(seconds) = throttle::seconds_locked(throttle::Action::Register, &email)
        .await
        .map_err(ServerFnError::new)?
    {
        let minutes = throttle::minutes_remaining(seconds);
        return Err(ServerFnError::new(format!(
            "Too many sign-up attempts. Please try again in about {minutes} minute{}.",
            if minutes == 1 { "" } else { "s" }
        )));
    }
    let _ = throttle::record_failure(throttle::Action::Register, &email).await;
    if users::email_exists(&email).await.map_err(ServerFnError::new)? {
        return Err(ServerFnError::new(
            "An account with that email already exists.",
        ));
    }

    let password_hash = hash_password(&password).map_err(ServerFnError::new)?;
    let account = PendingAccount {
        first_name,
        last_name,
        email: email.clone(),
        password_hash,
    };
    let case_id = ids::next(pool(), "c").await.map_err(ServerFnError::new)?;
    let agreement_evidence_id = evidence::reserve_id().await.map_err(ServerFnError::new)?;
    let agreement_blob_path = storage::blob_path(&case_id, &agreement_evidence_id);
    let agreement_sha256 = hex::encode(Sha256::digest(&agreement_bytes));
    let agreement_size_bytes = agreement_bytes.len() as i64;

    let mut metadata = HashMap::new();
    metadata.insert("case_id".to_string(), case_id.clone());
    metadata.insert("evidence_id".to_string(), agreement_evidence_id.clone());
    metadata.insert("uploaded_by".to_string(), account.full_name());
    metadata.insert("sha256".to_string(), agreement_sha256.clone());
    storage::put(
        &agreement_blob_path,
        agreement_bytes,
        content_type,
        metadata,
    )
    .await
    .map_err(|error| ServerFnError::new(format!("Storage error: {error}")))?;

    let signup = PendingCaseSignup {
        case_id,
        case_name: format!("{} case", account.full_name()),
        intake_json,
        agreement_evidence_id,
        agreement_original_filename: agreement_filename,
        agreement_content_type: content_type.to_string(),
        agreement_size_bytes,
        agreement_sha256,
        agreement_blob_path: agreement_blob_path.clone(),
    };
    let challenge = generate_token();
    let code = generate_code();
    if let Err(error) = pending_registrations::create_case_signup(&challenge, &account, &signup, &code).await {
        let _ = storage::delete(&agreement_blob_path).await;
        return Err(ServerFnError::new(error));
    }

    if let Err(error) = auth_email::send_email_verification(&email, &account.full_name(), &code).await {
        let _ = pending_registrations::delete(&challenge).await;
        let _ = storage::delete(&agreement_blob_path).await;
        tracing::warn!("failed to send verification code to {email}: {error}");
        return Err(ServerFnError::new(
            "We couldn't send your verification code. Please try again.",
        ));
    }

    if let Some(previous_challenge) = request_cookie(REGISTER_COOKIE_NAME).await {
        if let Ok(Some(previous_blob)) = pending_registrations::delete(&previous_challenge).await {
            if previous_blob != agreement_blob_path {
                let _ = storage::delete(&previous_blob).await;
            }
        }
    }
    append_cookie(build_register_cookie(challenge))?;
    Ok(())
}

/// Complete a registration by verifying the emailed one-time `code`. On success
/// the account is created, immediately signed in, and returned.
#[server(prefix = "/api")]
pub async fn verify_registration(code: String) -> Result<User, ServerFnError> {
    use crate::server::auth::{
        build_session_cookie, clear_register_cookie, REGISTER_COOKIE_NAME,
    };
    use crate::server::db::pending_registrations::{self, Verify};
    use crate::server::db::{cases, evidence, pool, sessions, throttle, users};
    use crate::server_fns::users::AccountRole;

    let code = code.trim();
    let challenge = request_cookie(REGISTER_COOKIE_NAME).await.ok_or_else(|| {
        ServerFnError::new("Your sign-up session expired. Please register again.")
    })?;

    let mut tx = pool().begin().await.map_err(ServerFnError::new)?;
    let pending = match pending_registrations::verify_in(&mut tx, &challenge, code)
        .await
        .map_err(ServerFnError::new)?
    {
        Verify::Ok(pending) => pending,
        Verify::WrongCode => {
            tx.commit().await.map_err(ServerFnError::new)?;
            return Err(ServerFnError::new("That code is incorrect. Please try again."));
        }
        Verify::Expired(blob_path) => {
            tx.commit().await.map_err(ServerFnError::new)?;
            if let Some(blob_path) = blob_path {
                if let Err(error) = crate::server::storage::delete(&blob_path).await {
                    tracing::warn!("failed to clean expired signup blob '{blob_path}': {error}");
                }
            }
            append_cookie(clear_register_cookie())?;
            return Err(ServerFnError::new(
                "Your code has expired. Please register again.",
            ));
        }
    };
    let account = &pending.account;
    let staged_blob = pending
        .case_signup
        .as_ref()
        .map(|signup| signup.agreement_blob_path.clone());

    // Guard against the email having been claimed while the code was in flight.
    let email_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM users WHERE lower(email) = lower($1))",
    )
    .bind(&account.email)
    .fetch_one(&mut *tx)
        .await
        .map_err(ServerFnError::new)?
    ;
    if email_exists {
        tx.commit().await.map_err(ServerFnError::new)?;
        if let Some(staged_blob) = staged_blob {
            if let Err(error) = crate::server::storage::delete(&staged_blob).await {
                tracing::warn!("failed to clean claimed-email signup blob '{staged_blob}': {error}");
            }
        }
        append_cookie(clear_register_cookie())?;
        return Err(ServerFnError::new(
            "An account with that email already exists.",
        ));
    }

    // The password was already hashed at sign-up time and stored on the pending
    // row, so it is reused as-is when materializing the account.
    let id = users::next_id();
    users::insert_in(
        &mut tx,
        &id,
        &account.first_name,
        &account.last_name,
        &account.email,
        "",
        "",
        &account.password_hash,
        AccountRole::Client,
    )
    .await
    .map_err(ServerFnError::new)?;

    if let Some(signup) = pending.case_signup {
        let intake: CaseIntake =
            serde_json::from_str(&signup.intake_json).map_err(ServerFnError::new)?;
        cases::create_from_signup_in(
            &mut tx,
            &signup.case_id,
            &id,
            &account.full_name(),
            &signup.case_name,
            intake.properties(),
            evidence::EvidenceFile {
                original_filename: &signup.agreement_original_filename,
                content_type: &signup.agreement_content_type,
                size_bytes: signup.agreement_size_bytes,
                sha256: &signup.agreement_sha256,
                blob_path: &signup.agreement_blob_path,
            },
        )
        .await
        .map_err(ServerFnError::new)?;
    }
    tx.commit().await.map_err(ServerFnError::new)?;

    let user = users::get(&id)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("User disappeared after insert."))?;

    // Verified sign-up succeeded: clear the abuse counter for this email.
    let _ = throttle::clear(throttle::Action::Register, &account.email).await;

    let raw = sessions::create(&id).await.map_err(ServerFnError::new)?;
    append_cookie(build_session_cookie(raw))?;
    append_cookie(clear_register_cookie())?;
    Ok(user)
}

/// Re-send a fresh verification code for the in-progress registration (same
/// challenge cookie, new code, reset attempt counter and expiry).
#[server(prefix = "/api")]
pub async fn resend_registration_code() -> Result<(), ServerFnError> {
    use crate::server::auth::{generate_code, REGISTER_COOKIE_NAME};
    use crate::server::db::pending_registrations;
    use crate::server::email::auth_notifications as auth_email;

    let challenge = request_cookie(REGISTER_COOKIE_NAME).await.ok_or_else(|| {
        ServerFnError::new("Your sign-up session expired. Please register again.")
    })?;
    let account = match pending_registrations::account_for(&challenge)
        .await
        .map_err(ServerFnError::new)?
    {
        Some(account) => account,
        None => {
            if let Ok(Some(path)) = pending_registrations::delete(&challenge).await {
                let _ = crate::server::storage::delete(&path).await;
            }
            return Err(ServerFnError::new(
                "Your sign-up session expired. Please register again.",
            ));
        }
    };

    let code = generate_code();
    pending_registrations::create(&challenge, &account, &code)
        .await
        .map_err(ServerFnError::new)?;
    auth_email::send_email_verification(&account.email, &account.full_name(), &code)
        .await
        .map_err(|e| {
            tracing::warn!("failed to resend verification code to {}: {e}", account.email);
            ServerFnError::new("We couldn't resend your code. Please try again.")
        })?;
    Ok(())
}

/// Request a password-reset link by email
#[server(prefix = "/api")]
pub async fn request_password_reset(email: String) -> Result<(), ServerFnError> {
    use crate::server::auth::generate_token;
    use crate::server::config::Brand;
    use crate::server::db::{password_reset, throttle, users};
    use crate::server::email::auth_notifications as auth_email;

    let email = email.trim();
    if email.is_empty() {
        return Err(ServerFnError::new("Please enter your email address."));
    }

    // Rate-limit reset requests per email so this endpoint cannot be used to
    // email-bomb a victim. The check runs before the account lookup, so the
    // response never reveals whether the account exists.
    if let Some(secs) = throttle::seconds_locked(throttle::Action::PasswordReset, email)
        .await
        .map_err(ServerFnError::new)?
    {
        let minutes = throttle::minutes_remaining(secs);
        return Err(ServerFnError::new(format!(
            "Too many reset requests. Please try again in about {minutes} minute{}.",
            if minutes == 1 { "" } else { "s" }
        )));
    }
    // Every request counts toward the limit (there is no "success" to reset it).
    let _ = throttle::record_failure(throttle::Action::PasswordReset, email).await;

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

/// Resolve the signed-in user from the session cookie, or `None` when there is
/// no live session.
///
/// The browser holds the session in an `HttpOnly` cookie, which script cannot
/// read, so after a full page load the client has no idea who it is until it
/// asks. Every protected route depends on this: without it a refresh (or
/// opening a link in a new tab) looks exactly like being signed out. Returns
/// `None` rather than an error, since "not signed in" is an ordinary answer
/// here and not a failure.
///
/// Deliberately returns a [`UserSummary`] rather than the full [`User`]: this
/// runs on every page load, and the summary is all the client keeps, so there
/// is no reason to ship the visitor's address and case assignments along with
/// it.
#[server(prefix = "/api")]
pub async fn current_user() -> Result<Option<UserSummary>, ServerFnError> {
    use crate::server::auth::AuthUser;

    Ok(leptos_axum::extract::<AuthUser>()
        .await
        .ok()
        .map(|AuthUser(user)| user.into()))
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
