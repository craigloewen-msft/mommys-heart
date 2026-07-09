use leptos::prelude::*;

use crate::components::guard::deny_redirect;
use crate::components::layout::Layout;
use crate::mockdata::ORG_NAME;
use crate::state::AppState;
use crate::types::{CaseStatus, Permission};

/// Admin/Staff-only data-governance view: records-retention status, legal holds,
/// disposal eligibility, and an explicit statement of data ownership.
#[component]
pub fn GovernancePage() -> impl IntoView {
    let state = expect_context::<AppState>();

    if let Some(redirect) = deny_redirect(&state, Permission::ManageRetention) {
        return redirect;
    }

    let rows = move || {
        state
            .cases
            .get()
            .into_iter()
            .map(|c| {
                let hold_id = c.id.clone();
                let dispose_id = c.id.clone();
                let eligible = state.is_disposal_eligible(&c);
                let (retention_label, disposal_text, disposal_class) = if c.legal_hold {
                    (c.retention.label(), "On legal hold", "text-rose-300")
                } else if eligible {
                    (c.retention.label(), "Eligible for disposal", "text-amber-300")
                } else if c.status == CaseStatus::Closed {
                    (c.retention.label(), "Retained", "text-slate-400")
                } else {
                    (c.retention.label(), "Active — retained", "text-slate-400")
                };
                view! {
                    <tr class="border-b border-slate-800 last:border-0 hover:bg-slate-800/40">
                        <td class="px-4 py-3">
                            <p class="font-medium text-slate-100">{c.title.clone()}</p>
                            <p class="text-xs text-slate-500">{c.status.label()}</p>
                        </td>
                        <td class="px-4 py-3 text-slate-300">{retention_label}</td>
                        <td class="px-4 py-3">
                            <p class="text-slate-200">{c.steward.clone()}</p>
                            <p class="text-xs text-slate-500">
                                "Owner: " {ORG_NAME}
                            </p>
                        </td>
                        <td class=format!("px-4 py-3 {disposal_class}")>{disposal_text}</td>
                        <td class="px-4 py-3">
                            <div class="flex flex-wrap gap-2">
                                <button
                                    on:click=move |_| state.set_legal_hold(&hold_id, !c.legal_hold)
                                    class="rounded-lg border border-slate-700 px-2.5 py-1 text-xs font-medium text-slate-200 hover:bg-slate-800"
                                >
                                    {if c.legal_hold { "Release hold" } else { "Place hold" }}
                                </button>
                                <button
                                    prop:disabled=!eligible
                                    on:click=move |_| {
                                        let _ = state.dispose_case(&dispose_id);
                                    }
                                    class=if eligible {
                                        "rounded-lg border border-rose-500/40 px-2.5 py-1 text-xs font-medium text-rose-300 hover:bg-rose-500/10"
                                    } else {
                                        "rounded-lg border border-slate-800 px-2.5 py-1 text-xs font-medium text-slate-600 cursor-not-allowed"
                                    }
                                >
                                    "Dispose"
                                </button>
                            </div>
                        </td>
                    </tr>
                }
            })
            .collect_view()
    };

    view! {
        <Layout title="Data governance">
            <div class="mb-6 rounded-xl border border-slate-800 bg-slate-900 p-5">
                <h2 class="text-sm font-semibold text-white">"Data ownership"</h2>
                <p class="mt-1 max-w-3xl text-sm text-slate-400">
                    "All cases, documents, communications, and knowledge are owned by "
                    <span class="font-medium text-slate-200">{ORG_NAME}</span>
                    ", not by any individual volunteer, intern, or staff member. Individuals
                    are recorded as contributors and stewards for provenance, but the
                    organization retains the records when a person leaves."
                </p>
            </div>

            <p class="mb-3 text-sm text-slate-400">
                "Records-retention policy (demo). Standard = 7 years, Extended = 10 years,
                Permanent = never auto-dispose. A legal hold blocks disposal regardless of
                policy. Only closed, non-permanent records without a hold are eligible."
            </p>

            <div class="overflow-hidden rounded-xl border border-slate-800 bg-slate-900">
                <table class="w-full text-sm">
                    <thead class="border-b border-slate-800 text-left text-slate-400">
                        <tr>
                            <th class="px-4 py-3 font-medium">"Record"</th>
                            <th class="px-4 py-3 font-medium">"Retention"</th>
                            <th class="px-4 py-3 font-medium">"Steward / owner"</th>
                            <th class="px-4 py-3 font-medium">"Disposal status"</th>
                            <th class="px-4 py-3 font-medium">"Actions"</th>
                        </tr>
                    </thead>
                    <tbody>{rows}</tbody>
                </table>
            </div>
        </Layout>
    }
    .into_any()
}
