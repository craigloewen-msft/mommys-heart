//! Best-effort email notifications for case activity (SSR only).
//!
//! The case-mutating server functions in [`crate::server_fns`] call the helpers
//! here *after* a change succeeds. Each helper resolves the recipients, filters
//! them by their own notification settings, and sends the emails on a background
//! task via [`crate::server::email`]. Nothing here ever blocks or fails the
//! originating request — failures are logged and swallowed, mirroring
//! [`crate::server::rag::start_background_ingest`] and the audit retention task.
//!
//! When ACS Email is not configured (see [`EmailConfig::is_configured`]), every
//! helper is a silent no-op.

use crate::server::config::EmailConfig;
use crate::server::db::settings::{self, Recipient};
use crate::server::db::cases;
use crate::server::email::{send_email, EmailMessage};
use crate::server_fns::settings::NotificationKind;

/// The active email config, or `None` when ACS Email is not configured — in
/// which case every notification helper is a silent no-op.
fn configured_email() -> Option<EmailConfig> {
    let cfg = EmailConfig::from_env();
    cfg.is_configured().then_some(cfg)
}

/// Fire a best-effort case notification to everyone assigned to the case (with
/// `view_case`) except the actor, honoring each recipient's settings.
///
/// `detail` is a human sentence describing what the actor did, e.g. `changed the
/// status to "Closed"`; it is combined with `actor_name` and the resolved case
/// name to build the email. Returns immediately.
pub fn notify_case(
    case_id: String,
    actor_id: String,
    actor_name: String,
    kind: NotificationKind,
    detail: String,
) {
    let Some(cfg) = configured_email() else {
        return;
    };
    tokio::spawn(async move {
        let recipients = match settings::recipients_for_case(&case_id, &actor_id).await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("notify_case: recipient lookup failed for {case_id}: {e}");
                return;
            }
        };
        if recipients.iter().all(|r| !r.settings.wants(kind)) {
            return;
        }
        let case = case_name(&case_id).await;
        let subject = format!("[Mommy's Heart] {case}: {}", kind.label());
        let (html, text) = body(&case, &actor_name, &detail);
        dispatch(&cfg, recipients, kind, &subject, &html, &text).await;
    });
}

/// Fire a best-effort "you were assigned to a case" notification to the newly
/// assigned user (the recipient is that user, not the case's other members).
pub fn notify_assignment(user_id: String, actor_name: String, case_id: String) {
    let Some(cfg) = configured_email() else {
        return;
    };
    tokio::spawn(async move {
        let recipient = match settings::recipient_for_user(&user_id).await {
            Ok(Some(r)) => r,
            Ok(None) => return,
            Err(e) => {
                tracing::warn!("notify_assignment: lookup failed for {user_id}: {e}");
                return;
            }
        };
        if !recipient.settings.wants(NotificationKind::Assigned) {
            return;
        }
        let case = case_name(&case_id).await;
        let subject = format!("[Mommy's Heart] You've been given access to {case}");
        let detail = format!("gave you access to the case \"{case}\"");
        let (html, text) = body(&case, &actor_name, &detail);
        dispatch(
            &cfg,
            vec![recipient],
            NotificationKind::Assigned,
            &subject,
            &html,
            &text,
        )
        .await;
    });
}

/// Send `msg` to each recipient that still wants `kind`, logging per-recipient
/// outcomes. Runs on the background task, so it never affects the request.
async fn dispatch(
    cfg: &EmailConfig,
    recipients: Vec<Recipient>,
    kind: NotificationKind,
    subject: &str,
    html: &str,
    text: &str,
) {
    for r in recipients {
        if !r.settings.wants(kind) {
            continue;
        }
        if cfg.dry_run {
            tracing::info!(
                "[email dry-run] would send \"{subject}\" to {} (set EMAIL_DRY_RUN=false to send)",
                r.email
            );
            continue;
        }
        let msg = EmailMessage {
            to_address: r.email.clone(),
            to_name: r.name.clone(),
            subject: subject.to_string(),
            html: html.to_string(),
            plain_text: text.to_string(),
        };
        match send_email(cfg, &msg).await {
            Ok(()) => tracing::info!("notification email accepted for {}", r.email),
            Err(e) => tracing::warn!("notification email to {} failed: {e}", r.email),
        }
    }
}

/// The case's display name, falling back to its id if the row is missing.
async fn case_name(case_id: &str) -> String {
    cases::name(case_id)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| case_id.to_string())
}

/// Build the (HTML, plain-text) email body from the event pieces.
fn body(case: &str, actor_name: &str, detail: &str) -> (String, String) {
    let actor = if actor_name.trim().is_empty() {
        "Someone"
    } else {
        actor_name.trim()
    };
    let text = format!(
        "{actor} {detail} on the case \"{case}\".\n\n\
         Sign in to the Mommy's Heart CRM to see the details.\n\n\
         You are receiving this because of your notification settings. \
         You can change them on your Settings page."
    );
    let html = format!(
        "<p>{} {} on the case <strong>{}</strong>.</p>\
         <p>Sign in to the Mommy's Heart CRM to see the details.</p>\
         <p style=\"color:#64748b;font-size:12px\">You are receiving this because of your \
         notification settings. You can change them on your Settings page.</p>",
        escape(actor),
        escape(detail),
        escape(case),
    );
    (html, text)
}

/// Minimal HTML escaping for the small set of interpolated values.
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
