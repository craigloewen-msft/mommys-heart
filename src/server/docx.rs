use std::io::{Cursor, Write};

use zip::write::SimpleFileOptions;

pub const DOCX_MIME: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

pub enum Block {
    Title(String),
    Heading(String),
    Field(String, String),
    Body(String),
    Note(String),
    Spacer,
}

pub fn render(blocks: &[Block]) -> Result<Vec<u8>, String> {
    package(&document_xml(blocks))
}

pub fn placeholder(value: &str) -> String {
    if value.trim().is_empty() {
        "Not recorded".to_string()
    } else {
        value.to_string()
    }
}


// OOXML sizes text in half-points: 24 is 12pt.
const BODY_SIZE: u32 = 22;
const TITLE_SIZE: u32 = 36;
const HEADING_SIZE: u32 = 26;
const NOTE_SIZE: u32 = 18;

fn document_xml(blocks: &[Block]) -> String {
    let mut body = String::new();
    for block in blocks {
        match block {
            Block::Title(text) => {
                body.push_str(&paragraph(&[run(text, TITLE_SIZE, true, None)], "240"))
            }
            Block::Heading(text) => {
                body.push_str(&paragraph(&[run(text, HEADING_SIZE, true, None)], "180"))
            }
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

fn paragraph(runs: &[String], space_after: &str) -> String {
    format!(
        "<w:p><w:pPr><w:spacing w:after=\"{space_after}\"/></w:pPr>{}</w:p>",
        runs.concat()
    )
}

// xml:space="preserve" keeps the space after a field label.
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

// XML 1.0 can't represent most control characters, so they are dropped.
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

// [Content_Types].xml must come first: upload sniffing identifies OOXML by it.
fn package(document: &str) -> Result<Vec<u8>, String> {
    let mut buffer = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buffer);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
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
