//! Per-user settings shared by the client and the server.
//!
//! [`UserSettings`] is a user's whole settings set. Today its only part is email
//! notifications: [`NotificationKind`] is the vocabulary of categories a user
//! can toggle and [`NotificationSettings`] is the notification preference set (a
//! master switch plus one flag per category). Grouping them under
//! [`UserSettings`] leaves room for future, non-notification settings. All are
//! consumed by the `/settings` page, the self-service server functions here, and
//! the server-side dispatcher in [`crate::server::notifications`], so they live
//! in a shared module.
//!
//! Settings are **self-service**: every account manages only its own
//! preferences (unlike the admin-gated user management in
//! [`crate::server_fns::users`]).

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// A user's complete settings set. Notification preferences are currently the
/// only part; new settings groups can be added as additional fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserSettings {
    /// Email-notification preferences (master switch + per-category flags).
    pub notifications: NotificationSettings,
}

/// A category of email notification a user may independently enable or disable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationKind {
    /// A new message was posted in a case's chat thread.
    NewMessage,
    /// A case's core data changed (status, name, owner, or properties).
    CaseData,
    /// A note was added to a case.
    NoteAdded,
    /// A case's evidence changed (added or removed).
    EvidenceChanged,
    /// The recipient was assigned to a case.
    Assigned,
    /// An administrative request changes or a client completes a case signup.
    AdminRequests,
}

impl NotificationKind {
    pub const ALL: [NotificationKind; 6] = [
        NotificationKind::NewMessage,
        NotificationKind::CaseData,
        NotificationKind::NoteAdded,
        NotificationKind::EvidenceChanged,
        NotificationKind::Assigned,
        NotificationKind::AdminRequests,
    ];

    pub fn label(self) -> &'static str {
        match self {
            NotificationKind::NewMessage => "New chat message",
            NotificationKind::CaseData => "Case data changed",
            NotificationKind::NoteAdded => "Note added",
            NotificationKind::EvidenceChanged => "Evidence changed",
            NotificationKind::Assigned => "Assigned to a case",
            NotificationKind::AdminRequests => "Administrative updates",
        }
    }

    /// Short helper text shown under each toggle on the settings page.
    pub fn description(self) -> &'static str {
        match self {
            NotificationKind::NewMessage => {
                "Someone posts a message in the chat of a case you're on."
            }
            NotificationKind::CaseData => {
                "A case's status, name, owner, or properties are changed."
            }
            NotificationKind::NoteAdded => "A note is added to a case you're on.",
            NotificationKind::EvidenceChanged => "Evidence is added to or removed from a case.",
            NotificationKind::Assigned => "You are given access to a new case.",
            NotificationKind::AdminRequests => {
                "A client completes a case signup, an admin request needs review, or a request you filed is decided."
            }
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            NotificationKind::NewMessage => "new_message",
            NotificationKind::CaseData => "case_data",
            NotificationKind::NoteAdded => "note_added",
            NotificationKind::EvidenceChanged => "evidence_changed",
            NotificationKind::Assigned => "assigned",
            NotificationKind::AdminRequests => "admin_requests",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.slug() == s)
    }
}

/// A user's full email-notification preference set: a master switch plus one
/// flag per [`NotificationKind`]. The default is "everything on", matching the
/// database column defaults and how a user with no saved row is treated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationSettings {
    /// Master switch. When `false`, no email is sent regardless of the per-kind
    /// flags below.
    pub emails_enabled: bool,
    pub new_message: bool,
    pub case_data: bool,
    pub note_added: bool,
    pub evidence_changed: bool,
    pub assigned: bool,
    pub admin_requests: bool,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self::all_on()
    }
}

impl NotificationSettings {
    /// Every category enabled — the default for a user who has not customized
    /// their preferences.
    pub const fn all_on() -> Self {
        Self {
            emails_enabled: true,
            new_message: true,
            case_data: true,
            note_added: true,
            evidence_changed: true,
            assigned: true,
            admin_requests: true,
        }
    }

    /// The flag for a single category (ignoring the master switch).
    pub fn category(&self, kind: NotificationKind) -> bool {
        match kind {
            NotificationKind::NewMessage => self.new_message,
            NotificationKind::CaseData => self.case_data,
            NotificationKind::NoteAdded => self.note_added,
            NotificationKind::EvidenceChanged => self.evidence_changed,
            NotificationKind::Assigned => self.assigned,
            NotificationKind::AdminRequests => self.admin_requests,
        }
    }

    /// Mutable reference to a single category's flag (for the settings UI).
    pub fn category_mut(&mut self, kind: NotificationKind) -> &mut bool {
        match kind {
            NotificationKind::NewMessage => &mut self.new_message,
            NotificationKind::CaseData => &mut self.case_data,
            NotificationKind::NoteAdded => &mut self.note_added,
            NotificationKind::EvidenceChanged => &mut self.evidence_changed,
            NotificationKind::Assigned => &mut self.assigned,
            NotificationKind::AdminRequests => &mut self.admin_requests,
        }
    }

    /// Whether an email of the given kind should actually be sent: the master
    /// switch must be on *and* the category enabled.
    pub fn wants(&self, kind: NotificationKind) -> bool {
        self.emails_enabled && self.category(kind)
    }
}

/// Load the signed-in user's own settings, falling back to defaults
/// ("everything on") when they have never saved any.
#[server(prefix = "/api")]
pub async fn load_user_settings() -> Result<UserSettings, ServerFnError> {
    use crate::server::db::settings as settings_repo;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    settings_repo::get_settings(&user.id)
        .await
        .map_err(ServerFnError::new)
}

/// Save the signed-in user's own settings.
#[server(prefix = "/api")]
pub async fn save_user_settings(settings: UserSettings) -> Result<(), ServerFnError> {
    use crate::server::db::settings as settings_repo;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    settings_repo::upsert_settings(&user.id, &settings)
        .await
        .map_err(ServerFnError::new)
}
