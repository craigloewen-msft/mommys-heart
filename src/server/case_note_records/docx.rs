//! Rendering a case note as a Word (`.docx`) document.
//!
//! Pure: a [`CaseNoteDetail`] in, bytes out. No database, no library, no clock —
//! everything the document says comes from the note it was given, so the same
//! note always renders the same document.

use crate::server::docx::{placeholder, Block};
use crate::server_fns::case_notes::{CaseNoteDetail, CaseNoteState};

pub use crate::server::docx::DOCX_MIME;

/// Render a note as a `.docx`, including every addendum attached to it.
///
/// `case_name` is passed in rather than looked up so this stays pure; the
/// caller has it already.
pub fn render(detail: &CaseNoteDetail, case_name: &str) -> Result<Vec<u8>, String> {
    crate::server::docx::render(&blocks_for(detail, case_name))
}

/// The file name a note's document is filed under.
///
/// Begins with the activity date so the folder sorts chronologically, and ends
/// with the note id so the file names a single record unambiguously even when
/// two notes share a date and an author.
pub fn file_name(detail: &CaseNoteDetail) -> String {
    let date = if detail.draft.activity_date.is_empty() {
        detail
            .created_at
            .get(0..10)
            .unwrap_or_default()
            .to_string()
    } else {
        detail.draft.activity_date.clone()
    };
    let date = if date.is_empty() {
        "undated".to_string()
    } else {
        date
    };
    // The author's name is part of the name because that is what someone
    // scanning the folder is looking for; it is sanitized because it is
    // ultimately user-supplied.
    let author = crate::server_fns::documents::sanitize_filename(&detail.author);
    let author: String = author.chars().take(60).collect();
    let author = author.trim().trim_matches('.').trim();
    if author.is_empty() {
        format!("Case note {date} ({}).docx", detail.id)
    } else {
        format!("Case note {date} {author} ({}).docx", detail.id)
    }
}

/// The document's content, in order.
fn blocks_for(detail: &CaseNoteDetail, case_name: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let draft = &detail.draft;

    out.push(Block::Title("Case note".to_string()));
    out.push(Block::Note(format!(
        "{} — {}. This is the filed record of case note {}.",
        case_name, detail.case_id, detail.id
    )));
    out.push(Block::Spacer);

    out.push(Block::Heading("Record".to_string()));
    out.push(Block::Field(
        "State".into(),
        detail.state.label().to_string(),
    ));
    out.push(Block::Field("Audience".into(), detail.audience.label().to_string()));
    out.push(Block::Field("Author".into(), role_line(detail)));
    out.push(Block::Field("Created".into(), detail.created_at.clone()));
    if !detail.finalized_at.is_empty() {
        out.push(Block::Field("Finalized".into(), detail.finalized_at.clone()));
    }
    if !detail.signature_name.is_empty() {
        out.push(Block::Field(
            "Typed signature".into(),
            format!(
                "{} (signed {})",
                detail.signature_name, detail.signature_signed_at
            ),
        ));
    }

    // A legacy note has no structured fields; its whole content is the body.
    if detail.state == CaseNoteState::Legacy {
        out.push(Block::Spacer);
        out.push(Block::Heading("Note".to_string()));
        out.push(Block::Body(placeholder(&detail.legacy_body)));
        push_addenda(&mut out, detail);
        push_footer(&mut out, detail);
        return out;
    }

    out.push(Block::Spacer);
    out.push(Block::Heading("Activity".to_string()));
    out.push(Block::Field(
        "Activity date".into(),
        placeholder(&draft.activity_date),
    ));
    out.push(Block::Field(
        "Time".into(),
        match (draft.start_time.as_str(), draft.end_time.as_str()) {
            ("", "") => "Not recorded".to_string(),
            (start, end) => format!("{start} – {end}"),
        },
    ));
    out.push(Block::Field(
        "Total time".into(),
        duration_label(detail.total_minutes),
    ));
    out.push(Block::Field("Location".into(), placeholder(&draft.location)));
    if !draft.delayed_entry_reason.is_empty() {
        out.push(Block::Field(
            "Reason for delayed entry".into(),
            draft.delayed_entry_reason.clone(),
        ));
    }
    out.push(Block::Field(
        "Primary interaction".into(),
        optional_label(draft.primary_interaction.map(|v| v.label())),
    ));
    out.push(Block::Field(
        "Contact category".into(),
        optional_label(draft.contact_category.map(|v| v.label())),
    ));
    out.push(Block::Field(
        "Contact direction".into(),
        optional_label(draft.contact_direction.map(|v| v.label())),
    ));
    out.push(Block::Field(
        "Completion outcome".into(),
        optional_label(draft.completion_outcome.map(|v| v.label())),
    ));
    out.push(Block::Field(
        "Participants".into(),
        placeholder(&draft.participant_summary),
    ));
    out.push(Block::Field(
        "Service areas".into(),
        join_labels(draft.service_areas.iter().map(|v| v.label())),
    ));

    out.push(Block::Spacer);
    out.push(Block::Heading("Content".to_string()));
    out.push(Block::Field(
        "Purpose or objective".into(),
        placeholder(&draft.purpose),
    ));
    out.push(Block::Field(
        "Client-reported information".into(),
        placeholder(&draft.client_reported_info),
    ));
    out.push(Block::Field(
        "Verified or observed information".into(),
        placeholder(&draft.verified_observed_info),
    ));
    out.push(Block::Field(
        "Information sources".into(),
        join_labels(draft.information_sources.iter().map(|v| v.label())),
    ));
    out.push(Block::Field(
        "Actions taken".into(),
        placeholder(&draft.actions_taken),
    ));
    out.push(Block::Field(
        "Client response or outcome".into(),
        placeholder(&draft.outcome_response),
    ));
    out.push(Block::Field(
        "Progress and barriers".into(),
        placeholder(&draft.progress_barriers),
    ));

    out.push(Block::Spacer);
    out.push(Block::Heading("Urgency and next steps".to_string()));
    out.push(Block::Field(
        "Urgency".into(),
        optional_label(draft.urgency.map(|v| v.label())),
    ));
    if !draft.urgency_details.is_empty() {
        out.push(Block::Field(
            "Urgency details".into(),
            draft.urgency_details.clone(),
        ));
    }
    out.push(Block::Field(
        "Next steps".into(),
        if draft.next_steps_not_applicable {
            "Not applicable".to_string()
        } else {
            placeholder(&draft.next_steps)
        },
    ));

    out.push(Block::Spacer);
    out.push(Block::Heading("Narrative".to_string()));
    out.push(Block::Body(placeholder(&draft.narrative)));

    push_addenda(&mut out, detail);
    push_footer(&mut out, detail);
    out
}

/// The addenda, each as its own section. Appended rather than merged into the
/// note above, because an addendum supplements the record and never rewrites it.
fn push_addenda(out: &mut Vec<Block>, detail: &CaseNoteDetail) {
    if detail.addenda.is_empty() {
        return;
    }
    out.push(Block::Spacer);
    out.push(Block::Heading(format!(
        "Addenda ({})",
        detail.addenda.len()
    )));
    out.push(Block::Note(
        "Each addendum below was signed separately and appended after the note was finalized. \
         The note above is unchanged."
            .to_string(),
    ));
    for (index, addendum) in detail.addenda.iter().enumerate() {
        out.push(Block::Spacer);
        out.push(Block::Heading(format!(
            "Addendum {} of {}",
            index + 1,
            detail.addenda.len()
        )));
        out.push(Block::Field(
            "Signed".into(),
            format!("{} by {}", addendum.signed_at, addendum.author),
        ));
        out.push(Block::Field("Reason".into(), placeholder(&addendum.reason)));
        out.push(Block::Field(
            "Supplemental or corrected information".into(),
            placeholder(&addendum.information),
        ));
        out.push(Block::Field(
            "Affected categories".into(),
            join_labels(addendum.affected_categories.iter().map(|v| v.label())),
        ));
        out.push(Block::Field(
            "Follow-up".into(),
            placeholder(&addendum.follow_up),
        ));
        out.push(Block::Field(
            "Typed signature".into(),
            placeholder(&addendum.signature_name),
        ));
    }
}

/// The provenance footer, so a printed or forwarded copy still says what it is
/// and how complete it was when it was written.
fn push_footer(out: &mut Vec<Block>, detail: &CaseNoteDetail) {
    out.push(Block::Spacer);
    out.push(Block::Note(format!(
        "Filed record of case note {} on case {}, including {} addend{}. \
         Generated by Mommy's Heart from the note record; the note itself is immutable \
         and corrections are made by appending a further addendum.",
        detail.id,
        detail.case_id,
        detail.addenda.len(),
        if detail.addenda.len() == 1 { "um" } else { "a" }
    )));
}

fn role_line(detail: &CaseNoteDetail) -> String {
    match detail.author_role_snapshot {
        Some(role) => format!("{} ({})", detail.author, role.label()),
        None => detail.author.clone(),
    }
}

fn optional_label(label: Option<&str>) -> String {
    label.unwrap_or("Not recorded").to_string()
}

fn join_labels<'a>(labels: impl Iterator<Item = &'a str>) -> String {
    let joined = labels.collect::<Vec<_>>().join(", ");
    placeholder(&joined)
}

fn duration_label(minutes: Option<i32>) -> String {
    match minutes {
        Some(minutes) if minutes > 0 => {
            let (hours, mins) = (minutes / 60, minutes % 60);
            match (hours, mins) {
                (0, m) => format!("{m}m"),
                (h, 0) => format!("{h}h"),
                (h, m) => format!("{h}h {m}m"),
            }
        }
        _ => "Not calculated".to_string(),
    }
}

