//! Process-local Tokio service for one cancellable contact-mail task at a time.

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use chrono::Utc;
use tokio::sync::{Mutex, Notify};

use crate::server::config::{Brand, EmailConfig};
use crate::server::db::contact_mail::{self as repository, MailTaskRecord};
use crate::server::db::email_failures;
use crate::server::email::templates;
use crate::server::email::{send_email, EmailKind, EmailMessage, EmailRecipient, EmailRecipients};
use crate::server_fns::contact_mail::{
    ContactMailSelection, ContactMailTask, ContactMailTaskStatus,
};

const SEND_INTERVAL: Duration = Duration::from_secs(60);
const FINAL_WRITE_RETRY_DELAY: Duration = Duration::from_secs(5);

static SERVICE: OnceLock<ContactMailTaskService> = OnceLock::new();

fn service() -> &'static ContactMailTaskService {
    SERVICE.get_or_init(ContactMailTaskService::new)
}

/// Close records left active by a previous process, then initialize the service.
pub async fn initialize() -> Result<(), sqlx::Error> {
    repository::fail_interrupted_tasks().await?;
    let _ = service();
    Ok(())
}

pub async fn current_task() -> Option<ContactMailTask> {
    service().current_task().await
}

pub async fn start(
    selection: &ContactMailSelection,
    subject: &str,
    body: &str,
    creator_id: &str,
    creator_name: &str,
) -> Result<ContactMailTask, String> {
    service()
        .start(selection, subject, body, creator_id, creator_name)
        .await
}

pub async fn cancel(
    task_id: &str,
    actor_id: &str,
    actor_name: &str,
) -> Result<Option<ContactMailTask>, String> {
    service().cancel(task_id, actor_id, actor_name).await
}

struct ContactMailTaskService {
    active: Arc<Mutex<Option<ActiveTask>>>,
}

struct ActiveTask {
    task: Arc<Mutex<MailTaskRecord>>,
    cancel: Arc<Notify>,
}

impl ContactMailTaskService {
    fn new() -> Self {
        Self {
            active: Arc::new(Mutex::new(None)),
        }
    }

    async fn current_task(&self) -> Option<ContactMailTask> {
        let task = {
            let active = self.active.lock().await;
            active.as_ref().map(|active| Arc::clone(&active.task))
        }?;
        let public = task.lock().await.to_public();
        Some(public)
    }

    async fn start(
        &self,
        selection: &ContactMailSelection,
        subject: &str,
        body: &str,
        creator_id: &str,
        creator_name: &str,
    ) -> Result<ContactMailTask, String> {
        let mut active_slot = self.active.lock().await;
        if let Some(existing) = active_slot.as_ref() {
            if existing.task.lock().await.status.is_active() {
                return Err("Another contact mail task is still active.".to_string());
            }
            *active_slot = None;
        }
        let task = repository::start_task(selection, subject, body, creator_id, creator_name)
            .await
            .map_err(|error| error.to_string())?;
        let public = task.to_public();
        let task = Arc::new(Mutex::new(task));
        let cancel = Arc::new(Notify::new());
        *active_slot = Some(ActiveTask {
            task: Arc::clone(&task),
            cancel: Arc::clone(&cancel),
        });
        drop(active_slot);

        let active = Arc::clone(&self.active);
        tokio::spawn(async move {
            run_task(Arc::clone(&task), cancel).await;
            let mut current = active.lock().await;
            if current
                .as_ref()
                .is_some_and(|active| Arc::ptr_eq(&active.task, &task))
            {
                *current = None;
            }
        });
        Ok(public)
    }

    async fn cancel(
        &self,
        task_id: &str,
        actor_id: &str,
        actor_name: &str,
    ) -> Result<Option<ContactMailTask>, String> {
        let (task, cancel) = {
            let active = self.active.lock().await;
            let Some(active) = active.as_ref() else {
                return Ok(None);
            };
            (Arc::clone(&active.task), Arc::clone(&active.cancel))
        };
        let public = {
            let mut task = task.lock().await;
            if task.id != task_id {
                return Ok(None);
            }
            if task.status == ContactMailTaskStatus::Running {
                task.status = ContactMailTaskStatus::Cancelling;
                task.cancel_requested_at = Some(Utc::now());
                task.cancel_requested_by_id = Some(actor_id.to_string());
                task.cancel_requested_by_name = actor_name.to_string();
            }
            task.to_public()
        };
        cancel.notify_one();
        Ok(Some(public))
    }
}

async fn run_task(task: Arc<Mutex<MailTaskRecord>>, cancel: Arc<Notify>) {
    let (task_id, subject, body, recipients) = {
        let task = task.lock().await;
        (
            task.id.clone(),
            task.subject.clone(),
            task.body.clone(),
            task.recipients.clone(),
        )
    };
    let cfg = EmailConfig::from_env();
    let rendered = templates::contact_mail(&Brand::from_env(), &subject, &body);
    let started_at = Utc::now();
    let schedule_interval =
        chrono::Duration::from_std(SEND_INTERVAL).expect("send interval fits chrono");
    let mut next_send_at = started_at;

    for (index, recipient) in recipients.iter().enumerate() {
        if index > 0 {
            while next_send_at <= Utc::now() {
                next_send_at += schedule_interval;
            }
            {
                let mut task = task.lock().await;
                task.next_send_at = Some(next_send_at);
            }
            tokio::select! {
                _ = tokio::time::sleep((next_send_at - Utc::now()).to_std().unwrap_or_default()) => {}
                _ = cancel.notified() => break,
            }
        }
        {
            let mut task = task.lock().await;
            if task.status == ContactMailTaskStatus::Cancelling {
                break;
            }
            let recipient = &mut task.recipients[index];
            recipient.status = "sending".to_string();
            recipient.attempt_started_at = Some(Utc::now());
        }

        let result = send_one(&cfg, &rendered, recipient).await;
        if let Err(error) = &result {
            email_failures::record(
                &recipient.email,
                &subject,
                &format!("Contact mail task {task_id}"),
                error,
            )
            .await;
        }
        let now = Utc::now();
        let mut state = task.lock().await;
        let recipient = &mut state.recipients[index];
        recipient.finished_at = Some(now);
        match result {
            Ok(()) => {
                recipient.status = "accepted".to_string();
                state.accepted_count += 1;
            }
            Err(error) => {
                recipient.status = "failed".to_string();
                recipient.error = error;
                state.failed_count += 1;
            }
        }
    }

    {
        let mut state = task.lock().await;
        state.status = if state.status == ContactMailTaskStatus::Cancelling {
            ContactMailTaskStatus::Cancelled
        } else {
            ContactMailTaskStatus::Completed
        };
        state.completed_at = Some(Utc::now());
        state.next_send_at = None;
    }

    loop {
        let final_snapshot = task.lock().await.clone();
        match repository::finish_task(&final_snapshot).await {
            Ok(()) => return,
            Err(error) => {
                tracing::warn!(task_id = %task_id, "contact mail final history write failed: {error}; retrying");
                tokio::time::sleep(FINAL_WRITE_RETRY_DELAY).await;
            }
        }
    }
}

async fn send_one(
    cfg: &EmailConfig,
    rendered: &templates::RenderedEmail,
    recipient: &repository::MailRecipientRecord,
) -> Result<(), String> {
    if cfg.dry_run {
        tracing::warn!(
            position = recipient.position,
            "[email dry-run] contact mail recipient accepted without delivery"
        );
        return Ok(());
    }
    if !cfg.is_configured() {
        return Err(
            "Email delivery became unavailable before this recipient was sent.".to_string(),
        );
    }
    send_email(
        cfg,
        &EmailMessage {
            kind: EmailKind::Standard,
            recipients: EmailRecipients::To(EmailRecipient {
                address: recipient.email.clone(),
                name: recipient.name.clone(),
            }),
            subject: rendered.subject.clone(),
            html: rendered.html.clone(),
            plain_text: rendered.plain_text.clone(),
        },
    )
    .await
}
