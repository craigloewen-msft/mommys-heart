use serde::{Deserialize, Serialize};

use crate::server_fns::crm::coded_enum;

coded_enum!(MemoStatus {
    Draft => ("draft", "Draft"),
    InReview => ("in_review", "Ready for attorney review"),
    RevisionsRequested => ("revisions_requested", "Revisions requested"),
    Approved => ("approved", "Approved"),
});

coded_enum!(FactSource {
    ClientReports => ("client_reports", "Client reports"),
    Document => ("document", "Document"),
    ThirdParty => ("third_party", "Third party"),
    Verified => ("verified", "Verified"),
});

coded_enum!(EventStatus {
    Unconfirmed => ("unconfirmed", "Unconfirmed"),
    Confirmed => ("confirmed", "Confirmed"),
});

coded_enum!(EvidenceStatus {
    NotReceived => ("not_received", "Not received"),
    Received => ("received", "Received"),
    Reviewed => ("reviewed", "Reviewed"),
});

coded_enum!(ReviewCheck {
    FactsVerified => ("facts_verified", "Facts verified"),
    ResearchChecked => ("research_checked", "Research and subsequent treatment checked"),
    AnalysisRevised => ("analysis_revised", "Analysis revised"),
    ExternalUse => ("external_use", "Approved for any external use"),
});

coded_enum!(VersionEvent {
    Sent => ("sent", "Sent for attorney review"),
    RevisionsRequested => ("revisions_requested", "Revisions requested"),
    Approved => ("approved", "Approved"),
});

impl ReviewCheck {
    pub const REQUIRED_FOR_APPROVAL: &'static [ReviewCheck] = &[
        ReviewCheck::FactsVerified,
        ReviewCheck::ResearchChecked,
    ];
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoFact {
    pub text: String,
    pub source: Option<FactSource>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProcedureEvent {
    pub date: String,
    pub event: String,
    pub source: String,
    pub status: EventStatus,
}

impl Default for ProcedureEvent {
    fn default() -> Self {
        Self {
            date: String::new(),
            event: String::new(),
            source: String::new(),
            status: EventStatus::Unconfirmed,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EvidenceItem {
    pub item: String,
    pub source: String,
    pub status: EvidenceStatus,
    pub date_received: String,
    pub follow_up: String,
}

impl Default for EvidenceItem {
    fn default() -> Self {
        Self {
            item: String::new(),
            source: String::new(),
            status: EvidenceStatus::NotReceived,
            date_received: String::new(),
            follow_up: String::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Authority {
    pub citation: String,
    pub court_level: String,
    pub link: String,
    pub date_checked: String,
    pub treatment_checked: bool,
}

pub const ANALYSIS_HEADINGS: [&str; 5] = [
    "A. Change in circumstances",
    "B. Best interests and protective conditions",
    "C. Domestic violence evidence",
    "D. Counterarguments and weaknesses",
    "E. Procedural and service limits",
];

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NextStep {
    pub action: String,
    pub owner: String,
    pub timing: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LegalMemoContent {
    pub memo_date: String,
    pub safety_flagged: Option<bool>,
    pub safety_answered_on: String,
    pub safety_escalated_on: String,
    pub safety_escalated_to: String,
    pub questions: Vec<String>,
    pub brief_answer: String,
    pub facts: Vec<MemoFact>,
    pub procedure: Vec<ProcedureEvent>,
    pub evidence: Vec<EvidenceItem>,
    pub authorities: Vec<Authority>,
    pub analysis: [String; 5],
    pub outstanding: Vec<String>,
    pub next_steps: Vec<NextStep>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReviewMark {
    pub check: ReviewCheck,
    pub by_name: String,
    pub at: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoReview {
    pub marks: Vec<ReviewMark>,
}

impl MemoReview {
    pub fn has(&self, check: ReviewCheck) -> bool {
        self.marks.iter().any(|m| m.check == check)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LegalMemoSummary {
    pub id: String,
    pub title: String,
    pub status: MemoStatus,
    pub version: i32,
    pub author_name: String,
    pub attorney_name: String,
    pub updated_on: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LegalMemo {
    pub id: String,
    pub case_id: String,
    pub case_name: String,
    pub docket: String,
    pub title: String,
    pub status: MemoStatus,
    pub version: i32,
    pub attorney_id: String,
    pub attorney_name: String,
    pub author_id: String,
    pub author_name: String,
    pub content: LegalMemoContent,
    pub review: MemoReview,
    pub approved_by_name: String,
    pub approved_at: String,
    pub updated_at: String,
    pub revision: String,
    pub can_edit: bool,
    pub can_review: bool,
    pub can_start_new: bool,
    pub can_file: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoVersionEntry {
    pub id: String,
    pub version: i32,
    pub event: VersionEvent,
    pub actor_name: String,
    pub comment: String,
    pub at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoVersionSnapshot {
    pub version: i32,
    pub event: VersionEvent,
    pub title: String,
    pub content: LegalMemoContent,
    pub attorney_name: String,
    pub actor_name: String,
    pub at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoAttorney {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoFiledDocument {
    pub version: i32,
    pub file_name: String,
    pub relative_path: String,
    pub web_url: String,
    pub filed_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoProblem {
    pub section: String,
    pub message: String,
}

pub const MEMO_FOLDER: [&str; 2] = ["Intake", "Legal Memo"];
pub const DOCKET_KEYS: [&str; 2] = ["Index or docket number", "Docket number 1"];
pub const COURT_KEYS: [&str; 3] = ["Court location", "Court", "Court 1"];

pub const MAX_SHORT: usize = 200;
pub const MAX_LONG: usize = 10_000;
pub const MAX_ROWS: usize = 50;

pub fn validate_draft(title: &str, content: &LegalMemoContent) -> Result<(), String> {
    if title.trim().is_empty() {
        return Err("Give the memo a title.".to_string());
    }
    let short = |value: &str, label: &str| check_len(value, label, MAX_SHORT);
    let long = |value: &str, label: &str| check_len(value, label, MAX_LONG);
    short(title, "title")?;
    short(&content.memo_date, "date")?;
    short(&content.safety_answered_on, "safety answer date")?;
    short(&content.safety_escalated_on, "escalation date")?;
    short(&content.safety_escalated_to, "escalated to")?;
    long(&content.brief_answer, "brief answer")?;
    rows(content.questions.len(), "questions")?;
    rows(content.facts.len(), "facts")?;
    rows(content.procedure.len(), "procedural history")?;
    rows(content.evidence.len(), "evidence log")?;
    rows(content.authorities.len(), "authorities")?;
    rows(content.outstanding.len(), "outstanding questions")?;
    rows(content.next_steps.len(), "next steps")?;
    for q in &content.questions {
        long(q, "question")?;
    }
    for f in &content.facts {
        long(&f.text, "fact")?;
    }
    for p in &content.procedure {
        short(&p.date, "event date")?;
        long(&p.event, "event")?;
        short(&p.source, "event source")?;
    }
    for e in &content.evidence {
        long(&e.item, "evidence item")?;
        short(&e.source, "evidence source")?;
        short(&e.date_received, "date received")?;
        long(&e.follow_up, "follow-up")?;
    }
    for a in &content.authorities {
        long(&a.citation, "citation")?;
        short(&a.court_level, "court level")?;
        long(&a.link, "link")?;
        short(&a.date_checked, "date checked")?;
    }
    for a in &content.analysis {
        long(a, "analysis")?;
    }
    for q in &content.outstanding {
        long(q, "outstanding question")?;
    }
    for n in &content.next_steps {
        long(&n.action, "next step")?;
        short(&n.owner, "proposed owner")?;
        short(&n.timing, "timing")?;
    }
    Ok(())
}

pub fn problems_before_review(attorney_id: &str, content: &LegalMemoContent) -> Vec<MemoProblem> {
    let mut out = Vec::new();
    let mut push = |section: &str, message: String| {
        out.push(MemoProblem {
            section: section.to_string(),
            message,
        })
    };
    if attorney_id.trim().is_empty() {
        push("Header", "Choose the attorney this memo goes to.".into());
    }
    match content.safety_flagged {
        None => push("Safety", "Answer whether a safety concern was flagged.".into()),
        Some(true) => {
            if content.safety_escalated_on.trim().is_empty() || content.safety_escalated_to.trim().is_empty() {
                push(
                    "Safety",
                    "Add the date the safety concern was escalated and who it was escalated to.".into(),
                );
            }
        }
        Some(false) => {}
    }
    if !content.questions.iter().any(|q| !q.trim().is_empty()) {
        push("1 Questions presented", "Add at least one question presented.".into());
    }
    if content.brief_answer.trim().is_empty() {
        push("2 Brief answer", "2 Brief answer is empty. It's required.".into());
    }
    let facts: Vec<&MemoFact> = content.facts.iter().filter(|f| !f.text.trim().is_empty()).collect();
    if facts.is_empty() {
        push("3 Relevant facts", "Add at least one relevant fact.".into());
    }
    for fact in facts.iter().filter(|f| f.source.is_none()) {
        push(
            "3 Relevant facts",
            format!("3 Relevant facts: \"{}\" needs a source.", excerpt(&fact.text)),
        );
    }
    for event in &content.procedure {
        if event.status == EventStatus::Confirmed && event.source.trim().is_empty() {
            push(
                "4 Procedural history",
                format!(
                    "4 Procedural history: \"{}\" is marked Confirmed without a source document.",
                    excerpt(&event.event)
                ),
            );
        }
    }
    for item in &content.evidence {
        if item.status == EvidenceStatus::Reviewed && item.date_received.trim().is_empty() {
            push(
                "5 Evidence log",
                format!(
                    "5 Evidence log: \"{}\" is marked Reviewed but hasn't been received.",
                    excerpt(&item.item)
                ),
            );
        }
    }
    out
}

fn excerpt(text: &str) -> String {
    let text = text.trim();
    if text.chars().count() > 60 {
        format!("{}\u{2026}", text.chars().take(60).collect::<String>())
    } else {
        text.to_string()
    }
}

fn check_len(value: &str, label: &str, max: usize) -> Result<(), String> {
    if value.chars().count() > max {
        Err(format!("The {label} must be {max} characters or fewer."))
    } else {
        Ok(())
    }
}

fn rows(count: usize, label: &str) -> Result<(), String> {
    if count > MAX_ROWS {
        Err(format!("The {label} can have at most {MAX_ROWS} rows."))
    } else {
        Ok(())
    }
}
