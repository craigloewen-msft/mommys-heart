//! The AI reports workspace: describe a report, get a chart, a table and a CSV.
//!
//! The page is deliberately one input and one result. Everything that makes the
//! answer checkable — the queries the agent ran while it explored, and the final
//! SQL — is kept on the page but folded away, so the report reads as a report
//! and can still be audited when a number looks wrong.

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::chart::ReportChart;
use crate::components::guard::require_operations_admin;
use crate::components::layout::Layout;
use crate::server_fns::err_text;
use crate::server_fns::reports::{build_report, Report, MAX_REQUEST_CHARS};
use crate::state::AppState;

const PANEL: &str = "rounded-xl border border-slate-800 bg-slate-900 p-5";

/// Starting points, so the first use of the page is not a blank box.
const EXAMPLES: &[&str] = &[
    "Funding received by month this year, as a line chart",
    "How many cases are in each status",
    "Top 10 organizations by total funding, excluding voided records",
    "Volunteer hours logged per volunteer over the last 6 months",
];

#[component]
pub fn ReportsPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    require_operations_admin(state, move || {
        view! {
            <Layout title="Reports".to_string()>
                <ReportsWorkspace />
            </Layout>
        }
        .into_any()
    })
}

#[component]
fn ReportsWorkspace() -> impl IntoView {
    let request = RwSignal::new(String::new());
    let report = RwSignal::new(None::<Report>);
    let error = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let show_working = RwSignal::new(false);

    let run = move || {
        let text = request.get_untracked().trim().to_string();
        if text.is_empty() {
            error.set("Describe the report you want.".to_string());
            return;
        }
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        error.set(String::new());
        report.set(None);
        show_working.set(false);
        spawn_local(async move {
            match build_report(text).await {
                Ok(result) => report.set(Some(result)),
                Err(server_error) => error.set(err_text(server_error)),
            }
            busy.set(false);
        });
    };

    // Ctrl/Cmd+Enter submits, since the request box is a textarea.
    let on_keydown = move |event: leptos::ev::KeyboardEvent| {
        if event.key() == "Enter" && (event.ctrl_key() || event.meta_key()) {
            event.prevent_default();
            run();
        }
    };

    view! {
        <div class="space-y-6">
            <section class=PANEL>
                <label for="report-request" class="mb-1 block text-sm font-medium text-slate-200">
                    "What do you want to know?"
                </label>
                <p class="mb-3 text-xs text-slate-400">
                    "Describe the report in plain language. An assistant reads the database structure, runs read-only queries to check the data, then builds the chart and table. It can never change or delete anything."
                </p>
                <textarea
                    id="report-request"
                    rows="3"
                    maxlength=MAX_REQUEST_CHARS.to_string()
                    placeholder="e.g. Total funding received per month this year, as a line chart"
                    class="w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/40 disabled:cursor-not-allowed disabled:opacity-50"
                    prop:value=move || request.get()
                    on:input=move |event| request.set(event_target_value(&event))
                    on:keydown=on_keydown
                    disabled=move || busy.get()
                ></textarea>

                <div class="mt-3 flex flex-wrap items-center gap-3">
                    <button
                        on:click=move |_| run()
                        disabled=move || busy.get()
                        class="rounded-lg bg-primary-500 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-primary-400 disabled:cursor-not-allowed disabled:opacity-60"
                    >
                        {move || if busy.get() { "Building\u{2026}" } else { "Build report" }}
                    </button>
                    <span class="text-xs text-slate-500">"Ctrl+Enter to run"</span>
                </div>

                <div class="mt-4 flex flex-wrap gap-2">
                    {EXAMPLES
                        .iter()
                        .map(|example| {
                            view! {
                                <button
                                    on:click=move |_| request.set(example.to_string())
                                    disabled=move || busy.get()
                                    class="rounded-full border border-slate-700 px-3 py-1 text-xs text-slate-300 transition-colors hover:border-primary-500/40 hover:bg-slate-800 disabled:opacity-50"
                                >
                                    {*example}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
            </section>

            {move || {
                let message = error.get();
                (!message.is_empty())
                    .then(|| {
                        view! {
                            <p class="rounded-lg border border-rose-500/40 bg-rose-500/10 px-4 py-3 text-sm text-rose-200">
                                {message}
                            </p>
                        }
                    })
            }}

            {move || {
                busy.get()
                    .then(|| {
                        view! {
                            <div class=PANEL>
                                <div class="flex items-center gap-3 text-sm text-slate-400">
                                    <span class="h-4 w-4 animate-spin rounded-full border-2 border-slate-700 border-t-primary-500"></span>
                                    "Reading the database structure and checking the data\u{2026} this usually takes under a minute."
                                </div>
                            </div>
                        }
                    })
            }}

            {move || {
                report
                    .get()
                    .map(|result| view! { <ReportView report=result show_working /> })
            }}
        </div>
    }
}

#[component]
fn ReportView(report: Report, show_working: RwSignal<bool>) -> impl IntoView {
    let table = report.table.clone();
    let download_name = format!("{}.csv", slugify(&report.title));
    let download_href = csv_data_url(&report.csv);
    let row_label = format!(
        "{} row{}{}",
        table.rows.len(),
        if table.rows.len() == 1 { "" } else { "s" },
        if table.truncated {
            format!(
                " (capped at {})",
                crate::server_fns::reports::MAX_REPORT_ROWS
            )
        } else {
            String::new()
        },
    );
    let step_count = report.steps.len();
    let summary = report.summary.clone();

    view! {
        <section class="space-y-6">
            <div class=PANEL>
                <div class="flex flex-wrap items-start justify-between gap-4">
                    <div class="min-w-0">
                        <h2 class="text-lg font-semibold tracking-tight text-slate-100">
                            {report.title.clone()}
                        </h2>
                        {(!summary.is_empty())
                            .then(|| {
                                view! { <p class="mt-1 text-sm text-slate-400">{summary}</p> }
                            })}
                        <p class="mt-2 text-xs text-slate-500">{row_label}</p>
                    </div>
                    <a
                        href=download_href
                        download=download_name
                        class="shrink-0 rounded-lg border border-primary-500/40 bg-primary-500/10 px-4 py-2 text-sm font-medium text-primary-300 transition-colors hover:bg-primary-500/20"
                    >
                        "Export to CSV"
                    </a>
                </div>
            </div>

            <ReportChart table=report.table.clone() chart=report.chart.clone() />

            <ResultTable table=table.clone() />

            <div class=PANEL>
                <button
                    on:click=move |_| show_working.update(|open| *open = !*open)
                    class="text-sm font-medium text-slate-300 hover:text-slate-100"
                >
                    {move || {
                        if show_working.get() {
                            "Hide how this was built".to_string()
                        } else {
                            format!(
                                "Show how this was built ({step_count} exploratory quer{})",
                                if step_count == 1 { "y" } else { "ies" },
                            )
                        }
                    }}
                </button>

                <div class:hidden=move || !show_working.get() class="mt-4 space-y-4">
                    {report
                        .steps
                        .iter()
                        .map(|step| {
                            let outcome = if step.error.is_empty() {
                                format!("{} row(s)", step.row_count)
                            } else {
                                step.error.clone()
                            };
                            let outcome_class = if step.error.is_empty() {
                                "mt-1 text-xs text-slate-500"
                            } else {
                                "mt-1 text-xs text-amber-300"
                            };
                            view! {
                                <div class="rounded-lg border border-slate-800 bg-slate-950 p-3">
                                    <p class="text-xs font-medium text-slate-300">
                                        {step.note.clone()}
                                    </p>
                                    <pre class="mt-2 overflow-x-auto text-xs leading-relaxed text-slate-400">
                                        {step.sql.clone()}
                                    </pre>
                                    <p class=outcome_class>{outcome}</p>
                                </div>
                            }
                        })
                        .collect_view()}

                    <div>
                        <p class="text-xs font-medium text-slate-300">"Final query"</p>
                        <pre class="mt-2 overflow-x-auto rounded-lg border border-slate-800 bg-slate-950 p-3 text-xs leading-relaxed text-slate-300">
                            {report.sql.clone()}
                        </pre>
                    </div>
                </div>
            </div>
        </section>
    }
}

#[component]
fn ResultTable(table: crate::server_fns::reports::ReportTable) -> impl IntoView {
    if table.columns.is_empty() {
        return ().into_any();
    }
    if table.rows.is_empty() {
        return view! {
            <div class=PANEL>
                <p class="text-sm text-slate-400">"That query matched no rows."</p>
            </div>
        }
        .into_any();
    }

    let numeric: Vec<bool> = table.columns.iter().map(|c| c.numeric).collect();

    view! {
        <div class="overflow-x-auto rounded-xl border border-slate-800 bg-slate-900">
            <table class="min-w-full text-sm">
                <thead class="border-b border-slate-800 bg-slate-900/60">
                    <tr>
                        {table
                            .columns
                            .iter()
                            .map(|column| {
                                let align = if column.numeric { "text-right" } else { "text-left" };
                                view! {
                                    <th class=format!(
                                        "whitespace-nowrap px-4 py-2.5 font-medium text-slate-300 {align}",
                                    )>{column.name.clone()}</th>
                                }
                            })
                            .collect_view()}
                    </tr>
                </thead>
                <tbody>
                    {table
                        .rows
                        .iter()
                        .map(|row| {
                            let numeric = numeric.clone();
                            view! {
                                <tr class="border-b border-slate-800/60 last:border-0 hover:bg-slate-800/40">
                                    {row
                                        .iter()
                                        .enumerate()
                                        .map(|(index, cell)| {
                                            let align = if numeric.get(index).copied().unwrap_or(false) {
                                                "text-right tabular-nums"
                                            } else {
                                                "text-left"
                                            };
                                            view! {
                                                <td class=format!(
                                                    "px-4 py-2 text-slate-300 {align}",
                                                )>{cell.clone()}</td>
                                            }
                                        })
                                        .collect_view()}
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
    .into_any()
}

/// A `data:` URL the browser saves straight to disk.
///
/// The CSV is already in hand, so there is nothing to fetch: an ordinary
/// `<a download>` is enough and the export needs no extra endpoint. The
/// leading BOM is what makes Excel read UTF-8 accents correctly.
fn csv_data_url(csv: &str) -> String {
    let mut encoded = String::with_capacity(csv.len() * 3);
    for byte in format!("\u{feff}{csv}").bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char)
            }
            other => encoded.push_str(&format!("%{other:02X}")),
        }
    }
    format!("data:text/csv;charset=utf-8,{encoded}")
}

/// A filename-safe version of the report title.
fn slugify(title: &str) -> String {
    let slug: String = title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "report".to_string()
    } else {
        slug.chars().take(60).collect()
    }
}
