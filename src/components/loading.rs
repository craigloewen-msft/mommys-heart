//! A small, reusable loading indicator.
//!
//! Detail views fetch their data asynchronously in the browser, so while a
//! request is in flight they render this instead of an empty or misleading
//! "nothing here" state. It is a spinning ring next to a short label.

use leptos::prelude::*;

/// An inline loading indicator: a spinning ring beside a short label.
#[component]
pub fn Loading(
    /// Text shown beside the spinner.
    #[prop(into, default = "Loading\u{2026}".to_string())]
    label: String,
) -> impl IntoView {
    view! {
        <div class="flex items-center justify-center gap-3 p-6 text-sm text-slate-400">
            <span class="h-4 w-4 animate-spin rounded-full border-2 border-slate-700 border-t-primary-500"></span>
            {label}
        </div>
    }
}
