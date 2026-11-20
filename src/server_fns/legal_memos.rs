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
