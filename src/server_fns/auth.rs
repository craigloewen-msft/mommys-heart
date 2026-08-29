//! Authentication server functions: register (with email-OTP verification),
//! login, MFA verification, logout, and self-service password reset.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
use crate::helpers::case_intake::CaseIntake;
use crate::server_fns::users::{User, UserSummary};

/// The result of a successful password check
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LoginOutcome {
    /// Password *and* second factor satisfied.
    ///
    /// Boxed so the enum is not sized by its largest variant: [`User`] carries
    /// the account's case assignments and deactivation record. `Box<T>`
    /// serializes exactly as `T`, so the wire format is unchanged.
    Authenticated(Box<User>),
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

/// Opportunistically remove abandoned registration rows whenever somebody starts
/// a fresh signup. An abandoned signup no longer leaves a staged file behind, so
/// deleting the row is the whole cleanup.
#[cfg(feature = "ssr")]
async fn cleanup_expired_registrations() {
    use crate::server::db::pending_registrations;

    if let Err(error) = pending_registrations::delete_expired().await {
        tracing::warn!("failed to prune expired registrations: {error}");
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

    // Checked after the password verify, deliberately: answering before it
    // would turn this into an oracle for which addresses have accounts.
    if user.role.is_deactivated() {
        return Err(ServerFnError::new(
            "This account has been deactivated. Please contact an administrator.",
        ));
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
            return Ok(LoginOutcome::Authenticated(Box::new(user)));
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
        return Ok(LoginOutcome::Authenticated(Box::new(user)));
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

    let user_id = match mfa::verify(&challenge, code)
        .await
        .map_err(ServerFnError::new)?
    {
        mfa::Verify::Ok(uid) => uid,
        mfa::Verify::WrongCode => {
            return Err(ServerFnError::new(
                "That code is incorrect. Please try again.",
            ));
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

/// Minimum password length for new accounts.
#[cfg(feature = "ssr")]
pub const MIN_PASSWORD_LENGTH: usize = 8;

/// Upper bound for free-text account fields (names).
#[cfg(feature = "ssr")]
const MAX_NAME_LENGTH: usize = 100;

/// Upper bound for an email address, matching the practical SMTP limit.
#[cfg(feature = "ssr")]
const MAX_EMAIL_LENGTH: usize = 254;

/// Upper bound for a password, so hashing cost stays bounded.
#[cfg(feature = "ssr")]
const MAX_PASSWORD_LENGTH: usize = 200;

/// Reject a password that is too short or implausibly long.
#[cfg(feature = "ssr")]
fn validate_password(password: &str) -> Result<(), ServerFnError> {
    let length = password.chars().count();
    if length < MIN_PASSWORD_LENGTH {
        return Err(ServerFnError::new(format!(
            "Please choose a password of at least {MIN_PASSWORD_LENGTH} characters."
        )));
    }
    if length > MAX_PASSWORD_LENGTH {
        return Err(ServerFnError::new(format!(
            "Passwords must be {MAX_PASSWORD_LENGTH} characters or fewer."
        )));
    }
    Ok(())
}

/// Reject account fields that exceed their maximum length.
#[cfg(feature = "ssr")]
fn validate_account_lengths(
    first_name: &str,
    last_name: &str,
    email: &str,
) -> Result<(), ServerFnError> {
    if first_name.chars().count() > MAX_NAME_LENGTH || last_name.chars().count() > MAX_NAME_LENGTH {
        return Err(ServerFnError::new(format!(
            "Names must be {MAX_NAME_LENGTH} characters or fewer."
        )));
    }
    if email.chars().count() > MAX_EMAIL_LENGTH {
        return Err(ServerFnError::new(format!(
            "Email addresses must be {MAX_EMAIL_LENGTH} characters or fewer."
        )));
    }
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
    use crate::server::auth::{
        build_register_cookie, generate_code, generate_token, hash_password, REGISTER_COOKIE_NAME,
    };
    use crate::server::db::pending_registrations::{self, PendingAccount};
    use crate::server::db::{throttle, users};
    use crate::server::email::auth_notifications as auth_email;

    cleanup_expired_registrations().await;

    let first_name = first_name.trim().to_string();
    let last_name = last_name.trim().to_string();
    let email = email.trim().to_lowercase();
    if first_name.is_empty() || last_name.is_empty() || email.is_empty() || password.is_empty() {
        return Err(ServerFnError::new(
            "Please fill in first name, last name, email, and password.",
        ));
    }
    validate_account_lengths(&first_name, &last_name, &email)?;
    validate_password(&password)?;

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

    if users::email_exists(&email)
        .await
        .map_err(ServerFnError::new)?
    {
        return Err(ServerFnError::new(
            "An account with that email already exists.",
        ));
    }

    // Counted only once a code is actually sent, so rejected attempts (duplicate
    // email, bad input) cannot lock a legitimate user out of their own sign-up.
    let _ = throttle::record_failure(throttle::Action::Register, &email).await;

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
    if let Err(error) =
        auth_email::send_email_verification(&email, &account.full_name(), &code).await
    {
        let _ = pending_registrations::delete(&challenge).await;
        tracing::warn!("failed to send verification code to {email}: {error}");
        return Err(ServerFnError::new(
            "We couldn't send your verification code. Please try again.",
        ));
    }
    if let Some(previous_challenge) = request_cookie(REGISTER_COOKIE_NAME).await {
        let _ = pending_registrations::delete(&previous_challenge).await;
    }
    append_cookie(build_register_cookie(challenge))?;
    Ok(())
}

/// Begin a client signup that will create both an account and a case after email
/// verification. The client has already accepted the Terms and Conditions to
/// reach this form; `terms_version` says which wording they were shown, and is
/// carried on the pending row so the acceptance is recorded only if the
/// registration actually completes.
#[server(prefix = "/api")]
pub async fn register_case_signup(
    first_name: String,
    last_name: String,
    email: String,
    password: String,
    password_confirmation: String,
    intake_json: String,
    terms_version: String,
) -> Result<(), ServerFnError> {
    use crate::server::auth::{
        build_register_cookie, generate_code, generate_token, hash_password, REGISTER_COOKIE_NAME,
    };
    use crate::server::db::pending_registrations::{self, PendingAccount, PendingCaseSignup};
    use crate::server::db::{ids, pool, throttle, users};
    use crate::server::email::auth_notifications as auth_email;

    cleanup_expired_registrations().await;

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
    validate_account_lengths(&first_name, &last_name, &email)?;
    validate_password(&password)?;

    // Consent is checked here, not just in the browser: a form posted without
    // the current terms version never saw the wording it claims to accept.
    if !crate::helpers::terms::is_current(terms_version.trim()) {
        return Err(ServerFnError::new(
            "Please read and accept the Terms and Conditions before submitting your case.",
        ));
    }
    let terms_version = terms_version.trim().to_string();

    let intake: CaseIntake = serde_json::from_str(&intake_json)
        .map_err(|_| ServerFnError::new("The case information could not be read."))?;
    intake.validate().map_err(ServerFnError::new)?;
    let intake_json = serde_json::to_string(&intake).map_err(ServerFnError::new)?;

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
    if users::email_exists(&email)
        .await
        .map_err(ServerFnError::new)?
    {
        return Err(ServerFnError::new(
            "An account with that email already exists.",
        ));
    }
    // Counted only once a code is actually sent. See `register`.
    let _ = throttle::record_failure(throttle::Action::Register, &email).await;

    let password_hash = hash_password(&password).map_err(ServerFnError::new)?;
    let account = PendingAccount {
        first_name,
        last_name,
        email: email.clone(),
        password_hash,
    };
    let case_id = ids::next(pool(), "c").await.map_err(ServerFnError::new)?;
    let signup = PendingCaseSignup {
        case_id,
        case_name: format!("{} case", account.full_name()),
        intake_json,
        terms_version,
    };
    let challenge = generate_token();
    let code = generate_code();
    pending_registrations::create_case_signup(&challenge, &account, &signup, &code)
        .await
        .map_err(ServerFnError::new)?;

    if let Err(error) =
        auth_email::send_email_verification(&email, &account.full_name(), &code).await
    {
        let _ = pending_registrations::delete(&challenge).await;
        tracing::warn!("failed to send verification code to {email}: {error}");
        return Err(ServerFnError::new(
            "We couldn't send your verification code. Please try again.",
        ));
    }

    if let Some(previous_challenge) = request_cookie(REGISTER_COOKIE_NAME).await {
        let _ = pending_registrations::delete(&previous_challenge).await;
    }
    append_cookie(build_register_cookie(challenge))?;
    Ok(())
}

/// Complete a registration by verifying the emailed one-time `code`. On success
/// the account is created, immediately signed in, and returned.
#[server(prefix = "/api")]
pub async fn verify_registration(code: String) -> Result<User, ServerFnError> {
    use crate::server::auth::{build_session_cookie, clear_register_cookie, REGISTER_COOKIE_NAME};
    use crate::server::db::pending_registrations::{self, Verify};
    use crate::server::db::{
        case_contacts, cases, clients, contacts, pool, sessions, throttle, users,
    };
    use crate::server_fns::case_contacts::CaseContactRole;
    use crate::server_fns::contacts::{ContactInput, ContactType};
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
            return Err(ServerFnError::new(
                "That code is incorrect. Please try again.",
            ));
        }
        Verify::Expired => {
            tx.commit().await.map_err(ServerFnError::new)?;
            append_cookie(clear_register_cookie())?;
            return Err(ServerFnError::new(
                "Your code has expired. Please register again.",
            ));
        }
    };
    let account = &pending.account;
    let signup_notification_details = pending.case_signup.as_ref().map(|signup| {
        (
            account.full_name(),
            account.email.clone(),
            signup.case_name.clone(),
        )
    });

    // Guard against the email having been claimed while the code was in flight.
    let email_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE lower(email) = lower($1))")
            .bind(&account.email)
            .fetch_one(&mut *tx)
            .await
            .map_err(ServerFnError::new)?;
    if email_exists {
        tx.commit().await.map_err(ServerFnError::new)?;
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

    // REQ-CRM-048: contact/case audit rows created below inherit the new
    // account's stable identity while keeping the registration name snapshot.
    crate::server::db::audit::set_actor_in_transaction(&mut tx, &id)
        .await
        .map_err(ServerFnError::new)?;

    // Every registration creates a client account, so it gets the client record
    // that `users` is the base of, in the same transaction.
    clients::insert_in(&mut tx, &id)
        .await
        .map_err(ServerFnError::new)?;

    let contact_id = contacts::create_linked_in(
        &mut tx,
        &ContactInput {
            first_name: account.first_name.clone(),
            last_name: account.last_name.clone(),
            email: account.email.clone(),
            types: vec![ContactType::Client],
            source: "Account registration".to_string(),
            ..Default::default()
        },
        &id,
        &account.full_name(),
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
            &signup.terms_version,
        )
        .await
        .map_err(ServerFnError::new)?;
        case_contacts::add_in(
            &mut tx,
            &signup.case_id,
            &contact_id,
            CaseContactRole::Client,
            "",
            true,
            &account.full_name(),
        )
        .await
        .map_err(ServerFnError::new)?;
    }
    tx.commit().await.map_err(ServerFnError::new)?;

    if let Some((client_name, client_email, case_name)) = signup_notification_details {
        crate::server::notifications::notify_case_signup(client_name, client_email, case_name);
    }

    let user = users::get(&id)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("User disappeared after insert."))?;

    // Verified sign-up succeeded: clear the abuse counter for this email.
    let _ = throttle::clear(throttle::Action::Register, &account.email).await;
    let _ = throttle::clear(throttle::Action::ResendCode, &account.email).await;

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
    use crate::server::db::throttle;
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
            let _ = pending_registrations::delete(&challenge).await;
            return Err(ServerFnError::new(
                "Your sign-up session expired. Please register again.",
            ));
        }
    };

    // Resending emails an attacker-chosen address, so it needs its own budget:
    // without this it bypasses the `Register` throttle entirely.
    if let Some(seconds) = throttle::seconds_locked(throttle::Action::ResendCode, &account.email)
        .await
        .map_err(ServerFnError::new)?
    {
        let minutes = throttle::minutes_remaining(seconds);
        return Err(ServerFnError::new(format!(
            "Too many code requests. Please try again in about {minutes} minute{}.",
            if minutes == 1 { "" } else { "s" }
        )));
    }
    let _ = throttle::record_failure(throttle::Action::ResendCode, &account.email).await;

    let code = generate_code();
    pending_registrations::create(&challenge, &account, &code)
        .await
        .map_err(ServerFnError::new)?;
    auth_email::send_email_verification(&account.email, &account.full_name(), &code)
        .await
        .map_err(|e| {
            tracing::warn!(
                "failed to resend verification code to {}: {e}",
                account.email
            );
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
                "https://app.example.org".to_string()
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
    use crate::server::db::{password_reset, pool, trusted_devices};

    // REQ-SEC-002/003: one password policy and one transaction for token,
    // credential, sessions, and remembered devices.
    validate_password(&new_password)?;
    let password_hash = hash_password(&new_password).map_err(ServerFnError::new)?;
    let mut tx = pool().begin().await.map_err(ServerFnError::new)?;
    let user_id = password_reset::consume_in(&mut tx, &token)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| {
            ServerFnError::new(
                "This reset link is invalid or has expired. Please request a new one.",
            )
        })?;
    sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
        .bind(&password_hash)
        .bind(&user_id)
        .execute(&mut *tx)
        .await
        .map_err(ServerFnError::new)?;
    sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(&user_id)
        .execute(&mut *tx)
        .await
        .map_err(ServerFnError::new)?;
    trusted_devices::delete_all_for_user_in(&mut tx, &user_id)
        .await
        .map_err(ServerFnError::new)?;
    tx.commit().await.map_err(ServerFnError::new)?;
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
