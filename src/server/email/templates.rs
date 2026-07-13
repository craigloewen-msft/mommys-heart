//! Branded HTML email templates for the CRM's notifications (SSR only).
//!
//! Every notification email is built here so the look — colors, header, footer,
//! button, spacing — is defined once and shared by all message types. Callers in
//! [`crate::server::notifications`] hand us the *event* (who did what, on which
//! case) and get back a [`RenderedEmail`] with the subject line plus matching
//! HTML and plain-text bodies, ready for [`crate::server::email::send_email`].
//!
//! The HTML is deliberately old-school — table-based layout with inline styles —
//! because that is what renders reliably across email clients (Outlook, Gmail,
//! Apple Mail). Shared building blocks ([`layout`], [`button`], the
//! [`palette`](super::palette) colors sourced from the app's Tailwind theme, and
//! the per-kind [`Theme`]) keep the individual templates short.
//!
//! To *see* these without sending anything, use the offline preview tooling
//! ([`samples`], wired to the `preview-emails` CLI command) which renders every
//! template with placeholder data to standalone HTML files.

use crate::server::config::Brand;
use crate::server::email::palette;
use crate::server_fns::settings::NotificationKind;

/// A fully rendered email: the subject line and both body representations ACS
/// expects (HTML plus a plain-text fallback for clients that prefer it).
#[derive(Clone, Debug)]
pub struct RenderedEmail {
    pub subject: String,
    pub html: String,
    pub plain_text: String,
}

/// Per-notification-kind theming: the accent color for the callout rule and a
/// small emoji glyph, so each kind is visually distinct while sharing the layout.
/// Accents are Tailwind status tokens resolved via [`palette`].
struct Theme {
    accent: &'static str,
    emoji: &'static str,
}

fn theme(kind: NotificationKind) -> Theme {
    match kind {
        NotificationKind::NewMessage => Theme { accent: palette::color("sky-300"), emoji: "\u{1F4AC}" }, // 💬
        NotificationKind::CaseData => Theme { accent: palette::color("amber-300"), emoji: "\u{270F}\u{FE0F}" }, // ✏️
        NotificationKind::NoteAdded => Theme { accent: palette::color("emerald-300"), emoji: "\u{1F4DD}" }, // 📝
        NotificationKind::EvidenceChanged => Theme { accent: palette::color("rose-400"), emoji: "\u{1F4CE}" }, // 📎
        NotificationKind::Assigned => Theme { accent: palette::color("primary-500"), emoji: "\u{1F511}" }, // 🔑
    }
}

/// Build the email for a case-activity event: `actor_name` did `detail` on the
/// case, categorized as `kind`. Used for status/name/owner/property changes,
/// notes, evidence, and new chat messages.
pub fn case_event(
    brand: &Brand,
    kind: NotificationKind,
    case_name: &str,
    actor_name: &str,
    detail: &str,
) -> RenderedEmail {
    let theme = theme(kind);
    let actor = actor_or_default(actor_name);
    let subject = format!("[{}] {}: {}", brand.name, case_name, kind.label());

    let callout = format!(
        "<strong>{actor}</strong> {detail} on the case <strong>{case}</strong>.",
        actor = escape(actor),
        detail = escape(detail),
        case = escape(case_name),
    );
    let html = layout(
        brand,
        &theme,
        &LayoutParts {
            preheader: &format!("{actor} {detail} on \u{201C}{case_name}\u{201D}."),
            eyebrow: kind.label(),
            heading: &format!("Update on {}", case_name),
            callout_html: &callout,
            cta_label: "Open the case",
            cta_path: "/cases",
        },
    );
    let plain = plain(
        brand,
        &format!("{actor} {detail} on the case \u{201C}{case_name}\u{201D}."),
        "/cases",
    );

    RenderedEmail { subject, html, plain_text: plain }
}

/// Build the "you've been given access to a case" email, whose single recipient
/// is the newly assigned user (a warmer, welcome-style message).
pub fn assignment(brand: &Brand, case_name: &str, actor_name: &str) -> RenderedEmail {
    let theme = theme(NotificationKind::Assigned);
    let actor = actor_or_default(actor_name);
    let subject = format!("[{}] You've been given access to {}", brand.name, case_name);

    let callout = format!(
        "<strong>{actor}</strong> gave you access to the case <strong>{case}</strong>. \
         You can now view its details, notes, evidence, and chat.",
        actor = escape(actor),
        case = escape(case_name),
    );
    let html = layout(
        brand,
        &theme,
        &LayoutParts {
            preheader: &format!("{actor} gave you access to \u{201C}{case_name}\u{201D}."),
            eyebrow: "New case access",
            heading: &format!("You've been added to {}", case_name),
            callout_html: &callout,
            cta_label: "View the case",
            cta_path: "/cases",
        },
    );
    let plain = plain(
        brand,
        &format!("{actor} gave you access to the case \u{201C}{case_name}\u{201D}."),
        "/cases",
    );

    RenderedEmail { subject, html, plain_text: plain }
}

/// The pieces that vary between templates; everything else is shared chrome.
struct LayoutParts<'a> {
    /// Hidden preview text shown by inboxes next to the subject.
    preheader: &'a str,
    /// Small uppercase label above the heading (the notification category).
    eyebrow: &'a str,
    /// The prominent heading.
    heading: &'a str,
    /// Pre-escaped HTML for the highlighted event sentence.
    callout_html: &'a str,
    /// Call-to-action button label.
    cta_label: &'a str,
    /// Path (relative to the app URL) the CTA links to.
    cta_path: &'a str,
}

/// Assemble the full, email-client-safe HTML document around the varying parts.
fn layout(brand: &Brand, theme: &Theme, parts: &LayoutParts) -> String {
    let (primary, muted, border) = (palette::primary(), palette::muted(), palette::border());
    let (canvas, card, text, callout_bg) =
        (palette::canvas(), palette::card(), palette::text(), palette::callout_bg());

    let cta = brand
        .app_url
        .is_empty()
        .then(String::new)
        .unwrap_or_else(|| {
            button(
                &format!("{}{}", brand.app_url, parts.cta_path),
                parts.cta_label,
                primary,
            )
        });

    // The settings link becomes a real link when we know the app URL, otherwise
    // plain guidance so the sentence still reads correctly.
    let settings = if brand.app_url.is_empty() {
        "your Settings page".to_string()
    } else {
        format!(
            "<a href=\"{url}/settings\" style=\"color:{muted};text-decoration:underline;\">\
             your Settings page</a>",
            url = brand.app_url,
            muted = muted,
        )
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<meta name="color-scheme" content="light only" />
<title>{brand_name}</title>
</head>
<body style="margin:0;padding:0;background-color:{canvas};">
<div style="display:none;max-height:0;overflow:hidden;opacity:0;">{preheader}</div>
<table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="background-color:{canvas};padding:24px 12px;">
<tr><td align="center">
<table role="presentation" width="600" cellpadding="0" cellspacing="0" style="width:600px;max-width:100%;background-color:{card};border:1px solid {border};border-radius:16px;overflow:hidden;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;">
<tr><td style="padding:24px 32px;border-bottom:3px solid {primary};">
<span style="font-size:20px;font-weight:700;color:{text};letter-spacing:-0.01em;">
<span style="color:{primary};">&#9829;</span>&nbsp;{brand_name}</span>
</td></tr>
<tr><td style="padding:32px 32px 8px 32px;">
<div style="font-size:12px;font-weight:700;letter-spacing:0.08em;text-transform:uppercase;color:{accent};">{emoji}&nbsp;{eyebrow}</div>
<h1 style="margin:8px 0 0 0;font-size:22px;line-height:1.3;font-weight:700;color:{text};">{heading}</h1>
</td></tr>
<tr><td style="padding:16px 32px 8px 32px;">
<table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="background-color:{callout_bg};border-left:4px solid {accent};border-radius:8px;">
<tr><td style="padding:16px 18px;font-size:16px;line-height:1.6;color:{text};">{callout}</td></tr>
</table>
</td></tr>
{cta}
<tr><td style="padding:8px 32px 32px 32px;font-size:14px;line-height:1.6;color:{muted};">
Sign in to the {brand_name} CRM to see the full details.
</td></tr>
<tr><td style="padding:20px 32px;border-top:1px solid {border};font-size:12px;line-height:1.6;color:{muted};background-color:{callout_bg};">
You're receiving this because of your notification settings. Change what {brand_name} emails you about on {settings}.
</td></tr>
</table>
</td></tr>
</table>
</body>
</html>"#,
        brand_name = escape(&brand.name),
        canvas = canvas,
        card = card,
        border = border,
        primary = primary,
        text = text,
        muted = muted,
        callout_bg = callout_bg,
        accent = theme.accent,
        emoji = theme.emoji,
        preheader = escape(parts.preheader),
        eyebrow = escape(parts.eyebrow),
        heading = escape(parts.heading),
        callout = parts.callout_html,
        cta = cta,
        settings = settings,
    )
}

/// A "bulletproof" table-based button that renders in Outlook as well as modern
/// clients. Returns a full table row (`<tr>`), or empty when there is no URL.
fn button(url: &str, label: &str, color: &str) -> String {
    format!(
        r#"<tr><td style="padding:20px 32px 8px 32px;">
<table role="presentation" cellpadding="0" cellspacing="0"><tr>
<td style="border-radius:10px;background-color:{color};">
<a href="{url}" style="display:inline-block;padding:12px 26px;font-size:15px;font-weight:600;color:#ffffff;text-decoration:none;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;">{label}</a>
</td></tr></table>
</td></tr>"#,
        url = escape(url),
        label = escape(label),
        color = color,
    )
}

/// The plain-text fallback body, matching the HTML content for clients that
/// prefer text. `event` is the one-sentence summary; `cta_path` deep-links.
fn plain(brand: &Brand, event: &str, cta_path: &str) -> String {
    let action = if brand.app_url.is_empty() {
        format!("Sign in to the {} CRM to see the full details.", brand.name)
    } else {
        format!(
            "Open the case: {}{}\nOr sign in to the {} CRM to see the full details.",
            brand.app_url, cta_path, brand.name
        )
    };
    let settings = if brand.app_url.is_empty() {
        "your Settings page".to_string()
    } else {
        format!("{}/settings", brand.app_url)
    };
    format!(
        "{event}\n\n{action}\n\n\
         You're receiving this because of your notification settings. \
         Change what {brand} emails you about on {settings}",
        event = event,
        action = action,
        brand = brand.name,
        settings = settings,
    )
}

/// Normalize an actor's display name, falling back to "Someone" when blank.
fn actor_or_default(actor_name: &str) -> &str {
    let trimmed = actor_name.trim();
    if trimmed.is_empty() {
        "Someone"
    } else {
        trimmed
    }
}

/// Minimal HTML escaping for interpolated, user-controlled values.
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// One labelled example email, for the offline preview/test tooling.
pub struct Sample {
    /// Filename-safe identifier (used for the preview `.html` file name).
    pub key: String,
    /// Human-readable label for the gallery.
    pub label: String,
    pub email: RenderedEmail,
}

/// Render one representative email for every template with placeholder data, so
/// the whole set can be eyeballed offline without touching the email service.
/// Backs the `preview-emails` CLI command.
pub fn samples(brand: &Brand) -> Vec<Sample> {
    let case = "Rivera Family — Custody Support";
    let actor = "Jordan Alvarez";

    let mut samples: Vec<Sample> = NotificationKind::ALL
        .iter()
        .filter(|k| **k != NotificationKind::Assigned)
        .map(|kind| {
            let detail = sample_detail(*kind);
            Sample {
                key: kind.slug().to_string(),
                label: kind.label().to_string(),
                email: case_event(brand, *kind, case, actor, detail),
            }
        })
        .collect();

    samples.push(Sample {
        key: NotificationKind::Assigned.slug().to_string(),
        label: NotificationKind::Assigned.label().to_string(),
        email: assignment(brand, case, actor),
    });
    samples
}

/// A believable `detail` sentence for each case-event kind, used only by the
/// preview/test samples.
fn sample_detail(kind: NotificationKind) -> &'static str {
    match kind {
        NotificationKind::NewMessage => {
            "posted a new message: \u{201C}I've uploaded the latest court filing\u{2026}\u{201D}"
        }
        NotificationKind::CaseData => "changed the status to \u{201C}In review\u{201D}",
        NotificationKind::NoteAdded => "added a note",
        NotificationKind::EvidenceChanged => "added evidence \u{201C}hearing-notes.pdf\u{201D}",
        NotificationKind::Assigned => "gave you access to the case",
    }
}
