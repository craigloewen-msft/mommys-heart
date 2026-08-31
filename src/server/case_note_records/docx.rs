//! Rendering a case note as a Word (`.docx`) document.
//!
//! Pure: a [`CaseNoteDetail`] in, bytes out. No database, no library, no clock —
//! everything the document says comes from the note it was given, so the same
//! note always renders the same document.
//!
//! # Why the XML is written by hand
//!
//! A `.docx` is a zip of XML parts, and the app already carries both halves of
//! that: `zip` with deflate, and `quick-xml` (which
//! [`crate::server::docs`] uses to *read* WordprocessingML). Writing the four
//! parts a text document needs is about a page of markup, against a new
//! dependency and a font story for the alternative. What is emitted here is
//! deliberately the minimum a conforming reader needs: no styles part, no
//! theme, no settings — formatting is direct run and paragraph properties,
//! which Word, Word Online, LibreOffice and SharePoint's own preview all honor.

use std::io::{Cursor, Write};

use zip::write::SimpleFileOptions;

use crate::server_fns::case_notes::{CaseNoteDetail, CaseNoteState};

/// The MIME type of what [`render`] produces.
pub const DOCX_MIME: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

/// One line of the document: how it is laid out.
enum Block {
    /// The document's title.
    Title(String),
    /// A section heading.
    Heading(String),
    /// A labelled value, rendered as "Label: value" with the label in bold.
    Field(String, String),
    /// A run of body text, which may contain newlines.
    Body(String),
    /// Small grey text, for the footer and provenance lines.
    Note(String),
    /// Vertical space.
    Spacer,
}

/// Render a note as a `.docx`, including every addendum attached to it.
///
/// `case_name` is passed in rather than looked up so this stays pure; the
/// caller has it already.
pub fn render(detail: &CaseNoteDetail, case_name: &str) -> Result<Vec<u8>, String> {
    let blocks = blocks_for(detail, case_name);
    package(&document_xml(&blocks))
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

fn placeholder(value: &str) -> String {
    if value.trim().is_empty() {
        "Not recorded".to_string()
    } else {
        value.to_string()
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

// ---------------------------------------------------------------------------
// OOXML
// ---------------------------------------------------------------------------

/// Half-points, which is how OOXML sizes text: 24 = 12pt.
const BODY_SIZE: u32 = 22;
const TITLE_SIZE: u32 = 36;
const HEADING_SIZE: u32 = 26;
const NOTE_SIZE: u32 = 18;

/// Turn the blocks into the body of `word/document.xml`.
fn document_xml(blocks: &[Block]) -> String {
    let mut body = String::new();
    for block in blocks {
        match block {
            Block::Title(text) => body.push_str(&paragraph(
                &[run(text, TITLE_SIZE, true, None)],
                "240",
            )),
            Block::Heading(text) => body.push_str(&paragraph(
                &[run(text, HEADING_SIZE, true, None)],
                "180",
            )),
            Block::Field(label, value) => {
                let mut runs = vec![run(&format!("{label}: "), BODY_SIZE, true, None)];
                runs.extend(multiline_runs(value, BODY_SIZE, false, None));
                body.push_str(&paragraph(&runs, "60"));
            }
            Block::Body(text) => {
                body.push_str(&paragraph(
                    &multiline_runs(text, BODY_SIZE, false, None),
                    "60",
                ));
            }
            Block::Note(text) => body.push_str(&paragraph(
                &multiline_runs(text, NOTE_SIZE, false, Some("595959")),
                "60",
            )),
            Block::Spacer => body.push_str(&paragraph(&[], "0")),
        }
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{body}<w:sectPr><w:pgSz w:w="12240" w:h="15840"/><w:pgMar w:top="1134" w:right="1134" w:bottom="1134" w:left="1134"/></w:sectPr></w:body></w:document>"#
    )
}

/// One paragraph, with space after it in twentieths of a point.
fn paragraph(runs: &[String], space_after: &str) -> String {
    format!(
        "<w:p><w:pPr><w:spacing w:after=\"{space_after}\"/></w:pPr>{}</w:p>",
        runs.concat()
    )
}

/// One run of text. `xml:space="preserve"` keeps the trailing space after a
/// field label from being dropped.
fn run(text: &str, size: u32, bold: bool, color: Option<&str>) -> String {
    let mut props = String::new();
    if bold {
        props.push_str("<w:b/>");
    }
    if let Some(color) = color {
        props.push_str(&format!("<w:color w:val=\"{color}\"/>"));
    }
    props.push_str(&format!(
        "<w:sz w:val=\"{size}\"/><w:szCs w:val=\"{size}\"/>"
    ));
    format!(
        "<w:r><w:rPr>{props}</w:rPr><w:t xml:space=\"preserve\">{}</w:t></w:r>",
        escape(text)
    )
}

/// Runs for text that may contain newlines, which OOXML expresses as an explicit
/// break rather than as a literal character.
fn multiline_runs(text: &str, size: u32, bold: bool, color: Option<&str>) -> Vec<String> {
    let mut out = Vec::new();
    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            out.push("<w:r><w:br/></w:r>".to_string());
        }
        out.push(run(line, size, bold, color));
    }
    out
}

/// XML text escaping. Control characters are dropped rather than escaped:
/// XML 1.0 cannot represent most of them at all, and a stray one would make the
/// whole document unreadable.
fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\t' => out.push(' '),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#;

const PACKAGE_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

const DOCUMENT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"></Relationships>"#;

/// Zip the four parts into the `.docx` package.
///
/// Part order matters more than it looks: content sniffers (including the
/// `infer` check every upload in this app goes through) identify an OOXML file
/// by finding `[Content_Types].xml` at the head of the archive.
fn package(document: &str) -> Result<Vec<u8>, String> {
    let mut buffer = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buffer);
        let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, contents) in [
            ("[Content_Types].xml", CONTENT_TYPES),
            ("_rels/.rels", PACKAGE_RELS),
            ("word/_rels/document.xml.rels", DOCUMENT_RELS),
            ("word/document.xml", document),
        ] {
            zip.start_file(name, options)
                .map_err(|e| format!("could not start '{name}' in the document: {e}"))?;
            zip.write_all(contents.as_bytes())
                .map_err(|e| format!("could not write '{name}' in the document: {e}"))?;
        }
        zip.finish()
            .map_err(|e| format!("could not finish the document: {e}"))?;
    }
    Ok(buffer.into_inner())
}
