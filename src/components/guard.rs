//! Reusable, reactive route guards.
//!
//! Session resolution now happens asynchronously in the browser (the client
//! fetches `/api/auth/me` after hydration), so guards must react to
//! [`AuthPhase`] rather than reading the user once. While the session is still
//! resolving they show a lightweight loading screen; only once we know the
//! visitor is signed out do we redirect.
//!
//! Usage: a protected page builds its signals as usual, then returns
//! `require_login(state, move || <content>.into_any())` (or `require_admin`).
//! The `content` closure is only invoked once a matching session is confirmed.

use leptos::prelude::*;
use leptos_router::components::Redirect;

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
        } else {
            view! { <Redirect path="/login" /> }.into_any()
        }
    })
    .into_any()
}

/// Show `content` only to admins. Signed-out visitors go to `/login`; signed-in
/// non-admins go to `/cases`. Reactive on the auth phase.
pub fn require_admin<F>(state: AppState, content: F) -> AnyView
where
    F: Fn() -> AnyView + Send + 'static,
{
    (move || {
        if state.is_authenticated() {
            if state.is_admin() {
                content()
            } else {
                view! { <Redirect path="/cases" /> }.into_any()
            }
        } else {
            view! { <Redirect path="/login" /> }.into_any()
        }
    })
    .into_any()
}
