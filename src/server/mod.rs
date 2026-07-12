//! Server-side (SSR-only) code: the dedicated JSON API, authorization layer,
//! RAG chatbot pipeline, and CAPTCHA verification. The CRM's request handlers
//! themselves live in [`crate::server_fns`] as Leptos server functions.

pub mod api;
pub mod auth;
pub mod captcha;
pub mod config;
pub mod db;
pub mod docs;
pub mod email;
pub mod notifications;
pub mod permissions;
pub mod rag;
pub mod service;
pub mod storage;
pub mod telemetry;
