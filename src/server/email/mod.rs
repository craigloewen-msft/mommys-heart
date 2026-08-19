//! Outbound email (SSR only): the branded templates for app notifications,
//! the ACS REST transport that delivers them, and offline preview tooling.
//!
//! Each concern lives in its own submodule:
//! - [`templates`] builds the shared, branded HTML/plain-text bodies;
//! - [`palette`] resolves the app's Tailwind colors to hex for inline styling;
//! - [`communicationservice`] signs and POSTs a message to Azure Communication Services;
//! - [`preview`] renders every template to disk for offline design iteration.
//!
//! The direct [`send_email`] transport and high-level [`send_contact_batch`]
//! delivery path are re-exported here so callers use `crate::server::email`.

pub mod auth_notifications;
pub mod communicationservice;
pub mod palette;
pub mod preview;
pub mod templates;

pub use communicationservice::{
    send_contact_batch, send_email, EmailBatchOutcome, EmailBatchRecipient, EmailKind,
    EmailMessage, EmailRecipient, EmailRecipients, MAX_RECIPIENTS_PER_MESSAGE,
};
