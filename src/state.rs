//! Client-side application state for the demo: the signed-in user plus the
//! local volunteer/case store. All data lives in reactive signals so the admin
//! screens can edit it live. Nothing is persisted — this is a local demo.

use leptos::prelude::*;

use crate::mockdata;
use crate::types::{Case, CaseDocument, Role, User, Volunteer, VolunteerStatus};

/// Shared, reactive application state. `RwSignal` is `Copy`, so the whole
/// struct is cheap to copy and can be pulled from context anywhere.
#[derive(Clone, Copy)]
pub struct AppState {
    pub current_user: RwSignal<Option<User>>,
    pub users: RwSignal<Vec<User>>,
    pub volunteers: RwSignal<Vec<Volunteer>>,
    pub cases: RwSignal<Vec<Case>>,
    seq: RwSignal<u32>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            current_user: RwSignal::new(None),
            users: RwSignal::new(mockdata::users()),
            volunteers: RwSignal::new(mockdata::volunteers()),
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

    pub fn register(
        &self,
        name: &str,
        email: &str,
        password: &str,
    ) -> Result<User, String> {
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

    pub fn set_case_status(&self, id: &str, status: crate::types::CaseStatus) {
        self.cases.update(|list| {
            if let Some(c) = list.iter_mut().find(|c| c.id == id) {
                c.status = status;
            }
        });
    }

    pub fn toggle_case_volunteer(&self, case_id: &str, volunteer_id: &str) {
        self.cases.update(|list| {
            if let Some(c) = list.iter_mut().find(|c| c.id == case_id) {
                if let Some(pos) = c
                    .assigned_volunteer_ids
                    .iter()
                    .position(|v| v == volunteer_id)
                {
                    c.assigned_volunteer_ids.remove(pos);
                } else {
                    c.assigned_volunteer_ids.push(volunteer_id.to_string());
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
        self.cases.update(|list| {
            if let Some(c) = list.iter_mut().find(|c| c.id == case_id) {
                c.documents.push(CaseDocument {
                    id: format!("d-{seq}"),
                    name,
                    uploaded_at: "just now".into(),
                });
            }
        });
    }

    pub fn add_case(&self, title: &str, client_name: &str, summary: &str) -> Result<(), String> {
        let title = title.trim().to_string();
        if title.is_empty() {
            return Err("A case title is required.".into());
        }
        let seq = self.next_seq();
        let case = Case {
            id: format!("c-{seq}"),
            title,
            client_name: {
                let c = client_name.trim();
                if c.is_empty() {
                    "Client (confidential)".into()
                } else {
                    c.to_string()
                }
            },
            summary: summary.trim().to_string(),
            status: crate::types::CaseStatus::Open,
            priority: crate::types::CasePriority::Medium,
            assigned_volunteer_ids: Vec::new(),
            documents: Vec::new(),
            opened_at: "just now".into(),
        };
        self.cases.update(|list| list.push(case));
        Ok(())
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

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
