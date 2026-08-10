//! Client-side application state for the CRM. Holds the signed-in user plus
//! reactive caches (users, cases, messages) that are **loaded from and
//! written through to the server** via the Leptos server functions in
//! [`crate::server_fns`]. The server (backed by PostgreSQL) is the source of
//! truth; these signals are a live cache so the UI stays reactive. After every
//! mutation we reconcile the affected cache from the server so ids and derived
//! data stay correct.

use leptos::prelude::*;
use leptos::task::spawn_local;

use serde::{Deserialize, Serialize};

use crate::server_fns::auth::LoginOutcome;
use crate::server_fns::channel_notifications::{self, ChannelUnread};
use crate::server_fns::users::{AccountRole, UserSummary};
use crate::server_fns::{auth, err_text};

/// The counts the app chrome badges. A view over several tables rather than an
/// entity, so it lives with the state it feeds instead of in `server_fns`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AppBadges {
    pub unread: Vec<ChannelUnread>,
    pub admin_requests_pending: i64,
    pub cases_pending_review: i64,
    pub volunteer_requests_pending: i64,
}

/// Load every badge count for the signed-in user in one round trip. Counts the
/// caller may not see are zero rather than an error.
#[server(prefix = "/api")]
pub async fn load_app_badges() -> Result<AppBadges, ServerFnError> {
    use crate::server::db::{admin_requests, cases, channel_notifications, volunteers};
    use crate::server::permissions::require_user;

    let user = require_user().await?;

    let unread = channel_notifications::unread_for_user(&user.id)
        .await
        .map_err(ServerFnError::new)?;
    let admin_requests_pending = if user.role.is_site_admin() {
        admin_requests::pending_count()
            .await
            .map_err(ServerFnError::new)?
    } else {
        0
    };
    // Any admin may review a case, unlike approval requests.
    let cases_pending_review = if user.role.has_operations_admin_permissions() {
        cases::pending_review_count()
            .await
            .map_err(ServerFnError::new)?
    } else {
        0
    };

    // Site admins only, matching who may see and decide the applications.
    let volunteer_requests_pending = if user.role.is_site_admin() {
        volunteers::pending_count()
            .await
            .map_err(ServerFnError::new)?
    } else {
        0
    };

    Ok(AppBadges {
        unread,
        admin_requests_pending,
        cases_pending_review,
        volunteer_requests_pending,
    })
}

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
    pub auth_resolved: RwSignal<bool>,
    pub unread: RwSignal<Vec<ChannelUnread>>,
    pub admin_request_pending: RwSignal<i64>,
    /// Cases waiting for an admin accept/decline; drives the count badges.
    pub cases_pending_review: RwSignal<i64>,
    /// Volunteer applications waiting on a decision.
    pub volunteer_requests_pending: RwSignal<i64>,
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
            auth_resolved: RwSignal::new(false),
            unread: RwSignal::new(Vec::new()),
            admin_request_pending: RwSignal::new(0),
            cases_pending_review: RwSignal::new(0),
            volunteer_requests_pending: RwSignal::new(0),
        }
    }

    // --- auth ---------------------------------------------------------------

    /// Ask the server who the session cookie belongs to and record the answer.
    ///
    /// Called once when the app boots. The session lives in an `HttpOnly`
    /// cookie that script cannot read, so this round-trip is the only way the
    /// client learns it is already signed in after a page load.
    ///
    /// Browser-only: there is no local task executor during server rendering
    /// (`spawn_local` panics there), and the server-rendered pass has nothing
    /// to restore anyway — it emits the loading state, and the client resolves
    /// the session as soon as it hydrates.
    pub fn restore_session(self) {
        if !cfg!(feature = "hydrate") {
            return;
        }
        spawn_local(async move {
            if let Ok(Some(user)) = auth::current_user().await {
                self.current_user_summary.set(Some(user));
                self.refresh_badges();
            }
            self.auth_resolved.set(true);
        });
    }

    pub fn refresh_unread(self) {
        if !cfg!(feature = "hydrate") {
            return;
        }
        if self.current_user_summary.with_untracked(|u| u.is_none()) {
            return;
        }
        spawn_local(async move {
            if let Ok(list) = channel_notifications::load_unread_notifications().await {
                self.unread.set(list);
            }
        });
    }

    /// Refresh every badge count in one request.
    pub fn refresh_badges(self) {
        if !cfg!(feature = "hydrate") {
            return;
        }
        if self.current_user_summary.with_untracked(|u| u.is_none()) {
            return;
        }
        spawn_local(async move {
            if let Ok(badges) = load_app_badges().await {
                self.unread.set(badges.unread);
                self.admin_request_pending
                    .set(badges.admin_requests_pending);
                self.cases_pending_review.set(badges.cases_pending_review);
                self.volunteer_requests_pending
                    .set(badges.volunteer_requests_pending);
            }
        });
    }

    /// Optimistically drop a channel's unread entry once the user opens it, so
    /// the badge clears immediately without waiting for the next server refresh.
    pub fn clear_channel_unread(self, channel_id: &str) {
        self.unread
            .update(|list| list.retain(|c| c.channel_id != channel_id));
    }

    pub fn total_unread(&self) -> i64 {
        self.unread.get().iter().map(|c| c.count).sum()
    }

    pub fn is_authenticated(&self) -> bool {
        self.current_user_summary.get().is_some()
    }

    pub fn role(&self) -> Option<AccountRole> {
        self.current_user_summary.get().map(|u| u.role)
    }

    /// Whether the signed-in user may perform operations-admin actions.
    pub fn has_operations_admin_permissions(&self) -> bool {
        self.role()
            .map(|role| role.has_operations_admin_permissions())
            .unwrap_or(false)
    }

    pub fn is_site_admin(&self) -> bool {
        self.role().map(|r| r.is_site_admin()).unwrap_or(false)
    }

    pub fn is_volunteer_or_admin(&self) -> bool {
        self.role()
            .map(|r| !matches!(r, AccountRole::Client))
            .unwrap_or(false)
    }

    /// Attempt a sign-in.
    pub async fn login(self, email: &str, password: &str) -> Result<LoginOutcome, String> {
        let outcome = auth::login(email.trim().to_string(), password.to_string())
            .await
            .map_err(err_text)?;
        if let LoginOutcome::Authenticated(user) = &outcome {
            self.current_user_summary.set(Some(user.clone().into()));
            self.refresh_badges();
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
        self.refresh_badges();
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
        self.refresh_badges();
        Ok(())
    }

    pub fn logout(self) {
        self.current_user_summary.set(None);
        self.unread.set(Vec::new());
        self.admin_request_pending.set(0);
        self.cases_pending_review.set(0);
        self.volunteer_requests_pending.set(0);
        spawn_local(async move {
            let _ = auth::logout().await;
        });
    }
}
