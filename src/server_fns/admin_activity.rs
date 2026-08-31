//! The admin activity feed: a read over the audit logs, for administrators.
//!
//! This is **not** a second record of what happened. Every event shown here is a
//! row the audit trail already wrote — [`crate::server::db::audit`] for cases,
//! contacts, organizations, documents and folders, plus the restricted
//! `case_note_audit_log` for case notes (which stays separate per ADR-0003, and
//! is projected here without any note content).
//!
//! What this module adds is *classification*: audit rows are field-level and
//! include work administrators did not ask to be alerted about (role changes,
//! capability grants, imports). [`AdminActivityCategory::classify`] maps an audit
//! row onto one of the categories the alert covers, or `None` to leave it out of
//! the feed. That mapping is the whole difference between the Change Log and
//! this feed, so it lives in one function that both the feed and the digest use.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::server_fns::crm::coded_enum;
use crate::server_fns::pagination::Page;

coded_enum!(AdminActivityCategory {
    CaseCreated => ("case_created", "New case"),
    CaseNote => ("case_note", "Case note"),
    CaseInformation => ("case_information", "Case information"),
    Document => ("document", "Documents and files"),
    Contact => ("contact", "Contacts and organizations"),
});

// Which record an event happened to. Decides where the feed row links.
coded_enum!(AdminActivitySubject {
    Case => ("case", "Case"),
    Contact => ("contact", "Contact"),
    Organization => ("organization", "Organization"),
});

/// Audit `field` values on a case that are *not* case information edits: each
/// belongs to a more specific category, or to no category at all.
///
/// `evidence` is the historical name for `document`, written by the blob-backed
/// implementation whose audit rows outlived it. Both are listed so the feed
/// keeps showing activity recorded before case files moved to SharePoint.
const CASE_DOCUMENT_FIELDS: &[&str] = &["document", "folder", "evidence"];
const CASE_CONTACT_FIELDS: &[&str] = &["case contact"];
/// Chat and transcript activity has its own notification category
/// ([`crate::server_fns::settings::NotificationKind::NewMessage`]) and is not
/// part of what this alert covers.
const CASE_EXCLUDED_FIELDS: &[&str] = &["message", "message channel", "message transcript export"];

impl AdminActivityCategory {
    /// Which category an audit row belongs to, or `None` to leave it out of the
    /// feed.
    ///
    /// `entity` is the audit row's `entity_type` and `field` its `field`. The
    /// `user` entity is always excluded: role changes, information-access
    /// grants and per-case capability assignments are account administration,
    /// not the case and record activity this alert is about.
    pub fn classify(entity: &str, field: &str) -> Option<Self> {
        match entity {
            // A case's own creation, written by `db::cases::create`.
            "case" if field == "case" => Some(Self::CaseCreated),
            // Projected from `case_note_audit_log` by the feed query.
            "case" if field == "case note" => Some(Self::CaseNote),
            "case" if CASE_EXCLUDED_FIELDS.contains(&field) => None,
            "case" if CASE_DOCUMENT_FIELDS.contains(&field) => Some(Self::Document),
            "case" if CASE_CONTACT_FIELDS.contains(&field) => Some(Self::Contact),
            // Everything else on a case is information: status, name, owner,
            // and every intake/case property (whose `field` is the property's
            // own name, so it cannot be enumerated).
            "case" => Some(Self::CaseInformation),
            "contact" | "organization" => Some(Self::Contact),
            _ => None,
        }
    }

    /// The subject kind a classified audit row refers to.
    pub fn subject_for(entity: &str) -> Option<AdminActivitySubject> {
        match entity {
            "case" => Some(AdminActivitySubject::Case),
            "contact" => Some(AdminActivitySubject::Contact),
            "organization" => Some(AdminActivitySubject::Organization),
            _ => None,
        }
    }
}

/// One recorded action, as the feed and the digest email show it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdminActivityEvent {
    pub id: String,
    pub category: AdminActivityCategory,
    /// Display name of whoever acted.
    pub actor: String,
    /// A human sentence describing what they did, e.g. `created the case`.
    pub summary: String,
    pub subject: AdminActivitySubject,
    pub subject_id: String,
    /// The record's *current* display name, joined at read time; falls back to
    /// the id when the record has since been deleted.
    pub subject_name: String,
    /// Human-readable timestamp (`YYYY-MM-DD HH:MM`, local time).
    pub at: String,
}

impl AdminActivityEvent {
    /// Where the feed row links, or `None` when the subject has no page.
    pub fn href(&self) -> Option<String> {
        if self.subject_id.is_empty() {
            return None;
        }
        Some(match self.subject {
            AdminActivitySubject::Case => format!("/admin/cases/{}", self.subject_id),
            AdminActivitySubject::Contact => format!("/contacts/{}", self.subject_id),
            AdminActivitySubject::Organization => format!("/organizations/{}", self.subject_id),
        })
    }
}

/// One page of recorded activity, newest first, optionally narrowed to a single
/// category. Requires operations-admin permissions.
#[server(prefix = "/api")]
pub async fn list_admin_activity_page(
    category: Option<AdminActivityCategory>,
    offset: i64,
    limit: i64,
) -> Result<Page<AdminActivityEvent>, ServerFnError> {
    use crate::server::db::audit;
    use crate::server::permissions::{require_operations_admin, require_user};

    /// Hard server-side cap on rows per request, regardless of what the client
    /// asks for — the feed paginates in small windows, matching the audit and
    /// email-failure viewers.
    const MAX_LIMIT: i64 = 200;

    let user = require_user().await?;
    require_operations_admin(&user)?;

    audit::activity_page(category, offset.max(0), limit.clamp(1, MAX_LIMIT))
        .await
        .map_err(ServerFnError::new)
}
