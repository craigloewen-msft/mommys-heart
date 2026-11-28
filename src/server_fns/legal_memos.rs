use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::helpers::dates::parse_iso;
use crate::server_fns::crm::{coded_enum, MAX_SHORT_TEXT};

coded_enum!(MemoStatus {
    Draft => ("draft", "Draft"),
    InReview => ("in_review", "Ready for attorney review"),
    RevisionsRequested => ("revisions_requested", "Revisions requested"),
    Approved => ("approved", "Approved"),
});

coded_enum!(VersionEvent {
    Sent => ("sent", "Sent for attorney review"),
    RevisionsRequested => ("revisions_requested", "Revisions requested"),
    Approved => ("approved", "Approved"),
    Withdrawn => ("withdrawn", "Withdrawn from review"),
    Started => ("started", "Started a new version"),
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
});

impl ReviewCheck {
    pub const REQUIRED_FOR_APPROVAL: &'static [ReviewCheck] =
        &[ReviewCheck::FactsVerified, ReviewCheck::ResearchChecked];
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoLine {
    pub text: String,
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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AnalysisSection {
    pub heading: String,
    pub body: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NextStep {
    pub action: String,
    pub owner: String,
    pub timing: String,
}

impl MemoFact {
    pub fn is_blank(&self) -> bool {
        self.text.trim().is_empty()
    }
}

impl ProcedureEvent {
    pub fn is_blank(&self) -> bool {
        [&self.date, &self.event, &self.source]
            .iter()
            .all(|v| v.trim().is_empty())
    }
}

impl EvidenceItem {
    pub fn is_blank(&self) -> bool {
        [
            &self.item,
            &self.source,
            &self.date_received,
            &self.follow_up,
        ]
        .iter()
        .all(|v| v.trim().is_empty())
    }
}

impl Authority {
    pub fn is_blank(&self) -> bool {
        [
            &self.citation,
            &self.court_level,
            &self.link,
            &self.date_checked,
        ]
        .iter()
        .all(|v| v.trim().is_empty())
    }
}

impl AnalysisSection {
    pub fn is_blank(&self) -> bool {
        self.heading.trim().is_empty() && self.body.trim().is_empty()
    }
}

impl NextStep {
    pub fn is_blank(&self) -> bool {
        [&self.action, &self.owner, &self.timing]
            .iter()
            .all(|v| v.trim().is_empty())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LegalMemoContent {
    pub memo_date: String,
    pub safety_flagged: Option<bool>,
    pub safety_escalated_on: String,
    pub safety_escalated_to: String,
    pub questions: Vec<MemoLine>,
    pub brief_answer: String,
    pub facts: Vec<MemoFact>,
    pub procedure: Vec<ProcedureEvent>,
    pub evidence: Vec<EvidenceItem>,
    pub authorities: Vec<Authority>,
    pub analysis: Vec<AnalysisSection>,
    pub outstanding: Vec<MemoLine>,
    pub next_steps: Vec<NextStep>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReviewMark {
    pub check: ReviewCheck,
    #[serde(default)]
    pub by_user_id: String,
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
    pub updated_on: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoEvent {
    pub actor_name: String,
    pub comment: String,
    pub at: String,
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
    pub revision: String,
    pub last_sent: Option<MemoEvent>,
    pub last_revisions_request: Option<MemoEvent>,
    pub last_approval: Option<MemoEvent>,
    pub can_edit: bool,
    pub can_review: bool,
    pub can_withdraw: bool,
    pub can_start_new: bool,
    pub can_file: bool,
    pub filing_may_be_running: bool,
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
    pub review: MemoReview,
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
    pub filed_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoProblem {
    pub section: String,
    pub message: String,
}

pub const STALE_MEMO_ERROR: &str =
    "This memo was changed in another window. Reload to get the latest version.";

pub const MEMO_FOLDER: [&str; 2] = ["Intake", "Legal Memo"];
pub const DOCKET_KEYS: [&str; 2] = ["Index or docket number", "Docket number 1"];
pub const COURT_KEYS: [&str; 3] = ["Court location", "Court", "Court 1"];

pub const MAX_MEMO_TEXT: usize = 10_000;
pub const MAX_ROWS: usize = 50;

pub const SECTION_HEADER: &str = "Header";
pub const SECTION_SAFETY: &str = "Safety";
pub const SECTION_QUESTIONS: &str = "1 Questions presented";
pub const SECTION_BRIEF: &str = "2 Brief answer";
pub const SECTION_FACTS: &str = "3 Relevant facts";
pub const SECTION_PROCEDURE: &str = "4 Procedural history";
pub const SECTION_EVIDENCE: &str = "5 Evidence log";

pub fn validate_draft(title: &str, content: &LegalMemoContent) -> Result<(), String> {
    if title.trim().is_empty() {
        return Err("Give the memo a title.".to_string());
    }
    let short = |value: &str, label: &str| check_len(value, label, MAX_SHORT_TEXT);
    let long = |value: &str, label: &str| check_len(value, label, MAX_MEMO_TEXT);
    let date = |value: &str, label: &str| {
        if value.is_empty() || parse_iso(value).is_some() {
            Ok(())
        } else {
            Err(format!("The {label} must be a real date."))
        }
    };
    short(title, "title")?;
    date(&content.memo_date, "date")?;
    date(&content.safety_escalated_on, "escalation date")?;
    short(&content.safety_escalated_to, "escalated to")?;
    long(&content.brief_answer, "brief answer")?;
    rows(content.questions.len(), "questions")?;
    rows(content.facts.len(), "facts")?;
    rows(content.procedure.len(), "procedural history")?;
    rows(content.evidence.len(), "evidence log")?;
    rows(content.authorities.len(), "authorities")?;
    rows(content.analysis.len(), "analysis")?;
    rows(content.outstanding.len(), "outstanding questions")?;
    rows(content.next_steps.len(), "next steps")?;
    for q in &content.questions {
        long(&q.text, "question")?;
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
        date(&e.date_received, "date received")?;
        long(&e.follow_up, "follow-up")?;
    }
    for a in &content.authorities {
        long(&a.citation, "citation")?;
        short(&a.court_level, "court level")?;
        long(&a.link, "link")?;
        date(&a.date_checked, "date checked")?;
    }
    for a in &content.analysis {
        short(&a.heading, "analysis heading")?;
        long(&a.body, "analysis")?;
    }
    for q in &content.outstanding {
        long(&q.text, "outstanding question")?;
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
        push(
            SECTION_HEADER,
            "Choose the reviewer this memo goes to.".into(),
        );
    }
    match content.safety_flagged {
        None => push(
            SECTION_SAFETY,
            "Answer whether a safety concern was flagged.".into(),
        ),
        Some(true)
            if content.safety_escalated_on.is_empty()
                || content.safety_escalated_to.trim().is_empty() =>
        {
            push(
                SECTION_SAFETY,
                "Add the date the safety concern was escalated and who it was escalated to.".into(),
            )
        }
        _ => {}
    }
    if !content.questions.iter().any(|q| !q.text.trim().is_empty()) {
        push(
            SECTION_QUESTIONS,
            "Add at least one question presented.".into(),
        );
    }
    if content.brief_answer.trim().is_empty() {
        push(SECTION_BRIEF, "Add a brief answer.".into());
    }
    let facts: Vec<&MemoFact> = content
        .facts
        .iter()
        .filter(|f| !f.text.trim().is_empty())
        .collect();
    if facts.is_empty() {
        push(SECTION_FACTS, "Add at least one relevant fact.".into());
    }
    for fact in facts.iter().filter(|f| f.source.is_none()) {
        push(
            SECTION_FACTS,
            format!("\"{}\" needs a source.", excerpt(&fact.text)),
        );
    }
    for p in content.procedure.iter().filter(|p| {
        !p.is_blank() && p.status == EventStatus::Confirmed && p.source.trim().is_empty()
    }) {
        push(
            SECTION_PROCEDURE,
            format!(
                "\"{}\" can't be Confirmed without a source document.",
                excerpt(if p.event.trim().is_empty() {
                    &p.date
                } else {
                    &p.event
                })
            ),
        );
    }
    for e in content.evidence.iter().filter(|e| {
        !e.is_blank() && e.status == EvidenceStatus::Reviewed && e.date_received.is_empty()
    }) {
        push(
            SECTION_EVIDENCE,
            format!(
                "\"{}\" can't be Reviewed before it has a date received.",
                excerpt(if e.item.trim().is_empty() {
                    &e.source
                } else {
                    &e.item
                })
            ),
        );
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
    require_cap(&user, &case_id, CaseCapability::ViewEvidence).await?;
    legal_memos::list(&case_id).await.map_err(memo_error)
}

#[server(prefix = "/api")]
pub async fn load_legal_memo(memo_id: String) -> Result<LegalMemo, ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    let case_id = authorize_memo_read(&user, &memo_id).await?.case_id;
    let mut memo = legal_memos::get(&memo_id)
        .await
        .map_err(memo_error)?
        .ok_or_else(|| ServerFnError::new("Legal memo not found."))?;
    let writable = case_accepts_changes(&case_id).await?;
    memo.can_edit = writable
        && memo.author_id == user.id
        && matches!(
            memo.status,
            MemoStatus::Draft | MemoStatus::RevisionsRequested
        )
        && can_add_notes(&user, &case_id).await;
    memo.can_withdraw = writable
        && memo.author_id == user.id
        && memo.status == MemoStatus::InReview
        && can_add_notes(&user, &case_id).await;
    memo.can_review = writable
        && memo.status == MemoStatus::InReview
        && memo.author_id != user.id
        && memo.attorney_id == user.id;
    memo.can_start_new = writable
        && memo.author_id == user.id
        && memo.status == MemoStatus::Approved
        && can_add_notes(&user, &case_id).await;
    memo.can_file = memo.status == MemoStatus::Approved && can_add_notes(&user, &case_id).await;
    Ok(memo)
}

#[server(prefix = "/api")]
pub async fn memo_attorney_choices(case_id: String) -> Result<Vec<MemoAttorney>, ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server::permissions::{require_cap, require_user};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    crate::server_fns::crm::require_staff(&user)?;
    require_cap(&user, &case_id, CaseCapability::ViewEvidence).await?;
    Ok(legal_memos::attorney_choices(&case_id)
        .await
        .map_err(memo_error)?
        .into_iter()
        .filter(|a| a.id != user.id)
        .collect())
}

#[server(prefix = "/api")]
pub async fn create_legal_memo(case_id: String) -> Result<String, ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server::permissions::{require_cap, require_user};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    crate::server_fns::crm::require_staff(&user)?;
    require_cap(&user, &case_id, CaseCapability::ViewEvidence).await?;
    require_cap(&user, &case_id, CaseCapability::AddNotes).await?;
    if !case_accepts_changes(&case_id).await? {
        return Err(ServerFnError::new("This case can no longer be changed."));
    }
    legal_memos::create(&case_id, "Untitled memo", &user.id, &user.full_name())
        .await
        .map_err(memo_error)
}

#[server(prefix = "/api")]
pub async fn save_legal_memo_draft(
    memo_id: String,
    title: String,
    attorney_id: String,
    content: LegalMemoContent,
    revision: String,
) -> Result<String, ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    let case_id = authorize_memo_author(&user, &memo_id).await?;
    let (title, attorney) = clean_header(&case_id, &title, &attorney_id, &user.id, false).await?;
    validate_draft(&title, &content).map_err(ServerFnError::new)?;
    legal_memos::save_draft(
        &memo_id,
        &user.id,
        &revision,
        &title,
        attorney.as_ref(),
        &content,
    )
    .await
    .map_err(memo_error)
}

#[server(prefix = "/api")]
pub async fn send_legal_memo_for_review(
    memo_id: String,
    title: String,
    attorney_id: String,
    content: LegalMemoContent,
    revision: String,
) -> Result<(), ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    let case_id = authorize_memo_author(&user, &memo_id).await?;
    let (title, attorney) = clean_header(&case_id, &title, &attorney_id, &user.id, true).await?;
    validate_draft(&title, &content).map_err(ServerFnError::new)?;
    let problems = problems_before_review(&attorney_id, &content);
    if !problems.is_empty() {
        let lines: Vec<String> = problems.into_iter().map(|p| p.message).collect();
        return Err(ServerFnError::new(lines.join("\n")));
    }
    let attorney =
        attorney.ok_or_else(|| ServerFnError::new("Choose the reviewer this memo goes to."))?;
    legal_memos::send_for_review(
        &memo_id,
        &case_id,
        &user.id,
        &user.full_name(),
        &revision,
        &title,
        &attorney,
        &content,
    )
    .await
    .map_err(memo_error)
}

#[server(prefix = "/api")]
pub async fn request_memo_revisions(
    memo_id: String,
    version: i32,
    comment: String,
) -> Result<(), ServerFnError> {
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
        version,
        &user.id,
        &user.full_name(),
        &comment,
    )
    .await
    .map_err(memo_error)
}

#[server(prefix = "/api")]
pub async fn approve_legal_memo(
    memo_id: String,
    version: i32,
    comment: String,
) -> Result<(), ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server::permissions::require_user;
    use crate::server_fns::crm::{clean_text, MAX_LONG_TEXT};

    let user = require_user().await?;
    let case_id = authorize_memo_review(&user, &memo_id).await?;
    let comment = clean_text(&comment, "comment", MAX_LONG_TEXT).map_err(ServerFnError::new)?;
    legal_memos::approve(
        &memo_id,
        &case_id,
        version,
        &user.id,
        &user.full_name(),
        &comment,
    )
    .await
    .map_err(memo_error)?;
    crate::server::case_records::legal_memo::file_memo_in_background(memo_id, version);
    Ok(())
}

#[server(prefix = "/api")]
pub async fn set_memo_review_check(
    memo_id: String,
    version: i32,
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
        version,
        check,
        checked,
        &user.id,
        &user.full_name(),
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
pub async fn legal_memo_versions(memo_id: String) -> Result<Vec<MemoVersionEntry>, ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    authorize_memo_read(&user, &memo_id).await?;
    legal_memos::versions(&memo_id).await.map_err(memo_error)
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
pub async fn legal_memo_document(
    memo_id: String,
) -> Result<Option<MemoFiledDocument>, ServerFnError> {
    use crate::server::db::{case_record_files, legal_memos};
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    authorize_memo_read(&user, &memo_id).await?;
    let Some(version) = legal_memos::latest_approved_version(&memo_id)
        .await
        .map_err(memo_error)?
    else {
        return Ok(None);
    };
    Ok(case_record_files::find(
        crate::server::case_records::legal_memo::KIND,
        &memo_id,
        version,
    )
    .await
    .map_err(memo_error)?
    .map(|filed| MemoFiledDocument {
        version: filed.version,
        file_name: filed.file_name,
        relative_path: filed.relative_path,
        filed_at: filed.filed_at,
    }))
}

#[server(prefix = "/api")]
pub async fn file_legal_memo_document(memo_id: String) -> Result<(), ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server::permissions::{require_cap, require_user};
    use crate::server_fns::capabilities::CaseCapability;

    let user = require_user().await?;
    let case_id = authorize_memo_read(&user, &memo_id).await?.case_id;
    require_cap(&user, &case_id, CaseCapability::AddNotes).await?;
    let version = legal_memos::latest_approved_version(&memo_id)
        .await
        .map_err(memo_error)?
        .ok_or_else(|| ServerFnError::new("Only an approved memo is filed."))?;
    crate::server::case_records::legal_memo::file_memo(&memo_id, version)
        .await
        .map_err(|e| {
            tracing::warn!("filing legal memo {memo_id} version {version} failed: {e}");
            ServerFnError::new("The memo couldn't be filed. Try again later.")
        })
}

#[server(prefix = "/api")]
pub async fn withdraw_legal_memo(memo_id: String) -> Result<(), ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    let case_id = authorize_memo_author(&user, &memo_id).await?;
    if !case_accepts_changes(&case_id).await? {
        return Err(ServerFnError::new("This case can no longer be changed."));
    }
    legal_memos::withdraw_from_review(&memo_id, &case_id, &user.id, &user.full_name())
        .await
        .map_err(memo_error)
}

#[server(prefix = "/api")]
pub async fn delete_legal_memo_draft(memo_id: String) -> Result<(), ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server::permissions::require_user;

    let user = require_user().await?;
    let case_id = authorize_memo_author(&user, &memo_id).await?;
    legal_memos::delete_draft(&memo_id, &case_id, &user.id, &user.full_name())
        .await
        .map_err(memo_error)
}

#[cfg(feature = "ssr")]
async fn authorize_memo_read(
    user: &crate::server_fns::users::User,
    memo_id: &str,
) -> Result<crate::server::db::legal_memos::MemoHeader, ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server::permissions::require_cap;
    use crate::server_fns::capabilities::CaseCapability;

    crate::server_fns::crm::require_staff(user)?;
    let not_found = || ServerFnError::new("Legal memo not found.");
    let header = legal_memos::header(memo_id)
        .await
        .map_err(memo_error)?
        .ok_or_else(not_found)?;
    require_cap(user, &header.case_id, CaseCapability::ViewEvidence)
        .await
        .map_err(|_| not_found())?;
    Ok(header)
}

#[cfg(feature = "ssr")]
async fn authorize_memo_author(
    user: &crate::server_fns::users::User,
    memo_id: &str,
) -> Result<String, ServerFnError> {
    use crate::server::permissions::require_cap;
    use crate::server_fns::capabilities::CaseCapability;

    let header = authorize_memo_read(user, memo_id).await?;
    require_cap(user, &header.case_id, CaseCapability::AddNotes).await?;
    if header.author_id != user.id {
        return Err(ServerFnError::new("Only the memo's author can change it."));
    }
    Ok(header.case_id)
}

#[cfg(feature = "ssr")]
async fn authorize_memo_review(
    user: &crate::server_fns::users::User,
    memo_id: &str,
) -> Result<String, ServerFnError> {
    let header = authorize_memo_read(user, memo_id).await?;
    if !case_accepts_changes(&header.case_id).await? {
        return Err(ServerFnError::new("This case can no longer be changed."));
    }
    Ok(header.case_id)
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
        .map_err(memo_error)?
        .is_some_and(|status| status.accepts_changes()))
}

#[cfg(feature = "ssr")]
async fn clean_header(
    case_id: &str,
    title: &str,
    attorney_id: &str,
    author_id: &str,
    sending: bool,
) -> Result<(String, Option<MemoAttorney>), ServerFnError> {
    use crate::server::db::legal_memos;
    use crate::server_fns::crm::clean_text;

    let title = clean_text(title, "title", MAX_SHORT_TEXT).map_err(ServerFnError::new)?;
    let attorney_id = attorney_id.trim();
    if attorney_id.is_empty() {
        return Ok((title, None));
    }
    if attorney_id == author_id {
        return Err(ServerFnError::new(
            "Choose a reviewer other than yourself. Someone else has to review your memo.",
        ));
    }
    let attorney = legal_memos::attorney_choices(case_id)
        .await
        .map_err(memo_error)?
        .into_iter()
        .find(|a| a.id == attorney_id);
    if sending && attorney.is_none() {
        return Err(ServerFnError::new("Choose a reviewer who is on this case."));
    }
    Ok((title, attorney))
}

#[cfg(feature = "ssr")]
fn memo_error(error: sqlx::Error) -> ServerFnError {
    match error {
        sqlx::Error::Protocol(message) => ServerFnError::new(message),
        other => {
            tracing::warn!("legal memo request failed: {other}");
            ServerFnError::new("Something went wrong with the legal memo. Try again.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_stored_as_slug<T>(all: &[T], slug: fn(T) -> &'static str)
    where
        T: Copy + PartialEq + std::fmt::Debug + Serialize + for<'de> Deserialize<'de>,
    {
        for v in all {
            assert_eq!(serde_json::to_value(v).unwrap(), slug(*v), "{v:?}");
            assert_eq!(serde_json::from_value::<T>(slug(*v).into()).unwrap(), *v);
        }
    }

    #[test]
    fn stored_values_match_slugs() {
        assert_stored_as_slug(FactSource::ALL, FactSource::slug);
        assert_stored_as_slug(EventStatus::ALL, EventStatus::slug);
        assert_stored_as_slug(EvidenceStatus::ALL, EvidenceStatus::slug);
        assert_stored_as_slug(ReviewCheck::ALL, ReviewCheck::slug);
    }
}
