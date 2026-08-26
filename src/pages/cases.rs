use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;

use crate::components::case_contacts::CaseContactsPanel;
use crate::components::case_intake::{CaseIntakeFields, CaseIntakeState};
use crate::components::change_log::ChangeLog;
use crate::components::guard::require_login;
use crate::components::layout::Layout;
use crate::components::loading::Loading;
use crate::components::profile_link::ProfileLink;
use crate::helpers::sections;
use crate::helpers::visibility::Visibility;
use crate::pages::case_notes::CaseNotesPanel;
use crate::server_fns::audit::AuditScope;
use crate::server_fns::capabilities::CaseCapability;
use crate::server_fns::case_properties::{self, CaseProperty};
use crate::server_fns::cases::{self, Case, CaseStatus, CaseSummary};
use crate::server_fns::err_text;
use crate::server_fns::evidence::EVIDENCE_UNAVAILABLE_MESSAGE;
use crate::server_fns::users::{search_users, AccountRole, UserSummary};
use crate::state::AppState;

/// How many cases the list loads per "page" (each "Load more" click grows the
/// visible window by this much).
const PAGE_SIZE: i64 = 10;

/// One editable property row while a case is in edit mode
#[derive(Clone, Copy)]
struct PropRow {
    id: usize,
    key: RwSignal<String>,
    value: RwSignal<String>,
    section: RwSignal<String>,
    visibility: Visibility,
}

/// The case's properties arranged for display: visibility first, then section.
///
/// Section order is the order each name first appears in the case's own row
/// order, so the intake/outtake fields a case was created with keep the order
/// they were defined in and anything added later follows.
///
/// Files are *not* here: they live in the case's folder tree instead, which the
/// file browser renders on its own.
fn group_case_properties(case: &Case) -> Vec<(Visibility, Vec<(String, Vec<CaseProperty>)>)> {
    Visibility::ALL
        .into_iter()
        .filter_map(|visibility| {
            let props = case
                .properties
                .iter()
                .filter(|p| p.visibility == visibility);

            let mut order: Vec<String> = Vec::new();
            for name in props.clone().map(|p| p.section.clone()) {
                if !order.contains(&name) {
                    order.push(name);
                }
            }
            if order.is_empty() {
                return None;
            }

            let sections = order
                .into_iter()
                .map(|name| {
                    let in_section: Vec<CaseProperty> = props
                        .clone()
                        .filter(|p| p.section == name)
                        .cloned()
                        .collect();
                    (name, in_section)
                })
                .collect();
            Some((visibility, sections))
        })
        .collect()
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

#[component]
pub fn CaseHomePage() -> impl IntoView {
    let state = expect_context::<AppState>();
    let selected = RwSignal::new(None::<String>);
    // The folder open in the selected case's file browser. Kept here so it
    // survives the reload every case mutation triggers.
    let open_folder = RwSignal::new(None::<String>);

    // The fetched window of case summaries plus the total match count. `query`
    // is bound to the input for instant feedback; `debounced_query` drives the
    // actual fetch so we don't hit the server on every keystroke.
    let cases = RwSignal::new(Vec::<CaseSummary>::new());
    let total = RwSignal::new(0i64);
    let query = RwSignal::new(String::new());
    let debounced_query = RwSignal::new(String::new());
    // How many rows the current window requests; grows on "Load more".
    let window = RwSignal::new(PAGE_SIZE);
    let loading = RwSignal::new(false);
    let load_error = RwSignal::new(None::<String>);
    // Bumped after a mutation to force the current window to reload.
    let reload = RwSignal::new(0u32);

    Effect::new(move |_| {
        let count = window.get();
        let q = debounced_query.get();
        reload.track();
        if !state.is_authenticated() {
            return;
        }
        loading.set(true);
        spawn_local(async move {
            match cases::load_case_summaries_for_user(0, count, q).await {
                Ok(page) => {
                    cases.set(page.items);
                    total.set(page.total);
                    load_error.set(None);
                }
                Err(e) => load_error.set(Some(err_text(e))),
            }
            loading.set(false);
        });
    });

    require_login(state, move || {
        let cases_list = move || {
            if let Some(msg) = load_error.get() {
                return view! {
                    <p class="text-sm text-rose-300">"Could not load cases: " {msg}</p>
                }
                .into_any();
            }
            let all = cases.get();
            if all.is_empty() {
                let text = if loading.get() {
                    "Loading\u{2026}"
                } else if query.get().trim().is_empty() {
                    "You have no cases yet."
                } else {
                    "No cases match your search."
                };
                return view! { <p class="text-sm text-slate-400">{text}</p> }.into_any();
            }
            all.into_iter()
                .map(|c| {
                    let case_id = c.id.clone();
                    let is_selected = {
                        let case_id = case_id.clone();
                        move || selected.get().as_deref() == Some(case_id.as_str())
                    };
                    let caps = c.capabilities.clone();
                    let access_badge = access_label(&caps)
                        .map(|(label, classes)| {
                            view! { <span class=badge(classes)>{label}</span> }.into_any()
                        })
                        .unwrap_or_else(|| ().into_any());
                    let inactive_badge = c
                        .inactive
                        .then(|| {
                            view! {
                                <span class=badge("bg-amber-500/15 text-amber-300 ring-1 ring-inset ring-amber-500/30")>
                                    "\u{26a0} No activity in 30+ days"
                                </span>
                            }
                            .into_any()
                        })
                        .unwrap_or_else(|| ().into_any());
                    let status = c.status;
                    let name = c.name.clone();
                    let owner = c.owner_full_name();
                    let select = {
                        let case_id = case_id.clone();
                        move |_| {
                            selected.set(Some(case_id.clone()));
                            open_folder.set(None);
                        }
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
                            <span class="min-w-0 truncate font-medium">{name}</span>
                            <span class=badge(status.badge_classes())>{status.label()}</span>
                        </div>
                        <div class="mt-2 flex flex-wrap items-center gap-2 text-xs text-slate-400">
                            <span>"Owner: " {owner}</span>
                            {access_badge}
                            {inactive_badge}
                        </div>
                    </button>
                }
                .into_any()
                })
                .collect_view()
                .into_any()
        };

        let footer = move || {
            let shown = cases.get().len() as i64;
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
            match cases.get().into_iter().find(|c| c.id == id) {
                Some(c) => view! {
                    <CaseDetail summary=c reload=reload open_folder=open_folder admin_read=false />
                }
                .into_any(),
                None => view! {
                    <p class="text-sm text-slate-400">"Case not found."</p>
                }
                .into_any(),
            }
        }
    }
        };

        let input_class = "w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/40";

        // Debounce the search: update the visible input immediately, but wait 1s of
        // idle typing before firing the fetch (and resetting the window).
        let mut on_search = debounce(std::time::Duration::from_secs(1), move |val: String| {
            window.set(PAGE_SIZE);
            debounced_query.set(val);
        });

        view! {
        <Layout title="Cases".to_string()>
            <div class="grid gap-6 lg:grid-cols-[22rem_1fr]">
                <div
                    class="space-y-4 lg:block"
                    class:hidden=move || selected.get().is_some()
                >
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
                            let val = event_target_value(&ev);
                            query.set(val.clone());
                            on_search(val);
                        }
                    />
                    <div class="space-y-3">{cases_list}</div>
                    {footer}
                </div>
                <div class="lg:block" class:hidden=move || selected.get().is_none()>
                    <Show when=move || selected.get().is_some()>
                        <button
                            on:click=move |_| selected.set(None)
                            class="mb-4 inline-flex items-center gap-1.5 rounded-lg border border-slate-700 px-3 py-1.5 text-sm font-medium text-slate-200 hover:bg-slate-800 lg:hidden"
                        >
                            "\u{2190} Back to cases"
                        </button>
                    </Show>
                    {detail}
                </div>
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
    let intake = CaseIntakeState::new();
    let error = RwSignal::new(String::new());

    let input_class = "w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/40";
    let label_class = "block text-xs font-medium text-slate-400";

    require_login(state, move || {
        let submit = {
            let navigate = navigate.clone();
            move |_| {
                let navigate = navigate.clone();
                let status =
                    CaseStatus::from_slug(&status.get_untracked()).unwrap_or(CaseStatus::Open);
                let intake = intake.value();
                if let Err(message) = intake.validate() {
                    error.set(message);
                    return;
                }
                let name_val = name.get_untracked();
                spawn_local(async move {
                    match cases::create_case(name_val, status, intake).await {
                        Ok(_) => navigate("/cases", Default::default()),
                        Err(e) => error.set(err_text(e)),
                    }
                });
            }
        };

        view! {
        <Layout title="New case".to_string()>
            <div class="mx-auto max-w-2xl space-y-6">
                <div class="rounded-xl border border-slate-800 bg-slate-900 p-6 space-y-5">
                    <div>
                        <label class=label_class>
                            "Case name "
                            <span class="text-rose-400" aria-hidden="true">"*"</span>
                        </label>
                        <input
                            class=format!("mt-1 {input_class}")
                            placeholder="e.g. Rivera custody support"
                            required
                            prop:value=move || name.get()
                            on:input=move |ev| name.set(event_target_value(&ev))
                        />
                    </div>
                    <Show when=move || {
                        !matches!(state.role(), Some(AccountRole::Client))
                    }>
                        <div>
                            <label class=label_class>"Status"</label>
                            <select
                                class=format!("mt-1 {input_class}")
                                on:change=move |ev| status.set(event_target_value(&ev))
                            >
                                {CaseStatus::STAFF_SELECTABLE
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
                    </Show>
                    // A client's case goes to an admin for review, so offering
                    // them a status choice would be a lie.
                    <Show when=move || matches!(state.role(), Some(AccountRole::Client))>
                        <p class="rounded-lg bg-slate-800/60 px-3 py-2 text-sm text-slate-400">
                            "A coordinator will review this case before it is opened."
                        </p>
                    </Show>
                    <div class="border-t border-slate-800 pt-5">
                        <div class="mb-5">
                            <h2 class="text-sm font-semibold text-slate-200">"Case intake"</h2>
                            <p class="mt-1 text-sm text-slate-400">"Fields marked with * are required."</p>
                        </div>
                        <CaseIntakeFields state=intake />
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

/// The case detail panel shared by the normal case page and the admin read view.
#[component]
pub fn CaseDetail(
    summary: CaseSummary,
    reload: RwSignal<u32>,
    open_folder: RwSignal<Option<String>>,
    admin_read: bool,
) -> impl IntoView {
    let state = expect_context::<AppState>();
    let case_id = summary.id.clone();

    let owner = StoredValue::new(Owner::current().expect("component owner"));

    // Capability gates for this case, resolved server-side and delivered with
    // the case summary. No implicit grants for owners or admins.
    let caps = summary.capabilities.clone();
    // Mirrors `CaseStatus::accepts_changes`: a declined case refuses writes
    // server-side, so the UI hides the write affordances too.
    let accepts_changes = summary.status.accepts_changes();
    let can_edit = caps.contains(&CaseCapability::EditCase) && accepts_changes;
    let role = state
        .current_user_summary
        .with_untracked(|user| user.as_ref().map(|user| user.role));
    let has_operations_admin_permissions =
        role.is_some_and(|role| role.has_operations_admin_permissions());
    let has_information_management_access = state.has_information_management_access();
    let is_site_admin = role.is_some_and(|role| role.is_site_admin());
    let is_client = matches!(role, Some(AccountRole::Client));
    let can_note = caps.contains(&CaseCapability::AddNotes) && accepts_changes;
    let can_view_evidence = caps.contains(&CaseCapability::ViewEvidence);
    let can_manage_case_information = can_edit;
    let has_stored_write_capability = caps.iter().any(|cap| cap.is_write());
    let can_read_case_material = admin_read || can_view_evidence;

    // The full case behind the summary — the heavy sub-resources are pulled on
    // demand only for the open case, keeping the list load lightweight.
    let case_sv = StoredValue::new(case_id.clone());
    let detail = RwSignal::new(None::<Case>);
    let detail_generation = RwSignal::new(0u64);
    // Tracks the in-flight fetch of the full case so the view can show a loading
    // state instead of a premature "empty" one while the request is pending.
    let detail_loading = RwSignal::new(true);
    {
        let case_id = case_id.clone();
        Effect::new(move |_| {
            reload.track();
            let case_id = case_id.clone();
            detail_loading.set(true);
            detail_generation.update(|generation| *generation += 1);
            let generation = detail_generation.get_untracked();
            spawn_local(async move {
                let loaded = if admin_read {
                    cases::load_admin_case(case_id).await
                } else {
                    cases::load_case(case_id).await
                };
                if detail_generation.get_untracked() != generation {
                    return;
                }
                detail.set(loaded.ok().flatten());
                detail_loading.set(false);
            });
        });
    }

    let owner_name = StoredValue::new(summary.owner_full_name());

    // Owner picker (edit mode only)
    let owner_query = RwSignal::new(String::new());
    let owner_label = RwSignal::new(String::new());
    let owner_results = RwSignal::new(Vec::<UserSummary>::new());
    let owner_picker_open = RwSignal::new(false);
    let active_owner_result = RwSignal::new(None::<usize>);
    let owner_search_generation = RwSignal::new(0u64);
    let owner_search_id = StoredValue::new(format!("case-owner-search-{case_id}"));
    let owner_results_id = StoredValue::new(format!("case-owner-results-{case_id}"));

    // Refetch matching users whenever the query changes while the picker is
    // open. An empty query returns a starting set. Only editors ever open it.
    Effect::new(move |_| {
        if !owner_picker_open.get() {
            return;
        }
        let q = owner_query.get();
        owner_search_generation.update(|generation| *generation += 1);
        let generation = owner_search_generation.get_untracked();
        spawn_local(async move {
            let response = search_users(q).await;
            if owner_search_generation.get_untracked() != generation {
                return;
            }
            if let Ok(list) = response {
                owner_results.set(list);
                active_owner_result.set(None);
            }
        });
    });

    let live_case = move || detail.get();

    let input_class = "w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/40";
    // The card styling shared by every panel on the page. Named `panel` rather
    // than `section` so it is never confused with a case information *section*.
    let panel = "rounded-xl border border-slate-800 bg-slate-900 p-4";

    // --- global edit mode ---
    let editing = RwSignal::new(false);
    let edit_name = RwSignal::new(String::new());
    let edit_status = RwSignal::new(String::new());
    let edit_owner = RwSignal::new(String::new());
    let edit_error = RwSignal::new(String::new());
    let case_name_input_id = StoredValue::new(format!("case-name-{case_id}"));
    let case_status_input_id = StoredValue::new(format!("case-status-{case_id}"));

    let edit_props: RwSignal<Vec<PropRow>> = RwSignal::new(Vec::new());
    let props_error = RwSignal::new(String::new());
    let row_seq = RwSignal::new(0usize);

    // Keep the shared case-detail signature stable while evidence is unavailable.
    let _ = open_folder;

    let make_row =
        move |key: String, value: String, section: String, visibility: Visibility| -> PropRow {
            let id = row_seq.get_untracked();
            row_seq.set(id + 1);
            owner.with_value(|o| {
                o.with(|| PropRow {
                    id,
                    key: RwSignal::new(key),
                    value: RwSignal::new(value),
                    section: RwSignal::new(section),
                    visibility,
                })
            })
        };

    let begin_edit = move |_| {
        if let Some(c) = live_case() {
            edit_name.set(c.name.clone());
            edit_status.set(c.status.slug().to_string());
            edit_owner.set(c.owner_id.clone());
            owner_label.set(owner_name.get_value());
            owner_query.set(String::new());
            owner_picker_open.set(false);
            edit_error.set(String::new());
            props_error.set(String::new());
            edit_props.set(
                c.properties
                    .iter()
                    .map(|p| {
                        make_row(
                            p.key.clone(),
                            p.value.clone(),
                            p.section.clone(),
                            p.visibility,
                        )
                    })
                    .collect(),
            );
            editing.set(true);
        }
    };

    let cancel_edit = move |_| {
        edit_error.set(String::new());
        props_error.set(String::new());
        owner_picker_open.set(false);
        editing.set(false);
    };

    let save_edit = move |_| {
        let case_id = case_sv.get_value();
        let name = edit_name.get_untracked();
        let status_slug = edit_status.get_untracked();
        let owner = edit_owner.get_untracked();
        let can_see_restricted = state.is_volunteer_or_admin();
        let properties = edit_props.get_untracked();
        spawn_local(async move {
            if can_edit {
                if let Err(e) = cases::set_case_name(case_id.clone(), name).await {
                    edit_error.set(err_text(e));
                    return;
                }
                if can_see_restricted {
                    if let Some(s) = CaseStatus::from_slug(&status_slug) {
                        if let Err(e) = cases::set_case_status(case_id.clone(), s).await {
                            edit_error.set(err_text(e));
                            return;
                        }
                    }
                }
                if is_site_admin {
                    if let Err(e) = cases::set_case_owner(case_id.clone(), owner).await {
                        edit_error.set(err_text(e));
                        return;
                    }
                }

                for visibility in Visibility::ALL {
                    if visibility.is_restricted() && !can_see_restricted {
                        continue;
                    }
                    let rows = properties
                        .iter()
                        .filter(|row| row.visibility == visibility)
                        .map(|row| CaseProperty {
                            key: row.key.get_untracked(),
                            value: row.value.get_untracked(),
                            section: row.section.get_untracked(),
                            visibility,
                        })
                        .collect();
                    if let Err(e) =
                        case_properties::set_case_properties(case_id.clone(), visibility, rows)
                            .await
                    {
                        props_error.set(err_text(e));
                        return;
                    }
                }
            }
            edit_error.set(String::new());
            props_error.set(String::new());
            editing.set(false);
            reload.update(|n| *n += 1);
        });
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
                            <div class="mb-2 flex flex-wrap gap-2">
                                <span class="rounded-full bg-sky-500/15 px-2 py-0.5 text-xs font-medium text-sky-300 ring-1 ring-sky-500/30">"Legacy"</span>
                                <span class="rounded-full bg-slate-700/50 px-2 py-0.5 text-xs text-slate-300">"Shared historical note"</span>
                            </div>
                            <p class="text-sm text-slate-200">{n.body}</p>
                            <p class="mt-1 text-xs text-slate-500">
                                {n.author} " · " {n.created_at}
                            </p>
                            {n.addenda.into_iter().map(|addendum| view! {
                                <div class="mt-3 border-l-2 border-sky-500/40 pl-3">
                                    <p class="text-xs font-semibold uppercase tracking-wide text-sky-300">"Signed addendum"</p>
                                    <p class="mt-1 text-sm text-slate-200">{addendum.information}</p>
                                    <p class="mt-1 text-xs text-slate-500">
                                        "Reason: " {addendum.reason} " · " {addendum.author} " · " {addendum.signed_at}
                                    </p>
                                    {(!addendum.follow_up.is_empty()).then(|| view! {
                                        <p class="mt-1 text-xs text-slate-400">"Follow-up: " {addendum.follow_up}</p>
                                    })}
                                </div>
                            }).collect_view()}
                        </div>
                    }
                    .into_any()
                })
                .collect_view()
                .into_any()
        }
    };

    // The rendering of one visibility: its sections, each listing that section's
    // properties.
    let sections_view = move |visibility: Visibility, groups: Vec<(String, Vec<CaseProperty>)>| {
        if groups.is_empty() {
            return view! {
                <p class="px-4 py-5 text-sm text-slate-500">
                    "No information has been added yet."
                </p>
            }
            .into_any();
        }
        groups
            .into_iter()
            .map(|(name, props)| {
                let heading = sections::label(&name).to_string();
                let property_rows = if editing.get() && can_edit {
                    let row_section = name.clone();
                    let add_section = name.clone();
                    view! {
                        <div class="space-y-2">
                            <For
                                each=move || {
                                    let section = row_section.clone();
                                    edit_props
                                        .get()
                                        .into_iter()
                                        .filter(move |row| {
                                            row.visibility == visibility
                                                && row.section.get() == section
                                        })
                                        .collect::<Vec<_>>()
                                }
                                key=|row| row.id
                                let:row
                            >
                                <div class="flex flex-col gap-2 sm:flex-row">
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
                                            edit_props.update(|rows| {
                                                rows.retain(|entry| entry.id != row.id)
                                            })
                                        }
                                        class="shrink-0 rounded-lg border border-rose-500/40 px-3 py-2 text-sm font-medium text-rose-300 hover:bg-rose-500/10"
                                    >
                                        "Remove"
                                    </button>
                                </div>
                            </For>
                            <button
                                on:click=move |_| {
                                    let row = make_row(
                                        String::new(),
                                        String::new(),
                                        add_section.clone(),
                                        visibility,
                                    );
                                    edit_props.update(|rows| rows.push(row));
                                }
                                class="rounded-lg border border-slate-700 px-3 py-1.5 text-sm font-medium text-slate-200 hover:bg-slate-800"
                            >
                                "+ Add property"
                            </button>
                        </div>
                    }
                    .into_any()
                } else {
                    props
                        .into_iter()
                        .map(|p| {
                            let value = p.value.clone();
                            // A long property name is free text, so it must be
                            // able to wrap mid-token; otherwise it sets the row's
                            // intrinsic width and stretches the whole page.
                            let shown = if value.trim().is_empty() {
                                view! {
                                    <span class="min-w-0 wrap-anywhere text-slate-600 italic sm:text-right">
                                        "Not filled in"
                                    </span>
                                }
                                    .into_any()
                            } else {
                                view! {
                                    <span class="min-w-0 wrap-anywhere text-slate-200 sm:text-right">
                                        {value}
                                    </span>
                                }
                                    .into_any()
                            };
                            view! {
                                <div class="flex flex-col gap-1 border-b border-slate-800 py-1.5 text-sm sm:flex-row sm:items-baseline sm:justify-between sm:gap-4">
                                    <span class="min-w-0 wrap-anywhere text-slate-400 sm:flex-1">{p.key}</span>
                                    {shown}
                                </div>
                            }
                            .into_any()
                        })
                        .collect_view()
                        .into_any()
                };
                view! {
                    <section class="border-t border-slate-800 px-4 py-5 first:border-t-0">
                        <h4 class="text-base font-semibold text-slate-100">
                            {heading}
                        </h4>
                        <div class="mt-2">{property_rows}</div>
                    </section>
                }
                .into_any()
            })
            .collect_view()
            .into_any()
    };

    // Case information: what is recorded about the case, grouped by who can see
    // it and then by section. Anything the viewer may not see never reaches the
    // browser, so this renders only what they are allowed to know about. The
    // case's *files* are not here — they have their own folder tree below.
    let case_information = move || {
        if !can_read_case_material {
            return ().into_any();
        }
        let Some(c) = live_case() else {
            return ().into_any();
        };
        let mut grouped = group_case_properties(&c);
        // A visibility with nothing in it yet still needs somewhere to add the
        // first entry, so every one the viewer may write to is shown.
        if can_edit {
            for visibility in Visibility::ALL {
                let allowed = !visibility.is_restricted() || state.is_volunteer_or_admin();
                if allowed && !grouped.iter().any(|(v, _)| *v == visibility) {
                    grouped.push((visibility, Vec::new()));
                }
            }
            grouped.sort_by_key(|(v, _)| Visibility::ALL.iter().position(|x| x == v));
        }
        if grouped.is_empty() {
            return ().into_any();
        }

        let groups = grouped
            .into_iter()
            .map(|(visibility, groups)| {
                let (heading, container_class, header_class) = match visibility {
                    Visibility::VolunteerOnly => (
                        "Volunteer only",
                        "overflow-hidden rounded-xl border border-amber-500/30 bg-slate-900",
                        "border-b border-amber-500/20 bg-amber-500/10 px-4 py-4",
                    ),
                    Visibility::Shared => (
                        "Shared with volunteers and client",
                        "overflow-hidden rounded-xl border border-slate-800 bg-slate-900",
                        "border-b border-slate-800 bg-slate-800/40 px-4 py-4",
                    ),
                };
                let section_groups = if groups.is_empty() && editing.get() {
                    vec![(String::new(), Vec::new())]
                } else {
                    groups
                };
                view! {
                    <section class=container_class>
                        <div class=header_class>
                            <div>
                                <h3 class="text-base font-semibold text-slate-100">{heading}</h3>
                                <p class="mt-1 text-sm text-slate-400">
                                    {visibility.description()}
                                </p>
                            </div>
                        </div>
                        {sections_view(visibility, section_groups)}
                    </section>
                }
                .into_any()
            })
            .collect_view();

        view! {
            <div>
                <div class="mb-3">
                    <h2 class="text-lg font-semibold text-slate-100">"Case information"</h2>
                    <p class="mt-1 text-sm text-slate-500">
                        "Information organized by who can see it."
                    </p>
                </div>
                <div class="space-y-5">{groups}</div>
                <Show when=move || !props_error.get().is_empty()>
                    <p class="mt-3 text-sm text-rose-400">{move || props_error.get()}</p>
                </Show>
            </div>
        }
        .into_any()
    };

    // Evidence is intentionally unavailable while its replacement is designed.
    let evidence_panel = move || {
        if !can_read_case_material {
            return ().into_any();
        }
        view! {
            <section aria-labelledby="evidence-heading">
                <div class="mb-3">
                    <h2 id="evidence-heading" class="text-lg font-semibold text-slate-100">
                        "Evidence"
                    </h2>
                </div>
                <div class="rounded-xl border border-amber-400 bg-amber-50 p-4">
                    <div class="flex items-start gap-3">
                        <span aria-hidden="true" class="text-xl">"🚧"</span>
                        <div>
                            <h3 class="font-semibold text-amber-900">"Under construction"</h3>
                            <p class="mt-1 text-sm text-amber-800">
                                {EVIDENCE_UNAVAILABLE_MESSAGE}
                            </p>
                        </div>
                    </div>
                </div>
            </section>
        }
        .into_any()
    };

    let log_open = RwSignal::new(false);
    let log_case_id = StoredValue::new(case_id.clone());
    let log_region_id = StoredValue::new(format!("case-change-log-{case_id}"));
    let audit_view = move || {
        if !log_open.get() {
            return ().into_any();
        }
        view! { <ChangeLog scope=AuditScope::Case entity_id=log_case_id.get_value() /> }.into_any()
    };

    let details_section = move || {
        let Some(c) = live_case() else {
            return ().into_any();
        };
        if !editing.get() || !can_edit {
            let owner_name = owner_name.get_value();
            let status = c.status;
            let terms_accepted = c.terms_accepted.clone();
            let client_banner = if state.is_volunteer_or_admin() {
                ().into_any()
            } else {
                // A decline is a dead end without a next step.
                let next_step = if status == CaseStatus::Declined {
                    view! {
                        <p class="mt-2 text-sm text-slate-400">
                            "If your circumstances have changed, you can "
                            <A
                                href="/cases/new"
                                attr:class="font-medium text-primary-400 hover:text-primary-300"
                            >
                                "submit a new case"
                            </A>
                            "."
                        </p>
                    }
                    .into_any()
                } else {
                    ().into_any()
                };
                view! {
                    <div>
                        <p class=format!(
                            "mt-3 rounded-lg px-3 py-2 text-sm {}",
                            c.status.badge_classes(),
                        )>{c.client_message()}</p>
                        {next_step}
                    </div>
                }
                .into_any()
            };
            let edit_controls = if editing.get() {
                view! {
                    <div class="flex shrink-0 items-center gap-2">
                        <button
                            on:click=save_edit
                            class="rounded-lg bg-primary-500 px-3 py-1.5 text-sm font-semibold text-white hover:bg-primary-600"
                        >
                            "Save"
                        </button>
                        <button
                            on:click=cancel_edit
                            class="rounded-lg border border-slate-700 px-3 py-1.5 text-sm font-medium text-slate-300 hover:bg-slate-800"
                        >
                            "Cancel"
                        </button>
                    </div>
                }
                .into_any()
            } else if can_manage_case_information {
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
                <div class=panel>
                    <div class="flex items-start justify-between gap-3">
                        <div class="min-w-0">
                            <h2 class="text-lg font-semibold">{c.name.clone()}</h2>
                            <p class="mt-1 text-sm text-slate-400">
                                "Filed by "
                                <ProfileLink user_id=c.owner_id.clone() name=owner_name />
                            </p>
                            {terms_accepted.map(|accepted| view! {
                                <p class="mt-1 text-xs text-slate-500">
                                    "Terms accepted " {accepted}
                                </p>
                            })}
                        </div>
                        <div class="flex shrink-0 items-center gap-2">
                            <span class=badge(status.badge_classes())>{status.label()}</span>
                            {edit_controls}
                        </div>
                    </div>
                    {client_banner}
                    {if admin_read && !has_stored_write_capability {
                        view! {
                            <p class="mt-3 rounded-lg border border-sky-500/30 bg-sky-500/10 px-3 py-2 text-sm text-sky-200">
                                "Admin read-only view. You can inspect this case without assigning yourself, but stored case capabilities are still required for any changes."
                            </p>
                        }
                            .into_any()
                    } else {
                        view! {
                            <Show when=move || !can_edit && !can_note>
                                <p class="mt-2 text-xs text-slate-500">
                                    "You have view-only access to this case."
                                </p>
                            </Show>
                        }
                            .into_any()
                    }}
                </div>
            }
            .into_any();
        }

        // --- edit mode ---
        // The dropdown of owner search results; clicking one selects it.
        let owner_result_list = move || {
            if !owner_picker_open.get() {
                return ().into_any();
            }
            let items = owner_results.get();
            if items.is_empty() {
                return view! {
                    <div
                        id=owner_results_id.get_value()
                        role="listbox"
                        class="absolute z-10 mt-1 w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-xs text-slate-500"
                    >
                        "No matching users."
                    </div>
                }
                .into_any();
            }
            let rows = items
                .into_iter()
                .enumerate()
                .map(|(index, u)| {
                    let id = u.id.clone();
                    let name = u.full_name();
                    let label = format!("{} ({})", u.full_name(), u.id);
                    let option_id = format!("{}-{index}", owner_results_id.get_value());
                    let select = move |_| {
                        edit_owner.set(id.clone());
                        owner_label.set(name.clone());
                        owner_picker_open.set(false);
                        active_owner_result.set(None);
                    };
                    view! {
                        <button
                            id=option_id
                            type="button"
                            role="option"
                            tabindex="-1"
                            aria-selected=move || (active_owner_result.get() == Some(index)).to_string()
                            on:click=select
                            class=move || if active_owner_result.get() == Some(index) {
                                "block w-full truncate bg-slate-800 px-3 py-1.5 text-left text-sm text-slate-100"
                            } else {
                                "block w-full truncate px-3 py-1.5 text-left text-sm text-slate-200 hover:bg-slate-800"
                            }
                        >
                            {label}
                        </button>
                    }
                    .into_any()
                })
                .collect_view();
            view! {
                <div
                    id=owner_results_id.get_value()
                    role="listbox"
                    class="absolute z-10 mt-1 max-h-48 w-full overflow-y-auto rounded-lg border border-slate-700 bg-slate-950"
                >
                    {rows}
                </div>
            }
            .into_any()
        };
        view! {
            <div class=panel>
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
                        <label for=case_name_input_id.get_value() class="text-xs font-medium text-slate-400">
                            "Case name"
                        </label>
                        <input
                            id=case_name_input_id.get_value()
                            class=input_class
                            prop:value=move || edit_name.get()
                            on:input=move |ev| edit_name.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="grid gap-4 sm:grid-cols-2">
                        <Show when=move || state.is_volunteer_or_admin()>
                        <div>
                            <label for=case_status_input_id.get_value() class="text-xs font-medium text-slate-400">
                                "Status"
                            </label>
                            <select
                                id=case_status_input_id.get_value()
                                class=input_class
                                on:change=move |ev| edit_status.set(event_target_value(&ev))
                            >
                                {CaseStatus::STAFF_SELECTABLE
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
                        </Show>
                        <div>
                            <label
                                for=owner_search_id.get_value()
                                class="text-xs font-medium text-slate-400"
                            >
                                "Owner (who filed it)"
                            </label>
                            {if is_site_admin {
                                view! {
                                    <div class="relative">
                                        <input
                                            id=owner_search_id.get_value()
                                            type="search"
                                            role="combobox"
                                            aria-autocomplete="list"
                                            aria-expanded=move || owner_picker_open.get().to_string()
                                            aria-controls=owner_results_id.get_value()
                                            aria-activedescendant=move || {
                                                active_owner_result
                                                    .get()
                                                    .map(|index| format!("{}-{index}", owner_results_id.get_value()))
                                            }
                                            class=input_class
                                            placeholder="Search users by name or email\u{2026}"
                                            prop:value=move || {
                                                if owner_picker_open.get() {
                                                    owner_query.get()
                                                } else {
                                                    owner_label.get()
                                                }
                                            }
                                            on:focus=move |_| {
                                                owner_query.set(String::new());
                                                owner_picker_open.set(true);
                                                active_owner_result.set(None);
                                            }
                                            on:input=move |ev| {
                                                owner_picker_open.set(true);
                                                active_owner_result.set(None);
                                                owner_query.set(event_target_value(&ev));
                                            }
                                            on:keydown=move |event: leptos::ev::KeyboardEvent| {
                                                let count = owner_results.with(Vec::len);
                                                match event.key().as_str() {
                                                    "ArrowDown" if count > 0 => {
                                                        event.prevent_default();
                                                        active_owner_result.update(|active| {
                                                            *active = Some(active.map_or(0, |index| (index + 1).min(count - 1)));
                                                        });
                                                    }
                                                    "ArrowUp" if count > 0 => {
                                                        event.prevent_default();
                                                        active_owner_result.update(|active| {
                                                            *active = Some(active.map_or(count - 1, |index| index.saturating_sub(1)));
                                                        });
                                                    }
                                                    "Enter" => {
                                                        if let Some(index) = active_owner_result.get_untracked() {
                                                            event.prevent_default();
                                                            if let Some(user) = owner_results
                                                                .get_untracked()
                                                                .get(index)
                                                                .cloned()
                                                            {
                                                                let name = user.full_name();
                                                                edit_owner.set(user.id);
                                                                owner_label.set(name);
                                                                owner_picker_open.set(false);
                                                                active_owner_result.set(None);
                                                            }
                                                        }
                                                    }
                                                    "Escape" => {
                                                        owner_picker_open.set(false);
                                                        active_owner_result.set(None);
                                                    }
                                                    _ => {}
                                                }
                                            }
                                        />
                                        {owner_result_list}
                                    </div>
                                }
                                    .into_any()
                            } else {
                                view! {
                                    <input
                                        id=owner_search_id.get_value()
                                        class=input_class
                                        prop:value=move || owner_label.get()
                                        disabled=true
                                        title="Only a site administrator can change the owner."
                                    />
                                }
                                    .into_any()
                            }}
                        </div>
                    </div>
                </div>
            </div>
        }
        .into_any()
    };

    view! {
        <div>
            // While the full case is loading, show only a loading indicator and
            // hide the (empty) section scaffolding beneath it.
            <div class=move || {
                if detail_loading.get() { panel.to_string() } else { "hidden".to_string() }
            }>
                <Loading label="Loading case details\u{2026}" />
            </div>
            <div class=move || {
                if detail_loading.get() { "hidden".to_string() } else { "space-y-6".to_string() }
            }>
            {details_section}

            {evidence_panel}

            {case_information}

            // People on the case: staff-only, like Case Notes.
            {if is_client || !has_information_management_access {
                ().into_any()
            } else {
                view! {
                    <CaseContactsPanel
                        case_id=case_sv.get_value()
                        can_edit=can_edit && has_information_management_access
                    />
                }
                .into_any()
            }}

            // Notes: clients retain only legacy shared notes; staff get structured Case Notes.
            {if is_client {
                view! {
                    <div class=panel>
                        <h3 class="text-sm font-semibold text-slate-200">"Notes"</h3>
                        <div class="mt-3 space-y-2">{notes_view}</div>
                    </div>
                }
                    .into_any()
            } else {
                view! { <CaseNotesPanel case_id=case_sv.get_value() can_add=can_note /> }.into_any()
            }}

            // Audit log
            {if has_operations_admin_permissions {
                view! {
                    <div class=panel>
                        <div class="flex items-center justify-between">
                            <h3 class="text-sm font-semibold text-slate-200">"Change log"</h3>
                            <button
                                type="button"
                                on:click=move |_| log_open.update(|o| *o = !*o)
                                aria-expanded=move || log_open.get().to_string()
                                aria-controls=log_region_id.get_value()
                                class="rounded-lg border border-slate-700 px-2 py-1 text-xs font-medium text-slate-300 hover:bg-slate-800"
                            >
                                {move || if log_open.get() { "Hide" } else { "Open change log" }}
                            </button>
                        </div>
                        <div id=log_region_id.get_value() class="mt-3 space-y-1.5">{audit_view}</div>
                    </div>
                }
                    .into_any()
            } else {
                view! {}.into_any()
            }}
            </div>
        </div>
    }
}
