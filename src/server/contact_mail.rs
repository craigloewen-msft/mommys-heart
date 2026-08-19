//! Process-local Tokio service for one cancellable contact-mail task at a time.

use std::collections::HashSet;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use chrono::Utc;
use tokio::sync::{Mutex, Notify};

use crate::server::db::contact_mail::{self as repository, MailTaskRecord};
use crate::server::email::{
    send_contact_batch, EmailBatchOutcome, EmailBatchRecipient, MAX_RECIPIENTS_PER_MESSAGE,
};
use crate::server_fns::contact_mail::{
    ContactMailSelection, ContactMailTask, ContactMailTaskStatus,
};

const MAIL_SEND_INTERVAL: Duration = Duration::from_secs(60);
const FINAL_WRITE_RETRY_DELAY: Duration = Duration::from_secs(5);
const BLOCKED_ERROR: &str = "Blocked: contact is marked do not contact.";

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
    let started_at = Utc::now();
    let schedule_interval =
        chrono::Duration::from_std(MAIL_SEND_INTERVAL).expect("batch interval fits chrono");
    let mut next_send_at = started_at;

    for (batch_index, batch) in recipients.chunks(MAX_RECIPIENTS_PER_MESSAGE).enumerate() {
        if batch_index > 0 {
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

        let claimed_at = Utc::now();
        // Fail-safe: recheck the do-not-contact flag right before dispatch, since a
        // task can run for hours after its recipient list was frozen.
        let contact_ids: Vec<String> = batch
            .iter()
            .map(|recipient| recipient.contact_id.clone())
            .collect();
        let suppression = repository::suppressed_contact_ids(&contact_ids).await;
        let blocked: HashSet<String> = match &suppression {
            Ok(blocked) => blocked.clone(),
            Err(error) => {
                // Fail closed: if we cannot verify preferences, send nothing.
                tracing::error!(task_id = %task_id, "do-not-contact recheck failed: {error}");
                contact_ids.iter().cloned().collect()
            }
        };
        let block_reason = match &suppression {
            Ok(_) => BLOCKED_ERROR.to_string(),
            Err(error) => format!("Blocked: could not verify contact preferences ({error})."),
        };

        let batch_recipients = {
            let mut task = task.lock().await;
            if task.status == ContactMailTaskStatus::Cancelling {
                break;
            }
            let mut sendable = Vec::with_capacity(batch.len());
            for recipient in batch {
                let record = &mut task.recipients[(recipient.position - 1) as usize];
                if blocked.contains(&recipient.contact_id) {
                    record.status = "failed".to_string();
                    record.attempt_started_at = Some(claimed_at);
                    record.finished_at = Some(claimed_at);
                    record.error = block_reason.clone();
                    task.failed_count += 1;
                    tracing::warn!(
                        task_id = %task_id,
                        contact_id = %recipient.contact_id,
                        "contact mail recipient blocked before send"
                    );
                    continue;
                }
                record.status = "sending".to_string();
                record.attempt_started_at = Some(claimed_at);
                sendable.push(EmailBatchRecipient {
                    key: recipient.position,
                    address: recipient.email.clone(),
                    name: recipient.name.clone(),
                });
            }
            sendable
        };

        if batch_recipients.is_empty() {
            continue;
        }

        // The hourly task group is the in-flight unit. The lower service owns its
        // provider chunks and returns one final outcome for each contact.
        let outcomes = send_contact_batch(&task_id, &batch_recipients, &subject, &body).await;
        apply_batch_outcomes(&task, outcomes).await;
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

async fn apply_batch_outcomes(task: &Arc<Mutex<MailTaskRecord>>, outcomes: Vec<EmailBatchOutcome>) {
    let finished_at = Utc::now();
    let mut task = task.lock().await;
    for outcome in outcomes {
        let recipient = &mut task.recipients[(outcome.recipient.key - 1) as usize];
        recipient.finished_at = Some(finished_at);
        match outcome.result {
            Ok(()) => {
                recipient.status = "accepted".to_string();
                task.accepted_count += 1;
            }
            Err(error) => {
                recipient.status = "failed".to_string();
                recipient.error = error;
                task.failed_count += 1;
            }
        }
    }
}
