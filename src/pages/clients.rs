//! Client-centric case management: a list of everyone served, and a per-client
//! "pathway" view that shows all of a client's interconnected cases together
//! rather than as isolated interactions.

use leptos::prelude::*;
use leptos_router::components::{Redirect, A};
use leptos_router::hooks::use_params_map;

use crate::components::case_card::CaseCard;
use crate::components::layout::Layout;
use crate::state::AppState;
use crate::types::{CaseStatus, Role};

/// Admin-only directory of clients, each summarizing how many needs (cases) are
/// in flight, plus a form to intake a new client.
#[component]
pub fn ClientsPage() -> impl IntoView {
    let state = expect_context::<AppState>();

    match state.current_user.get_untracked() {
        Some(u) if u.role == Role::Admin => {}
        Some(_) => return view! { <Redirect path="/volunteer" /> }.into_any(),
        None => return view! { <Redirect path="/login" /> }.into_any(),
    }

    let input_class = "w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/40";

    let ncl_name = RwSignal::new(String::new());
    let ncl_phone = RwSignal::new(String::new());
    let ncl_summary = RwSignal::new(String::new());
    let ncl_error = RwSignal::new(String::new());
    let add_client = move |_| match state.add_client(
        &ncl_name.get(),
        "",
        &ncl_phone.get(),
        &ncl_summary.get(),
    ) {
        Ok(_) => {
            ncl_name.set(String::new());
            ncl_phone.set(String::new());
            ncl_summary.set(String::new());
            ncl_error.set(String::new());
        }
        Err(e) => ncl_error.set(e),
    };

    let client_rows = move || {
        let cases = state.cases.get();
        state
            .clients
            .get()
            .into_iter()
            .map(|cl| {
                let client_cases: Vec<_> =
                    cases.iter().filter(|c| c.client_id == cl.id).collect();
                let total = client_cases.len();
                let open = client_cases
                    .iter()
                    .filter(|c| c.status != CaseStatus::Closed)
                    .count();
                let href = format!("/clients/{}", cl.id);
                view! {
                    <A
                        href=href
                        attr:class="block rounded-xl border border-slate-800 bg-slate-900 p-5 transition-colors hover:border-primary-500/40 hover:bg-slate-800/60"
                    >
                        <div class="flex items-start justify-between gap-3">
                            <div>
                                <h3 class="font-semibold text-white">{cl.display_name.clone()}</h3>
                                <p class="mt-1 text-xs text-slate-500">
                                    "Intake " {cl.intake_date.clone()}
                                </p>
                            </div>
                            <span class="shrink-0 rounded-full bg-primary-500/15 px-2 py-0.5 text-xs font-medium text-primary-300 ring-1 ring-primary-500/30">
                                {format!("{total} need{}", if total == 1 { "" } else { "s" })}
                            </span>
                        </div>
                        <p class="mt-2 line-clamp-2 text-sm text-slate-400">{cl.summary.clone()}</p>
                        <p class="mt-3 text-xs text-slate-500">
                            {format!("{open} open \u{00b7} {total} total")}
                        </p>
                    </A>
                }
            })
            .collect_view()
    };

    view! {
        <Layout title="Clients">
            <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">{client_rows}</div>

            <div class="mt-6 rounded-xl border border-slate-800 bg-slate-900 p-4">
                <p class="mb-2 text-xs font-medium uppercase tracking-wide text-slate-400">
                    "Intake a new client"
                </p>
                <div class="grid gap-2 sm:grid-cols-2">
                    <input
                        class=input_class
                        placeholder="Name or reference (e.g. Client E.)"
                        prop:value=move || ncl_name.get()
                        on:input=move |ev| ncl_name.set(event_target_value(&ev))
                    />
                    <input
                        class=input_class
                        placeholder="Phone (optional)"
                        prop:value=move || ncl_phone.get()
                        on:input=move |ev| ncl_phone.set(event_target_value(&ev))
                    />
                </div>
                <textarea
                    class=format!("{input_class} mt-2")
                    rows="2"
                    placeholder="Presenting needs / summary"
                    prop:value=move || ncl_summary.get()
                    on:input=move |ev| ncl_summary.set(event_target_value(&ev))
                ></textarea>
                <div class="mt-2 flex items-center gap-3">
                    <button
                        on:click=add_client
                        class="rounded-lg bg-primary-500 px-4 py-2 text-sm font-semibold text-white hover:bg-primary-600"
                    >
                        "Add client"
                    </button>
                    <Show when=move || !ncl_error.get().is_empty()>
                        <p class="text-xs text-rose-400">{move || ncl_error.get()}</p>
                    </Show>
                </div>
            </div>
        </Layout>
    }
    .into_any()
}

/// A single client's pathway: their profile plus every case they have, so
/// interconnected needs are seen together.
#[component]
pub fn ClientDetailPage() -> impl IntoView {
    let state = expect_context::<AppState>();

    match state.current_user.get_untracked() {
        Some(u) if u.role == Role::Admin => {}
        Some(_) => return view! { <Redirect path="/volunteer" /> }.into_any(),
        None => return view! { <Redirect path="/login" /> }.into_any(),
    }

    let params = use_params_map();
    let client_id = move || params.read().get("id").unwrap_or_default();

    let header = move || match state.client(&client_id()) {
        Some(cl) => view! {
            <div class="rounded-xl border border-slate-800 bg-slate-900 p-5">
                <h2 class="text-xl font-semibold text-white">{cl.display_name}</h2>
                <p class="mt-1 text-sm text-slate-400">{cl.summary}</p>
                <div class="mt-3 flex flex-wrap gap-4 text-xs text-slate-500">
                    <span>"Intake: " {cl.intake_date}</span>
                    <Show when={
                        let phone = cl.phone.clone();
                        move || !phone.is_empty()
                    }>
                        <span>"Phone: " {cl.phone.clone()}</span>
                    </Show>
                </div>
            </div>
        }
        .into_any(),
        None => view! {
            <p class="text-sm text-slate-400">"Client not found."</p>
        }
        .into_any(),
    };

    let pathway = move || {
        let cases = state.cases_for_client(&client_id());
        if cases.is_empty() {
            return view! {
                <div class="rounded-xl border border-dashed border-slate-700 bg-slate-900 p-8 text-center">
                    <p class="text-slate-300">"No cases yet for this client."</p>
                    <p class="mt-1 text-sm text-slate-500">
                        "Open a case from the admin dashboard to start their pathway."
                    </p>
                </div>
            }
            .into_any();
        }
        cases
            .into_iter()
            .map(|c| view! { <CaseCard id=c.id.clone() /> })
            .collect_view()
            .into_any()
    };

    view! {
        <Layout title="Client pathway">
            <A
                href="/clients"
                attr:class="inline-block mb-4 text-sm text-slate-400 hover:text-slate-200"
            >
                "\u{2190} Back to clients"
            </A>
            {header}
            <h3 class="mb-3 mt-8 text-lg font-semibold">"Cases & service pathway"</h3>
            <p class="mb-4 max-w-2xl text-sm text-slate-400">
                "Each card is one need area. Use \u{201c}Related cases\u{201d} on a card to link "
                "interconnected needs so this client\u{2019}s pathway stays connected."
            </p>
            <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">{pathway}</div>
        </Layout>
    }
    .into_any()
}
