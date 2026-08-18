//! Delivery of transactional authentication emails (SSR only): the MFA one-time
//! code and the password-reset link.
//!
//! Unlike [`crate::server::notifications`] (best-effort, background, honoring
//! per-user notification settings), these are foreground and their outcome
//! matters — a user cannot finish signing in without the code. When ACS Email is
//! not configured, `EMAIL_DRY_RUN` is set, or the server is a debug build, the
//! secret is written to the log so local development and the demo accounts still
//! work even without a live inbox. See [`deliver`] for the exact rules.

use crate::server::config::{Brand, EmailConfig};
use crate::server::email::templates::{self, RenderedEmail};
use crate::server::email::{send_email, send_otp_email, EmailMessage};

/// Email a user their one-time MFA code. Returns `Err` only when ACS is
/// configured and the send itself fails.
pub async fn send_mfa_code(to_email: &str, to_name: &str, code: &str) -> Result<(), String> {
    let brand = Brand::from_env();
    let rendered = templates::auth_code(&brand, code);
    deliver(
        to_email,
        to_name,
        &rendered,
        &format!("verification code {code}"),
        true,
    )
    .await
}

/// Email a new registrant their one-time email-verification code
pub async fn send_email_verification(
    to_email: &str,
    to_name: &str,
    code: &str,
) -> Result<(), String> {
    let brand = Brand::from_env();
    let rendered = templates::verify_email(&brand, code);
    deliver(
        to_email,
        to_name,
        &rendered,
        &format!("email verification code {code}"),
        true,
    )
    .await
}

/// Email a user their password-reset link (`reset_url` is the full tokenized
/// URL). Returns `Err` only when ACS is configured and the send itself fails.
pub async fn send_password_reset(
    to_email: &str,
    to_name: &str,
    reset_url: &str,
) -> Result<(), String> {
    let brand = Brand::from_env();
    let rendered = templates::password_reset(&brand, reset_url);
    deliver(
        to_email,
        to_name,
        &rendered,
        &format!("password-reset link {reset_url}"),
        false,
    )
    .await
}

/// Send a rendered auth email.
async fn deliver(
    to_email: &str,
    to_name: &str,
    email: &RenderedEmail,
    dev_detail: &str,
    is_otp: bool,
) -> Result<(), String> {
    let cfg = EmailConfig::from_env();

    let will_send = cfg.is_configured() && !cfg.dry_run;

    if !will_send || cfg!(debug_assertions) {
        tracing::warn!(
            "[auth email dev] {to_email}: {dev_detail} \
             (logged for local development; configure ACS Email and unset \
             EMAIL_DRY_RUN in a release build to deliver for real only)"
        );
    }

    if !will_send {
        return Ok(());
    }

    let msg = EmailMessage {
        recipients: crate::server::email::EmailRecipients::To(
            crate::server::email::EmailRecipient {
                address: to_email.to_string(),
                name: to_name.to_string(),
            },
        ),
        subject: email.subject.clone(),
        html: email.html.clone(),
        plain_text: email.plain_text.clone(),
    };
    let result = if is_otp {
        send_otp_email(&cfg, &msg).await
    } else {
        send_email(&cfg, &msg).await
    };
    if let Err(e) = result {
        crate::server::db::email_failures::record(
            to_email,
            &email.subject,
            "Authentication email",
            &e,
        )
        .await;
        return Err(e);
    }
    Ok(())
}
