//! The CRM's server functions, grouped by concept.
//!
//! Each `#[server]` function is written **once**: Leptos generates the server
//! endpoint *and* the browser-side call, so there is no hand-written API client
//! and no separate request/response DTO layer. The client just calls these
//! functions like normal `async fn`s; under the hood they become HTTP calls to
//! auto-registered endpoints under `/api/rpc/*`.
//!
//! On the server, each function resolves the caller from the session cookie
//! (via [`crate::server::permissions`]), enforces authorization, and delegates
//! persistence to the repositories in [`crate::server::db`]. On the client, only
//! the function signature is compiled — the body never ships to the browser.
//!
//! The only surface that stays a plain REST endpoint is the chat widget
//! (`/api/chat`), which is called cross-origin from the Squarespace site and so
//! cannot use same-origin server functions.

pub mod auth;
pub mod cases;
pub mod grants;
pub mod session;
pub mod users;
