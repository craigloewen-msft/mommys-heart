//! Shared request center for site-admin approvals and operations-admin history.

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::server_fns::admin_requests::{
    decide_admin_request, list_active_admin_requests, list_admin_request_history, AdminRequest,
    AdminRequestStatus,
};
use crate::server_fns::err_text;
use crate::state::AppState;

const HISTORY_PAGE_SIZE: i64 = 20;

fn status_classes(status: AdminRequestStatus) -> &'static str {
    match status {
        AdminRequestStatus::Pending => "bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/30",
        AdminRequestStatus::Approved => {
            "bg-emerald-500/15 text-emerald-300 ring-1 ring-emerald-500/30"
        }
        AdminRequestStatus::Denied => "bg-rose-500/15 text-rose-300 ring-1 ring-rose-500/30",
    }
}

#[component]
pub fn AdminRequestCenter(is_site_admin: bool, reload: RwSignal<u32>) -> impl IntoView {
    let state = expect_context::<AppState>();
    let active = RwSignal::new(Vec::<AdminRequest>::new());
    let history = RwSignal::new(Vec::<AdminRequest>::new());
    let history_total = RwSignal::new(0i64);
    let history_window = RwSignal::new(HISTORY_PAGE_SIZE);
    let history_open = RwSignal::new(false);
    let active_loading = RwSignal::new(true);
    let history_loading = RwSignal::new(false);
    let active_error = RwSignal::new(None::<String>);
    let history_error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        reload.track();
        if !state.is_authenticated() {
            return;
        }
        active_loading.set(true);
        spawn_local(async move {
            match list_active_admin_requests().await {
                Ok(items) => {
                    if is_site_admin {
                        state.admin_request_pending.set(items.len() as i64);
                    }
                    active.set(items);
                    active_error.set(None);
                }
                Err(error) => active_error.set(Some(err_text(error))),
            }
            active_loading.set(false);
        });
    });

    Effect::new(move |_| {
        if !history_open.get() {
            return;
        }
        let count = history_window.get();
        reload.track();
        if !state.is_authenticated() {
            return;
        }
        history_loading.set(true);
        spawn_local(async move {
            match list_admin_request_history(0, count).await {
                Ok(page) => {
                    history.set(page.items);
                    history_total.set(page.total);
                    history_error.set(None);
                }
                Err(error) => history_error.set(Some(err_text(error))),
            }
            history_loading.set(false);
        });
    });

    let request_list = move |items: Vec<AdminRequest>, actionable: bool| {
        items
            .into_iter()
            .map(|request| {
                view! {
                    <RequestCard request=request actionable=actionable reload=reload />
                }
            })
            .collect_view()
    };

    let active_section = move || {
        if let Some(error) = active_error.get() {
            return view! { <p class="text-sm text-rose-300">{error}</p> }.into_any();
        }
        if active_loading.get() && active.with(Vec::is_empty) {
            return view! { <p class="text-sm text-slate-500">"Loading requests..."</p> }
                .into_any();
        }
        let items = active.get();
        if items.is_empty() {
            view! {
                <div class="border-l-2 border-emerald-500/50 py-2 pl-3">
                    <p class="text-sm text-slate-400">
                        {if is_site_admin {
                            "No requests are waiting for review."
                        } else {
                            "You have no active requests."
                        }}
                    </p>
                </div>
            }
            .into_any()
        } else {
            view! { <div class="space-y-3">{request_list(items, is_site_admin)}</div> }.into_any()
        }
    };

    let history_section = move || {
        if let Some(error) = history_error.get() {
            return view! { <p class="text-sm text-rose-300">{error}</p> }.into_any();
        }
        if history_loading.get() && history.with(Vec::is_empty) {
            return view! { <p class="text-sm text-slate-500">"Loading history..."</p> }.into_any();
        }
        let items = history.get();
        if items.is_empty() {
            return view! {
                <p class="text-sm text-slate-500">"No requests have been decided yet."</p>
            }
            .into_any();
        }
        let shown = items.len() as i64;
        let total = history_total.get();
        let footer = if shown < total {
            view! {
                <div class="mt-3 flex items-center gap-3">
                    <button
                        type="button"
                        on:click=move |_| history_window.update(|window| *window += HISTORY_PAGE_SIZE)
                        prop:disabled=move || history_loading.get()
                        class="rounded-lg border border-slate-700 px-2.5 py-1.5 text-xs font-medium text-slate-300 hover:bg-slate-800 disabled:opacity-50"
                    >
                        {move || if history_loading.get() { "Loading..." } else { "Load more" }}
                    </button>
                    <span class="text-xs text-slate-500">"Showing " {shown} " of " {total}</span>
                </div>
            }
            .into_any()
        } else {
            view! { <p class="mt-3 text-xs text-slate-500">{total} " request(s) in history"</p> }
                .into_any()
        };
        view! {
            <div class="space-y-3">{request_list(items, false)}</div>
            {footer}
        }
        .into_any()
    };

    view! {
        <div class="space-y-8">
            <section>
                <div class="mb-4 flex flex-wrap items-end justify-between gap-3">
                    <div class="flex items-center gap-2">
                        <h2 class="text-base font-semibold text-slate-100">"Active requests"</h2>
                        <span class="inline-flex min-w-5 items-center justify-center rounded-full bg-amber-500/15 px-1.5 py-0.5 text-xs font-semibold text-amber-300">
                            {move || active.get().len()}
                        </span>
                    </div>
                    <button
                        type="button"
                        on:click=move |_| reload.update(|value| *value += 1)
                        prop:disabled=move || {
                            active_loading.get()
                                || (history_open.get() && history_loading.get())
                        }
                        class="rounded-lg border border-slate-700 px-2.5 py-1.5 text-xs font-medium text-slate-300 hover:bg-slate-800 disabled:opacity-50"
                    >
                        "Refresh"
                    </button>
                </div>
                {active_section}
            </section>
            <section class="border-t border-slate-800 pt-6">
                <h2>
                    <button
                        type="button"
                        on:click=move |_| history_open.update(|open| *open = !*open)
                        aria-expanded=move || history_open.get().to_string()
                        aria-controls="admin-request-history"
                        class="flex w-full items-center justify-between gap-3 text-left"
                    >
                        <span class="text-base font-semibold text-slate-100">"Request history"</span>
                        <span class="text-xs font-medium text-slate-400">
                            {move || if history_open.get() { "Hide" } else { "Show" }}
                        </span>
                    </button>
                </h2>
                <Show when=move || history_open.get()>
                    <div id="admin-request-history" class="mt-4">
                        {history_section}
                    </div>
                </Show>
            </section>
        </div>
    }
}

#[component]
fn RequestCard(request: AdminRequest, actionable: bool, reload: RwSignal<u32>) -> impl IntoView {
    let request_id = StoredValue::new(request.id.clone());
    let decision_note = RwSignal::new(String::new());
    let deciding = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    let decide = move |approve: bool| {
        if deciding.get_untracked() {
            return;
        }
        deciding.set(true);
        error.set(None);
        let request_id = request_id.get_value();
        let note = decision_note.get_untracked();
        spawn_local(async move {
            match decide_admin_request(request_id, approve, note).await {
                Ok(_) => reload.update(|value| *value += 1),
                Err(request_error) => error.set(Some(err_text(request_error))),
            }
            deciding.set(false);
        });
    };

    let context = request
        .case_name
        .as_deref()
        .map(|case| format!("Case: {case}"));
    let decided = match (&request.decided_by_name, &request.decided_at) {
        (Some(name), Some(at)) => Some(format!("Decided by {name} on {at}")),
        _ => None,
    };
    let status_class = format!(
        "inline-flex rounded-full px-2 py-0.5 text-xs font-medium {}",
        status_classes(request.status)
    );
    let status_label = request.status.label();
    let kind_label = request.kind.label();
    let change_summary = request.change_summary();
    let requested_by_name = request.requested_by_name;
    let target_user_name = request.target_user_name;
    let created_at = request.created_at;
    let request_note = request.request_note;
    let has_request_note = !request_note.is_empty();
    let decision_note_text = request.decision_note;
    let has_decision_note = !decision_note_text.is_empty();

    view! {
        <article class="rounded-lg border border-slate-800 bg-slate-900 p-4">
            <div class="flex flex-wrap items-start justify-between gap-3">
                <div class="min-w-0">
                    <div class="flex flex-wrap items-center gap-2">
                        <span class=status_class>{status_label}</span>
                        <span class="text-xs font-medium text-slate-400">{kind_label}</span>
                    </div>
                    <p class="mt-2 text-sm text-slate-200">
                        <span class="font-semibold">{requested_by_name}</span>
                        " requested a change for "
                        <span class="font-semibold">{target_user_name}</span>
                        "."
                    </p>
                    <p class="mt-1 text-sm text-slate-400">{change_summary}</p>
                    {context.map(|context| view! { <p class="mt-1 text-xs text-slate-500">{context}</p> })}
                </div>
                <time class="shrink-0 text-xs text-slate-500">{created_at}</time>
            </div>
            <Show when=move || has_request_note>
                <p class="mt-3 border-l-2 border-slate-700 pl-3 text-xs text-slate-400">
                    {request_note.clone()}
                </p>
            </Show>
            {decided.map(|decided| view! { <p class="mt-3 text-xs text-slate-500">{decided}</p> })}
            <Show when=move || has_decision_note>
                <p class="mt-1 text-xs text-slate-400">
                    "Decision note: " {decision_note_text.clone()}
                </p>
            </Show>
            <Show when=move || actionable>
                <div class="mt-4 flex flex-col gap-2 sm:flex-row sm:items-center">
                    <input
                        class="min-w-0 flex-1 rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500"
                        placeholder="Decision note (optional)"
                        maxlength="1000"
                        prop:value=move || decision_note.get()
                        on:input=move |event| decision_note.set(event_target_value(&event))
                    />
                    <button
                        type="button"
                        on:click=move |_| decide(true)
                        prop:disabled=move || deciding.get()
                        class="rounded-lg bg-emerald-600 px-3 py-2 text-sm font-semibold text-white hover:bg-emerald-500 disabled:opacity-50"
                    >
                        "Approve"
                    </button>
                    <button
                        type="button"
                        on:click=move |_| decide(false)
                        prop:disabled=move || deciding.get()
                        class="rounded-lg border border-rose-500/40 px-3 py-2 text-sm font-semibold text-rose-300 hover:bg-rose-500/10 disabled:opacity-50"
                    >
                        "Deny"
                    </button>
                </div>
            </Show>
            <Show when=move || error.get().is_some()>
                <p class="mt-2 text-xs text-rose-300">{move || error.get().unwrap_or_default()}</p>
            </Show>
        </article>
    }
}
