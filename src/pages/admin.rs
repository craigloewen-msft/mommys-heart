use leptos::prelude::*;
use leptos_router::components::Redirect;

use crate::components::layout::Layout;
use crate::state::AppState;
use crate::types::{CaseStatus, Role, VolunteerStatus};

/// Admin-only control center: manage volunteers, cases, documents, and
/// assignments over the local demo store.
#[component]
pub fn AdminDashboardPage() -> impl IntoView {
    let state = expect_context::<AppState>();

    // Role guard.
    match state.current_user.get_untracked() {
        Some(u) if u.role == Role::Admin => {}
        Some(_) => return view! { <Redirect path="/volunteer" /> }.into_any(),
        None => return view! { <Redirect path="/login" /> }.into_any(),
    }

    // --- new volunteer form ---
    let nv_name = RwSignal::new(String::new());
    let nv_email = RwSignal::new(String::new());
    let nv_specialty = RwSignal::new(String::new());
    let nv_error = RwSignal::new(String::new());
    let add_volunteer = move |_| {
        match state.add_volunteer(&nv_name.get(), &nv_email.get(), &nv_specialty.get()) {
            Ok(()) => {
                nv_name.set(String::new());
                nv_email.set(String::new());
                nv_specialty.set(String::new());
                nv_error.set(String::new());
            }
            Err(e) => nv_error.set(e),
        }
    };

    // --- new case form ---
    let nc_title = RwSignal::new(String::new());
    let nc_client = RwSignal::new(String::new());
    let nc_summary = RwSignal::new(String::new());
    let nc_error = RwSignal::new(String::new());
    let add_case = move |_| {
        match state.add_case(&nc_title.get(), &nc_client.get(), &nc_summary.get()) {
            Ok(()) => {
                nc_title.set(String::new());
                nc_client.set(String::new());
                nc_summary.set(String::new());
                nc_error.set(String::new());
            }
            Err(e) => nc_error.set(e),
        }
    };

    let input_class = "w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/40";
    let select_class = "rounded-lg border border-slate-700 bg-slate-950 px-2 py-1.5 text-xs text-slate-100 focus:border-primary-500 focus:outline-none";

    // Stats.
    let stat_cards = move || {
        let volunteers = state.volunteers.get();
        let cases = state.cases.get();
        let active = volunteers
            .iter()
            .filter(|v| v.status == VolunteerStatus::Active)
            .count();
        let open = cases
            .iter()
            .filter(|c| c.status != CaseStatus::Closed)
            .count();
        let unassigned = cases
            .iter()
            .filter(|c| c.assigned_volunteer_ids.is_empty())
            .count();
        [
            ("Volunteers", volunteers.len()),
            ("Active volunteers", active),
            ("Open cases", open),
            ("Unassigned cases", unassigned),
        ]
        .into_iter()
        .map(|(label, value)| {
            view! {
                <div class="rounded-xl border border-slate-800 bg-slate-900 p-5">
                    <p class="text-3xl font-semibold text-white">{value}</p>
                    <p class="mt-1 text-sm text-slate-400">{label}</p>
                </div>
            }
        })
        .collect_view()
    };

    // Volunteer rows.
    let volunteer_rows = move || {
        state
            .volunteers
            .get()
            .into_iter()
            .map(|v| {
                let id = v.id.clone();
                let badge = format!(
                    "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {}",
                    v.status.badge_classes(),
                );
                view! {
                    <tr class="border-b border-slate-800 last:border-0 hover:bg-slate-800/40">
                        <td class="px-4 py-3">
                            <p class="font-medium text-slate-100">{v.name.clone()}</p>
                            <p class="text-xs text-slate-500">{v.email.clone()}</p>
                        </td>
                        <td class="px-4 py-3 text-slate-300">{v.specialty.clone()}</td>
                        <td class="px-4 py-3">
                            <span class=badge>{v.status.label()}</span>
                        </td>
                        <td class="px-4 py-3">
                            <select
                                class=select_class
                                prop:value=v.status.slug()
                                on:change=move |ev| {
                                    if let Some(s) = VolunteerStatus::from_slug(
                                        &event_target_value(&ev),
                                    ) {
                                        state.set_volunteer_status(&id, s);
                                    }
                                }
                            >
                                {VolunteerStatus::ALL
                                    .into_iter()
                                    .map(|s| {
                                        view! { <option value=s.slug()>{s.label()}</option> }
                                    })
                                    .collect_view()}
                            </select>
                        </td>
                    </tr>
                }
            })
            .collect_view()
    };

    // Case cards.
    let case_cards = move || {
        state
            .cases
            .get()
            .into_iter()
            .map(|c| view! { <CaseCard id=c.id.clone() /> })
            .collect_view()
    };

    view! {
        <Layout title="Admin dashboard">
            <div class="grid grid-cols-2 gap-4 lg:grid-cols-4">{stat_cards}</div>

            // Volunteers management
            <section class="mt-8">
                <div class="mb-3 flex items-center justify-between">
                    <h2 class="text-lg font-semibold">"Volunteers"</h2>
                </div>
                <div class="overflow-hidden rounded-xl border border-slate-800 bg-slate-900">
                    <table class="w-full text-sm">
                        <thead class="border-b border-slate-800 text-left text-slate-400">
                            <tr>
                                <th class="px-4 py-3 font-medium">"Name"</th>
                                <th class="px-4 py-3 font-medium">"Specialty"</th>
                                <th class="px-4 py-3 font-medium">"Status"</th>
                                <th class="px-4 py-3 font-medium">"Change status"</th>
                            </tr>
                        </thead>
                        <tbody>{volunteer_rows}</tbody>
                    </table>
                    <div class="border-t border-slate-800 bg-slate-900/60 p-4">
                        <p class="mb-2 text-xs font-medium uppercase tracking-wide text-slate-400">
                            "Add volunteer"
                        </p>
                        <div class="flex flex-col gap-2 sm:flex-row">
                            <input
                                class=input_class
                                placeholder="Name"
                                prop:value=move || nv_name.get()
                                on:input=move |ev| nv_name.set(event_target_value(&ev))
                            />
                            <input
                                class=input_class
                                placeholder="Email"
                                prop:value=move || nv_email.get()
                                on:input=move |ev| nv_email.set(event_target_value(&ev))
                            />
                            <input
                                class=input_class
                                placeholder="Specialty"
                                prop:value=move || nv_specialty.get()
                                on:input=move |ev| nv_specialty.set(event_target_value(&ev))
                            />
                            <button
                                on:click=add_volunteer
                                class="shrink-0 rounded-lg bg-primary-500 px-4 py-2 text-sm font-semibold text-white hover:bg-primary-600"
                            >
                                "Add"
                            </button>
                        </div>
                        <Show when=move || !nv_error.get().is_empty()>
                            <p class="mt-2 text-xs text-rose-400">{move || nv_error.get()}</p>
                        </Show>
                    </div>
                </div>
            </section>

            // Cases management
            <section class="mt-8">
                <h2 class="mb-3 text-lg font-semibold">"Cases"</h2>
                <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">{case_cards}</div>

                <div class="mt-4 rounded-xl border border-slate-800 bg-slate-900 p-4">
                    <p class="mb-2 text-xs font-medium uppercase tracking-wide text-slate-400">
                        "Open a new case"
                    </p>
                    <div class="grid gap-2 sm:grid-cols-2">
                        <input
                            class=input_class
                            placeholder="Case title"
                            prop:value=move || nc_title.get()
                            on:input=move |ev| nc_title.set(event_target_value(&ev))
                        />
                        <input
                            class=input_class
                            placeholder="Client (confidential)"
                            prop:value=move || nc_client.get()
                            on:input=move |ev| nc_client.set(event_target_value(&ev))
                        />
                    </div>
                    <textarea
                        class=format!("{input_class} mt-2")
                        rows="2"
                        placeholder="Short summary"
                        prop:value=move || nc_summary.get()
                        on:input=move |ev| nc_summary.set(event_target_value(&ev))
                    ></textarea>
                    <div class="mt-2 flex items-center gap-3">
                        <button
                            on:click=add_case
                            class="rounded-lg bg-primary-500 px-4 py-2 text-sm font-semibold text-white hover:bg-primary-600"
                        >
                            "Create case"
                        </button>
                        <Show when=move || !nc_error.get().is_empty()>
                            <p class="text-xs text-rose-400">{move || nc_error.get()}</p>
                        </Show>
                    </div>
                </div>
            </section>
        </Layout>
    }
    .into_any()
}

/// A single, live-editable case card: status, volunteer assignments, and
/// documents. Reads its case reactively by id so edits reflect immediately
/// while local inputs keep focus.
#[component]
fn CaseCard(id: String) -> impl IntoView {
    let state = expect_context::<AppState>();

    let lookup_id = id.clone();
    let case = Memo::new(move |_| {
        state
            .cases
            .get()
            .into_iter()
            .find(|c| c.id == lookup_id)
    });

    let doc_name = RwSignal::new(String::new());
    let add_doc_id = id.clone();
    let add_doc = move |_| {
        state.add_case_document(&add_doc_id, &doc_name.get());
        doc_name.set(String::new());
    };

    let status_id = id.clone();
    let select_class = "rounded-lg border border-slate-700 bg-slate-950 px-2 py-1.5 text-xs text-slate-100 focus:border-primary-500 focus:outline-none";
    let doc_input_class = "flex-1 rounded-lg border border-slate-700 bg-slate-950 px-3 py-1.5 text-xs text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none";

    let assignments_id = id.clone();
    let assignments = move || {
        let current = case.get();
        let assigned: Vec<String> = current
            .as_ref()
            .map(|c| c.assigned_volunteer_ids.clone())
            .unwrap_or_default();
        state
            .volunteers
            .get()
            .into_iter()
            .map(|v| {
                let is_assigned = assigned.iter().any(|a| a == &v.id);
                let case_id = assignments_id.clone();
                let vid = v.id.clone();
                view! {
                    <label class="flex cursor-pointer items-center gap-2 rounded-lg border border-slate-700 px-2 py-1 text-xs text-slate-300 hover:bg-slate-800">
                        <input
                            r#type="checkbox"
                            class="accent-primary-500"
                            prop:checked=is_assigned
                            on:change=move |_| state.toggle_case_volunteer(&case_id, &vid)
                        />
                        {v.name.clone()}
                    </label>
                }
            })
            .collect_view()
    };

    let documents = move || {
        match case.get() {
            Some(c) if !c.documents.is_empty() => c
                .documents
                .into_iter()
                .map(|d| {
                    view! {
                        <li class="flex items-center justify-between rounded-lg bg-slate-950/70 px-3 py-1.5 text-xs">
                            <span class="text-slate-200">{d.name}</span>
                            <span class="text-slate-500">{d.uploaded_at}</span>
                        </li>
                    }
                })
                .collect_view()
                .into_any(),
            _ => view! {
                <li class="rounded-lg bg-slate-950/70 px-3 py-1.5 text-xs text-slate-500">
                    "No documents yet."
                </li>
            }
            .into_any(),
        }
    };

    view! {
        <div class="rounded-xl border border-slate-800 bg-slate-900 p-5">
            {move || match case.get() {
                Some(c) => {
                    let status_badge = format!(
                        "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {}",
                        c.status.badge_classes(),
                    );
                    let prio_badge = format!(
                        "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {}",
                        c.priority.badge_classes(),
                    );
                    view! {
                        <div class="flex items-start justify-between gap-3">
                            <div>
                                <h3 class="font-semibold text-white">{c.title}</h3>
                                <p class="text-xs text-slate-500">{c.client_name}</p>
                            </div>
                            <div class="flex shrink-0 flex-wrap justify-end gap-1">
                                <span class=status_badge>{c.status.label()}</span>
                                <span class=prio_badge>
                                    {format!("{} priority", c.priority.label())}
                                </span>
                            </div>
                        </div>
                        <p class="mt-2 text-sm text-slate-400">{c.summary}</p>
                    }
                        .into_any()
                }
                None => ().into_any(),
            }}

            <div class="mt-4 flex items-center gap-2">
                <span class="text-xs text-slate-400">"Status"</span>
                <select
                    class=select_class
                    prop:value=move || {
                        case.get().map(|c| c.status.slug()).unwrap_or("open")
                    }
                    on:change=move |ev| {
                        if let Some(s) = CaseStatus::from_slug(&event_target_value(&ev)) {
                            state.set_case_status(&status_id, s);
                        }
                    }
                >
                    {CaseStatus::ALL
                        .into_iter()
                        .map(|s| view! { <option value=s.slug()>{s.label()}</option> })
                        .collect_view()}
                </select>
            </div>

            <div class="mt-4">
                <p class="mb-1.5 text-xs font-medium uppercase tracking-wide text-slate-400">
                    "Assigned volunteers"
                </p>
                <div class="flex flex-wrap gap-2">{assignments}</div>
            </div>

            <div class="mt-4">
                <p class="mb-1.5 text-xs font-medium uppercase tracking-wide text-slate-400">
                    "Documents"
                </p>
                <ul class="space-y-1.5">{documents}</ul>
                <div class="mt-2 flex gap-2">
                    <input
                        class=doc_input_class
                        placeholder="Document name\u{2026}"
                        prop:value=move || doc_name.get()
                        on:input=move |ev| doc_name.set(event_target_value(&ev))
                    />
                    <button
                        on:click=add_doc
                        class="shrink-0 rounded-lg border border-slate-700 px-3 py-1.5 text-xs font-medium text-slate-200 hover:bg-slate-800"
                    >
                        "Add document"
                    </button>
                </div>
            </div>
        </div>
    }
}
