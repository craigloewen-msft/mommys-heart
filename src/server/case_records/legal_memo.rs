use crate::helpers::format::{date_long, matter_label};
use crate::server::db::{case_record_files, cases, legal_memos};
use crate::server::docx::{placeholder, Block};
use crate::server_fns::legal_memos::{LegalMemoContent, MemoLine, ReviewCheck, MEMO_FOLDER};

use super::file_case_document;

pub const KIND: &str = "legal_memo";

pub enum FileError {
    Store(String),
    Record(String),
}

impl std::fmt::Display for FileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileError::Store(e) | FileError::Record(e) => f.write_str(e),
        }
    }
}

pub fn file_memo_in_background(memo_id: String, version: i32) {
    tokio::spawn(async move {
        if let Err(e) = file_memo(&memo_id, version).await {
            tracing::warn!("could not file legal memo {memo_id} version {version}: {e}");
        }
    });
}

pub async fn file_memo(memo_id: &str, version: i32) -> Result<(), FileError> {
    let db = |e: sqlx::Error| FileError::Record(format!("database error: {e}"));
    let snapshot = legal_memos::approved_snapshot(memo_id, version)
        .await
        .map_err(db)?
        .ok_or_else(|| FileError::Record("that memo version is not approved".to_string()))?;
    let case_name = cases::name(&snapshot.case_id)
        .await
        .map_err(db)?
        .unwrap_or_else(|| snapshot.case_id.clone());

    let bytes = crate::server::docx::render(&memo_blocks(&snapshot, &case_name, version))
        .map_err(FileError::Record)?;
    let file_name = memo_file_name(memo_id, &snapshot.title, version);
    file_case_document(&snapshot.case_id, &MEMO_FOLDER, &file_name, bytes)
        .await
        .map_err(FileError::Store)?;
    let relative_path = format!("{}/{file_name}", MEMO_FOLDER.join("/"));
    case_record_files::record(
        KIND,
        memo_id,
        version,
        &snapshot.case_id,
        &relative_path,
        &file_name,
    )
    .await
    .map_err(db)?;
    tracing::info!("filed legal memo {memo_id} version {version} as '{relative_path}'");
    Ok(())
}

pub(super) async fn backfill() -> Result<(), String> {
    let mut after = 0;
    loop {
        let pending = legal_memos::needing_filing(KIND, after, super::BACKFILL_LIMIT)
            .await
            .map_err(|e| format!("database error: {e}"))?;
        let Some(&(last, _, _)) = pending.last() else {
            return Ok(());
        };
        for (_, memo_id, version) in pending {
            match file_memo(&memo_id, version).await {
                Ok(()) => {}
                Err(FileError::Store(e)) => return Err(e),
                Err(FileError::Record(e)) => {
                    tracing::warn!("skipped filing legal memo {memo_id} version {version}: {e}");
                }
            }
        }
        after = last;
    }
}

fn memo_file_name(memo_id: &str, title: &str, version: i32) -> String {
    const SUFFIX: &str = ".docx";
    const JOIN: &str = " - ";
    let prefix = format!("Legal memo {memo_id} v{version}");
    let budget = crate::server::sharepoint::MAX_NAME_LEN
        .saturating_sub(prefix.chars().count() + JOIN.len() + SUFFIX.len());
    let title = title.replace(['/', '\\'], "-");
    let title = if title.trim().is_empty() {
        String::new()
    } else {
        crate::server_fns::documents::sanitize_filename(&title)
    };
    let short: String = title.chars().take(budget).collect();
    let short = short.trim().trim_matches('.').trim();
    if short.is_empty() {
        format!("{prefix}{SUFFIX}")
    } else {
        format!("{prefix}{JOIN}{short}{SUFFIX}")
    }
}

fn memo_blocks(memo: &legal_memos::ApprovedSnapshot, case_name: &str, version: i32) -> Vec<Block> {
    let c: &LegalMemoContent = &memo.content;
    let mut out = vec![
        Block::Title(format!("Legal memo: {}", memo.title)),
        Block::Note("Approved for internal use. Not for client distribution.".into()),
        Block::Spacer,
        Block::Field("To".into(), placeholder(&memo.attorney_name)),
        Block::Field("From".into(), placeholder(&memo.author_name)),
        Block::Field("Date".into(), placeholder(&date_long(&c.memo_date))),
        Block::Field("Matter".into(), matter_label(case_name, &memo.docket)),
        Block::Field(
            "Status".into(),
            format!(
                "Approved by {} on {}, version {version}",
                memo.approved_by_name,
                date_long(&memo.approved_at)
            ),
        ),
        Block::Field(
            "Safety concern flagged".into(),
            match c.safety_flagged {
                Some(true) => format!(
                    "Yes. Escalated on {} to {}.",
                    placeholder(&date_long(&c.safety_escalated_on)),
                    placeholder(&c.safety_escalated_to)
                ),
                Some(false) => "No".into(),
                None => "Not recorded".into(),
            },
        ),
    ];
    if !memo.approval_comment.trim().is_empty() {
        out.push(Block::Field(
            "Approval comment".into(),
            memo.approval_comment.clone(),
        ));
    }

    heading(&mut out, "1. Questions presented");
    numbered(&mut out, &c.questions);
    heading(&mut out, "2. Brief answer");
    out.push(Block::Body(placeholder(&c.brief_answer)));
    heading(&mut out, "3. Relevant facts");
    lines(
        &mut out,
        c.facts.iter().filter(|f| !f.is_blank()).map(|f| {
            format!(
                "{} ({})",
                f.text,
                f.source.map(|s| s.label()).unwrap_or("Source not recorded")
            )
        }),
    );
    heading(&mut out, "4. Procedural history");
    lines(
        &mut out,
        c.procedure.iter().filter(|p| !p.is_blank()).map(|p| {
            format!(
                "{}: {}. Source: {}. {}",
                placeholder(&p.date),
                p.event,
                placeholder(&p.source),
                p.status.label()
            )
        }),
    );
    heading(&mut out, "5. Evidence log");
    lines(
        &mut out,
        c.evidence.iter().filter(|e| !e.is_blank()).map(|e| {
            format!(
                "{}. Source: {}. {}{}. Follow-up: {}",
                e.item,
                placeholder(&e.source),
                e.status.label(),
                if e.date_received.is_empty() {
                    String::new()
                } else {
                    format!(" (received {})", date_long(&e.date_received))
                },
                placeholder(&e.follow_up)
            )
        }),
    );
    heading(&mut out, "6. Authorities");
    lines(
        &mut out,
        c.authorities.iter().filter(|a| !a.is_blank()).map(|a| {
            format!(
                "{}. {}. {}. Checked {}. Subsequent treatment {}.",
                a.citation,
                placeholder(&a.court_level),
                placeholder(&a.link),
                placeholder(&date_long(&a.date_checked)),
                if a.treatment_checked {
                    "checked"
                } else {
                    "not checked"
                }
            )
        }),
    );
    heading(&mut out, "7. Analysis");
    for a in c.analysis.iter().filter(|a| !a.is_blank()) {
        out.push(if a.heading.trim().is_empty() {
            Block::Body(a.body.clone())
        } else {
            Block::Field(a.heading.clone(), placeholder(&a.body))
        });
    }
    heading(&mut out, "8. Outstanding questions");
    lines(
        &mut out,
        c.outstanding
            .iter()
            .filter(|q| !q.text.trim().is_empty())
            .map(|q| format!("\u{2022} {}", q.text)),
    );
    heading(&mut out, "9. Proposed next steps");
    lines(
        &mut out,
        c.next_steps.iter().filter(|n| !n.is_blank()).map(|n| {
            format!(
                "{}. Proposed owner: {}. Timing: {}",
                n.action,
                placeholder(&n.owner),
                placeholder(&n.timing)
            )
        }),
    );
    heading(&mut out, "Review checks");
    lines(
        &mut out,
        ReviewCheck::ALL.iter().map(|check| {
            match memo.review.marks.iter().find(|m| m.check == *check) {
                Some(mark) => format!(
                    "{}: checked by {} on {}",
                    check.label(),
                    mark.by_name,
                    date_long(&mark.at)
                ),
                None => format!("{}: not checked", check.label()),
            }
        }),
    );
    out.push(Block::Spacer);
    out.push(Block::Note(format!(
        "Filed record of legal memo version {version}. Generated by Mommy's Heart from the approved memo."
    )));
    out
}

fn heading(out: &mut Vec<Block>, text: &str) {
    out.push(Block::Spacer);
    out.push(Block::Heading(text.to_string()));
}

fn numbered(out: &mut Vec<Block>, items: &[MemoLine]) {
    lines(
        out,
        items
            .iter()
            .filter(|q| !q.text.trim().is_empty())
            .enumerate()
            .map(|(i, q)| format!("{}. {}", i + 1, q.text)),
    );
}

fn lines(out: &mut Vec<Block>, items: impl Iterator<Item = String>) {
    let before = out.len();
    out.extend(items.map(Block::Body));
    if out.len() == before {
        out.push(Block::Body("Not recorded".into()));
    }
}
