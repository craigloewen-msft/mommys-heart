use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;

use crate::components::guard::require_login;
use crate::components::layout::Layout;
use crate::state::{AppState, AuthPhase};
use crate::types::{Case, CaseCapability, CaseStatus};

/// How many cases the list loads per "page" (each "Load more" click grows the
/// visible window by this much).
const PAGE_SIZE: i64 = 10;

/// One editable property row while a case is in edit mode. Each field is its own
/// signal so typing never re-creates the row (keeps input focus stable).
#[derive(Clone, Copy)]
struct PropRow {
    id: usize,
    key: RwSignal<String>,
    value: RwSignal<String>,
}

/// Flatten a [`ServerFnError`] to the plain message we wrote server-side.
fn err_text(e: ServerFnError) -> String {
    match e {
        ServerFnError::ServerError(m) => m,
        other => other.to_string(),
    }
}

fn badge(classes: &str) -> String {
    format!("inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {classes}")
}

/// A short label summarizing a user's access to a case from their capabilities.
fn access_label(caps: &[CaseCapability]) -> Option<(&'static str, &'static str)> {
    if caps.is_empty() {
        return None;
    }
    if caps.len() == CaseCapability::ALL.len() {
        Some((
            "Full access",
            "bg-primary-500/15 text-primary-300 ring-1 ring-primary-500/30",
        ))
    } else if caps.contains(&CaseCapability::EditCase) {
        Some((
            "Manager",
            "bg-primary-500/15 text-primary-300 ring-1 ring-primary-500/30",
        ))
    } else if caps.contains(&CaseCapability::UploadEvidence)
        || caps.contains(&CaseCapability::AddNotes)
        || caps.contains(&CaseCapability::SendMessages)
    {
        Some((
            "Contributor",
            "bg-sky-500/15 text-sky-300 ring-1 ring-sky-500/30",
        ))
    } else {
        Some((
            "Viewer",
            "bg-slate-500/15 text-slate-300 ring-1 ring-slate-500/30",
        ))
    }
}

/// Case Home: view the cases you own / are assigned to and manage them. The list
/// is server-side paginated + searchable ("Load more"), so the browser never
/// pulls every case at once.
#[component]
pub fn CaseHomePage() -> impl IntoView {
    let state = expect_context::<AppState>();
    let selected = RwSignal::new(None::<String>);
    // Search text, the fetched window of cases, and the total match count.
    let query = RwSignal::new(String::new());
    let results = RwSignal::new(Vec::<Case>::new());
    let total = RwSignal::new(0i64);
    // How many rows the current window requests; grows on "Load more".
    let window = RwSignal::new(PAGE_SIZE);
    let loading = RwSignal::new(false);
    let load_error = RwSignal::new(None::<String>);
    // Bumped after a mutation to force the current window to reload.
    let reload = RwSignal::new(0u32);

    // (Re)load the window whenever the query, window size, or reload tick
    // changes — but only once a session is confirmed (server functions run in
    // the browser after hydration). We always fetch `[0, window)` so search
    // changes and post-mutation refreshes share one code path.
    Effect::new(move |_| {
        let count = window.get();
        let q = query.get();
        reload.track();
        if !matches!(state.auth.get(), AuthPhase::SignedIn) {
            return;
        }
        loading.set(true);
        spawn_local(async move {
            match crate::server_fns::cases::list_cases_page(0, count, q).await {
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

    require_login(state, move || {
    // Resolve a user id to a display name.
    let owner_name = move |owner_id: &str| -> String {
        state
            .users
            .get()
            .into_iter()
            .find(|u| u.id == owner_id)
            .map(|u| u.full_name())
            .unwrap_or_else(|| "—".into())
    };

    let cases_list = move || {
        if let Some(msg) = load_error.get() {
            return view! {
                <p class="text-sm text-rose-300">"Could not load cases: " {msg}</p>
            }
            .into_any();
        }
        let cases = results.get();
        if cases.is_empty() {
            let text = if loading.get() {
                "Loading\u{2026}"
            } else if query.get().trim().is_empty() {
                "You have no cases yet."
            } else {
                "No cases match your search."
            };
            return view! { <p class="text-sm text-slate-400">{text}</p> }.into_any();
        }
        cases
            .into_iter()
            .map(|c| {
                let case_id = c.id.clone();
                let is_selected = {
                    let case_id = case_id.clone();
                    move || selected.get().as_deref() == Some(case_id.as_str())
                };
                let caps = state.capabilities_on(&c);
                let access_badge = access_label(&caps)
                    .map(|(label, classes)| {
                        view! { <span class=badge(classes)>{label}</span> }.into_any()
                    })
                    .unwrap_or_else(|| ().into_any());
                let status = c.status;
                let name = c.name.clone();
                let owner = owner_name(&c.owner_id);
                let select = {
                    let case_id = case_id.clone();
                    move |_| selected.set(Some(case_id.clone()))
                };
                view! {
                    <button
                        on:click=select
                        class=move || {
                            let base = "w-full rounded-xl border p-4 text-left transition-colors";
                            if is_selected() {
                                format!("{base} border-primary-500/50 bg-slate-800")
                            } else {
                                format!("{base} border-slate-800 bg-slate-900 hover:bg-slate-800")
                            }
                        }
                    >
                        <div class="flex items-center justify-between gap-2">
                            <span class="font-medium">{name}</span>
                            <span class=badge(status.badge_classes())>{status.label()}</span>
                        </div>
                        <div class="mt-2 flex items-center gap-2 text-xs text-slate-400">
                            <span>"Owner: " {owner}</span>
                            {access_badge}
                        </div>
                    </button>
                }
                .into_any()
            })
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
            <div class="mt-3 flex items-center justify-between">
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

    let detail = move || {
        match selected.get() {
        None => view! {
            <div class="rounded-xl border border-dashed border-slate-700 p-8 text-center text-sm text-slate-500">
                "Select a case to view and manage it."
            </div>
        }
        .into_any(),
        Some(id) => {
            let case = results.get().into_iter().find(|c| c.id == id);
            match case {
                Some(c) => view! { <CaseDetail case=c source=results reload=reload /> }.into_any(),
                None => view! {
                    <p class="text-sm text-slate-400">"Case not found."</p>
                }
                .into_any(),
            }
        }
    }
    };

    let input_class = "w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/40";

    view! {
        <Layout title="Cases".to_string()>
            <div class="grid gap-6 lg:grid-cols-[22rem_1fr]">
                <div class="space-y-4">
                    <A
                        href="/cases/new"
                        attr:class="flex items-center justify-center rounded-xl border border-primary-500/40 bg-primary-500/10 px-4 py-3 text-sm font-semibold text-primary-200 hover:bg-primary-500/20"
                    >
                        "+ New case"
                    </A>
                    <input
                        class=input_class
                        placeholder="Search cases by name"
                        prop:value=move || query.get()
                        on:input=move |ev| {
                            query.set(event_target_value(&ev));
                            window.set(PAGE_SIZE);
                        }
                    />
                    <div class="space-y-3">{cases_list}</div>
                    {footer}
                </div>
                <div>{detail}</div>
            </div>
        </Layout>
    }
    .into_any()
    })
}

/// New Case: a dedicated form to open a case with its key details.
#[component]
pub fn NewCasePage() -> impl IntoView {
    let state = expect_context::<AppState>();

    let navigate = use_navigate();

    let name = RwSignal::new(String::new());
    let status = RwSignal::new(CaseStatus::Open.slug().to_string());
    let attorney = RwSignal::new(String::new());
    let opposing = RwSignal::new(String::new());
    let court = RwSignal::new(String::new());
    let docket = RwSignal::new(String::new());
    let first_note = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());

    let input_class = "w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/40";
    let label_class = "block text-xs font-medium text-slate-400";

    require_login(state, move || {
    let submit = {
        let navigate = navigate.clone();
        move |_| {
            let navigate = navigate.clone();
            let status = CaseStatus::from_slug(&status.get_untracked()).unwrap_or(CaseStatus::Open);
            let properties = vec![
                ("Attorney".to_string(), attorney.get_untracked()),
                ("Opposing attorney".to_string(), opposing.get_untracked()),
                ("Court".to_string(), court.get_untracked()),
                ("Docket number".to_string(), docket.get_untracked()),
            ];
            let note = first_note.get_untracked();
            let note = if note.trim().is_empty() {
                None
            } else {
                Some(note)
            };
            let name_val = name.get_untracked();
            spawn_local(async move {
                match state
                    .add_case_full(&name_val, status, properties, note)
                    .await
                {
                    Ok(_) => navigate("/cases", Default::default()),
                    Err(e) => error.set(e),
                }
            });
        }
    };

    view! {
        <Layout title="New case".to_string()>
            <div class="mx-auto max-w-2xl space-y-6">
                <div class="rounded-xl border border-slate-800 bg-slate-900 p-6 space-y-5">
                    <div>
                        <label class=label_class>"Case name"</label>
                        <input
                            class=format!("mt-1 {input_class}")
                            placeholder="e.g. Rivera custody support"
                            prop:value=move || name.get()
                            on:input=move |ev| name.set(event_target_value(&ev))
                        />
                    </div>
                    <div>
                        <label class=label_class>"Status"</label>
                        <select
                            class=format!("mt-1 {input_class}")
                            on:change=move |ev| status.set(event_target_value(&ev))
                        >
                            {CaseStatus::ALL
                                .into_iter()
                                .map(|s| {
                                    view! {
                                        <option value=s.slug() selected=s == CaseStatus::Open>
                                            {s.label()}
                                        </option>
                                    }
                                })
                                .collect_view()}
                        </select>
                    </div>
                    <div class="grid gap-4 sm:grid-cols-2">
                        <div>
                            <label class=label_class>"Attorney"</label>
                            <input
                                class=format!("mt-1 {input_class}")
                                placeholder="Lead attorney"
                                prop:value=move || attorney.get()
                                on:input=move |ev| attorney.set(event_target_value(&ev))
                            />
                        </div>
                        <div>
                            <label class=label_class>"Opposing attorney"</label>
                            <input
                                class=format!("mt-1 {input_class}")
                                placeholder="Opposing counsel"
                                prop:value=move || opposing.get()
                                on:input=move |ev| opposing.set(event_target_value(&ev))
                            />
                        </div>
                        <div>
                            <label class=label_class>"Court"</label>
                            <input
                                class=format!("mt-1 {input_class}")
                                placeholder="Court / jurisdiction"
                                prop:value=move || court.get()
                                on:input=move |ev| court.set(event_target_value(&ev))
                            />
                        </div>
                        <div>
                            <label class=label_class>"Docket number"</label>
                            <input
                                class=format!("mt-1 {input_class}")
                                placeholder="Docket #"
                                prop:value=move || docket.get()
                                on:input=move |ev| docket.set(event_target_value(&ev))
                            />
                        </div>
                    </div>
                    <div>
                        <label class=label_class>"Initial note (optional)"</label>
                        <textarea
                            class=format!("mt-1 {input_class}")
                            rows="3"
                            placeholder="Intake summary, next steps, etc."
                            prop:value=move || first_note.get()
                            on:input=move |ev| first_note.set(event_target_value(&ev))
                        ></textarea>
                    </div>
                    <Show when=move || !error.get().is_empty()>
                        <p class="text-sm text-rose-300">{move || error.get()}</p>
                    </Show>
                    <div class="flex gap-3">
                        <button
                            on:click=submit
                            class="rounded-lg bg-primary-500 px-4 py-2 text-sm font-semibold text-white hover:bg-primary-600"
                        >
                            "Create case"
                        </button>
                        <A
                            href="/cases"
                            attr:class="rounded-lg border border-slate-700 px-4 py-2 text-sm font-semibold text-slate-300 hover:bg-slate-800"
                        >
                            "Cancel"
                        </A>
                    </div>
                </div>
            </div>
        </Layout>
    }
    .into_any()
    })
}

/// The management panel for a single case. `source` is the parent's fetched
/// window of cases (the paginated list); we re-read the live case from it so
/// edits show as soon as the window reloads. `reload` is bumped after any
/// mutation to trigger that reload.
#[component]
fn CaseDetail(case: Case, source: RwSignal<Vec<Case>>, reload: RwSignal<u32>) -> impl IntoView {
    let state = expect_context::<AppState>();
    let case_id = case.id.clone();

    let owner = StoredValue::new(Owner::current().expect("component owner"));

    // Capability gates for this case.
    let can_edit = state.case_can(&case, CaseCapability::EditCase);
    let can_note = state.case_can(&case, CaseCapability::AddNotes);
    let can_view_evidence = state.case_can(&case, CaseCapability::ViewEvidence);
    let can_upload_evidence = state.case_can(&case, CaseCapability::UploadEvidence);
    let can_delete_evidence = state.case_can(&case, CaseCapability::DeleteEvidence);

    // Reactively re-read the case from the parent's fetched window so edits show
    // as soon as it reloads.
    let case_sv = StoredValue::new(case_id.clone());
    let live_case = move || {
        source
            .get()
            .into_iter()
            .find(|c| c.id == case_sv.get_value())
    };

    let input_class = "w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/40";

    // --- edit mode (name, status, owner, properties) ---
    let editing = RwSignal::new(false);
    let edit_name = RwSignal::new(String::new());
    let edit_status = RwSignal::new(String::new());
    let edit_owner = RwSignal::new(String::new());
    let edit_props: RwSignal<Vec<PropRow>> = RwSignal::new(Vec::new());
    let edit_error = RwSignal::new(String::new());
    let row_seq = RwSignal::new(0usize);

    let make_row = move |key: String, value: String| -> PropRow {
        let id = row_seq.get_untracked();
        row_seq.set(id + 1);
        owner.with_value(|o| {
            o.with(|| PropRow {
                id,
                key: RwSignal::new(key),
                value: RwSignal::new(value),
            })
        })
    };

    let begin_edit = move |_| {
        if let Some(c) = live_case() {
            edit_name.set(c.name.clone());
            edit_status.set(c.status.slug().to_string());
            edit_owner.set(c.owner_id.clone());
            let rows = c
                .properties
                .iter()
                .map(|p| make_row(p.key.clone(), p.value.clone()))
                .collect::<Vec<_>>();
            edit_props.set(rows);
            edit_error.set(String::new());
            editing.set(true);
        }
    };

    let cancel_edit = move |_| {
        edit_error.set(String::new());
        editing.set(false);
    };

    let add_prop_row = move |_| {
        let row = make_row(String::new(), String::new());
        edit_props.update(|rows| rows.push(row));
    };

    let save_edit = move |_| {
        let case_id = case_sv.get_value();
        let name = edit_name.get_untracked();
        let status_slug = edit_status.get_untracked();
        let owner = edit_owner.get_untracked();
        let props = edit_props
            .get_untracked()
            .into_iter()
            .map(|r| (r.key.get_untracked(), r.value.get_untracked()))
            .collect::<Vec<_>>();
        spawn_local(async move {
            if let Err(e) = state.set_case_name(&case_id, &name).await {
                edit_error.set(e);
                return;
            }
            if let Some(s) = CaseStatus::from_slug(&status_slug) {
                if let Err(e) = state.set_case_status(&case_id, s).await {
                    edit_error.set(e);
                    return;
                }
            }
            if let Err(e) = state.set_case_owner(&case_id, &owner).await {
                edit_error.set(e);
                return;
            }
            if let Err(e) = state.replace_case_properties(&case_id, props).await {
                edit_error.set(e);
                return;
            }
            edit_error.set(String::new());
            editing.set(false);
            reload.update(|n| *n += 1);
        });
    };

    // --- note form ---
    let note_body = RwSignal::new(String::new());
    let add_note = {
        let case_id = case_id.clone();
        move |_| {
            let case_id = case_id.clone();
            let body = note_body.get_untracked();
            spawn_local(async move {
                if state.add_case_note(&case_id, &body).await.is_ok() {
                    note_body.set(String::new());
                    reload.update(|n| *n += 1);
                }
            });
        }
    };

    // --- evidence form ---
    let evi_name = RwSignal::new(String::new());
    let evi_desc = RwSignal::new(String::new());
    let add_evidence = {
        let case_id = case_id.clone();
        move |_| {
            let case_id = case_id.clone();
            let name = evi_name.get_untracked();
            let desc = evi_desc.get_untracked();
            spawn_local(async move {
                if state
                    .add_case_evidence(&case_id, &name, &desc)
                    .await
                    .is_ok()
                {
                    evi_name.set(String::new());
                    evi_desc.set(String::new());
                    reload.update(|n| *n += 1);
                }
            });
        }
    };

    let notes_view = {
        move || {
            let notes = live_case().map(|c| c.notes).unwrap_or_default();
            if notes.is_empty() {
                return view! { <p class="text-sm text-slate-500">"No notes yet."</p> }.into_any();
            }
            notes
                .into_iter()
                .map(|n| {
                    view! {
                        <div class="rounded-lg border border-slate-800 bg-slate-950 p-3">
                            <p class="text-sm text-slate-200">{n.body}</p>
                            <p class="mt-1 text-xs text-slate-500">
                                {n.author} " · " {n.created_at}
                            </p>
                        </div>
                    }
                    .into_any()
                })
                .collect_view()
                .into_any()
        }
    };

    let evidence_view = {
        let case_id = case_id.clone();
        move || {
            let evidence = live_case().map(|c| c.evidence).unwrap_or_default();
            if evidence.is_empty() {
                return view! { <p class="text-sm text-slate-500">"No evidence yet."</p> }
                    .into_any();
            }
            evidence
                .into_iter()
                .map(|e| {
                    let delete = {
                        let case_id = case_id.clone();
                        let evidence_id = e.id.clone();
                        move |_| {
                            let case_id = case_id.clone();
                            let evidence_id = evidence_id.clone();
                            spawn_local(async move {
                                if state
                                    .delete_case_evidence(&case_id, &evidence_id)
                                    .await
                                    .is_ok()
                                {
                                    reload.update(|n| *n += 1);
                                }
                            });
                        }
                    };
                    let delete_btn = if can_delete_evidence {
                        view! {
                            <button
                                on:click=delete
                                class="shrink-0 rounded-lg border border-rose-500/40 px-2 py-1 text-xs font-medium text-rose-300 hover:bg-rose-500/10"
                            >
                                "Delete"
                            </button>
                        }
                        .into_any()
                    } else {
                        ().into_any()
                    };
                    view! {
                        <div class="rounded-lg border border-slate-800 bg-slate-950 p-3">
                            <div class="flex items-start justify-between gap-2">
                                <p class="text-sm font-medium text-slate-200">{e.name}</p>
                                {delete_btn}
                            </div>
                            <Show when={
                                let d = e.description.clone();
                                move || !d.is_empty()
                            }>
                                <p class="text-sm text-slate-400">{e.description.clone()}</p>
                            </Show>
                            <p class="mt-1 text-xs text-slate-500">
                                "Uploaded by " {e.uploaded_by} " · " {e.uploaded_at}
                            </p>
                        </div>
                    }
                    .into_any()
                })
                .collect_view()
                .into_any()
        }
    };

    let properties_view = {
        move || {
            let props = live_case().map(|c| c.properties).unwrap_or_default();
            if props.is_empty() {
                return view! { <p class="text-sm text-slate-500">"No properties yet."</p> }
                    .into_any();
            }
            props
                .into_iter()
                .map(|p| {
                    view! {
                        <div class="flex justify-between gap-4 border-b border-slate-800 py-1.5 text-sm">
                            <span class="text-slate-400">{p.key}</span>
                            <span class="text-slate-200">{p.value}</span>
                        </div>
                    }
                    .into_any()
                })
                .collect_view()
                .into_any()
        }
    };

    let audit_view = {
        move || {
            let log = live_case().map(|c| c.audit_log).unwrap_or_default();
            if log.is_empty() {
                return view! { <p class="text-sm text-slate-500">"No changes recorded."</p> }
                    .into_any();
            }
            log.into_iter()
                .map(|e| {
                    view! {
                        <div class="text-xs text-slate-400">
                            <span class="text-slate-300">{e.actor}</span>
                            " changed " <span class="text-slate-300">{e.field}</span>
                            " from \"" {e.old_value} "\" to \"" {e.new_value} "\" · " {e.at}
                        </div>
                    }
                    .into_any()
                })
                .collect_view()
                .into_any()
        }
    };

    let section = "rounded-xl border border-slate-800 bg-slate-900 p-4";

    let details_section = move || {
        let Some(c) = live_case() else {
            return ().into_any();
        };
        if !editing.get() {
            let owner_name = state.user_name(&c.owner_id);
            let status = c.status;
            let edit_btn = if can_edit {
                view! {
                    <button
                        on:click=begin_edit
                        class="shrink-0 rounded-lg border border-slate-700 px-3 py-1.5 text-sm font-medium text-slate-200 hover:bg-slate-800"
                    >
                        "Edit"
                    </button>
                }
                .into_any()
            } else {
                ().into_any()
            };
            return view! {
                <div class=section>
                    <div class="flex items-start justify-between gap-3">
                        <div>
                            <h2 class="text-lg font-semibold">{c.name.clone()}</h2>
                            <p class="mt-1 text-sm text-slate-400">"Filed by " {owner_name}</p>
                        </div>
                        <div class="flex items-center gap-2">
                            <span class=badge(status.badge_classes())>{status.label()}</span>
                            {edit_btn}
                        </div>
                    </div>
                    <Show when=move || !can_edit && !can_note && !can_upload_evidence>
                        <p class="mt-2 text-xs text-slate-500">
                            "You have view-only access to this case."
                        </p>
                    </Show>
                    <div class="mt-4">
                        <h3 class="text-sm font-semibold text-slate-200">"Properties"</h3>
                        <div class="mt-2">{properties_view()}</div>
                    </div>
                </div>
            }
            .into_any();
        }

        // --- edit mode ---
        let owner_options = state
            .users
            .get()
            .into_iter()
            .map(|u| {
                let selected = u.id == edit_owner.get();
                view! {
                    <option value=u.id.clone() selected=selected>
                        {u.full_name()}
                    </option>
                }
            })
            .collect_view();
        view! {
            <div class=section>
                <div class="flex items-center justify-between gap-3">
                    <h2 class="text-lg font-semibold">"Edit case"</h2>
                    <div class="flex items-center gap-2">
                        <button
                            on:click=save_edit
                            class="shrink-0 rounded-lg bg-primary-500 px-3 py-1.5 text-sm font-semibold text-white hover:bg-primary-600"
                        >
                            "Save"
                        </button>
                        <button
                            on:click=cancel_edit
                            class="shrink-0 rounded-lg border border-slate-700 px-3 py-1.5 text-sm font-medium text-slate-300 hover:bg-slate-800"
                        >
                            "Cancel"
                        </button>
                    </div>
                </div>
                <Show when=move || !edit_error.get().is_empty()>
                    <p class="mt-3 rounded-lg border border-rose-500/40 bg-rose-500/10 px-3 py-2 text-sm text-rose-300">
                        {move || edit_error.get()}
                    </p>
                </Show>
                <div class="mt-4 space-y-4">
                    <div>
                        <label class="text-xs font-medium text-slate-400">"Case name"</label>
                        <input
                            class=input_class
                            prop:value=move || edit_name.get()
                            on:input=move |ev| edit_name.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="grid gap-4 sm:grid-cols-2">
                        <div>
                            <label class="text-xs font-medium text-slate-400">"Status"</label>
                            <select
                                class=input_class
                                on:change=move |ev| edit_status.set(event_target_value(&ev))
                            >
                                {CaseStatus::ALL
                                    .into_iter()
                                    .map(|s| {
                                        view! {
                                            <option
                                                value=s.slug()
                                                selected=move || edit_status.get() == s.slug()
                                            >
                                                {s.label()}
                                            </option>
                                        }
                                    })
                                    .collect_view()}
                            </select>
                        </div>
                        <div>
                            <label class="text-xs font-medium text-slate-400">
                                "Owner (who filed it)"
                            </label>
                            <select
                                class=input_class
                                on:change=move |ev| edit_owner.set(event_target_value(&ev))
                            >
                                {owner_options}
                            </select>
                        </div>
                    </div>
                    <div>
                        <label class="text-xs font-medium text-slate-400">"Properties"</label>
                        <div class="mt-2 space-y-2">
                            <For each=move || edit_props.get() key=|r| r.id let:row>
                                <div class="flex gap-2">
                                    <input
                                        class=input_class
                                        placeholder="Name (e.g. Attorney)"
                                        prop:value=move || row.key.get()
                                        on:input=move |ev| row.key.set(event_target_value(&ev))
                                    />
                                    <input
                                        class=input_class
                                        placeholder="Value"
                                        prop:value=move || row.value.get()
                                        on:input=move |ev| row.value.set(event_target_value(&ev))
                                    />
                                    <button
                                        on:click=move |_| {
                                            edit_props.update(|rows| rows.retain(|x| x.id != row.id))
                                        }
                                        class="shrink-0 rounded-lg border border-rose-500/40 px-3 py-2 text-sm font-medium text-rose-300 hover:bg-rose-500/10"
                                    >
                                        "Remove"
                                    </button>
                                </div>
                            </For>
                        </div>
                        <button
                            on:click=add_prop_row
                            class="mt-2 rounded-lg border border-slate-700 px-3 py-1.5 text-sm font-medium text-slate-200 hover:bg-slate-800"
                        >
                            "+ Add property"
                        </button>
                    </div>
                </div>
            </div>
        }
        .into_any()
    };

    view! {
        <div class="space-y-6">
            {details_section}

            // Notes
            <div class=section>
                <h3 class="text-sm font-semibold text-slate-200">"Notes"</h3>
                <div class="mt-3 space-y-2">{notes_view}</div>
                {if can_note {
                    view! {
                        <div class="mt-3 flex gap-2">
                            <input
                                class=input_class
                                placeholder="Add a note"
                                prop:value=move || note_body.get()
                                on:input=move |ev| note_body.set(event_target_value(&ev))
                            />
                            <button
                                on:click=add_note
                                class="shrink-0 rounded-lg bg-primary-500 px-3 py-2 text-sm font-semibold text-white hover:bg-primary-600"
                            >
                                "Add"
                            </button>
                        </div>
                    }
                        .into_any()
                } else {
                    ().into_any()
                }}
            </div>

            // Evidence
            {if can_view_evidence {
                view! {
                    <div class=section>
                        <h3 class="text-sm font-semibold text-slate-200">"Evidence"</h3>
                        <div class="mt-3 space-y-2">{evidence_view}</div>
                        {if can_upload_evidence {
                            view! {
                                <div class="mt-3 space-y-2">
                                    <input
                                        class=input_class
                                        placeholder="Evidence name"
                                        prop:value=move || evi_name.get()
                                        on:input=move |ev| evi_name.set(event_target_value(&ev))
                                    />
                                    <input
                                        class=input_class
                                        placeholder="Extra information (optional)"
                                        prop:value=move || evi_desc.get()
                                        on:input=move |ev| evi_desc.set(event_target_value(&ev))
                                    />
                                    <button
                                        on:click=add_evidence
                                        class="rounded-lg bg-primary-500 px-3 py-2 text-sm font-semibold text-white hover:bg-primary-600"
                                    >
                                        "Add evidence"
                                    </button>
                                </div>
                            }
                                .into_any()
                        } else {
                            ().into_any()
                        }}
                    </div>
                }
                    .into_any()
            } else {
                ().into_any()
            }}

            // Audit log
            <div class=section>
                <h3 class="text-sm font-semibold text-slate-200">"Change log"</h3>
                <div class="mt-3 space-y-1.5">{audit_view}</div>
            </div>
        </div>
    }
}
