//! Client-side application state for the V1 app: the signed-in user plus the
//! local in-memory stores (users, cases, grants, messages). All data lives in
//! reactive signals so screens can edit it live. Nothing is persisted — this is
//! a mock until a real backend is wired in.

use leptos::prelude::*;

use crate::mockdata;
use crate::types::{
    AccountRole, Case, CaseAssignment, CaseCapability, CaseNote, CaseProperty, CaseStatus,
    ChangeLogEntry, Evidence, Grant, Message, User,
};

/// Current local date-time as `YYYY-MM-DD HH:MM`, read from the browser clock.
/// On the server build (where user mutations never run) it returns a fixed
/// placeholder so the same code compiles for both targets.
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

/// Shared, reactive application state. `RwSignal` is `Copy`, so the whole
/// struct is cheap to copy and can be pulled from context anywhere.
#[derive(Clone, Copy)]
pub struct AppState {
    pub current_user: RwSignal<Option<User>>,
    pub users: RwSignal<Vec<User>>,
    pub cases: RwSignal<Vec<Case>>,
    pub grants: RwSignal<Vec<Grant>>,
    pub messages: RwSignal<Vec<Message>>,
    seq: RwSignal<u32>,
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
            users: RwSignal::new(mockdata::users()),
            cases: RwSignal::new(mockdata::cases()),
            grants: RwSignal::new(mockdata::grants()),
            messages: RwSignal::new(mockdata::messages()),
            seq: RwSignal::new(5000),
        }
    }

    /// Monotonic id source for newly created records.
    fn next_seq(&self) -> u32 {
        let next = self.seq.get_untracked() + 1;
        self.seq.set(next);
        next
    }

    /// Display name of the signed-in user (or "system").
    fn actor_name(&self) -> String {
        self.current_user
            .get_untracked()
            .map(|u| u.full_name())
            .unwrap_or_else(|| "system".into())
    }

    fn change_entry(&self, field: &str, old_value: &str, new_value: &str) -> ChangeLogEntry {
        let seq = self.next_seq();
        ChangeLogEntry {
            id: format!("cl-{seq}"),
            actor: self.actor_name(),
            field: field.into(),
            old_value: old_value.into(),
            new_value: new_value.into(),
            at: now_stamp(),
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

    pub fn login(&self, email: &str, password: &str) -> Result<User, String> {
        let email = email.trim().to_lowercase();
        let found = self
            .users
            .get_untracked()
            .into_iter()
            .find(|u| u.email.to_lowercase() == email && u.password == password);
        match found {
            Some(user) => {
                self.current_user.set(Some(user.clone()));
                Ok(user)
            }
            None => Err("Invalid email or password.".into()),
        }
    }

    pub fn register(
        &self,
        first_name: &str,
        last_name: &str,
        email: &str,
        password: &str,
    ) -> Result<User, String> {
        let first_name = first_name.trim().to_string();
        let last_name = last_name.trim().to_string();
        let email = email.trim().to_string();
        if first_name.is_empty() || email.is_empty() || password.is_empty() {
            return Err("Please fill in first name, email, and password.".into());
        }
        let exists = self
            .users
            .get_untracked()
            .iter()
            .any(|u| u.email.to_lowercase() == email.to_lowercase());
        if exists {
            return Err("An account with that email already exists.".into());
        }

        // Self-service sign-ups become clients by default; admins promote later.
        let seq = self.next_seq();
        let user = User {
            id: format!("u-{seq}"),
            first_name,
            last_name,
            email,
            phone: String::new(),
            home_address: String::new(),
            password: password.to_string(),
            role: AccountRole::Client,
            assigned_cases: Vec::new(),
            audit_log: Vec::new(),
        };
        self.users.update(|u| u.push(user.clone()));
        self.current_user.set(Some(user.clone()));
        Ok(user)
    }

    pub fn logout(&self) {
        self.current_user.set(None);
    }

    // --- cases --------------------------------------------------------------

    /// Cases the signed-in user can see: admins see all; everyone else sees
    /// only the cases they are assigned to (or own).
    pub fn visible_cases(&self) -> Vec<Case> {
        let all = self.cases.get();
        match self.current_user.get() {
            Some(u) if u.role.is_admin() => all,
            Some(u) => all
                .into_iter()
                .filter(|c| c.owner_id == u.id || u.is_assigned_to(&c.id))
                .collect(),
            None => Vec::new(),
        }
    }

    /// The signed-in user's capabilities on a case. Admins and the case owner
    /// implicitly hold every capability; everyone else holds exactly the set
    /// granted by their assignment.
    pub fn capabilities_on(&self, case: &Case) -> Vec<CaseCapability> {
        match self.current_user.get() {
            Some(u) if u.role.is_admin() || case.owner_id == u.id => CaseCapability::ALL.to_vec(),
            Some(u) => u.capabilities_for(&case.id),
            None => Vec::new(),
        }
    }

    /// Whether the signed-in user holds a specific capability on a case.
    pub fn case_can(&self, case: &Case, cap: CaseCapability) -> bool {
        self.capabilities_on(case).contains(&cap)
    }

    /// Create a new case owned by the signed-in user, with an initial status,
    /// key properties, and an optional first note. Returns the new case id.
    pub fn add_case_full(
        &self,
        name: &str,
        status: CaseStatus,
        properties: Vec<(String, String)>,
        first_note: Option<String>,
    ) -> Result<String, String> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err("Case name is required.".into());
        }
        let owner = self
            .current_user
            .get_untracked()
            .ok_or("You must be signed in.")?;
        let seq = self.next_seq();
        let id = format!("c-{seq}");

        let properties: Vec<CaseProperty> = properties
            .into_iter()
            .filter_map(|(k, v)| {
                let key = k.trim().to_string();
                let value = v.trim().to_string();
                if key.is_empty() || value.is_empty() {
                    None
                } else {
                    Some(CaseProperty { key, value })
                }
            })
            .collect();

        let mut notes = Vec::new();
        if let Some(body) = first_note {
            let body = body.trim().to_string();
            if !body.is_empty() {
                let nseq = self.next_seq();
                notes.push(CaseNote {
                    id: format!("n-{nseq}"),
                    author: owner.full_name(),
                    body,
                    created_at: now_stamp(),
                });
            }
        }

        let case = Case {
            id: id.clone(),
            name,
            status,
            owner_id: owner.id.clone(),
            notes,
            evidence: Vec::new(),
            properties,
            audit_log: Vec::new(),
        };
        self.cases.update(|c| c.push(case));
        // Give the owner an explicit full-control assignment too.
        self.assign_user_to_case(&owner.id, &id, CaseCapability::ALL.to_vec());
        Ok(id)
    }

    fn with_case<F: FnOnce(&mut Case)>(&self, case_id: &str, f: F) {
        self.cases.update(|cases| {
            if let Some(case) = cases.iter_mut().find(|c| c.id == case_id) {
                f(case);
            }
        });
    }

    pub fn set_case_status(&self, case_id: &str, status: CaseStatus) {
        let entry = {
            let case = self
                .cases
                .get_untracked()
                .into_iter()
                .find(|c| c.id == case_id);
            match case {
                Some(c) if c.status != status => {
                    Some(self.change_entry("status", c.status.slug(), status.slug()))
                }
                _ => None,
            }
        };
        self.with_case(case_id, |c| {
            c.status = status;
            if let Some(e) = entry {
                c.audit_log.insert(0, e);
            }
        });
    }

    pub fn set_case_name(&self, case_id: &str, name: &str) -> Result<(), String> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err("Case name is required.".into());
        }
        let entry = {
            let case = self
                .cases
                .get_untracked()
                .into_iter()
                .find(|c| c.id == case_id);
            match case {
                Some(c) if c.name != name => Some(self.change_entry("name", &c.name, &name)),
                _ => None,
            }
        };
        self.with_case(case_id, |c| {
            c.name = name;
            if let Some(e) = entry {
                c.audit_log.insert(0, e);
            }
        });
        Ok(())
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

    /// Change who filed (owns) a case. `owner_id` must map to an existing user.
    pub fn set_case_owner(&self, case_id: &str, owner_id: &str) -> Result<(), String> {
        if !self.users.get_untracked().iter().any(|u| u.id == owner_id) {
            return Err("Unknown owner.".into());
        }
        let entry = {
            let case = self
                .cases
                .get_untracked()
                .into_iter()
                .find(|c| c.id == case_id);
            match case {
                Some(c) if c.owner_id != owner_id => Some(self.change_entry(
                    "owner",
                    &self.user_name(&c.owner_id),
                    &self.user_name(owner_id),
                )),
                _ => None,
            }
        };
        self.with_case(case_id, |c| {
            c.owner_id = owner_id.to_string();
            if let Some(e) = entry {
                c.audit_log.insert(0, e);
            }
        });
        Ok(())
    }

    /// Replace a case's whole property set (empty keys are dropped). Records a
    /// single audit entry when the set actually changes.
    pub fn replace_case_properties(&self, case_id: &str, props: Vec<(String, String)>) {
        let cleaned: Vec<CaseProperty> = props
            .into_iter()
            .filter_map(|(k, v)| {
                let key = k.trim().to_string();
                if key.is_empty() {
                    None
                } else {
                    Some(CaseProperty {
                        key,
                        value: v.trim().to_string(),
                    })
                }
            })
            .collect();
        let changed = self
            .cases
            .get_untracked()
            .into_iter()
            .find(|c| c.id == case_id)
            .map(|c| c.properties != cleaned)
            .unwrap_or(false);
        let entry = if changed {
            Some(self.change_entry("properties", "", "updated"))
        } else {
            None
        };
        self.with_case(case_id, |c| {
            c.properties = cleaned;
            if let Some(e) = entry {
                c.audit_log.insert(0, e);
            }
        });
    }

    pub fn add_case_note(&self, case_id: &str, body: &str) -> Result<(), String> {
        let body = body.trim().to_string();
        if body.is_empty() {
            return Err("Note cannot be empty.".into());
        }
        let author = self.actor_name();
        let seq = self.next_seq();
        let note = CaseNote {
            id: format!("n-{seq}"),
            author,
            body,
            created_at: now_stamp(),
        };
        self.with_case(case_id, |c| c.notes.push(note));
        Ok(())
    }

    pub fn add_case_evidence(
        &self,
        case_id: &str,
        name: &str,
        description: &str,
    ) -> Result<(), String> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err("Evidence name is required.".into());
        }
        let uploaded_by = self.actor_name();
        let seq = self.next_seq();
        let item = Evidence {
            id: format!("e-{seq}"),
            name,
            case_id: case_id.to_string(),
            uploaded_by,
            uploaded_at: now_stamp(),
            description: description.trim().to_string(),
        };
        self.with_case(case_id, |c| c.evidence.push(item));
        Ok(())
    }

    pub fn delete_case_evidence(&self, case_id: &str, evidence_id: &str) {
        self.with_case(case_id, |c| c.evidence.retain(|e| e.id != evidence_id));
    }

    // --- grants -------------------------------------------------------------

    pub fn add_grant(&self, name: &str) -> Result<(), String> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err("Grant name is required.".into());
        }
        let seq = self.next_seq();
        let grant = Grant {
            id: format!("g-{seq}"),
            name,
        };
        self.grants.update(|g| g.push(grant));
        Ok(())
    }

    pub fn rename_grant(&self, grant_id: &str, name: &str) -> Result<(), String> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err("Grant name is required.".into());
        }
        self.grants.update(|grants| {
            if let Some(g) = grants.iter_mut().find(|g| g.id == grant_id) {
                g.name = name;
            }
        });
        Ok(())
    }

    pub fn delete_grant(&self, grant_id: &str) {
        self.grants.update(|g| g.retain(|g| g.id != grant_id));
    }

    // --- messages (per-case chat) ------------------------------------------

    /// All chat messages for a given case, in send order.
    pub fn messages_for_case(&self, case_id: &str) -> Vec<Message> {
        self.messages
            .get()
            .into_iter()
            .filter(|m| m.case_id == case_id)
            .collect()
    }

    /// Post a message to a case's chat as the signed-in user.
    pub fn send_case_message(&self, case_id: &str, body: &str) -> Result<(), String> {
        let body = body.trim().to_string();
        if body.is_empty() {
            return Err("Message cannot be empty.".into());
        }
        let user = self
            .current_user
            .get_untracked()
            .ok_or("You must be signed in.")?;
        let seq = self.next_seq();
        let message = Message {
            id: format!("m-{seq}"),
            case_id: case_id.to_string(),
            author_id: user.id.clone(),
            author: user.full_name(),
            body,
            sent_at: now_stamp(),
        };
        self.messages.update(|m| m.push(message));
        Ok(())
    }

    // --- users / admin ------------------------------------------------------

    /// Change a user's global account role, recording the change on their log.
    pub fn set_user_role(&self, user_id: &str, role: AccountRole) {
        let entry = {
            let user = self
                .users
                .get_untracked()
                .into_iter()
                .find(|u| u.id == user_id);
            match user {
                Some(u) if u.role != role => {
                    Some(self.change_entry("role", u.role.slug(), role.slug()))
                }
                _ => None,
            }
        };
        self.users.update(|users| {
            if let Some(u) = users.iter_mut().find(|u| u.id == user_id) {
                u.role = role;
                if let Some(e) = entry {
                    u.audit_log.insert(0, e);
                }
            }
        });
        self.sync_current_user(user_id);
    }

    /// Grant (or replace) a user's capability set on a case.
    pub fn assign_user_to_case(
        &self,
        user_id: &str,
        case_id: &str,
        capabilities: Vec<CaseCapability>,
    ) {
        let summary = capabilities
            .iter()
            .map(|c| c.slug())
            .collect::<Vec<_>>()
            .join(", ");
        let entry = self.change_entry(&format!("case:{case_id}"), "", &summary);
        self.users.update(|users| {
            if let Some(u) = users.iter_mut().find(|u| u.id == user_id) {
                if let Some(a) = u.assigned_cases.iter_mut().find(|a| a.case_id == case_id) {
                    a.capabilities = capabilities;
                } else {
                    u.assigned_cases.push(CaseAssignment {
                        case_id: case_id.to_string(),
                        capabilities,
                    });
                }
                u.audit_log.insert(0, entry);
            }
        });
        self.sync_current_user(user_id);
    }

    /// Toggle a single capability for a user on a case (adds an assignment if
    /// none exists yet).
    pub fn toggle_case_capability(
        &self,
        user_id: &str,
        case_id: &str,
        cap: CaseCapability,
        enabled: bool,
    ) {
        let (old, new) = if enabled {
            ("", cap.slug())
        } else {
            (cap.slug(), "")
        };
        let entry = self.change_entry(&format!("case:{case_id}:{}", cap.slug()), old, new);
        self.users.update(|users| {
            if let Some(u) = users.iter_mut().find(|u| u.id == user_id) {
                if let Some(a) = u.assigned_cases.iter_mut().find(|a| a.case_id == case_id) {
                    if enabled {
                        if !a.capabilities.contains(&cap) {
                            a.capabilities.push(cap);
                        }
                    } else {
                        a.capabilities.retain(|c| *c != cap);
                    }
                } else if enabled {
                    u.assigned_cases.push(CaseAssignment {
                        case_id: case_id.to_string(),
                        capabilities: vec![cap],
                    });
                }
                u.audit_log.insert(0, entry);
            }
        });
        self.sync_current_user(user_id);
    }

    /// Remove a user's assignment to a case.
    pub fn unassign_user_from_case(&self, user_id: &str, case_id: &str) {
        let entry = self.change_entry(&format!("case:{case_id}"), "assigned", "removed");
        self.users.update(|users| {
            if let Some(u) = users.iter_mut().find(|u| u.id == user_id) {
                u.assigned_cases.retain(|a| a.case_id != case_id);
                u.audit_log.insert(0, entry);
            }
        });
        self.sync_current_user(user_id);
    }

    /// Keep `current_user` in sync when the signed-in user's record changes.
    fn sync_current_user(&self, changed_id: &str) {
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
    }
}
