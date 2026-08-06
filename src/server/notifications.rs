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
//! helper is a silent no-op unless dry-run mode is enabled.

use crate::helpers::visibility::Visibility;
use crate::server::config::{Brand, EmailConfig};
use crate::server::db::cases;
use crate::server::db::settings::{self, Recipient};
use crate::server::email::templates::{self, RenderedEmail};
use crate::server::email::{
    send_email, EmailMessage, EmailRecipient, EmailRecipients, MAX_RECIPIENTS_PER_MESSAGE,
};
use crate::server_fns::admin_requests::AdminRequest;
use crate::server_fns::settings::NotificationKind;

/// The active email config, or `None` when neither delivery nor dry-run logging
/// is configured.
fn configured_email() -> Option<EmailConfig> {
    let cfg = EmailConfig::from_env();
    (cfg.dry_run || cfg.is_configured()).then_some(cfg)
}

/// Who a case notification should reach.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Audience {
    Everyone,
    StaffOnly,
}

/// Who to notify about a change to case information of a given visibility.
pub fn audience_for(visibility: Visibility) -> Audience {
    if visibility.is_restricted() {
        Audience::StaffOnly
    } else {
        Audience::Everyone
    }
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
    audience: Audience,
) {
    let Some(cfg) = configured_email() else {
        return;
    };
    tokio::spawn(async move {
        let staff_only = audience == Audience::StaffOnly;
        let mut recipients =
            match settings::recipients_for_case(&case_id, &actor_id, staff_only).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!("notify_case: recipient lookup failed for {case_id}: {e}");
                    return;
                }
            };
        recipients.retain(|recipient| recipient.settings.wants(kind));
        if recipients.is_empty() {
            return;
        }
        let case = case_name(&case_id).await;
        let email = templates::case_event(&Brand::from_env(), kind, &case, &actor_name, &detail);
        dispatch(&cfg, recipients, &email, "Case notification").await;
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
        let email = templates::assignment(&Brand::from_env(), &case, &actor_name);
        dispatch(&cfg, vec![recipient], &email, "Case notification").await;
    });
}

/// Notify all site admins that an operations-admin request is waiting for
/// review, honouring each administrator's notification settings.
pub fn notify_admin_request_filed(request: AdminRequest) {
    let Some(cfg) = configured_email() else {
        return;
    };
    tokio::spawn(async move {
        let mut recipients = match settings::recipients_for_site_admins().await {
            Ok(recipients) => recipients,
            Err(error) => {
                tracing::warn!("admin request recipient lookup failed: {error}");
                return;
            }
        };
        recipients.retain(|recipient| recipient.settings.wants(NotificationKind::AdminRequests));
        if recipients.is_empty() {
            return;
        }
        let email = templates::admin_request_filed(&Brand::from_env(), &request);
        dispatch(&cfg, recipients, &email, "Admin request notification").await;
    });
}

/// Notify the requester when a site admin approves or denies an administrative
/// request. Affected users receive the notification for the resulting domain
/// event, such as [`notify_assignment`], instead.
pub fn notify_admin_request_decided(request: AdminRequest) {
    let Some(cfg) = configured_email() else {
        return;
    };
    tokio::spawn(async move {
        let recipients = match settings::recipient_for_user(&request.requested_by_id).await {
            Ok(Some(recipient)) if recipient.settings.wants(NotificationKind::AdminRequests) => {
                vec![recipient]
            }
            Ok(None) => Vec::new(),
            Ok(Some(_)) => Vec::new(),
            Err(error) => {
                tracing::warn!(
                    "admin request decision recipient lookup failed for {}: {error}",
                    request.requested_by_id
                );
                return;
            }
        };
        let email = templates::admin_request_decided(&Brand::from_env(), &request);
        dispatch(&cfg, recipients, &email, "Admin request notification").await;
    });
}

/// Notify site and operations administrators after a client verifies their
/// email and their new account and case have been committed.
pub fn notify_case_signup(client_name: String, client_email: String, case_name: String) {
    let Some(cfg) = configured_email() else {
        return;
    };
    tokio::spawn(async move {
        let mut recipients = match settings::recipients_for_admins().await {
            Ok(recipients) => recipients,
            Err(error) => {
                tracing::warn!("case signup recipient lookup failed: {error}");
                return;
            }
        };
        recipients.retain(|recipient| recipient.settings.wants(NotificationKind::AdminRequests));
        if recipients.is_empty() {
            return;
        }
        let email =
            templates::case_signup(&Brand::from_env(), &client_name, &client_email, &case_name);
        dispatch(&cfg, recipients, &email, "Case signup notification").await;
    });
}

/// Send one direct message or BCC chunks when the rendered content is shared.
async fn dispatch(
    cfg: &EmailConfig,
    recipients: Vec<Recipient>,
    email: &RenderedEmail,
    context: &str,
) {
    for chunk in recipients.chunks(MAX_RECIPIENTS_PER_MESSAGE) {
        if cfg.dry_run {
            let addresses = chunk
                .iter()
                .map(|recipient| recipient.email.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            tracing::warn!(
                "[email dry-run] would send \"{}\" to {} ({} recipient(s); \
                 set EMAIL_DRY_RUN=false to send)",
                email.subject,
                addresses,
                chunk.len()
            );
            continue;
        }

        let recipients = if let [recipient] = chunk {
            EmailRecipients::To(email_recipient(recipient))
        } else {
            EmailRecipients::Bcc(chunk.iter().map(email_recipient).collect())
        };
        let message = EmailMessage {
            recipients,
            subject: email.subject.clone(),
            html: email.html.clone(),
            plain_text: email.plain_text.clone(),
        };
        if let Err(error) = send_email(cfg, &message).await {
            tracing::warn!(
                "notification email batch for {} recipient(s) failed: {error}",
                chunk.len()
            );
            for recipient in chunk {
                crate::server::db::email_failures::record(
                    &recipient.email,
                    &email.subject,
                    context,
                    &error,
                )
                .await;
            }
        }
    }
}

fn email_recipient(recipient: &Recipient) -> EmailRecipient {
    EmailRecipient {
        address: recipient.email.clone(),
        name: recipient.name.clone(),
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
