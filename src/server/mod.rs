//! Server-side (SSR-only) code: the dedicated JSON API, business-logic service
//! layer, RAG chatbot pipeline, CAPTCHA verification, and mock CRM data.

pub mod api;
pub mod captcha;
pub mod comm_store;
pub mod config;
pub mod data;
pub mod docs;
pub mod rag;
pub mod service;
