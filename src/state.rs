//! Client-side application state for the CRM. Holds the signed-in user plus
//! reactive caches (users, cases, grants, messages) that are **loaded from and
//! written through to the server** via the Leptos server functions in
//! [`crate::server_fns`]. The server (backed by PostgreSQL) is the source of
//! truth; these signals are a live cache so the UI stays reactive. After every
//! mutation we reconcile the affected cache from the server so ids and derived
//! data stay correct.

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::server_fns::{auth, err_text};
use crate::types::AccountRole;
use crate::server_fns::users::User;

/// Current local date-time as `YYYY-MM-DD HH:MM`, read from the browser clock.
/// Retained for any client-side display needs; persisted timestamps are now
/// produced on the server.
pub fn now_stamp() -> String {
    #[cfg(feature = "hydrate")]
    {
        let d = js_sys::Date::new_0();
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}",
            d.get_full_year(),
            d.get_month() + 1,
            d.get_date(),
            d.get_hours(),
            d.get_minutes(),
        )
    }
    #[cfg(not(feature = "hydrate"))]
    {
        "1970-01-01 00:00".to_string()
    }
}

/// Current local date as `YYYY-MM-DD` (see [`now_stamp`]).
pub fn today() -> String {
    now_stamp().chars().take(10).collect()
}

/// Shared, reactive application state. `RwSignal` is `Copy`, so the whole struct
/// is cheap to copy and can be pulled from context anywhere.
#[derive(Clone, Copy)]
pub struct AppState {
    pub current_user: RwSignal<Option<User>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            current_user: RwSignal::new(None),
        }
    }

    // --- auth ---------------------------------------------------------------

    pub fn is_authenticated(&self) -> bool {
        self.current_user.get().is_some()
    }

    pub fn role(&self) -> Option<AccountRole> {
        self.current_user.get().map(|u| u.role)
    }

    pub fn is_admin(&self) -> bool {
        self.role().map(|r| r.is_admin()).unwrap_or(false)
    }

    pub async fn login(self, email: &str, password: &str) -> Result<(), String> {
        let user = auth::login(email.trim().to_string(), password.to_string())
            .await
            .map_err(err_text)?;
        self.current_user.set(Some(user));
        Ok(())
    }

    pub async fn register(
        self,
        first_name: &str,
        last_name: &str,
        email: &str,
        password: &str,
    ) -> Result<(), String> {
        let user = auth::register(
            first_name.trim().to_string(),
            last_name.trim().to_string(),
            email.trim().to_string(),
            password.to_string(),
        )
        .await
        .map_err(err_text)?;
        self.current_user.set(Some(user));
        Ok(())
    }

    pub fn logout(self) {
        self.current_user.set(None);
        spawn_local(async move {
            let _ = auth::logout().await;
        });
    }
}
