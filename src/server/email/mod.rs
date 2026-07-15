//! Outbound email (SSR only): the branded templates for the CRM's notifications,
//! the ACS REST transport that delivers them, and offline preview tooling.
//!
//! Each concern lives in its own submodule:
//! - [`templates`] builds the shared, branded HTML/plain-text bodies;
//! - [`palette`] resolves the app's Tailwind colors to hex for inline styling;
//! - [`communicationservice`] signs and POSTs a message to Azure Communication Services;
//! - [`preview`] renders every template to disk for offline design iteration.
//!
//! The delivery entry point [`send_email`] and its [`EmailMessage`] input are
//! re-exported here so callers use `crate::server::email::{send_email, EmailMessage}`.

pub mod auth_notifications;
pub mod communicationservice;
pub mod palette;
pub mod preview;
pub mod templates;

pub use communicationservice::{send_email, EmailMessage};
