//! `.docx` document viewer + download (SSR only).
//!
//! Renders a `.docx` file as a styled HTML page (with heading anchors,
//! bold/italic/underline runs, lists and tables) so the chat widget's "Read
//! more" links resolve to a readable page, and serves the original `.docx` for
//! the "Download" links.

use std::io::Read;
use std::path::{Path, PathBuf};

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::server::rag::documents::slugify;

/// MIME type for a Word `.docx` file.
pub const DOCX_MIME: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

/// Directory holding the `.docx` corpus, from the `DOCS_DIR` setting.
fn docs_dir() -> String {
    std::env::var("DOCS_DIR")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "docs".into())
}

/// Resolve a request path to a safe `.docx` file inside [`docs_dir`].
///
/// Guards against path traversal: only a bare filename (no separators) ending
/// in `.docx` and pointing at an existing file is accepted.
fn safe_docx_path(filename: &str) -> Option<PathBuf> {
    let base = Path::new(filename)
        .file_name()?
        .to_string_lossy()
        .to_string();
    if base != filename {
        return None;
    }
    let path = Path::new(&docs_dir()).join(&base);
    if path.extension().and_then(|e| e.to_str()) != Some("docx") {
        return None;
    }
    if !path.is_file() {
        return None;
    }
    Some(path)
}

/// Read the raw bytes of a `.docx` for download, returning `(bytes, filename)`.
pub fn read_docx_bytes(filename: &str) -> Option<(Vec<u8>, String)> {
    let path = safe_docx_path(filename)?;
    let bytes = std::fs::read(&path).ok()?;
    let name = path.file_name()?.to_string_lossy().to_string();
    Some((bytes, name))
}

/// Render a `.docx` file as a full styled HTML page, or `None` if it does not
/// exist.
pub fn render_docx_to_html(filename: &str) -> Option<String> {
    let path = safe_docx_path(filename)?;
    let xml = read_document_xml(&path).ok()?;
    let body_html = render_body(&xml).ok()?;
    let title = filename.strip_suffix(".docx").unwrap_or(filename);
    Some(
        PAGE_TEMPLATE
            .replace("__TITLE__", &escape_html(title))
            .replace("__BODY__", &body_html),
    )
}

// ---------------------------------------------------------------------------
// docx XML rendering
// ---------------------------------------------------------------------------

/// A run of text with its inline formatting.
#[derive(Default)]
struct Run {
    text: String,
    bold: bool,
    italic: bool,
    underline: bool,
}

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

/// Walk the WordprocessingML body in order, emitting HTML blocks for top-level
/// paragraphs (headings / list items / paragraphs, preserving run formatting)
/// and top-level tables.
fn render_body(xml: &str) -> Result<String, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut out: Vec<String> = Vec::new();

    // Table nesting state.
    let mut table_stack: Vec<Vec<Vec<String>>> = Vec::new();
    let mut row_stack: Vec<Vec<String>> = Vec::new();
    let mut cell_stack: Vec<String> = Vec::new();

    // Current paragraph state.
    let mut in_para = false;
    let mut para_plain = String::new();
    let mut runs: Vec<Run> = Vec::new();
    let mut is_heading = false;
    let mut heading_level: u8 = 2;
    let mut is_list = false;
    let mut in_ppr = false;

    // Current run state.
    let mut in_run = false;
    let mut cur = Run::default();
    let mut in_rpr = false;
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
                    para_plain.clear();
                    runs.clear();
                    is_heading = false;
                    heading_level = 2;
                    is_list = false;
                }
                b"pPr" => in_ppr = true,
                b"pStyle" => set_heading(&e, in_para, &mut is_heading, &mut heading_level),
                b"numPr" => {
                    if in_ppr {
                        is_list = true;
                    }
                }
                b"r" => {
                    in_run = true;
                    cur = Run::default();
                }
                b"rPr" => in_rpr = true,
                b"b" => {
                    if in_rpr {
                        cur.bold = bool_val(&e);
                    }
                }
                b"i" => {
                    if in_rpr {
                        cur.italic = bool_val(&e);
                    }
                }
                b"u" => {
                    if in_rpr {
                        cur.underline = bool_val(&e);
                    }
                }
                b"t" => in_text = true,
                _ => {}
            },
            Ok(Event::Empty(e)) => match local_name(e.name().as_ref()) {
                b"pStyle" => set_heading(&e, in_para, &mut is_heading, &mut heading_level),
                b"numPr" => {
                    if in_ppr {
                        is_list = true;
                    }
                }
                b"b" => {
                    if in_rpr {
                        cur.bold = bool_val(&e);
                    }
                }
                b"i" => {
                    if in_rpr {
                        cur.italic = bool_val(&e);
                    }
                }
                b"u" => {
                    if in_rpr {
                        cur.underline = bool_val(&e);
                    }
                }
                _ => {}
            },
            Ok(Event::Text(t)) if in_text => {
                let txt = t
                    .unescape()
                    .map_err(|e| format!("unescape: {e}"))?
                    .into_owned();
                para_plain.push_str(&txt);
                if in_run {
                    cur.text.push_str(&txt);
                }
            }
            Ok(Event::End(e)) => match local_name(e.name().as_ref()) {
                b"t" => in_text = false,
                b"rPr" => in_rpr = false,
                b"pPr" => in_ppr = false,
                b"r" => {
                    if in_run {
                        runs.push(std::mem::take(&mut cur));
                        in_run = false;
                    }
                }
                b"p" => {
                    in_para = false;
                    if let Some(cell) = cell_stack.last_mut() {
                        // Paragraph inside a table cell: joined with newlines
                        // rather than emitted as a block.
                        if !cell.is_empty() {
                            cell.push('\n');
                        }
                        cell.push_str(&para_plain);
                    } else {
                        emit_paragraph(
                            &mut out,
                            &para_plain,
                            &runs,
                            is_heading,
                            heading_level,
                            is_list,
                        );
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
                        // Only top-level tables are rendered; a nested table
                        // stays part of its parent cell's text.
                        if table_stack.is_empty() {
                            out.push(table_to_html(&table));
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
        buf.clear();
    }

    Ok(out.join("\n        "))
}

/// Mark the current paragraph as a heading when its style starts with "Heading".
fn set_heading(e: &BytesStart, in_para: bool, is_heading: &mut bool, level: &mut u8) {
    if !in_para {
        return;
    }
    if let Some(val) = attr_val(e, b"val") {
        if val.starts_with("Heading") {
            *is_heading = true;
            *level = heading_level_from(&val);
        }
    }
}

/// Emit the HTML for one top-level paragraph.
fn emit_paragraph(
    out: &mut Vec<String>,
    para_plain: &str,
    runs: &[Run],
    is_heading: bool,
    level: u8,
    is_list: bool,
) {
    let text = para_plain.trim();
    if text.is_empty() {
        return;
    }
    if is_heading {
        let anchor = slugify(text);
        let mut inner = runs_to_html(runs);
        if inner.is_empty() {
            inner = escape_html(text);
        }
        out.push(format!(
            "<h{level} id=\"{anchor}\"><a class=\"anchor-link\" href=\"#{anchor}\">#</a>{inner}</h{level}>"
        ));
    } else {
        let inner = runs_to_html(runs);
        if !inner.is_empty() {
            if is_list {
                out.push(format!("<li>{inner}</li>"));
            } else {
                out.push(format!("<p>{inner}</p>"));
            }
        }
    }
}

/// Render a paragraph's runs to HTML, preserving bold/italic/underline.
fn runs_to_html(runs: &[Run]) -> String {
    let mut s = String::new();
    for r in runs {
        if r.text.is_empty() {
            continue;
        }
        let mut t = escape_html(&r.text);
        if r.bold {
            t = format!("<strong>{t}</strong>");
        }
        if r.italic {
            t = format!("<em>{t}</em>");
        }
        if r.underline {
            t = format!("<u>{t}</u>");
        }
        s.push_str(&t);
    }
    s
}

/// Render a table (rows of cell texts) to an HTML table; the first row is a
/// header row.
fn table_to_html(table: &[Vec<String>]) -> String {
    let mut rows = String::new();
    for (i, row) in table.iter().enumerate() {
        let mut cells = String::new();
        let tag = if i == 0 { "th" } else { "td" };
        for cell in row {
            let text = escape_html(cell.trim());
            cells.push_str(&format!("<{tag}>{text}</{tag}>"));
        }
        rows.push_str(&format!("<tr>{cells}</tr>"));
    }
    format!("<table>{rows}</table>")
}

/// Escape the HTML entities that matter in document text (`&`, `<`, `>`).
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Extract a heading level from a style name like "Heading2" (capped at 6,
/// defaulting to 2): the first digit in the style name.
fn heading_level_from(style: &str) -> u8 {
    style
        .chars()
        .find(|c| c.is_ascii_digit())
        .and_then(|c| c.to_digit(10))
        .map(|d| (d as u8).min(6))
        .unwrap_or(2)
}

/// Interpret a WordML on/off toggle (`<w:b/>`, `<w:b w:val="false"/>`, …).
fn bool_val(e: &BytesStart) -> bool {
    match attr_val(e, b"val") {
        Some(v) => !matches!(v.as_str(), "false" | "0" | "off" | "none"),
        None => true,
    }
}

/// Strip the namespace prefix (e.g. `w:`) from an element name.
fn local_name(name: &[u8]) -> &[u8] {
    match name.iter().position(|&b| b == b':') {
        Some(i) => &name[i + 1..],
        None => name,
    }
}

/// Read an attribute value (matched by local name) off a start/empty element.
fn attr_val(e: &BytesStart, want: &[u8]) -> Option<String> {
    e.attributes().flatten().find_map(|a| {
        if local_name(a.key.as_ref()) == want {
            Some(String::from_utf8_lossy(a.value.as_ref()).into_owned())
        } else {
            None
        }
    })
}

/// The page shell wrapped around the rendered document.
const PAGE_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>__TITLE__ — Mommy's Heart</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #f8f9fa;
            color: #1a1a1a;
            line-height: 1.7;
        }
        .content {
            max-width: 800px;
            margin: 0 auto;
            padding: 32px 24px 80px;
            background: white;
            min-height: 100vh;
            box-shadow: 0 0 20px rgba(0,0,0,0.05);
        }
        .doc-title {
            font-size: 26px;
            font-weight: 700;
            color: #1a1a1a;
            margin-bottom: 24px;
            padding-bottom: 16px;
            border-bottom: 2px solid #1a73e8;
        }
        h1, h2, h3, h4, h5, h6 {
            margin: 28px 0 12px 0;
            color: #1a1a1a;
            position: relative;
        }
        h1 { font-size: 24px; }
        h2 { font-size: 20px; }
        h3 { font-size: 17px; }
        h4, h5, h6 { font-size: 15px; }
        p { margin: 0 0 12px 0; font-size: 15px; }
        li { margin: 0 0 6px 24px; font-size: 15px; }
        strong { font-weight: 600; }
        table {
            width: 100%;
            border-collapse: collapse;
            margin: 16px 0;
            font-size: 14px;
        }
        th, td {
            border: 1px solid #dadce0;
            padding: 8px 12px;
            text-align: left;
        }
        th { background: #f1f3f4; font-weight: 600; }
        tr:nth-child(even) { background: #fafafa; }
        .anchor-link {
            color: #dadce0;
            text-decoration: none;
            font-weight: 400;
            margin-right: 6px;
            font-size: 0.8em;
        }
        .anchor-link:hover { color: #1a73e8; }
        /* Scroll target highlight */
        :target {
            background: #fff3cd;
            padding: 4px 8px;
            border-radius: 4px;
            transition: background 2s;
        }
    </style>
</head>
<body>
    <div class="content">
        <div class="doc-title">__TITLE__</div>
        __BODY__
    </div>
</body>
</html>"#;
