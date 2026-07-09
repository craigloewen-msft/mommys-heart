//! Client-side application state for the demo: the signed-in user plus the
//! local volunteer/case store. All data lives in reactive signals so the admin
//! screens can edit it live. Nothing is persisted — this is a local demo.

use leptos::prelude::*;

use crate::mockdata;
use crate::types::{
    Case, CaseDocument, CaseNote, Client, NeedCategory, Role, TimelineEvent, TimelineKind, User,
    Volunteer, VolunteerStatus,
};

/// Shared, reactive application state. `RwSignal` is `Copy`, so the whole
/// struct is cheap to copy and can be pulled from context anywhere.
#[derive(Clone, Copy)]
pub struct AppState {
    pub current_user: RwSignal<Option<User>>,
    pub users: RwSignal<Vec<User>>,
    pub volunteers: RwSignal<Vec<Volunteer>>,
    pub clients: RwSignal<Vec<Client>>,
    pub cases: RwSignal<Vec<Case>>,
    seq: RwSignal<u32>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            current_user: RwSignal::new(None),
            users: RwSignal::new(mockdata::users()),
            volunteers: RwSignal::new(mockdata::volunteers()),
            clients: RwSignal::new(mockdata::clients()),
            cases: RwSignal::new(mockdata::cases()),
            seq: RwSignal::new(2000),
        }
    }

    /// Monotonic id source for newly created records.
    fn next_seq(&self) -> u32 {
        let next = self.seq.get_untracked() + 1;
        self.seq.set(next);
        next
    }

    // --- auth ---------------------------------------------------------------

    pub fn is_authenticated(&self) -> bool {
        self.current_user.get().is_some()
    }

    pub fn role(&self) -> Option<Role> {
        self.current_user.get().map(|u| u.role)
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

    pub fn register(&self, name: &str, email: &str, password: &str) -> Result<User, String> {
        let name = name.trim().to_string();
        let email = email.trim().to_string();
        if name.is_empty() || email.is_empty() || password.is_empty() {
            return Err("Please fill in every field.".into());
        }
        let exists = self
            .users
            .get_untracked()
            .iter()
            .any(|u| u.email.to_lowercase() == email.to_lowercase());
        if exists {
            return Err("An account with that email already exists.".into());
        }

        // New self-service sign-ups become volunteers with a matching record.
        let seq = self.next_seq();
        let volunteer_id = format!("v-{seq}");
        let volunteer = Volunteer {
            id: volunteer_id.clone(),
            name: name.clone(),
            email: email.clone(),
            phone: String::new(),
            specialty: "General support".into(),
            status: VolunteerStatus::Pending,
        };
        let user = User {
            id: format!("u-{seq}"),
            name,
            email,
            password: password.to_string(),
            role: Role::Volunteer,
            volunteer_id: Some(volunteer_id),
        };
        self.volunteers.update(|v| v.push(volunteer));
        self.users.update(|u| u.push(user.clone()));
        self.current_user.set(Some(user.clone()));
        Ok(user)
    }

    pub fn logout(&self) {
        self.current_user.set(None);
    }

    // --- volunteers ---------------------------------------------------------

    pub fn add_volunteer(&self, name: &str, email: &str, specialty: &str) -> Result<(), String> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err("A name is required.".into());
        }
        let seq = self.next_seq();
        let volunteer = Volunteer {
            id: format!("v-{seq}"),
            name,
            email: email.trim().to_string(),
            phone: String::new(),
            specialty: {
                let s = specialty.trim();
                if s.is_empty() {
                    "General support".into()
                } else {
                    s.to_string()
                }
            },
            status: VolunteerStatus::Pending,
        };
        self.volunteers.update(|v| v.push(volunteer));
        Ok(())
    }

    pub fn set_volunteer_status(&self, id: &str, status: VolunteerStatus) {
        self.volunteers.update(|list| {
            if let Some(v) = list.iter_mut().find(|v| v.id == id) {
                v.status = status;
            }
        });
    }

    // --- cases --------------------------------------------------------------

    /// Build a timeline event with a fresh id and a demo "just now" timestamp.
    fn new_event(&self, kind: TimelineKind, summary: String) -> TimelineEvent {
        let seq = self.next_seq();
        TimelineEvent {
            id: format!("e-{seq}"),
            at: "just now".into(),
            kind,
            summary,
        }
    }

    /// Name of the currently signed-in user, for authoring notes/events.
    fn actor_name(&self) -> String {
        self.current_user
            .get_untracked()
            .map(|u| u.name)
            .unwrap_or_else(|| "System".into())
    }

    pub fn set_case_status(&self, id: &str, status: crate::types::CaseStatus) {
        let event = self.new_event(
            TimelineKind::StatusChanged,
            format!("Status set to {}", status.label()),
        );
        self.cases.update(|list| {
            if let Some(c) = list.iter_mut().find(|c| c.id == id) {
                if c.status == status {
                    return;
                }
                c.status = status;
                c.timeline.push(event);
            }
        });
    }

    pub fn toggle_case_volunteer(&self, case_id: &str, volunteer_id: &str) {
        let volunteer_name = self
            .volunteers
            .get_untracked()
            .into_iter()
            .find(|v| v.id == volunteer_id)
            .map(|v| v.name)
            .unwrap_or_else(|| volunteer_id.to_string());
        let assign_event = self.new_event(
            TimelineKind::VolunteerAssigned,
            format!("Assigned {volunteer_name}"),
        );
        let unassign_event = self.new_event(
            TimelineKind::VolunteerUnassigned,
            format!("Unassigned {volunteer_name}"),
        );
        self.cases.update(|list| {
            if let Some(c) = list.iter_mut().find(|c| c.id == case_id) {
                if let Some(pos) = c
                    .assigned_volunteer_ids
                    .iter()
                    .position(|v| v == volunteer_id)
                {
                    c.assigned_volunteer_ids.remove(pos);
                    c.timeline.push(unassign_event);
                } else {
                    c.assigned_volunteer_ids.push(volunteer_id.to_string());
                    c.timeline.push(assign_event);
                }
            }
        });
    }

    /// Toggle a two-way cross-link between two of a client's cases so their
    /// interconnected needs can be navigated together.
    pub fn toggle_related_case(&self, case_id: &str, other_id: &str) {
        if case_id == other_id {
            return;
        }
        self.cases.update(|list| {
            let currently_linked = list
                .iter()
                .find(|c| c.id == case_id)
                .map(|c| c.related_case_ids.iter().any(|r| r == other_id))
                .unwrap_or(false);
            for c in list.iter_mut() {
                if c.id == case_id {
                    toggle_link(&mut c.related_case_ids, other_id, currently_linked);
                } else if c.id == other_id {
                    toggle_link(&mut c.related_case_ids, case_id, currently_linked);
                }
            }
        });
    }

    pub fn add_case_document(&self, case_id: &str, name: &str) {
        let name = name.trim().to_string();
        if name.is_empty() {
            return;
        }
        let seq = self.next_seq();
        let event = self.new_event(TimelineKind::DocumentAdded, format!("Added \"{name}\""));
        self.cases.update(|list| {
            if let Some(c) = list.iter_mut().find(|c| c.id == case_id) {
                c.documents.push(CaseDocument {
                    id: format!("d-{seq}"),
                    name,
                    uploaded_at: "just now".into(),
                });
                c.timeline.push(event);
            }
        });
    }

    pub fn add_case_note(&self, case_id: &str, body: &str) {
        let body = body.trim().to_string();
        if body.is_empty() {
            return;
        }
        let author = self.actor_name();
        let seq = self.next_seq();
        let event = self.new_event(TimelineKind::NoteAdded, "Note added".into());
        self.cases.update(|list| {
            if let Some(c) = list.iter_mut().find(|c| c.id == case_id) {
                c.notes.push(CaseNote {
                    id: format!("n-{seq}"),
                    author,
                    body,
                    created_at: "just now".into(),
                });
                c.timeline.push(event);
            }
        });
    }

    pub fn add_case(
        &self,
        client_id: &str,
        title: &str,
        category: NeedCategory,
        summary: &str,
    ) -> Result<(), String> {
        let title = title.trim().to_string();
        if title.is_empty() {
            return Err("A case title is required.".into());
        }
        if !self
            .clients
            .get_untracked()
            .iter()
            .any(|c| c.id == client_id)
        {
            return Err("Please choose a client for this case.".into());
        }
        let seq = self.next_seq();
        let opened = self.new_event(TimelineKind::Opened, "Case opened".into());
        let case = Case {
            id: format!("c-{seq}"),
            title,
            client_id: client_id.to_string(),
            category,
            summary: summary.trim().to_string(),
            status: crate::types::CaseStatus::Open,
            priority: crate::types::CasePriority::Medium,
            assigned_volunteer_ids: Vec::new(),
            related_case_ids: Vec::new(),
            notes: Vec::new(),
            documents: Vec::new(),
            timeline: vec![opened],
            opened_at: "just now".into(),
        };
        self.cases.update(|list| list.push(case));
        Ok(())
    }

    pub fn add_client(
        &self,
        display_name: &str,
        email: &str,
        phone: &str,
        summary: &str,
    ) -> Result<String, String> {
        let display_name = display_name.trim().to_string();
        if display_name.is_empty() {
            return Err("A client name or reference is required.".into());
        }
        let seq = self.next_seq();
        let id = format!("cl-{seq}");
        let client = Client {
            id: id.clone(),
            display_name,
            phone: phone.trim().to_string(),
            email: email.trim().to_string(),
            intake_date: "just now".into(),
            summary: summary.trim().to_string(),
        };
        self.clients.update(|list| list.push(client));
        Ok(id)
    }

    /// Look up a single client by id.
    pub fn client(&self, id: &str) -> Option<Client> {
        self.clients.get().into_iter().find(|c| c.id == id)
    }

    /// Display name for a client id, falling back to the raw id.
    pub fn client_name(&self, id: &str) -> String {
        self.client(id)
            .map(|c| c.display_name)
            .unwrap_or_else(|| id.to_string())
    }

    /// All cases belonging to a given client.
    pub fn cases_for_client(&self, client_id: &str) -> Vec<Case> {
        self.cases
            .get()
            .into_iter()
            .filter(|c| c.client_id == client_id)
            .collect()
    }

    /// Cases assigned to a given volunteer id.
    pub fn cases_for_volunteer(&self, volunteer_id: &str) -> Vec<Case> {
        self.cases
            .get()
            .into_iter()
            .filter(|c| c.assigned_volunteer_ids.iter().any(|v| v == volunteer_id))
            .collect()
    }
}

/// Add or remove `target` from a case's related-case list based on the current
/// linked state (kept as a free function so the borrow checker is happy inside
/// the `update` closure).
fn toggle_link(list: &mut Vec<String>, target: &str, currently_linked: bool) {
    if currently_linked {
        list.retain(|r| r != target);
    } else if !list.iter().any(|r| r == target) {
        list.push(target.to_string());
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
