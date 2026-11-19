use leptos::prelude::*;
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

#[server(prefix = "/api")]
pub async fn list_legal_memos(case_id: String) -> Result<Vec<LegalMemoSummary>, ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server::permissions::{require_cap, require_user};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    crate::server_fns::crm::require_staff(&user)?;
    require_cap(&user, &case_id, CaseCapability::ViewCase).await?;
    legal_memos::list(&case_id).await.map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn load_legal_memo(memo_id: String) -> Result<LegalMemo, ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    let case_id = authorize_memo_read(&user, &memo_id).await?;
    let mut memo = legal_memos::get(&memo_id)
        .await
        .map_err(memo_error)?
        .ok_or_else(|| ServerFnError::new("Legal memo not found."))?;
    let writable = case_accepts_changes(&case_id).await?;
    let adds_notes = can_add_notes(&user, &case_id).await;
    let is_author = memo.author_id == user.id;
    memo.can_edit = writable
        && is_author
        && matches!(memo.status, MemoStatus::Draft | MemoStatus::RevisionsRequested)
        && adds_notes;
    memo.can_start_new =
        writable && is_author && memo.status == MemoStatus::Approved && adds_notes;
    memo.can_file = adds_notes;
    memo.can_review = writable
        && memo.status == MemoStatus::InReview
        && is_reviewer(&user, &memo.attorney_id, &memo.author_id);
    Ok(memo)
}

#[server(prefix = "/api")]
pub async fn legal_memo_versions(memo_id: String) -> Result<Vec<MemoVersionEntry>, ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    authorize_memo_read(&user, &memo_id).await?;
    legal_memos::versions(&memo_id).await.map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn load_legal_memo_version(
    memo_id: String,
    version_id: String,
) -> Result<MemoVersionSnapshot, ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    authorize_memo_read(&user, &memo_id).await?;
    legal_memos::version_snapshot(&memo_id, &version_id)
        .await
        .map_err(memo_error)?
        .ok_or_else(|| ServerFnError::new("That version was not found."))
}

#[server(prefix = "/api")]
pub async fn memo_attorney_choices(case_id: String) -> Result<Vec<MemoAttorney>, ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server::permissions::{require_cap, require_user};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    crate::server_fns::crm::require_staff(&user)?;
    require_cap(&user, &case_id, CaseCapability::ViewCase).await?;
    Ok(legal_memos::attorney_choices(&case_id)
        .await
        .map_err(ServerFnError::new)?
        .into_iter()
        .filter(|a| a.id != user.id)
        .collect())
}

#[server(prefix = "/api")]
pub async fn create_legal_memo(case_id: String, title: String) -> Result<String, ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server::permissions::{require_cap, require_user};
    use crate::server_fns::capabilities::CaseCapability;
    use crate::server_fns::crm::{clean_text, MAX_SHORT_TEXT};

    let user = require_user().await?;
    crate::server_fns::crm::require_staff(&user)?;
    require_cap(&user, &case_id, CaseCapability::AddNotes).await?;
    let title = clean_text(&title, "title", MAX_SHORT_TEXT).map_err(ServerFnError::new)?;
    let title = if title.is_empty() { "Untitled memo".to_string() } else { title };
    legal_memos::create(&case_id, &title, &user.id, &user.full_name())
        .await
        .map_err(ServerFnError::new)
}

#[server(prefix = "/api")]
pub async fn save_legal_memo_draft(
    memo_id: String,
    title: String,
    attorney_id: String,
    content: LegalMemoContent,
) -> Result<(), ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    let case_id = authorize_memo_author(&user, &memo_id).await?;
    let (title, attorney) = clean_header(&case_id, &title, &attorney_id, &user.id).await?;
    validate_draft(&title, &content).map_err(ServerFnError::new)?;
    legal_memos::save_draft(&memo_id, &user.id, &title, attorney.as_ref(), &content)
        .await
        .map_err(memo_error)
}

#[server(prefix = "/api")]
pub async fn send_legal_memo_for_review(
    memo_id: String,
    title: String,
    attorney_id: String,
    content: LegalMemoContent,
) -> Result<(), ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    let case_id = authorize_memo_author(&user, &memo_id).await?;
    let (title, attorney) = clean_header(&case_id, &title, &attorney_id, &user.id).await?;
    validate_draft(&title, &content).map_err(ServerFnError::new)?;
    let problems = problems_before_review(&attorney_id, &content);
    if !problems.is_empty() {
        let lines: Vec<String> = problems.into_iter().map(|p| p.message).collect();
        return Err(ServerFnError::new(lines.join("\n")));
    }
    let attorney = attorney.ok_or_else(|| ServerFnError::new("Choose the attorney this memo goes to."))?;
    legal_memos::send_for_review(
        &memo_id,
        &case_id,
        &user.id,
        &user.full_name(),
        &title,
        &attorney,
        &content,
    )
    .await
    .map_err(memo_error)
}

#[server(prefix = "/api")]
pub async fn start_new_memo_version(memo_id: String) -> Result<(), ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    let case_id = authorize_memo_author(&user, &memo_id).await?;
    legal_memos::start_new_version(&memo_id, &case_id, &user.id, &user.full_name())
        .await
        .map_err(memo_error)
}

#[server(prefix = "/api")]
pub async fn set_memo_review_check(
    memo_id: String,
    check: ReviewCheck,
    checked: bool,
) -> Result<MemoReview, ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    let case_id = authorize_memo_review(&user, &memo_id).await?;
    legal_memos::set_review_check(
        &memo_id,
        &case_id,
        check,
        checked,
        &user.id,
        &user.full_name(),
        user.role.is_site_admin(),
    )
    .await
    .map_err(memo_error)
}

#[server(prefix = "/api")]
pub async fn request_memo_revisions(memo_id: String, comment: String) -> Result<(), ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server::permissions::require_user;
    use crate::server_fns::crm::{clean_text, MAX_LONG_TEXT};

    let user = require_user().await?;
    let case_id = authorize_memo_review(&user, &memo_id).await?;
    let comment = clean_text(&comment, "comment", MAX_LONG_TEXT).map_err(ServerFnError::new)?;
    if comment.is_empty() {
        return Err(ServerFnError::new("Say what needs to change."));
    }
    legal_memos::request_revisions(
        &memo_id,
        &case_id,
        &user.id,
        &user.full_name(),
        &comment,
        user.role.is_site_admin(),
    )
    .await
    .map_err(memo_error)
}

#[server(prefix = "/api")]
pub async fn approve_legal_memo(memo_id: String) -> Result<(), ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    let case_id = authorize_memo_review(&user, &memo_id).await?;
    let version = legal_memos::approve(
        &memo_id,
        &case_id,
        &user.id,
        &user.full_name(),
        user.role.is_site_admin(),
    )
    .await
    .map_err(memo_error)?;
    crate::server::case_records::legal_memo::file_memo_in_background(memo_id, version);
    Ok(())
}

#[server(prefix = "/api")]
pub async fn legal_memo_document(memo_id: String) -> Result<Option<MemoFiledDocument>, ServerFnError> {
    use crate::server::db::case_record_files;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    authorize_memo_read(&user, &memo_id).await?;
    Ok(case_record_files::latest(crate::server::case_records::legal_memo::KIND, &memo_id)
        .await
        .map_err(ServerFnError::new)?
        .map(|filed| MemoFiledDocument {
            version: filed.version,
            file_name: filed.file_name,
            relative_path: filed.relative_path,
            web_url: filed.web_url,
            filed_at: filed.filed_at,
        }))
}

#[server(prefix = "/api")]
pub async fn file_legal_memo_document(memo_id: String) -> Result<(), ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server::permissions::{require_cap, require_user};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    let case_id = authorize_memo_read(&user, &memo_id).await?;
    require_cap(&user, &case_id, CaseCapability::AddNotes).await?;
    let version = legal_memos::latest_approved_version(&memo_id)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("Only an approved memo is filed."))?;
    crate::server::case_records::legal_memo::file_memo(&memo_id, version)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[cfg(feature = "ssr")]
async fn authorize_memo_read(
    user: &crate::server_fns::users::User,
    memo_id: &str,
) -> Result<String, ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server::permissions::require_cap;
    use crate::server_fns::capabilities::CaseCapability;

    crate::server_fns::crm::require_staff(user)?;
    let not_found = || ServerFnError::new("Legal memo not found.");
    let case_id = legal_memos::case_of(memo_id)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(not_found)?;
    require_cap(user, &case_id, CaseCapability::ViewCase)
        .await
        .map_err(|_| not_found())?;
    Ok(case_id)
}

#[cfg(feature = "ssr")]
async fn authorize_memo_author(
    user: &crate::server_fns::users::User,
    memo_id: &str,
) -> Result<String, ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server::permissions::require_cap;
    use crate::server_fns::capabilities::CaseCapability;

    let case_id = authorize_memo_read(user, memo_id).await?;
    require_cap(user, &case_id, CaseCapability::AddNotes).await?;
    let author = legal_memos::author_of(memo_id)
        .await
        .map_err(ServerFnError::new)?
        .unwrap_or_default();
    if author != user.id {
        return Err(ServerFnError::new("Only the memo's author can change it."));
    }
    Ok(case_id)
}

#[cfg(feature = "ssr")]
async fn authorize_memo_review(
    user: &crate::server_fns::users::User,
    memo_id: &str,
) -> Result<String, ServerFnError> {
    use crate::server::db::legal_memos;

    let case_id = authorize_memo_read(user, memo_id).await?;
    let attorney = legal_memos::attorney_of(memo_id)
        .await
        .map_err(ServerFnError::new)?
        .unwrap_or_default();
    let author = legal_memos::author_of(memo_id)
        .await
        .map_err(ServerFnError::new)?
        .unwrap_or_default();
    if author == user.id {
        return Err(ServerFnError::new(
            "You wrote this memo, so someone else has to review it.",
        ));
    }
    if !is_reviewer(user, &attorney, &author) {
        return Err(ServerFnError::new(
            "Only the attorney this memo is addressed to, or a site admin, can review it.",
        ));
    }
    if !case_accepts_changes(&case_id).await? {
        return Err(ServerFnError::new("This case can no longer be changed."));
    }
    Ok(case_id)
}

#[cfg(feature = "ssr")]
fn is_reviewer(user: &crate::server_fns::users::User, attorney_id: &str, author_id: &str) -> bool {
    author_id != user.id
        && (user.role.is_site_admin() || (!attorney_id.is_empty() && attorney_id == user.id))
}

#[cfg(feature = "ssr")]
async fn can_add_notes(user: &crate::server_fns::users::User, case_id: &str) -> bool {
    crate::server::permissions::require_cap(
        user,
        case_id,
        crate::server_fns::capabilities::CaseCapability::AddNotes,
    )
    .await
    .is_ok()
}

#[cfg(feature = "ssr")]
async fn case_accepts_changes(case_id: &str) -> Result<bool, ServerFnError> {
    Ok(crate::server::db::cases::status(case_id)
        .await
        .map_err(ServerFnError::new)?
        .is_some_and(|status| status.accepts_changes()))
}

#[cfg(feature = "ssr")]
async fn clean_header(
    case_id: &str,
    title: &str,
    attorney_id: &str,
    author_id: &str,
) -> Result<(String, Option<MemoAttorney>), ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server_fns::crm::{clean_text, MAX_SHORT_TEXT};

    let title = clean_text(title, "title", MAX_SHORT_TEXT).map_err(ServerFnError::new)?;
    let attorney_id = attorney_id.trim();
    if attorney_id.is_empty() {
        return Ok((title, None));
    }
    if attorney_id == author_id {
        return Err(ServerFnError::new(
            "Choose an attorney other than yourself. Someone else has to review your memo.",
        ));
    }
    let attorney = legal_memos::attorney_choices(case_id)
        .await
        .map_err(ServerFnError::new)?
        .into_iter()
        .find(|a| a.id == attorney_id)
        .ok_or_else(|| ServerFnError::new("Choose an attorney who is on this case."))?;
    Ok((title, Some(attorney)))
}

#[cfg(feature = "ssr")]
fn memo_error(error: sqlx::Error) -> ServerFnError {
    match error {
        sqlx::Error::Protocol(message) => ServerFnError::new(message),
        other => ServerFnError::new(other),
    }
}
