//! Reusable route guards.
//!
//! Pages call these at the top of their render. If access is denied, the helper
//! returns `Some(view)` the page should return immediately; otherwise `None`.

use leptos::prelude::*;
use leptos_router::components::Redirect;

use crate::state::AppState;

/// Redirect unauthenticated visitors to `/login`.
pub fn require_login(state: &AppState) -> Option<AnyView> {
    if state.current_user.get_untracked().is_none() {
        Some(view! { <Redirect path="/login" /> }.into_any())
    } else {
        None
    }
}

/// Redirect visitors who are not admins: signed-out users go to `/login`,
/// signed-in non-admins go to `/cases`.
pub fn require_admin(state: &AppState) -> Option<AnyView> {
    match state.current_user.get_untracked() {
        None => Some(view! { <Redirect path="/login" /> }.into_any()),
        Some(u) if u.role.is_admin() => None,
        Some(_) => Some(view! { <Redirect path="/cases" /> }.into_any()),
    }
}
