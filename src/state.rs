//! Client-side application state for the CRM. Holds the signed-in user plus
//! reactive caches (users, cases, grants, messages) that are **loaded from and
//! written through to the server** via the Leptos server functions in
//! [`crate::server_fns`]. The server (backed by PostgreSQL) is the source of
//! truth; these signals are a live cache so the UI stays reactive. After every
//! mutation we reconcile the affected cache from the server so ids and derived
//! data stay correct.

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::server_fns::users::{AccountRole, UserSummary};
use crate::server_fns::auth::LoginOutcome;
use crate::server_fns::{auth, err_text};

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
    pub current_user_summary: RwSignal<Option<UserSummary>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            current_user_summary: RwSignal::new(None),
        }
    }

    // --- auth ---------------------------------------------------------------

    pub fn is_authenticated(&self) -> bool {
        self.current_user_summary.get().is_some()
    }

    pub fn role(&self) -> Option<AccountRole> {
        self.current_user_summary.get().map(|u| u.role)
    }

    pub fn is_admin(&self) -> bool {
        self.role().map(|r| r.is_admin()).unwrap_or(false)
    }

    /// Attempt a sign-in.
    pub async fn login(self, email: &str, password: &str) -> Result<LoginOutcome, String> {
        let outcome = auth::login(email.trim().to_string(), password.to_string())
            .await
            .map_err(err_text)?;
        if let LoginOutcome::Authenticated(user) = &outcome {
            self.current_user_summary.set(Some(user.clone().into()));
        }
        Ok(outcome)
    }

    /// Finish a login by submitting the emailed one-time code. When
    /// `remember_device` is set, this browser is trusted for 30 days and can
    /// skip MFA on future logins.
    pub async fn verify_mfa(self, code: &str, remember_device: bool) -> Result<(), String> {
        let user = auth::verify_mfa(code.trim().to_string(), remember_device)
            .await
            .map_err(err_text)?;
        self.current_user_summary.set(Some(user.into()));
        Ok(())
    }

    /// Begin a self-service registration. This does **not** create the account
    /// or sign the user in; it emails a verification code and starts the
    /// email-OTP challenge. The client should route to the verification screen
    /// and call [`verify_registration`](Self::verify_registration).
    pub async fn register(
        self,
        first_name: &str,
        last_name: &str,
        email: &str,
        password: &str,
    ) -> Result<(), String> {
        auth::register(
            first_name.trim().to_string(),
            last_name.trim().to_string(),
            email.trim().to_string(),
            password.to_string(),
        )
        .await
        .map_err(err_text)?;
        Ok(())
    }

    /// Finish a registration by submitting the emailed one-time code. On success
    /// the account is created and this browser is signed in.
    pub async fn verify_registration(self, code: &str) -> Result<(), String> {
        let user = auth::verify_registration(code.trim().to_string())
            .await
            .map_err(err_text)?;
        self.current_user_summary.set(Some(user.into()));
        Ok(())
    }

    pub fn logout(self) {
        self.current_user_summary.set(None);
        spawn_local(async move {
            let _ = auth::logout().await;
        });
    }
}
