//! Offline email preview generator (SSR only, dev tooling).
//!
//! Renders every notification template from [`super::templates`] with
//! placeholder data and writes them to standalone HTML files, plus an
//! `index.html` gallery, so you can open them in a browser and iterate on the
//! design **without** configuring or hitting the email service. Wired to the
//! `preview-emails` CLI subcommand (see `main.rs`), it never sends anything.

use std::fs;
use std::path::{Path, PathBuf};

use crate::server::config::Brand;
use crate::server::email::templates::{self, Sample};

/// Render all sample emails and write them to `out_dir` (created if needed):
/// one `<key>.html` and `<key>.txt` per template plus an `index.html` gallery.
/// Returns the path to the gallery for the caller to print.
pub fn write_gallery(out_dir: &Path) -> std::io::Result<PathBuf> {
    fs::create_dir_all(out_dir)?;
    let brand = Brand::from_env();
    let samples = templates::samples(&brand);

    for s in &samples {
        fs::write(out_dir.join(format!("{}.html", s.key)), &s.email.html)?;
        fs::write(out_dir.join(format!("{}.txt", s.key)), &s.email.plain_text)?;
    }

    let index_path = out_dir.join("index.html");
    fs::write(&index_path, gallery_html(&samples))?;
    Ok(index_path)
}

/// Build the gallery page that embeds each rendered email in an iframe alongside
/// its subject line and a link to the plain-text version.
fn gallery_html(samples: &[Sample]) -> String {
    let cards = samples
        .iter()
        .map(|s| {
            format!(
                r#"<section class="card">
<header>
<h2>{label}</h2>
<p class="subject"><span>Subject</span> {subject}</p>
<p class="links"><a href="{key}.html" target="_blank">Open HTML</a> &middot; <a href="{key}.txt" target="_blank">Plain text</a></p>
</header>
<iframe src="{key}.html" title="{label}" loading="lazy"></iframe>
</section>"#,
                label = escape(&s.label),
                subject = escape(&s.email.subject),
                key = escape(&s.key),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>Mommy's Heart — Email preview</title>
<style>
  body {{ margin:0; background:#0f172a; color:#e2e8f0; font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif; }}
  header.page {{ padding:28px 32px; border-bottom:1px solid #1e293b; }}
  header.page h1 {{ margin:0; font-size:20px; }}
  header.page p {{ margin:6px 0 0; color:#94a3b8; font-size:14px; }}
  .grid {{ display:grid; grid-template-columns:repeat(auto-fill, minmax(560px, 1fr)); gap:24px; padding:24px 32px; }}
  .card {{ background:#1e293b; border:1px solid #334155; border-radius:14px; overflow:hidden; }}
  .card header {{ padding:16px 20px; border-bottom:1px solid #334155; }}
  .card h2 {{ margin:0; font-size:15px; }}
  .card .subject {{ margin:8px 0 0; font-size:13px; color:#cbd5e1; }}
  .card .subject span {{ display:inline-block; font-size:10px; text-transform:uppercase; letter-spacing:.06em; color:#64748b; margin-right:6px; }}
  .card .links {{ margin:8px 0 0; font-size:12px; }}
  .card .links a {{ color:#fbbf24; text-decoration:none; }}
  .card .links a:hover {{ text-decoration:underline; }}
  iframe {{ width:100%; height:640px; border:0; background:#f1f5f9; }}
</style>
</head>
<body>
<header class="page">
<h1>&#9829; Mommy's Heart — email templates</h1>
<p>Rendered with placeholder data. Nothing here is sent. Regenerate with <code>preview-emails</code>.</p>
</header>
<div class="grid">
{cards}
</div>
</body>
</html>"#,
        cards = cards,
    )
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
