//! The admin **Cases** directory: every case in the system, searchable,
//! filterable, and paginated.
//!
//! This is a management view, not a review queue. Accepting or declining a case
//! is one action available on a row -- the same row that also shows who owns it,
//! who is working it, and whether it has gone quiet. Splitting "decide" into a
//! screen of its own would hide the context needed to make the decision.

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::server_fns::cases::{
    admin_list_cases_page, set_case_review_state, CaseListFilter, CaseReviewState, CaseSummary,
};
use crate::server_fns::err_text;
use crate::state::AppState;

/// Rows per "page"; each "Load more" grows the visible window by this much.
const PAGE_SIZE: i64 = 10;

fn badge(classes: &str) -> String {
    format!("rounded-full px-2 py-0.5 text-xs font-medium {classes}")
}

#[component]
pub fn AdminCaseDirectory(reload: RwSignal<u32>) -> impl IntoView {
    let state = expect_context::<AppState>();
    let query = RwSignal::new(String::new());
    let debounced_query = RwSignal::new(String::new());
    let filter = RwSignal::new(CaseListFilter::All);
    let results = RwSignal::new(Vec::<CaseSummary>::new());
    let total = RwSignal::new(0i64);
    let window = RwSignal::new(PAGE_SIZE);
    let loading = RwSignal::new(false);
    let load_error = RwSignal::new(None::<String>);

    // One fetch path for search, filter, paging, and post-mutation refresh: each
    // of them only changes an input this effect already reads.
    Effect::new(move |_| {
        let count = window.get();
        let q = debounced_query.get();
        let f = filter.get();
        reload.track();
        if !state.has_operations_admin_permissions() {
            return;
        }
        loading.set(true);
        spawn_local(async move {
            match admin_list_cases_page(0, count, q, f).await {
                Ok(page) => {
                    results.set(page.items);
                    total.set(page.total);
                    load_error.set(None);
                }
                Err(e) => load_error.set(Some(err_text(e))),
            }
            loading.set(false);
        });
    });

    let filter_chips = move || {
        CaseListFilter::ALL
            .into_iter()
            .map(|f| {
                view! {
                    <button
                        type="button"
                        on:click=move |_| {
                            // Changing the lens resets the window, so the admin
                            // always lands at the top of the new list.
                            window.set(PAGE_SIZE);
                            filter.set(f);
                        }
                        class=move || {
                            let base = "rounded-full px-3 py-1 text-xs font-medium ring-1 transition-colors";
                            if filter.get() == f {
                                format!("{base} bg-primary-500/20 text-primary-200 ring-primary-500/40")
                            } else {
                                format!("{base} text-slate-400 ring-slate-700 hover:bg-slate-800")
                            }
                        }
                    >
                        <span class="inline-flex items-center gap-1.5">
                            {f.label()}
                            {move || {
                                // Only the pending lens carries a count: it is
                                // the one that means "someone is waiting".
                                let count = if f == CaseListFilter::PendingReview {
                                    state.cases_pending_review.get()
                                } else {
                                    0
                                };
                                (count > 0).then(|| view! {
                                    <span class="inline-flex min-w-4 items-center justify-center rounded-full bg-primary-500 px-1 py-0.5 text-[0.6rem] font-semibold leading-none text-white">
                                        {count}
                                    </span>
                                })
                            }}
                        </span>
                    </button>
                }
            })
            .collect_view()
    };

    let list = move || {
        if let Some(msg) = load_error.get() {
            return view! { <p class="text-sm text-rose-300">"Could not load cases: " {msg}</p> }
                .into_any();
        }
        let items = results.get();
        if items.is_empty() {
            let text = if loading.get() {
                "Loading\u{2026}"
            } else {
                "No cases match this view."
            };
            return view! { <p class="text-sm text-slate-500">{text}</p> }.into_any();
        }
        items
            .into_iter()
            .map(|c| view! { <CaseRow case=c reload=reload /> }.into_any())
            .collect_view()
            .into_any()
    };

    let footer = move || {
        let shown = results.get().len() as i64;
        let tot = total.get();
        if tot == 0 {
            return ().into_any();
        }
        let more = shown < tot;
        view! {
            <div class="mt-4 flex items-center justify-between">
                <p class="text-xs text-slate-500">"Showing " {shown} " of " {tot}</p>
                <Show when=move || more>
                    <button
                        on:click=move |_| window.update(|w| *w += PAGE_SIZE)
                        prop:disabled=move || loading.get()
                        class="rounded-lg border border-slate-700 px-3 py-1.5 text-sm font-medium text-slate-200 hover:bg-slate-800 disabled:opacity-50"
                    >
                        {move || if loading.get() { "Loading\u{2026}" } else { "Load more" }}
                    </button>
                </Show>
            </div>
        }
        .into_any()
    };

    // Same debounce the user directory uses: type freely, fetch once you stop.
    let mut on_search = debounce(std::time::Duration::from_secs(1), move |val: String| {
        window.set(PAGE_SIZE);
        debounced_query.set(val);
    });

    view! {
        <input
            class="mb-3 w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/40"
            placeholder="Search cases by name, id, or client"
            prop:value=move || query.get()
            on:input=move |ev| {
                let val = event_target_value(&ev);
                query.set(val.clone());
                on_search(val);
            }
        />
        <div class="mb-4 flex flex-wrap gap-2">{filter_chips}</div>
        <div class="space-y-3">{list}</div>
        {footer}
    }
}

/// One case in the directory: what it is, who has it, and what can be done about
/// it right now.
#[component]
fn CaseRow(case: CaseSummary, reload: RwSignal<u32>) -> impl IntoView {
    let state = expect_context::<AppState>();
    let case_id = StoredValue::new(case.id.clone());
    let work = case.work_state();
    let review_state = case.review_state;
    let pending = review_state == CaseReviewState::PendingReview;
    let saving = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    // Declining opens an inline reason field rather than a modal: the reason is
    // part of the decision, and the admin should still be able to see the case
    // they are declining while writing it.
    let declining = RwSignal::new(false);
    let reason = RwSignal::new(String::new());

    let decide = move |next: CaseReviewState, reason_text: String| {
        saving.set(true);
        error.set(None);
        spawn_local(async move {
            match set_case_review_state(case_id.get_value(), next, reason_text).await {
                Ok(()) => {
                    declining.set(false);
                    reason.set(String::new());
                    // Refresh the list and the badge together, so the row and the
                    // count can never disagree about what is still pending.
                    state.refresh_cases_pending_review();
                    reload.update(|r| *r += 1);
                }
                Err(e) => error.set(Some(err_text(e))),
            }
            saving.set(false);
        });
    };

    let accept = move |_| decide(CaseReviewState::Accepted, String::new());
    let confirm_decline = move |_| {
        let text = reason.get_untracked().trim().to_string();
        if text.is_empty() {
            error.set(Some(
                "Please give a reason so the client knows why.".to_string(),
            ));
            return;
        }
        decide(CaseReviewState::Declined, text);
    };

    let unstaffed = case.assigned_volunteers.is_empty();
    let assigned_text = if unstaffed {
        "Unstaffed".to_string()
    } else {
        case.assigned_volunteers.join(", ")
    };
    let inactive_badge = if case.inactive {
        view! {
            <span class=badge("bg-amber-500/15 text-amber-300 ring-1 ring-inset ring-amber-500/30")>
                "\u{26a0} No activity in 30+ days"
            </span>
        }
        .into_any()
    } else {
        ().into_any()
    };

    view! {
        <div class="rounded-xl border border-slate-800 bg-slate-900 p-4">
            <div class="flex flex-wrap items-center justify-between gap-2">
                <span class="min-w-0 truncate font-medium text-slate-100">{case.name.clone()}</span>
                <div class="flex flex-wrap items-center gap-2">
                    <span class=badge(case.status.badge_classes())>{case.status.label()}</span>
                    <span class=badge(work.badge_classes())>{work.label()}</span>
                </div>
            </div>
            <div class="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-slate-400">
                <span>"Client: " {case.owner_full_name()}</span>
                <span>"Working it: " {assigned_text}</span>
                {inactive_badge}
            </div>

            {move || {
                error.get().map(|msg| view! { <p class="mt-2 text-xs text-rose-300">{msg}</p> })
            }}

            <Show when=move || !declining.get()>
                <div class="mt-3 flex flex-wrap items-center gap-2">
                    // Accept stays available on a declined case and decline on an
                    // accepted one: a decision made in error should be fixable
                    // here rather than in the database.
                    <Show when=move || pending || review_state == CaseReviewState::Declined>
                        <button
                            on:click=accept
                            prop:disabled=move || saving.get()
                            class="rounded-lg bg-emerald-600 px-3 py-1.5 text-xs font-semibold text-white hover:bg-emerald-500 disabled:opacity-50"
                        >
                            {if pending { "Accept" } else { "Accept instead" }}
                        </button>
                    </Show>
                    <Show when=move || pending || review_state == CaseReviewState::Accepted>
                        <button
                            on:click=move |_| declining.set(true)
                            prop:disabled=move || saving.get()
                            class="rounded-lg border border-rose-500/40 px-3 py-1.5 text-xs font-semibold text-rose-300 hover:bg-rose-500/10 disabled:opacity-50"
                        >
                            {if pending { "Decline" } else { "Decline instead" }}
                        </button>
                    </Show>
                </div>
            </Show>

            <Show when=move || declining.get()>
                <div class="mt-3 space-y-2">
                    <label class="block text-xs font-medium text-slate-300">
                        "Reason (shown to the client)"
                    </label>
                    <textarea
                        rows="2"
                        class="w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none"
                        placeholder="e.g. Outside our service area; referred to Lakeside Legal Aid."
                        prop:value=move || reason.get()
                        on:input=move |ev| reason.set(event_target_value(&ev))
                    ></textarea>
                    <div class="flex gap-2">
                        <button
                            on:click=confirm_decline
                            prop:disabled=move || saving.get()
                            class="rounded-lg bg-rose-600 px-3 py-1.5 text-xs font-semibold text-white hover:bg-rose-500 disabled:opacity-50"
                        >
                            {move || if saving.get() { "Saving\u{2026}" } else { "Confirm decline" }}
                        </button>
                        <button
                            on:click=move |_| {
                                declining.set(false);
                                error.set(None);
                            }
                            class="rounded-lg border border-slate-700 px-3 py-1.5 text-xs font-medium text-slate-300 hover:bg-slate-800"
                        >
                            "Cancel"
                        </button>
                    </div>
                </div>
            </Show>

            // Accepting a case is only half of getting somebody helped; the next
            // step is staffing it. Say so, right where the gap is visible.
            <Show when=move || review_state == CaseReviewState::Accepted && unstaffed>
                <p class="mt-3 text-xs text-slate-500">
                    "Accepted, but nobody is assigned yet \u{2014} grant a volunteer access from the Case access tab to start work."
                </p>
            </Show>
        </div>
    }
}
