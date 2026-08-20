//! Process-local Tokio service for one cancellable spreadsheet import at a time.
//!
//! Structurally the same as [`crate::server::contact_mail`]: a single active
//! slot behind a mutex, a `Notify` for cancellation, and a spawned runner that
//! clears the slot when it finishes. One at a time is the point — two imports
//! racing over the same email addresses would produce duplicates that neither
//! run could be blamed for.

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use chrono::Utc;
use tokio::sync::{Mutex, Notify};

use crate::server::db::crm_import::{
    self as repository, ImportTaskRecord, Mapping, PreparedImport, RowOutcome,
};
use crate::server_fns::crm_import::{
    ColumnPlan, ImportPolicy, ImportRowFailure, ImportTask, ImportTaskStatus,
};
use crate::server_fns::property_filters::PropertySubject;

const FINAL_WRITE_RETRY_DELAY: Duration = Duration::from_secs(5);
/// How many failures the live panel carries. The full list stays in the
/// database; this only bounds what is sent on every poll.
const MAX_TRACKED_FAILURES: usize = 200;

static SERVICE: OnceLock<ImportService> = OnceLock::new();

fn service() -> &'static ImportService {
    SERVICE.get_or_init(ImportService::new)
}

/// Close imports left active by a previous process, then initialize the service.
pub async fn initialize() -> Result<(), sqlx::Error> {
    repository::fail_interrupted_tasks().await?;
    repository::start_retention_task();
    let _ = service();
    Ok(())
}

pub async fn current_task() -> Option<ImportTask> {
    service().current_task().await
}

pub async fn start(
    upload_id: &str,
    subject: PropertySubject,
    columns: &[ColumnPlan],
    policy: &ImportPolicy,
    creator_id: &str,
    creator_name: &str,
) -> Result<ImportTask, String> {
    service()
        .start(upload_id, subject, columns, policy, creator_id, creator_name)
        .await
}

pub async fn cancel(
    task_id: &str,
    actor_id: &str,
    actor_name: &str,
) -> Result<Option<ImportTask>, String> {
    service().cancel(task_id, actor_id, actor_name).await
}

struct ImportService {
    active: Arc<Mutex<Option<ActiveTask>>>,
}

struct ActiveTask {
    task: Arc<Mutex<ImportTaskRecord>>,
    cancel: Arc<Notify>,
}

impl ImportService {
    fn new() -> Self {
        Self {
            active: Arc::new(Mutex::new(None)),
        }
    }

    async fn current_task(&self) -> Option<ImportTask> {
        let task = {
            let active = self.active.lock().await;
            active.as_ref().map(|active| Arc::clone(&active.task))
        }?;
        let public = task.lock().await.to_public();
        Some(public)
    }

    async fn start(
        &self,
        upload_id: &str,
        subject: PropertySubject,
        columns: &[ColumnPlan],
        policy: &ImportPolicy,
        creator_id: &str,
        creator_name: &str,
    ) -> Result<ImportTask, String> {
        // The slot is held across the validate-and-insert so two requests
        // arriving together cannot both find it empty.
        let mut active_slot = self.active.lock().await;
        if let Some(existing) = active_slot.as_ref() {
            if existing.task.lock().await.status.is_active() {
                return Err(
                    "Another import is still running. Wait for it to finish, or cancel it first."
                        .to_string(),
                );
            }
            *active_slot = None;
        }

        let PreparedImport {
            record,
            rows,
            mapping,
            policy,
        } = repository::start_task(upload_id, subject, columns, policy, creator_id, creator_name)
            .await
            .map_err(|error| match error {
                // `Protocol` is how the repository reports a refusal meant for
                // the user; anything else is a genuine database fault.
                sqlx::Error::Protocol(message) => message,
                other => other.to_string(),
            })?;

        let public = record.to_public();
        let task = Arc::new(Mutex::new(record));
        let cancel = Arc::new(Notify::new());
        *active_slot = Some(ActiveTask {
            task: Arc::clone(&task),
            cancel: Arc::clone(&cancel),
        });
        drop(active_slot);

        let active = Arc::clone(&self.active);
        tokio::spawn(async move {
            run_task(Arc::clone(&task), rows, mapping, policy, cancel).await;
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
    ) -> Result<Option<ImportTask>, String> {
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
            if task.status == ImportTaskStatus::Running {
                task.status = ImportTaskStatus::Cancelling;
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

async fn run_task(
    task: Arc<Mutex<ImportTaskRecord>>,
    rows: Vec<Vec<String>>,
    mapping: Mapping,
    policy: ImportPolicy,
    cancel: Arc<Notify>,
) {
    let (task_id, subject, actor_id, actor) = {
        let task = task.lock().await;
        (
            task.id.clone(),
            task.subject,
            task.created_by_id.clone(),
            task.created_by_name.clone(),
        )
    };

    for (index, row) in rows.iter().enumerate() {
        // Cancellation is checked between rows, never mid-row: a row is either
        // fully imported or not started, so stopping never leaves a half-written
        // record behind.
        if task.lock().await.status == ImportTaskStatus::Cancelling {
            break;
        }

        let position = index as i32 + 1;
        let label = repository::row_label(subject, &mapping, row);
        let outcome =
            repository::import_row(subject, &mapping, &policy, row, &actor_id, &actor).await;

        {
            let mut task = task.lock().await;
            match &outcome {
                RowOutcome::Created(_) => task.created_count += 1,
                RowOutcome::Updated(_) => task.updated_count += 1,
                RowOutcome::Skipped => task.skipped_count += 1,
                RowOutcome::Failed(error) => {
                    task.failed_count += 1;
                    if task.failures.len() < MAX_TRACKED_FAILURES {
                        task.failures.push(ImportRowFailure {
                            row: position,
                            label: label.clone(),
                            error: error.clone(),
                        });
                    }
                }
            }
        }

        // Best-effort: the durable row status is useful for reconciliation after
        // a crash, but losing one must not abandon an otherwise fine import.
        if let Err(error) = repository::record_row(&task_id, position, &outcome, &label).await {
            tracing::warn!(task_id = %task_id, position, "import row status write failed: {error}");
        }

        // Yield so a cancel request is seen promptly on a single-row-per-tick
        // import; `notified` is drained rather than awaited so it never blocks.
        tokio::select! {
            biased;
            _ = cancel.notified() => {}
            _ = tokio::task::yield_now() => {}
        }
    }

    {
        let mut state = task.lock().await;
        state.status = if state.status == ImportTaskStatus::Cancelling {
            ImportTaskStatus::Cancelled
        } else {
            ImportTaskStatus::Completed
        };
        state.completed_at = Some(Utc::now());
    }

    // The final snapshot is what the history shows, so it is retried rather than
    // dropped on a transient database problem.
    loop {
        let snapshot = task.lock().await.clone();
        match repository::finish_task(&snapshot).await {
            Ok(()) => return,
            Err(error) => {
                tracing::warn!(task_id = %task_id, "import final history write failed: {error}; retrying");
                tokio::time::sleep(FINAL_WRITE_RETRY_DELAY).await;
            }
        }
    }
}
