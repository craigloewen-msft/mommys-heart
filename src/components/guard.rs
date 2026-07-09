//! Reusable route guard for permission-based access control (RBAC).
//!
//! Pages call [`deny_redirect`] at the top of their render. If the signed-in
//! user lacks the required [`Permission`], it returns a redirect view the page
//! should return immediately; otherwise it returns `None` and the page renders.

use leptos::prelude::*;
use leptos_router::components::Redirect;

use crate::state::AppState;
use crate::types::Permission;

/// Returns `Some(redirect)` when the current user may **not** exercise `perm`,
/// and `None` when access is granted.
///
/// Unauthenticated users are sent to `/login`; authenticated users without the
/// permission are bounced back to the landing page for their authorization
/// level so they never see a screen they cannot use.
pub fn deny_redirect(state: &AppState, perm: Permission) -> Option<AnyView> {
    match state.current_user.get_untracked() {
        None => Some(view! { <Redirect path="/login" /> }.into_any()),
        Some(u) if u.role.can(perm) => None,
        Some(u) if u.role.is_staff_level() => Some(view! { <Redirect path="/admin" /> }.into_any()),
        Some(_) => Some(view! { <Redirect path="/volunteer" /> }.into_any()),
    }
}
