//! `.docx` extraction and chunking (SSR only).
//!
//! `.docx` is a zip archive; we stream `word/document.xml` with `quick-xml`,
//! tracking heading styles and table structure.

use std::io::Read;
use std::path::Path;

use quick_xml::events::Event;
use quick_xml::Reader;

/// Approximate chunk size in tokens (chars / 4).
const CHUNK_SIZE: usize = 500;
const CHUNK_OVERLAP: usize = 100;

/// A paragraph/table section with its heading context.
#[derive(Clone, Debug)]
pub struct Section {
    pub text: String,
    pub heading: String,
    pub filename: String,
}

/// A grouped chunk ready to be embedded and stored.
#[derive(Clone, Debug)]
pub struct DocChunk {
    pub text: String,
    pub heading: String,
    pub filename: String,
}

/// Extract heading-aware sections from a `.docx` file: top-level body
/// paragraphs first (tracking the current `Heading*` style), then top-level
/// tables.
pub fn extract_sections(filepath: &Path) -> Result<Vec<Section>, String> {
    let filename = filepath
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let xml = read_document_xml(filepath)?;
    let (body_paras, tables) = parse_document_xml(&xml)?;

    let mut sections = Vec::new();
    let mut current_heading = String::from("Introduction");

    for para in &body_paras {
        let text = para.text.trim();
        if text.is_empty() {
            continue;
        }
        if para.is_heading {
            current_heading = text.to_string();
            continue;
        }
        sections.push(Section {
            text: text.to_string(),
            heading: current_heading.clone(),
            filename: filename.clone(),
        });
    }

    // Tables are appended after all paragraphs, so they all carry the heading in
    // effect after the final paragraph.
    for table in &tables {
        let mut rows_text = Vec::new();
        for row in table {
            let cells: Vec<&str> = row
                .iter()
                .map(|c| c.trim())
                .filter(|c| !c.is_empty())
                .collect();
            if !cells.is_empty() {
                rows_text.push(cells.join(" | "));
            }
        }
        if !rows_text.is_empty() {
            sections.push(Section {
                text: rows_text.join("\n"),
                heading: current_heading.clone(),
                filename: filename.clone(),
            });
        }
    }

    Ok(sections)
}

/// Group sections into ~`CHUNK_SIZE`-token chunks with `CHUNK_OVERLAP` overlap.
pub fn chunk_sections(sections: &[Section]) -> Vec<DocChunk> {
    let mut chunks: Vec<DocChunk> = Vec::new();
    // Each entry: (text, heading).
    let mut current_chunk: Vec<(String, String)> = Vec::new();
    let mut current_chunk_len = 0usize;
    let mut current_filename = sections
        .first()
        .map(|s| s.filename.clone())
        .unwrap_or_default();

    let tokens = |t: &str| t.chars().count() / 4;

    for section in sections {
        let text_tokens = tokens(&section.text);

        if current_chunk_len + text_tokens > CHUNK_SIZE && !current_chunk.is_empty() {
            chunks.push(build_chunk(&current_chunk, &current_filename));

            // Overlap: keep the trailing paragraphs up to CHUNK_OVERLAP tokens.
            let mut overlap: Vec<(String, String)> = Vec::new();
            let mut overlap_len = 0usize;
            for s in current_chunk.iter().rev() {
                let s_len = tokens(&s.0);
                if overlap_len + s_len > CHUNK_OVERLAP {
                    break;
                }
                overlap.insert(0, s.clone());
                overlap_len += s_len;
            }
            current_chunk = overlap;
            current_chunk_len = overlap_len;
        }

        current_chunk.push((section.text.clone(), section.heading.clone()));
        current_chunk_len += text_tokens;
        current_filename = section.filename.clone();
    }

    if !current_chunk.is_empty() {
        chunks.push(build_chunk(&current_chunk, &current_filename));
    }

    chunks
}

fn build_chunk(current_chunk: &[(String, String)], filename: &str) -> DocChunk {
    // Sorted unique headings joined by ", ".
    let mut headings: Vec<String> = current_chunk.iter().map(|s| s.1.clone()).collect();
    headings.sort();
    headings.dedup();

    let text = current_chunk
        .iter()
        .map(|s| s.0.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    DocChunk {
        text,
        heading: headings.join(", "),
        filename: filename.to_string(),
    }
}

/// Convert heading text to a URL-safe anchor slug, matching the anchors the
/// document viewer emits.
pub fn slugify(text: &str) -> String {
    let lower = text.to_lowercase();
    let lower = lower.trim();

    // Keep only word chars, whitespace, and hyphens.
    let kept: String = lower
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || c.is_whitespace() || *c == '-')
        .collect();

    // Replace runs of whitespace/underscore with a single hyphen.
    let mut hyphenated = String::new();
    let mut prev_sep = false;
    for c in kept.chars() {
        if c.is_whitespace() || c == '_' {
            if !prev_sep {
                hyphenated.push('-');
                prev_sep = true;
            }
        } else {
            hyphenated.push(c);
            prev_sep = false;
        }
    }

    // Collapse consecutive hyphens and trim.
    let mut collapsed = String::new();
    let mut prev_dash = false;
    for c in hyphenated.chars() {
        if c == '-' {
            if !prev_dash {
                collapsed.push('-');
                prev_dash = true;
            }
        } else {
            collapsed.push(c);
            prev_dash = false;
        }
    }

    collapsed.trim_matches('-').to_string()
}

// ---------------------------------------------------------------------------
// Low-level docx XML parsing
// ---------------------------------------------------------------------------

struct ParaAcc {
    text: String,
    is_heading: bool,
}

/// A parsed table: rows of cell texts.
type Table = Vec<Vec<String>>;

/// Read `word/document.xml` out of the `.docx` zip archive.
fn read_document_xml(filepath: &Path) -> Result<String, String> {
    let file = std::fs::File::open(filepath).map_err(|e| format!("open {filepath:?}: {e}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("read zip {filepath:?}: {e}"))?;
    let mut doc = archive
        .by_name("word/document.xml")
        .map_err(|e| format!("no word/document.xml in {filepath:?}: {e}"))?;
    let mut xml = String::new();
    doc.read_to_string(&mut xml)
        .map_err(|e| format!("read document.xml: {e}"))?;
    Ok(xml)
}

/// Parse the WordprocessingML body into (top-level paragraphs, top-level tables).
/// Each table is a list of rows; each row a list of cell texts.
fn parse_document_xml(xml: &str) -> Result<(Vec<ParaAcc>, Vec<Table>), String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut body_paras: Vec<ParaAcc> = Vec::new();
    let mut tables: Vec<Table> = Vec::new();

    // Table nesting: each table is Vec<row>, each row is Vec<cell text>.
    let mut table_stack: Vec<Table> = Vec::new();
    let mut row_stack: Vec<Vec<String>> = Vec::new();
    // Cell text accumulators (nested tables push additional cells).
    let mut cell_stack: Vec<String> = Vec::new();

    // Current paragraph accumulation.
    let mut in_para = false;
    let mut para_text = String::new();
    let mut para_is_heading = false;
    let mut in_text = false;

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Err(e) => return Err(format!("xml parse error: {e}")),
            Ok(Event::Eof) => break,
            Ok(Event::Start(e)) => match local_name(e.name().as_ref()) {
                b"tbl" => table_stack.push(Vec::new()),
                b"tr" => row_stack.push(Vec::new()),
                b"tc" => cell_stack.push(String::new()),
                b"p" => {
                    in_para = true;
                    para_text.clear();
                    para_is_heading = false;
                }
                b"pStyle" => {
                    if in_para {
                        if let Some(val) = attr_val(&e, b"val") {
                            if val.starts_with("Heading") {
                                para_is_heading = true;
                            }
                        }
                    }
                }
                b"t" => in_text = true,
                _ => {}
            },
            Ok(Event::Empty(e)) => {
                // Self-closing pStyle, e.g. <w:pStyle w:val="Heading1"/>.
                if local_name(e.name().as_ref()) == b"pStyle" && in_para {
                    if let Some(val) = attr_val(&e, b"val") {
                        if val.starts_with("Heading") {
                            para_is_heading = true;
                        }
                    }
                }
            }
            Ok(Event::Text(t)) if in_text => {
                let txt = t
                    .unescape()
                    .map_err(|e| format!("unescape: {e}"))?
                    .into_owned();
                para_text.push_str(&txt);
            }
            Ok(Event::End(e)) => match local_name(e.name().as_ref()) {
                b"t" => in_text = false,
                b"p" => {
                    in_para = false;
                    if let Some(cell) = cell_stack.last_mut() {
                        // Paragraph inside a table cell: joined with newlines.
                        if !cell.is_empty() {
                            cell.push('\n');
                        }
                        cell.push_str(&para_text);
                    } else {
                        body_paras.push(ParaAcc {
                            text: std::mem::take(&mut para_text),
                            is_heading: para_is_heading,
                        });
                    }
                }
                b"tc" => {
                    let cell = cell_stack.pop().unwrap_or_default();
                    if let Some(row) = row_stack.last_mut() {
                        row.push(cell);
                    }
                }
                b"tr" => {
                    if let Some(row) = row_stack.pop() {
                        if let Some(table) = table_stack.last_mut() {
                            table.push(row);
                        }
                    }
                }
                b"tbl" => {
                    if let Some(table) = table_stack.pop() {
                        // Only top-level tables become sections; a nested table
                        // stays part of its parent cell's text.
                        if table_stack.is_empty() {
                            tables.push(table);
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
        buf.clear();
    }

    Ok((body_paras, tables))
}

/// Strip the `w:` (or any) namespace prefix from an element name.
fn local_name(name: &[u8]) -> &[u8] {
    match name.iter().position(|&b| b == b':') {
        Some(i) => &name[i + 1..],
        None => name,
    }
}

/// Read an attribute value (matched by local name) off a start/empty element.
fn attr_val(e: &quick_xml::events::BytesStart, want: &[u8]) -> Option<String> {
    e.attributes().flatten().find_map(|a| {
        if local_name(a.key.as_ref()) == want {
            Some(String::from_utf8_lossy(a.value.as_ref()).into_owned())
        } else {
            None
        }
    })
}
