//! Client-side application state for the CRM. Holds the signed-in user plus
//! reactive caches (users, cases, grants, messages) that are **loaded from and
//! written through to the server** via the Leptos server functions in
//! [`crate::server_fns`]. The server (backed by PostgreSQL) is the source of
//! truth; these signals are a live cache so the UI stays reactive. After every
//! mutation we reconcile the affected cache from the server so ids and derived
//! data stay correct.

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::server_fns::{auth, cases, grants, session, users};
use crate::types::{AccountRole, Case, CaseCapability, CaseStatus, Grant, Message, User};

/// Flatten a [`ServerFnError`] into the plain, user-facing message. Domain
/// errors are raised as `ServerFnError::ServerError(msg)`; we surface `msg`
/// directly (dropping the library's "error running server function:" prefix) so
/// the UI shows exactly the message we wrote.
fn err_msg(e: ServerFnError) -> String {
    match e {
        ServerFnError::ServerError(m) => m,
        other => other.to_string(),
    }
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

/// Where the app is in resolving the visitor's session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthPhase {
    /// Still checking with the server (initial page load).
    Loading,
    /// No valid session — the visitor must sign in.
    SignedOut,
    /// A user is signed in and the caches are populated.
    SignedIn,
}

/// Shared, reactive application state. `RwSignal` is `Copy`, so the whole struct
/// is cheap to copy and can be pulled from context anywhere.
#[derive(Clone, Copy)]
pub struct AppState {
    pub auth: RwSignal<AuthPhase>,
    pub current_user: RwSignal<Option<User>>,
    pub users: RwSignal<Vec<User>>,
    pub cases: RwSignal<Vec<Case>>,
    pub grants: RwSignal<Vec<Grant>>,
    pub messages: RwSignal<Vec<Message>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            auth: RwSignal::new(AuthPhase::Loading),
            current_user: RwSignal::new(None),
            users: RwSignal::new(Vec::new()),
            cases: RwSignal::new(Vec::new()),
            grants: RwSignal::new(Vec::new()),
            messages: RwSignal::new(Vec::new()),
        }
    }

    /// Kick off a session check + data load in the browser. On the server there
    /// is no session cookie context, so we stay in `Loading` (the client re-runs
    /// this after hydration).
    pub fn start_bootstrap(self) {
        #[cfg(feature = "hydrate")]
        spawn_local(async move {
            match session::bootstrap().await {
                Ok(data) => self.apply_bootstrap(data),
                Err(_) => self.auth.set(AuthPhase::SignedOut),
            }
        });
    }

    fn apply_bootstrap(self, data: crate::types::BootstrapResponse) {
        self.users.set(data.users);
        self.cases.set(data.cases);
        self.grants.set(data.grants);
        match data.current_user {
            Some(user) => {
                self.current_user.set(Some(user));
                self.auth.set(AuthPhase::SignedIn);
            }
            None => self.auth.set(AuthPhase::SignedOut),
        }
    }

    /// Reload users/cases/grants from the server (used to reconcile caches after
    /// a mutation).
    async fn refresh(self) -> Result<(), String> {
        let data = session::bootstrap().await.map_err(err_msg)?;
        self.apply_bootstrap(data);
        Ok(())
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
            .map_err(err_msg)?;
        self.current_user.set(Some(user));
        self.auth.set(AuthPhase::SignedIn);
        // Populate the caches for the freshly signed-in user.
        self.refresh().await
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
        .map_err(err_msg)?;
        self.current_user.set(Some(user));
        self.auth.set(AuthPhase::SignedIn);
        self.refresh().await
    }

    pub fn logout(self) {
        self.current_user.set(None);
        self.users.set(Vec::new());
        self.cases.set(Vec::new());
        self.grants.set(Vec::new());
        self.messages.set(Vec::new());
        self.auth.set(AuthPhase::SignedOut);
        spawn_local(async move {
            let _ = auth::logout().await;
        });
    }

    // --- cases (reads are synchronous against the cache) --------------------

    /// Cases the signed-in user can see. The server already filters to the
    /// caller's visible set, so this simply returns the cache.
    pub fn visible_cases(&self) -> Vec<Case> {
        self.cases.get()
    }

    /// The signed-in user's capabilities on a case. The case owner implicitly
    /// holds every capability; everyone else (including admins) holds exactly the
    /// set granted by their assignment.
    pub fn capabilities_on(&self, case: &Case) -> Vec<CaseCapability> {
        match self.current_user.get() {
            Some(u) if case.owner_id == u.id => CaseCapability::ALL.to_vec(),
            Some(u) => u.capabilities_for(&case.id),
            None => Vec::new(),
        }
    }

    pub fn case_can(&self, case: &Case, cap: CaseCapability) -> bool {
        self.capabilities_on(case).contains(&cap)
    }

    /// Display name for a user id (falls back to the id if unknown).
    pub fn user_name(&self, user_id: &str) -> String {
        self.users
            .get_untracked()
            .into_iter()
            .find(|u| u.id == user_id)
            .map(|u| u.full_name())
            .unwrap_or_else(|| user_id.to_string())
    }

    // --- case mutations (write through to the server) -----------------------

    pub async fn add_case_full(
        self,
        name: &str,
        status: CaseStatus,
        properties: Vec<(String, String)>,
        first_note: Option<String>,
    ) -> Result<String, String> {
        let id = cases::create_case(name.trim().to_string(), status, properties, first_note)
            .await
            .map_err(err_msg)?;
        self.refresh().await?;
        Ok(id)
    }

    pub async fn set_case_status(self, case_id: &str, status: CaseStatus) -> Result<(), String> {
        cases::set_case_status(case_id.to_string(), status)
            .await
            .map_err(err_msg)?;
        self.refresh().await
    }

    pub async fn set_case_name(self, case_id: &str, name: &str) -> Result<(), String> {
        cases::set_case_name(case_id.to_string(), name.trim().to_string())
            .await
            .map_err(err_msg)?;
        self.refresh().await
    }

    pub async fn set_case_owner(self, case_id: &str, owner_id: &str) -> Result<(), String> {
        cases::set_case_owner(case_id.to_string(), owner_id.to_string())
            .await
            .map_err(err_msg)?;
        self.refresh().await
    }

    pub async fn replace_case_properties(
        self,
        case_id: &str,
        props: Vec<(String, String)>,
    ) -> Result<(), String> {
        cases::set_case_properties(case_id.to_string(), props)
            .await
            .map_err(err_msg)?;
        self.refresh().await
    }

    pub async fn add_case_note(self, case_id: &str, body: &str) -> Result<(), String> {
        cases::add_case_note(case_id.to_string(), body.trim().to_string())
            .await
            .map_err(err_msg)?;
        self.refresh().await
    }

    pub async fn add_case_evidence(
        self,
        case_id: &str,
        name: &str,
        description: &str,
    ) -> Result<(), String> {
        cases::add_case_evidence(
            case_id.to_string(),
            name.trim().to_string(),
            description.trim().to_string(),
        )
        .await
        .map_err(err_msg)?;
        self.refresh().await
    }

    pub async fn delete_case_evidence(self, case_id: &str, evidence_id: &str) -> Result<(), String> {
        cases::delete_case_evidence(case_id.to_string(), evidence_id.to_string())
            .await
            .map_err(err_msg)?;
        self.refresh().await
    }

    // --- grants -------------------------------------------------------------

    pub async fn add_grant(self, name: &str) -> Result<(), String> {
        grants::add_grant(name.trim().to_string())
            .await
            .map_err(err_msg)?;
        self.refresh().await
    }

    pub async fn rename_grant(self, grant_id: &str, name: &str) -> Result<(), String> {
        grants::rename_grant(grant_id.to_string(), name.trim().to_string())
            .await
            .map_err(err_msg)?;
        self.refresh().await
    }

    pub async fn delete_grant(self, grant_id: &str) -> Result<(), String> {
        grants::delete_grant(grant_id.to_string())
            .await
            .map_err(err_msg)?;
        self.refresh().await
    }

    // --- messages (per-case chat) ------------------------------------------

    /// All chat messages for a given case currently in the cache, in send order.
    pub fn messages_for_case(&self, case_id: &str) -> Vec<Message> {
        self.messages
            .get()
            .into_iter()
            .filter(|m| m.case_id == case_id)
            .collect()
    }

    /// Fetch the most recent `limit` messages for a case into the cache and
    /// return the total number of messages in the thread (so the chat UI can
    /// decide whether to offer "Load more" for earlier messages).
    pub async fn load_messages(self, case_id: &str, limit: i64) -> Result<i64, String> {
        let page = cases::list_messages_page(case_id.to_string(), limit)
            .await
            .map_err(err_msg)?;
        // Replace this case's messages, keep other cases' cached messages.
        let case_id_owned = case_id.to_string();
        self.messages.update(|all| {
            all.retain(|m| m.case_id != case_id_owned);
            all.extend(page.items);
        });
        Ok(page.total)
    }

    /// Post a message to a case's chat as the signed-in user.
    pub async fn send_case_message(self, case_id: &str, body: &str) -> Result<(), String> {
        let msg = cases::send_message(case_id.to_string(), body.trim().to_string())
            .await
            .map_err(err_msg)?;
        self.messages.update(|all| all.push(msg));
        // Keep the lightweight directory count (shown as the inbox badge) in sync
        // without reloading every message body.
        self.cases.update(|cases| {
            if let Some(c) = cases.iter_mut().find(|c| c.id == case_id) {
                c.message_count += 1;
            }
        });
        Ok(())
    }

    // --- users / admin ------------------------------------------------------

    pub async fn set_user_role(self, user_id: &str, role: AccountRole) -> Result<(), String> {
        users::set_user_role(user_id.to_string(), role)
            .await
            .map_err(err_msg)?;
        self.refresh_keeping_session(user_id).await
    }

    pub async fn assign_user_to_case(
        self,
        user_id: &str,
        case_id: &str,
        capabilities: Vec<CaseCapability>,
    ) -> Result<(), String> {
        users::assign_case(user_id.to_string(), case_id.to_string(), capabilities)
            .await
            .map_err(err_msg)?;
        self.refresh_keeping_session(user_id).await
    }

    pub async fn toggle_case_capability(
        self,
        user_id: &str,
        case_id: &str,
        cap: CaseCapability,
        enabled: bool,
    ) -> Result<(), String> {
        users::toggle_capability(user_id.to_string(), case_id.to_string(), cap, enabled)
            .await
            .map_err(err_msg)?;
        self.refresh_keeping_session(user_id).await
    }

    pub async fn unassign_user_from_case(self, user_id: &str, case_id: &str) -> Result<(), String> {
        users::unassign_case(user_id.to_string(), case_id.to_string())
            .await
            .map_err(err_msg)?;
        self.refresh_keeping_session(user_id).await
    }

    /// Persist a batch of per-case permission changes for a user in one go, then
    /// refresh the caches a single time. Each change is either a new capability
    /// set for a case (`Some`) or a removal of the assignment (`None`). Backs the
    /// admin "Edit → Save" flow so many edits apply atomically from the UI's
    /// perspective rather than one server round-trip per checkbox.
    pub async fn save_case_permissions(
        self,
        user_id: &str,
        changes: Vec<(String, Option<Vec<CaseCapability>>)>,
    ) -> Result<(), String> {
        for (case_id, caps) in changes {
            match caps {
                Some(caps) => {
                    users::assign_case(user_id.to_string(), case_id, caps)
                        .await
                        .map_err(err_msg)?;
                }
                None => {
                    users::unassign_case(user_id.to_string(), case_id)
                        .await
                        .map_err(err_msg)?;
                }
            }
        }
        self.refresh_keeping_session(user_id).await
    }

    /// Refresh caches, and if the changed user is the signed-in user, keep
    /// `current_user` in sync from the reloaded list.
    async fn refresh_keeping_session(self, changed_id: &str) -> Result<(), String> {
        self.refresh().await?;
        if let Some(cu) = self.current_user.get_untracked() {
            if cu.id == changed_id {
                if let Some(updated) = self
                    .users
                    .get_untracked()
                    .into_iter()
                    .find(|u| u.id == changed_id)
                {
                    self.current_user.set(Some(updated));
                }
            }
        }
        Ok(())
    }
}
