//! Server-side (SSR-only) code: the dedicated JSON API, business-logic service
//! layer, RAG chatbot pipeline, and CAPTCHA verification.

pub mod api;
pub mod captcha;
pub mod config;
pub mod docs;
pub mod rag;
pub mod service;
