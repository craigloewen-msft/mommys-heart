//! Admin contact-mail campaigns: recipient selection, task progress, and cancellation.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::server_fns::contacts::ContactType;
use crate::server_fns::pagination::Page;

pub const MAX_MAIL_SUBJECT: usize = 200;
pub const MAX_MAIL_BODY: usize = 20_000;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ContactMailFilters {
    pub query: String,
    pub category_ids: Vec<String>,
    pub contact_type: Option<ContactType>,
    pub organization_id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ContactMailCandidate {
    pub id: String,
    pub name: String,
    pub email: String,
    pub organization: String,
    pub types: Vec<ContactType>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ContactMailSelection {
    /// Select every eligible contact matching the snapshotted filters.
    pub all_matching: bool,
    pub filters: ContactMailFilters,
    /// Explicit selections when `all_matching` is false.
    pub contact_ids: Vec<String>,
    /// Explicit exceptions when `all_matching` is true.
    pub excluded_contact_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactMailTaskStatus {
    Queued,
    Running,
    Cancelling,
    Completed,
    Cancelled,
    Failed,
}

impl ContactMailTaskStatus {
    pub fn from_slug(value: &str) -> Option<Self> {
        match value {
            "queued" => Some(Self::Queued),
            "running" => Some(Self::Running),
            "cancelling" => Some(Self::Cancelling),
            "completed" => Some(Self::Completed),
            "cancelled" => Some(Self::Cancelled),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Queued => "Queued",
            Self::Running => "Running",
            Self::Cancelling => "Cancelling",
            Self::Completed => "Completed",
            Self::Cancelled => "Cancelled",
            Self::Failed => "Failed",
        }
    }

    pub fn is_active(self) -> bool {
        matches!(self, Self::Queued | Self::Running | Self::Cancelling)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ContactMailFailure {
    pub name: String,
    pub email: String,
    pub error: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContactMailTask {
    pub id: String,
    pub status: ContactMailTaskStatus,
    pub subject: String,
    pub body: String,
    pub created_by_name: String,
    pub recipient_total: i32,
    pub accepted_count: i32,
    pub failed_count: i32,
    pub created_at: String,
    pub started_at: String,
    pub completed_at: String,
    pub next_send_at: String,
    pub cancel_requested_at: String,
    pub cancel_requested_by_name: String,
    pub error: String,
    pub failures: Vec<ContactMailFailure>,
}

impl ContactMailTask {
    pub fn processed_count(&self) -> i32 {
        self.accepted_count + self.failed_count
    }

    pub fn remaining_count(&self) -> i32 {
        (self.recipient_total - self.processed_count()).max(0)
    }
}

#[cfg(feature = "ssr")]
fn normalize_filters(mut filters: ContactMailFilters) -> ContactMailFilters {
    filters.query = filters.query.trim().to_string();
    filters.organization_id = filters.organization_id.trim().to_string();
    filters.category_ids = filters
        .category_ids
        .into_iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect();
    filters.category_ids.sort();
    filters.category_ids.dedup();
    filters
}

#[server(prefix = "/api", input = leptos::server_fn::codec::Json)]
pub async fn list_contact_mail_candidates(
    filters: ContactMailFilters,
    offset: i64,
    limit: i64,
) -> Result<Page<ContactMailCandidate>, ServerFnError> {
    let user = crate::server::permissions::require_user().await?;
    crate::server::permissions::require_operations_admin(&user)?;
    crate::server::permissions::require_information_management_access(&user)?;
    crate::server::db::contact_mail::candidate_page(&normalize_filters(filters), offset, limit)
        .await
        .map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn load_contact_mail_task() -> Result<Option<ContactMailTask>, ServerFnError> {
    let user = crate::server::permissions::require_user().await?;
    crate::server::permissions::require_operations_admin(&user)?;
    crate::server::permissions::require_information_management_access(&user)?;
    if let Some(task) = crate::server::contact_mail::current_task().await {
        Ok(Some(task))
    } else {
        crate::server::db::contact_mail::latest_task()
            .await
            .map_err(ServerFnError::new)
    }
}

#[server(prefix = "/api", input = leptos::server_fn::codec::Json)]
pub async fn start_contact_mail_task(
    selection: ContactMailSelection,
    subject: String,
    body: String,
) -> Result<ContactMailTask, ServerFnError> {
    let user = crate::server::permissions::require_user().await?;
    crate::server::permissions::require_operations_admin(&user)?;
    crate::server::permissions::require_information_management_access(&user)?;
    let cfg = crate::server::config::EmailConfig::from_env();
    if !cfg.dry_run && !cfg.is_configured() {
        return Err(ServerFnError::new(
            "Email delivery is not configured. Configure ACS Email or enable EMAIL_DRY_RUN.",
        ));
    }

    let subject = subject.trim().to_string();
    let body = body.trim().to_string();
    if subject.is_empty() {
        return Err(ServerFnError::new("Enter an email subject."));
    }
    if subject.chars().count() > MAX_MAIL_SUBJECT {
        return Err(ServerFnError::new(format!(
            "The subject must be {MAX_MAIL_SUBJECT} characters or fewer."
        )));
    }
    if subject.chars().any(char::is_control) {
        return Err(ServerFnError::new(
            "The subject cannot contain line breaks or control characters.",
        ));
    }
    if body.is_empty() {
        return Err(ServerFnError::new("Enter an email message."));
    }
    if body.chars().count() > MAX_MAIL_BODY {
        return Err(ServerFnError::new(format!(
            "The message must be {MAX_MAIL_BODY} characters or fewer."
        )));
    }
    if body.contains('\0') {
        return Err(ServerFnError::new(
            "The message contains an unsupported null character.",
        ));
    }

    let selection = ContactMailSelection {
        all_matching: selection.all_matching,
        filters: normalize_filters(selection.filters),
        contact_ids: clean_ids(selection.contact_ids),
        excluded_contact_ids: clean_ids(selection.excluded_contact_ids),
    };
    crate::server::contact_mail::start(&selection, &subject, &body, &user.id, &user.full_name())
        .await
        .map_err(ServerFnError::new)
}

#[cfg(feature = "ssr")]
fn clean_ids(ids: Vec<String>) -> Vec<String> {
    let mut ids = ids
        .into_iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    ids
}

#[server(prefix = "/api")]
pub async fn cancel_contact_mail_task(task_id: String) -> Result<ContactMailTask, ServerFnError> {
    let user = crate::server::permissions::require_user().await?;
    crate::server::permissions::require_operations_admin(&user)?;
    crate::server::permissions::require_information_management_access(&user)?;
    let task_id = task_id.trim();
    if task_id.is_empty() {
        return Err(ServerFnError::new("No mail task was requested."));
    }
    if let Some(task) = crate::server::contact_mail::cancel(task_id, &user.id, &user.full_name())
        .await
        .map_err(ServerFnError::new)?
    {
        return Ok(task);
    }
    let latest = crate::server::db::contact_mail::latest_task()
        .await
        .map_err(ServerFnError::new)?;
    latest
        .filter(|task| task.id == task_id)
        .ok_or_else(|| ServerFnError::new("Mail task not found."))
}
