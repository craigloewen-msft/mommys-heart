//! Client-side application state for the demo: the signed-in user plus the
//! local volunteer/case store. All data lives in reactive signals so the admin
//! screens can edit it live. Nothing is persisted — this is a local demo.

use leptos::prelude::*;

use crate::mockdata::{self, ORG_NAME};
use crate::taxonomy::{ServiceCategory, ServiceType};
use crate::types::{
    AuditAction, AuditEvent, Case, CaseDocument, CaseNote, CaseStatus, Client,
    DocumentClassification, EvidenceItem, EvidenceType, KnowledgeItem, NeedCategory, Permission,
    ReviewStatus, Role, TimelineEvent, TimelineKind, User, Volunteer, VolunteerStatus,
};

/// Fields collected when adding a new piece of evidence to a case.
#[derive(Clone, Debug, Default)]
pub struct EvidenceDraft {
    pub name: String,
    pub evidence_type: EvidenceType,
    pub description: String,
    pub source: String,
    pub party: String,
    pub occurred_on: String,
    pub tags: Vec<String>,
}

/// A flattened evidence item paired with its owning case, for the cross-case
/// Evidence Repository views.
#[derive(Clone, Debug, PartialEq)]
pub struct EvidenceRow {
    pub case_id: String,
    pub case_title: String,
    pub client_name: String,
    pub item: EvidenceItem,
}

/// Shared, reactive application state. `RwSignal` is `Copy`, so the whole
/// struct is cheap to copy and can be pulled from context anywhere.
#[derive(Clone, Copy)]
pub struct AppState {
    pub current_user: RwSignal<Option<User>>,
    pub users: RwSignal<Vec<User>>,
    pub volunteers: RwSignal<Vec<Volunteer>>,
    pub clients: RwSignal<Vec<Client>>,
    pub cases: RwSignal<Vec<Case>>,
    /// Append-only audit trail (demo, in-memory).
    pub audit_log: RwSignal<Vec<AuditEvent>>,
    /// Organization-owned institutional knowledge.
    pub knowledge: RwSignal<Vec<KnowledgeItem>>,
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
            audit_log: RwSignal::new(mockdata::audit_log()),
            knowledge: RwSignal::new(mockdata::knowledge()),
            seq: RwSignal::new(2000),
        }
    }

    /// Monotonic id source for newly created records.
    fn next_seq(&self) -> u32 {
        let next = self.seq.get_untracked() + 1;
        self.seq.set(next);
        next
    }

    // --- audit trail --------------------------------------------------------

    /// Append an entry to the audit trail, attributed to the signed-in user
    /// (or "system" when no one is signed in, e.g. registration).
    pub fn record(&self, action: AuditAction, target: impl Into<String>) {
        let (actor, actor_role) = match self.current_user.get_untracked() {
            Some(u) => (u.name, u.role),
            None => ("system".to_string(), Role::ReadOnly),
        };
        let seq = self.next_seq();
        let event = AuditEvent {
            id: format!("a-{seq}"),
            actor,
            actor_role,
            action,
            target: target.into(),
            at: "just now".into(),
        };
        // Newest first.
        self.audit_log.update(|log| log.insert(0, event));
    }

    // --- authorization (RBAC) ----------------------------------------------

    /// The permission set granted to the current user, if any.
    pub fn can(&self, perm: Permission) -> bool {
        self.role().map(|r| r.can(perm)).unwrap_or(false)
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
                self.record(AuditAction::Login, user.email.clone());
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
            hours_logged: 0.0,
            weekly_availability_hours: 0.0,
            client_contacts: 0,
            trainings: Vec::new(),
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
        self.record(AuditAction::Register, user.email.clone());
        Ok(user)
    }

    pub fn logout(&self) {
        if let Some(u) = self.current_user.get_untracked() {
            self.record(AuditAction::Logout, u.email);
        }
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
            name: name.clone(),
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
            hours_logged: 0.0,
            weekly_availability_hours: 0.0,
            client_contacts: 0,
            trainings: Vec::new(),
        };
        self.volunteers.update(|v| v.push(volunteer));
        self.record(AuditAction::AddVolunteer, name);
        Ok(())
    }

    pub fn set_volunteer_status(&self, id: &str, status: VolunteerStatus) {
        let mut label = String::new();
        self.volunteers.update(|list| {
            if let Some(v) = list.iter_mut().find(|v| v.id == id) {
                v.status = status;
                label = v.name.clone();
            }
        });
        self.record(
            AuditAction::UpdateVolunteerStatus,
            format!("{label} → {}", status.label()),
        );
    }

    /// Change a user's role / authorization level (RBAC administration).
    pub fn set_user_role(&self, user_id: &str, role: Role) {
        let mut label = String::new();
        self.users.update(|list| {
            if let Some(u) = list.iter_mut().find(|u| u.id == user_id) {
                u.role = role;
                label = u.name.clone();
            }
        });
        // Keep the signed-in user's cached copy in sync if they changed self.
        self.current_user.update(|cur| {
            if let Some(u) = cur.as_mut() {
                if u.id == user_id {
                    u.role = role;
                }
            }
        });
        self.record(
            AuditAction::ChangeUserRole,
            format!("{label} → {}", role.label()),
        );
    }

    /// Offboard a volunteer: mark them inactive and reassign their case
    /// stewardship to the organization so institutional knowledge and case
    /// context stay with Mommy's Heart rather than leaving with the person.
    pub fn offboard_volunteer(&self, volunteer_id: &str) {
        let mut name = String::new();
        self.volunteers.update(|list| {
            if let Some(v) = list.iter_mut().find(|v| v.id == volunteer_id) {
                v.status = VolunteerStatus::Inactive;
                name = v.name.clone();
            }
        });
        // Reassign stewardship of any case this volunteer stewarded to the org.
        self.cases.update(|list| {
            for c in list.iter_mut() {
                if c.steward == name {
                    c.steward = ORG_NAME.to_string();
                }
            }
        });
        self.record(AuditAction::OffboardVolunteer, name);
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

    /// Look up a single case by id.
    pub fn case(&self, id: &str) -> Option<Case> {
        self.cases.get().into_iter().find(|c| c.id == id)
    }

    /// Record that the signed-in user viewed a case (audit trail).
    pub fn view_case(&self, id: &str) {
        if let Some(c) = self.case(id) {
            self.record(AuditAction::ViewCase, c.title);
        }
    }

    pub fn set_case_status(&self, id: &str, status: CaseStatus) {
        let event = self.new_event(
            TimelineKind::StatusChanged,
            format!("Status set to {}", status.label()),
        );
        let mut label = String::new();
        self.cases.update(|list| {
            if let Some(c) = list.iter_mut().find(|c| c.id == id) {
                if c.status == status {
                    return;
                }
                c.status = status;
                c.timeline.push(event);
                label = c.title.clone();
            }
        });
        self.record(
            AuditAction::UpdateCaseStatus,
            format!("{label} → {}", status.label()),
        );
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
        let mut label = String::new();
        self.cases.update(|list| {
            if let Some(c) = list.iter_mut().find(|c| c.id == case_id) {
                label = c.title.clone();
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
        self.record(AuditAction::AssignVolunteer, label);
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

    /// Toggle a legal hold on a case (records-retention administration).
    pub fn set_legal_hold(&self, case_id: &str, hold: bool) {
        let mut label = String::new();
        self.cases.update(|list| {
            if let Some(c) = list.iter_mut().find(|c| c.id == case_id) {
                c.legal_hold = hold;
                label = c.title.clone();
            }
        });
        self.record(
            AuditAction::LegalHoldChange,
            format!("{label} → {}", if hold { "on hold" } else { "released" }),
        );
    }

    /// Whether a case is eligible for disposal: closed, not permanently
    /// retained, and not under legal hold.
    pub fn is_disposal_eligible(&self, case: &Case) -> bool {
        case.status == CaseStatus::Closed
            && case.retention != crate::types::RetentionClass::Permanent
            && !case.legal_hold
    }

    /// Dispose (delete) a case if retention policy permits it.
    pub fn dispose_case(&self, case_id: &str) -> Result<(), String> {
        let case = self.case(case_id).ok_or("Case not found.")?;
        if !self.is_disposal_eligible(&case) {
            return Err("Record is not eligible for disposal.".into());
        }
        self.cases.update(|list| list.retain(|c| c.id != case_id));
        self.record(AuditAction::DisposeRecord, case.title);
        Ok(())
    }

    pub fn add_case_document(
        &self,
        case_id: &str,
        name: &str,
        classification: DocumentClassification,
    ) {
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
                    name: name.clone(),
                    uploaded_at: "just now".into(),
                    classification,
                });
                c.timeline.push(event);
            }
        });
        self.record(
            AuditAction::UploadDocument,
            format!("{name} ({})", classification.label()),
        );
    }

    /// Whether the current user may open a document of the given
    /// classification. Sensitive documents require `ViewConfidentialDocs`.
    pub fn can_view_document(&self, classification: DocumentClassification) -> bool {
        if classification.is_sensitive() {
            self.can(Permission::ViewConfidentialDocs)
        } else {
            self.is_authenticated()
        }
    }

    /// Attempt to open a document, recording the access (or denial) in the
    /// audit trail. Returns whether access was granted.
    pub fn access_document(&self, doc: &CaseDocument) -> bool {
        let granted = self.can_view_document(doc.classification);
        let target = format!("{} ({})", doc.name, doc.classification.label());
        if granted {
            self.record(AuditAction::AccessDocument, target);
        } else {
            self.record(AuditAction::DeniedAccess, target);
        }
        granted
    }

    pub fn add_case_evidence(&self, case_id: &str, draft: EvidenceDraft) {
        let name = draft.name.trim().to_string();
        if name.is_empty() {
            return;
        }
        let tags: Vec<String> = draft
            .tags
            .into_iter()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();
        let seq = self.next_seq();
        let event =
            self.new_event(TimelineKind::DocumentAdded, format!("Added evidence \"{name}\""));
        self.cases.update(|list| {
            if let Some(c) = list.iter_mut().find(|c| c.id == case_id) {
                c.evidence.push(EvidenceItem {
                    id: format!("e-{seq}"),
                    name,
                    evidence_type: draft.evidence_type,
                    description: draft.description.trim().to_string(),
                    source: draft.source.trim().to_string(),
                    party: draft.party.trim().to_string(),
                    occurred_on: draft.occurred_on.trim().to_string(),
                    tags,
                    review_status: ReviewStatus::Unreviewed,
                    uploaded_at: "just now".into(),
                });
                c.timeline.push(event);
            }
        });
    }

    pub fn set_evidence_review_status(
        &self,
        case_id: &str,
        evidence_id: &str,
        status: ReviewStatus,
    ) {
        self.cases.update(|list| {
            if let Some(c) = list.iter_mut().find(|c| c.id == case_id) {
                if let Some(e) = c.evidence.iter_mut().find(|e| e.id == evidence_id) {
                    e.review_status = status;
                }
            }
        });
    }

    pub fn remove_evidence(&self, case_id: &str, evidence_id: &str) {
        self.cases.update(|list| {
            if let Some(c) = list.iter_mut().find(|c| c.id == case_id) {
                c.evidence.retain(|e| e.id != evidence_id);
            }
        });
    }

    /// Every evidence item across all cases, paired with its owning case, for
    /// the cross-case Evidence Repository.
    pub fn all_evidence(&self) -> Vec<EvidenceRow> {
        self.cases
            .get()
            .into_iter()
            .flat_map(|c| {
                let case_id = c.id.clone();
                let case_title = c.title.clone();
                let client_name = self.client_name(&c.client_id);
                c.evidence.into_iter().map(move |item| EvidenceRow {
                    case_id: case_id.clone(),
                    case_title: case_title.clone(),
                    client_name: client_name.clone(),
                    item,
                })
            })
            .collect()
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
        service_types: Vec<ServiceType>,
        summary: &str,
        matter_type: crate::types::MatterType,
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
        let creator = self
            .current_user
            .get_untracked()
            .map(|u| u.name)
            .unwrap_or_else(|| "system".into());
        let seq = self.next_seq();
        let opened = self.new_event(TimelineKind::Opened, "Case opened".into());
        let case = Case {
            id: format!("c-{seq}"),
            title: title.clone(),
            client_id: client_id.to_string(),
            category,
            service_types,
            summary: summary.trim().to_string(),
            status: CaseStatus::Open,
            priority: crate::types::CasePriority::Medium,
            assigned_volunteer_ids: Vec::new(),
            related_case_ids: Vec::new(),
            notes: Vec::new(),
            documents: Vec::new(),
            evidence: Vec::new(),
            timeline: vec![opened],
            opened_at: "just now".into(),
            matter_type,
            outcome: crate::types::CaseOutcome::Ongoing,
            intake_date: "just now".into(),
            resolved_date: None,
            referrals: Vec::new(),
            services: Vec::new(),
            follow_ups: Vec::new(),
            created_by: creator.clone(),
            // New cases are owned by the organization; the creator stewards them
            // until reassigned.
            steward: creator,
            retention: crate::types::RetentionClass::Standard,
            legal_hold: false,
        };
        self.cases.update(|list| list.push(case));
        self.record(AuditAction::CreateCase, title);
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

    /// Display name for the client that owns a case (falls back gracefully).
    pub fn client_name_for(&self, case: &Case) -> String {
        self.client(&case.client_id)
            .map(|c| c.display_name)
            .unwrap_or_else(|| "Unknown client".into())
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

    /// Add or remove a taxonomy service type tag on a case.
    pub fn toggle_case_service_type(&self, case_id: &str, service: ServiceType) {
        self.cases.update(|list| {
            if let Some(c) = list.iter_mut().find(|c| c.id == case_id) {
                if let Some(pos) = c.service_types.iter().position(|s| *s == service) {
                    c.service_types.remove(pos);
                } else {
                    c.service_types.push(service);
                }
            }
        });
    }

    // --- analytics (service pathways & gaps) --------------------------------

    /// Number of (open) cases touching each service type, in taxonomy order.
    /// When `open_only` is true, closed cases are excluded.
    pub fn service_type_counts(&self, open_only: bool) -> Vec<(ServiceType, usize)> {
        let cases = self.cases.get();
        ServiceType::ALL
            .into_iter()
            .map(|st| {
                let count = cases
                    .iter()
                    .filter(|c| !open_only || c.status != crate::types::CaseStatus::Closed)
                    .filter(|c| c.service_types.contains(&st))
                    .count();
                (st, count)
            })
            .collect()
    }

    /// Number of cases touching each service category, in taxonomy order.
    pub fn category_counts(&self) -> Vec<(ServiceCategory, usize)> {
        let cases = self.cases.get();
        ServiceCategory::ALL
            .into_iter()
            .map(|cat| {
                let count = cases
                    .iter()
                    .filter(|c| c.service_types.iter().any(|s| s.category() == cat))
                    .count();
                (cat, count)
            })
            .collect()
    }

    /// The set of service categories a client currently has needs in.
    pub fn categories_for_client(&self, client_id: &str) -> Vec<ServiceCategory> {
        let mut cats: Vec<ServiceCategory> = Vec::new();
        for case in self.cases_for_client(client_id) {
            for st in case.service_types {
                let cat = st.category();
                if !cats.contains(&cat) {
                    cats.push(cat);
                }
            }
        }
        ServiceCategory::ALL
            .into_iter()
            .filter(|c| cats.contains(c))
            .collect()
    }

    /// Referral pathways inferred from co-occurrence: how many clients have
    /// needs in both categories of each pair, most common first.
    pub fn category_cooccurrence(&self) -> Vec<(ServiceCategory, ServiceCategory, usize)> {
        let clients = self.clients.get();
        let mut pairs: Vec<(ServiceCategory, ServiceCategory, usize)> = Vec::new();
        let cats = ServiceCategory::ALL;
        for i in 0..cats.len() {
            for j in (i + 1)..cats.len() {
                let count = clients
                    .iter()
                    .filter(|cl| {
                        let client_cats = self.categories_for_client(&cl.id);
                        client_cats.contains(&cats[i]) && client_cats.contains(&cats[j])
                    })
                    .count();
                if count > 0 {
                    pairs.push((cats[i], cats[j], count));
                }
            }
        }
        pairs.sort_by_key(|p| std::cmp::Reverse(p.2));
        pairs
    }

    /// Service gaps: per category, the count of open needs that are unassigned
    /// or on hold — a rough signal of where capacity is missing.
    pub fn service_gaps(&self) -> Vec<(ServiceCategory, usize)> {
        use crate::types::CaseStatus;
        let cases = self.cases.get();
        ServiceCategory::ALL
            .into_iter()
            .map(|cat| {
                let count = cases
                    .iter()
                    .filter(|c| c.service_types.iter().any(|s| s.category() == cat))
                    .filter(|c| {
                        c.assigned_volunteer_ids.is_empty() || c.status == CaseStatus::OnHold
                    })
                    .filter(|c| c.status != CaseStatus::Closed)
                    .count();
                (cat, count)
            })
            .filter(|(_, count)| *count > 0)
            .collect()
    }

    // --- institutional knowledge -------------------------------------------

    /// Add an institutional-knowledge entry, owned by the organization.
    pub fn add_knowledge(
        &self,
        title: &str,
        category: crate::types::KnowledgeCategory,
        summary: &str,
    ) -> Result<(), String> {
        let title = title.trim().to_string();
        if title.is_empty() {
            return Err("A title is required.".into());
        }
        let contributor = self
            .current_user
            .get_untracked()
            .map(|u| u.name)
            .unwrap_or_else(|| "system".into());
        let seq = self.next_seq();
        let item = KnowledgeItem {
            id: format!("k-{seq}"),
            title: title.clone(),
            category,
            summary: summary.trim().to_string(),
            contributed_by: contributor,
            updated_at: "just now".into(),
        };
        self.knowledge.update(|list| list.push(item));
        self.record(AuditAction::EditKnowledge, title);
        Ok(())
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
