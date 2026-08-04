//! Reusable, reactive route guards.
//!
//! Session resolution happens asynchronously in the browser (the client asks
//! the server who the `HttpOnly` session cookie belongs to after hydration —
//! see [`AppState::restore_session`]), so guards must react to that phase
//! rather than reading the user once. While the session is still resolving they
//! show a lightweight loading screen; only once we know the visitor is signed
//! out do we redirect. Redirecting eagerly would bounce every signed-in visitor
//! to `/login` on a page refresh.
//!
//! Usage: a protected page builds its signals as usual, then returns
//! `require_login(state, move || <content>.into_any())` (or
//! `require_operations_admin`).
//! The `content` closure is only invoked once a matching session is confirmed.

use leptos::prelude::*;
use leptos_router::components::Redirect;

use crate::components::loading::Loading;
use crate::state::AppState;

/// Show `content` only to authenticated visitors; redirect signed-out visitors
/// to `/login`. Reactive on the auth phase.
pub fn require_login<F>(state: AppState, content: F) -> AnyView
where
    F: Fn() -> AnyView + Send + 'static,
{
    (move || {
        if state.is_authenticated() {
            content()
        } else if !state.auth_resolved.get() {
            view! { <Loading label="Loading\u{2026}" /> }.into_any()
        } else {
            view! { <Redirect path="/login" /> }.into_any()
        }
    })
    .into_any()
}

/// Show `content` only to operations administrators or higher. Signed-out
/// visitors go to `/login`; other signed-in users go to `/cases`.
pub fn require_operations_admin<F>(state: AppState, content: F) -> AnyView
where
    F: Fn() -> AnyView + Send + 'static,
{
    (move || {
        if state.is_authenticated() {
            if state.has_operations_admin_permissions() {
                content()
            } else {
                view! { <Redirect path="/cases" /> }.into_any()
            }
        } else if !state.auth_resolved.get() {
            view! { <Loading label="Loading\u{2026}" /> }.into_any()
        } else {
            view! { <Redirect path="/login" /> }.into_any()
        }
    })
    .into_any()
}

/// Show `content` only to site administrators. Other authenticated users return
/// to `/cases`; signed-out visitors return to `/login`.
pub fn require_site_admin<F>(state: AppState, content: F) -> AnyView
where
    F: Fn() -> AnyView + Send + 'static,
{
    (move || {
        if state.is_authenticated() {
            if state.is_site_admin() {
                content()
            } else {
                view! { <Redirect path="/cases" /> }.into_any()
            }
        } else if !state.auth_resolved.get() {
            view! { <Loading label="Loading\u{2026}" /> }.into_any()
        } else {
            view! { <Redirect path="/login" /> }.into_any()
        }
    })
    .into_any()
}
