//! Formatting helpers shared by the server and the UI.

/// The class list for a small rounded status pill, so every badge in the app is
/// shaped the same and only its colours differ.
pub fn badge_pill(colors: &str) -> String {
    format!("inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {colors}")
}

/// Format a byte count as a short human-readable size (e.g. "2.4 MB").
pub fn human_size(bytes: i64) -> String {
    let bytes = bytes.max(0) as f64;
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    if bytes >= MB {
        format!("{:.1} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes / KB)
    } else {
        format!("{bytes:.0} B")
    }
}
