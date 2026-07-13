//! Email color palette derived from the app's Tailwind theme (SSR only).
//!
//! Emails can't use CSS variables or external stylesheets — colors have to be
//! inlined as literal hex — so this module reads the same `--color-*` tokens the
//! UI uses from `style/tailwind.css` (embedded at compile time via
//! [`include_str!`]) and resolves them to hex. That keeps `style/tailwind.css`
//! the single source of truth: the palette is defined once and the emails follow
//! any change to the theme.
//!
//! [`color`] resolves an arbitrary token (e.g. `"primary-600"`); the named
//! helpers below give the small set of semantic roles the templates need.

use std::collections::HashMap;
use std::sync::OnceLock;

/// The app's Tailwind stylesheet, embedded so there is no runtime file
/// dependency and the tokens stay in lock-step with the compiled binary.
const TAILWIND_CSS: &str = include_str!("../../../style/tailwind.css");

/// Parse the `--color-<token>: <hex>;` declarations from `style/tailwind.css`
/// once, into a `token -> hex` map (e.g. `"primary-600" -> "#b45309"`).
fn tokens() -> &'static HashMap<String, String> {
    static TOKENS: OnceLock<HashMap<String, String>> = OnceLock::new();
    TOKENS.get_or_init(|| {
        TAILWIND_CSS
            .lines()
            .filter_map(|line| {
                let rest = line.trim().strip_prefix("--color-")?;
                let (name, value) = rest.split_once(':')?;
                let value = value.trim().trim_end_matches(';').trim();
                Some((name.trim().to_string(), value.to_string()))
            })
            .collect()
    })
}

/// Resolve a Tailwind color token (without the `--color-` prefix, e.g.
/// `"slate-500"`) to its hex value. Falls back to black if the token is missing
/// so a stray reference degrades visibly rather than panicking in production.
pub fn color(token: &str) -> &'static str {
    tokens().get(token).map(String::as_str).unwrap_or("#000000")
}

/// Warm gold — the primary accent (buttons, header rule).
pub fn primary() -> &'static str {
    color("primary-600")
}
/// Page background behind the email card.
pub fn canvas() -> &'static str {
    color("slate-950")
}
/// The card surface.
pub fn card() -> &'static str {
    color("slate-900")
}
/// Primary body text.
pub fn text() -> &'static str {
    color("slate-200")
}
/// Muted secondary text (footer, helper lines).
pub fn muted() -> &'static str {
    color("slate-500")
}
/// Hairline borders.
pub fn border() -> &'static str {
    color("slate-800")
}
/// Tinted callout / footer background.
pub fn callout_bg() -> &'static str {
    color("slate-950")
}
