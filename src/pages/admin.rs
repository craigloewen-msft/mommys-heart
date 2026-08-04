use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::admin_requests::AdminRequestCenter;
use crate::components::change_log::ChangeLog;
use crate::components::email_failures::EmailFailureLog;
use crate::components::guard::require_operations_admin;
use crate::components::layout::Layout;
use crate::server_fns::audit::AuditScope;
use crate::server_fns::capabilities::{CaseCapability, CasePreset};
use crate::server_fns::cases::CaseSummary;
use crate::server_fns::err_text;
use crate::server_fns::users::AccountRole;
use crate::server_fns::users::User;
use crate::state::AppState;

/// How many users the admin list loads per "page" (each "Load more" click grows
/// the visible window by this much).
const PAGE_SIZE: i64 = 4;

/// A single case assignment being edited in the admin capabilities "Edit" flow.
/// Holds the working capability set and a "marked for removal" flag; nothing is
/// persisted until the admin clicks "Save", at which point the whole draft is
/// diffed against the originals and applied in one batch.
#[derive(Clone)]
struct DraftAssignment {
    case_id: String,
    name: String,
    caps: RwSignal<Vec<CaseCapability>>,
    removed: RwSignal<bool>,
}

/// Order-insensitive equality of two capability sets.
fn same_caps(a: &[CaseCapability], b: &[CaseCapability]) -> bool {
    a.len() == b.len() && a.iter().all(|c| b.contains(c))
}

fn badge(classes: &str) -> String {
    format!("inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {classes}")
}

/// Admin dashboard: browse users (server-side paginated + searchable) and manage
/// their per-case capabilities.
#[component]
pub fn AdminDashboardPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    // Search text (bound to the input for instant feedback) and its debounced
    // mirror (drives the actual fetch, so we don't hit the server on every
    // keystroke). The fetched window of users plus the total match count.
    let query = RwSignal::new(String::new());
    let debounced_query = RwSignal::new(String::new());
    let results = RwSignal::new(Vec::<User>::new());
    let total = RwSignal::new(0i64);
    // How many rows the current window requests; grows on "Load more".
    let window = RwSignal::new(PAGE_SIZE);
    let loading = RwSignal::new(false);
    let load_error = RwSignal::new(None::<String>);
    // Bumped after a mutation to force the current window to reload.
    let reload = RwSignal::new(0u32);
    let requests_tab = RwSignal::new(false);
    // Whether the collapsible "Email delivery failures" panel is open. Mounting
    // the viewer only on open defers its fetch until the admin asks for it.
    let failures_open = RwSignal::new(false);

    // (Re)load the window whenever the debounced query, window size, or reload
    // tick changes — but only once a session is confirmed (server functions run
    // in the browser after hydration). We always fetch `[0, window)` so both
    // search changes and post-mutation refreshes are handled by one code path.
    Effect::new(move |_| {
        if requests_tab.get() {
            return;
        }
        let count = window.get();
        let q = debounced_query.get();
        reload.track();
        if !state.has_operations_admin_permissions() {
            return;
        }
        loading.set(true);
        spawn_local(async move {
            match crate::server_fns::users::list_users_page(0, count, q).await {
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

    require_operations_admin(state, move || {
        let actor = state
            .current_user_summary
            .get()
            .expect("admin guard requires a current user");
        let is_site_admin = actor.role.is_site_admin();
        let actor_user_id = StoredValue::new(actor.id);
        let list = move || {
            if let Some(msg) = load_error.get() {
                return view! {
                    <p class="text-sm text-rose-300">"Could not load users: " {msg}</p>
                }
                .into_any();
            }
            let items = results.get();
            if items.is_empty() {
                let text = if loading.get() {
                    "Loading\u{2026}"
                } else {
                    "No users match your search."
                };
                return view! { <p class="text-sm text-slate-500">{text}</p> }.into_any();
            }
            items
                .into_iter()
                .map(|u| {
                    view! {
                        <UserCard
                            user=u
                            reload=reload
                            actor_user_id=actor_user_id.get_value()
                            is_site_admin=is_site_admin
                        />
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

        // Debounce the search: update the visible input immediately, but wait 1s of
        // idle typing before firing the fetch (and resetting the window).
        let mut on_search = debounce(std::time::Duration::from_secs(1), move |val: String| {
            window.set(PAGE_SIZE);
            debounced_query.set(val);
        });

        view! {
        <Layout title="Admin".to_string()>
            <div class="mb-6 border-b border-slate-800" role="tablist" aria-label="Admin sections">
                <div class="flex gap-6">
                    <button
                        type="button"
                        role="tab"
                        aria-selected=move || (!requests_tab.get()).to_string()
                        on:click=move |_| requests_tab.set(false)
                        class=move || if requests_tab.get() {
                            "border-b-2 border-transparent px-1 pb-3 text-sm font-medium text-slate-400 hover:text-slate-200"
                        } else {
                            "border-b-2 border-primary-400 px-1 pb-3 text-sm font-medium text-primary-300"
                        }
                    >
                        "Case access"
                    </button>
                    <button
                        type="button"
                        role="tab"
                        aria-selected=move || requests_tab.get().to_string()
                        on:click=move |_| requests_tab.set(true)
                        class=move || if requests_tab.get() {
                            "border-b-2 border-primary-400 px-1 pb-3 text-sm font-medium text-primary-300"
                        } else {
                            "border-b-2 border-transparent px-1 pb-3 text-sm font-medium text-slate-400 hover:text-slate-200"
                        }
                    >
                        <span class="inline-flex items-center gap-2">
                            "Requests"
                            {move || {
                                let count = if is_site_admin {
                                    state.admin_request_pending.get()
                                } else {
                                    0
                                };
                                (count > 0).then(|| view! {
                                    <span class="inline-flex min-w-5 items-center justify-center rounded-full bg-primary-500 px-1.5 py-0.5 text-[0.65rem] font-semibold leading-none text-white">
                                        {count}
                                    </span>
                                })
                            }}
                        </span>
                    </button>
                </div>
            </div>

            <div role="tabpanel" class:hidden=move || requests_tab.get()>
                <p class="mb-6 text-sm text-slate-400">
                    {if is_site_admin {
                        "Manage global roles and case capabilities."
                    } else {
                        "Manage your case access and request changes for other users."
                    }}
                </p>
                <input
                    class="mb-4 w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/40"
                    placeholder="Search users by name or email"
                    prop:value=move || query.get()
                    on:input=move |ev| {
                        let val = event_target_value(&ev);
                        query.set(val.clone());
                        on_search(val);
                    }
                />
                <div class="space-y-4">{list}</div>
                {footer}
                <div class="mb-6 rounded-xl border border-slate-800 bg-slate-900 p-5">
                    <div class="flex items-center justify-between">
                        <div>
                            <h2 class="text-sm font-semibold text-slate-200">
                                "Email delivery failures"
                            </h2>
                            <p class="mt-0.5 text-xs text-slate-500">
                                "Outbound emails that failed to send, newest first \u{2014} check here instead of the server logs."
                            </p>
                        </div>
                        <button
                            on:click=move |_| failures_open.update(|o| *o = !*o)
                            class="rounded-lg border border-slate-700 px-2 py-1 text-xs font-medium text-slate-300 hover:bg-slate-800"
                        >
                            {move || if failures_open.get() { "Hide" } else { "Show" }}
                        </button>
                    </div>
                    <Show when=move || failures_open.get()>
                        <EmailFailureLog />
                    </Show>
                </div>
            </div>

            <div role="tabpanel" class:hidden=move || !requests_tab.get()>
                <p class="mb-6 text-sm text-slate-400">
                    {if is_site_admin {
                        "Review active operations-admin requests and browse past decisions."
                    } else {
                        "Track active requests you submitted and browse their history."
                    }}
                </p>
                <AdminRequestCenter is_site_admin=is_site_admin reload=reload />
            </div>
        </Layout>
    }
    .into_any()
    })
}

/// A management card for a single user. `reload` is bumped after any mutation so
/// the parent list refetches the current window and the card re-renders with
/// fresh data (assignments, role, audit log).
#[component]
fn UserCard(
    user: User,
    reload: RwSignal<u32>,
    actor_user_id: String,
    is_site_admin: bool,
) -> impl IntoView {
    let user_id = user.id.clone();
    let current_role = user.role;
    let can_edit_capabilities_directly = is_site_admin || user_id == actor_user_id;
    let can_request_changes = !is_site_admin && user_id != actor_user_id;

    // --- global role ---
    let role_change_target = StoredValue::new(user_id.clone());
    let role_change = move |ev| {
        if let Some(r) = AccountRole::from_slug(&event_target_value(&ev)) {
            let user_id = role_change_target.get_value();
            spawn_local(async move {
                if crate::server_fns::users::set_user_role(user_id, r)
                    .await
                    .is_ok()
                {
                    reload.update(|n| *n += 1);
                }
            });
        }
    };
    let requested_role = RwSignal::new(current_role.slug().to_string());
    let role_request_note = RwSignal::new(String::new());
    let role_requesting = RwSignal::new(false);
    let role_request_feedback = RwSignal::new(None::<Result<String, String>>);
    let role_request_target = StoredValue::new(user_id.clone());
    let submit_role_request = move |_: leptos::ev::MouseEvent| {
        let Some(role) = AccountRole::from_slug(&requested_role.get_untracked()) else {
            return;
        };
        if role == current_role || role_requesting.get_untracked() {
            return;
        }
        let target_id = role_request_target.get_value();
        let note = role_request_note.get_untracked();
        role_requesting.set(true);
        role_request_feedback.set(None);
        spawn_local(async move {
            match crate::server_fns::admin_requests::request_user_role_change(target_id, role, note)
                .await
            {
                Ok(_) => {
                    role_request_feedback.set(Some(Ok("Role request submitted.".to_string())));
                    role_request_note.set(String::new());
                    reload.update(|value| *value += 1);
                }
                Err(error) => role_request_feedback.set(Some(Err(err_text(error)))),
            }
            role_requesting.set(false);
        });
    };

    // --- add assignment (case typeahead + preset) ---
    // `new_case` holds the *selected* case id; `new_case_label` its display name.
    // The picker searches all cases server-side behind the operations-admin
    // gate, so it scales without loading them all into the browser.
    let new_case = RwSignal::new(String::new());
    let new_case_label = RwSignal::new(String::new());
    let new_preset = RwSignal::new(CasePreset::Viewer.slug().to_string());
    let case_query = RwSignal::new(String::new());
    let case_results = RwSignal::new(Vec::<CaseSummary>::new());
    let picker_open = RwSignal::new(false);

    // Refetch whenever the query changes (and the picker is open). Empty query
    // returns the first handful of cases as a starting point.
    Effect::new(move |_| {
        if !picker_open.get() {
            return;
        }
        let q = case_query.get();
        spawn_local(async move {
            if let Ok(cases) = crate::server_fns::cases::admin_search_cases(q).await {
                case_results.set(cases);
            }
        });
    });

    // Stable, Copy handle to the user id so the edit-mode handlers below can be
    // `Copy` (and thus reused inside the reactive capabilities section).
    let user_sv = StoredValue::new(user_id.clone());

    // Component owner: draft rows create per-row `RwSignal`s inside the "Edit"
    // click handler, whose transient reactive scope is disposed as soon as it
    // returns. Creating them under the component owner instead keeps them alive
    // for the lifetime of the card, so the reactive capabilities section can read
    // them without hitting a "reactive value has been disposed" panic.
    let owner = StoredValue::new(Owner::current().expect("component owner"));
    let make_draft = move |case_id: String, name: String, caps: Vec<CaseCapability>| {
        owner.with_value(|o| {
            o.with(|| DraftAssignment {
                case_id,
                name,
                caps: RwSignal::new(caps),
                removed: RwSignal::new(false),
            })
        })
    };

    // The admin's case cache is now scoped to their own cases, so a user's
    // assignments may reference cases the admin doesn't own. Resolve those names
    // once, admin-side, into a local map (id -> name) for display.
    let case_names = RwSignal::new(std::collections::HashMap::<String, String>::new());
    let assigned_ids: Vec<String> = user
        .assigned_cases
        .iter()
        .map(|a| a.case_id.clone())
        .collect();
    {
        let assigned_ids = assigned_ids.clone();
        Effect::new(move |_| {
            let ids = assigned_ids.clone();
            if ids.is_empty() {
                return;
            }
            spawn_local(async move {
                if let Ok(cases) = crate::server_fns::cases::admin_cases_by_ids(ids).await {
                    case_names.update(|m| {
                        for c in cases {
                            m.insert(c.id, c.name);
                        }
                    });
                }
            });
        });
    }

    let case_name = move |case_id: &str| -> String {
        case_names
            .with(|m| m.get(case_id).cloned())
            .unwrap_or_else(|| case_id.to_string())
    };

    let full_name = user.full_name();
    let email = user.email.clone();

    let originals = StoredValue::new(user.assigned_cases.clone());
    let editing = RwSignal::new(false);
    let draft: RwSignal<Vec<DraftAssignment>> = RwSignal::new(Vec::new());
    let save_error = RwSignal::new(String::new());
    let saving = RwSignal::new(false);
    let capability_request_note = RwSignal::new(String::new());
    let capability_note_id = StoredValue::new(format!("capability-note-{user_id}"));

    let begin_edit = move |_| {
        let rows = originals
            .get_value()
            .into_iter()
            .map(|a| make_draft(a.case_id.clone(), case_name(&a.case_id), a.capabilities))
            .collect::<Vec<_>>();
        draft.set(rows);
        save_error.set(String::new());
        capability_request_note.set(String::new());
        picker_open.set(false);
        new_case.set(String::new());
        new_case_label.set(String::new());
        case_query.set(String::new());
        editing.set(true);
    };

    let cancel_edit = move |_| {
        editing.set(false);
        save_error.set(String::new());
        capability_request_note.set(String::new());
        picker_open.set(false);
        new_case.set(String::new());
        new_case_label.set(String::new());
        case_query.set(String::new());
    };

    // Stage the picked case into the draft (or un-remove it if it was marked for
    // removal). Persisted only on save.
    let add_to_draft = move |_| {
        let case_id = new_case.get_untracked();
        if case_id.is_empty() {
            return;
        }
        if let Some(d) = draft
            .get_untracked()
            .into_iter()
            .find(|d| d.case_id == case_id)
        {
            d.removed.set(false);
        } else {
            let preset =
                CasePreset::from_slug(&new_preset.get_untracked()).unwrap_or(CasePreset::Viewer);
            let label = new_case_label.get_untracked();
            let name = if label.is_empty() {
                case_name(&case_id)
            } else {
                label
            };
            let row = make_draft(case_id.clone(), name, preset.capabilities());
            draft.update(|rows| rows.push(row));
        }
        new_case.set(String::new());
        new_case_label.set(String::new());
        case_query.set(String::new());
        picker_open.set(false);
    };

    let save_edit = move |_| {
        let user_id = user_sv.get_value();
        let originals = originals.get_value();
        let rows = draft.get_untracked();

        // Diff the draft against the originals: a removed (or fully-unchecked)
        // existing assignment is dropped; a changed set is replaced; a new case
        // with at least one capability is added. Unchanged rows are skipped.
        let mut changes: Vec<(String, Option<Vec<CaseCapability>>)> = Vec::new();
        for row in &rows {
            let caps = row.caps.get_untracked();
            let removed = row.removed.get_untracked();
            match originals.iter().find(|a| a.case_id == row.case_id) {
                Some(a) => {
                    if removed || caps.is_empty() {
                        changes.push((row.case_id.clone(), None));
                    } else if !same_caps(&a.capabilities, &caps) {
                        changes.push((row.case_id.clone(), Some(caps)));
                    }
                }
                None => {
                    if !removed && !caps.is_empty() {
                        changes.push((row.case_id.clone(), Some(caps)));
                    }
                }
            }
        }

        if changes.is_empty() {
            editing.set(false);
            save_error.set(String::new());
            return;
        }
        saving.set(true);
        spawn_local(async move {
            if can_edit_capabilities_directly {
                match crate::server_fns::users::save_case_capabilities(user_id, changes).await {
                    Ok(()) => {
                        save_error.set(String::new());
                        editing.set(false);
                        reload.update(|n| *n += 1);
                    }
                    Err(e) => save_error.set(err_text(e)),
                }
            } else {
                let note = capability_request_note.get_untracked();
                match crate::server_fns::admin_requests::request_user_case_capabilities(
                    user_id, changes, note,
                )
                .await
                {
                    Ok(_) => {
                        save_error.set(String::new());
                        editing.set(false);
                        reload.update(|n| *n += 1);
                    }
                    Err(error) => save_error.set(err_text(error)),
                }
            }
            saving.set(false);
        });
    };

    // The dropdown of search results; clicking one selects it (fills `new_case`).
    let case_result_list = move || {
        if !picker_open.get() {
            return ().into_any();
        }
        let items = case_results.get();
        if items.is_empty() {
            return view! {
                <div class="mt-1 rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-xs text-slate-500">
                    "No matching cases."
                </div>
            }
            .into_any();
        }
        let rows = items
            .into_iter()
            .map(|c| {
                let id = c.id.clone();
                let label = format!("{} ({})", c.name, c.id);
                let select = {
                    let id = id.clone();
                    let name = c.name.clone();
                    move |_| {
                        new_case.set(id.clone());
                        new_case_label.set(name.clone());
                        picker_open.set(false);
                    }
                };
                view! {
                    <button
                        type="button"
                        on:click=select
                        class="block w-full truncate px-3 py-1.5 text-left text-sm text-slate-200 hover:bg-slate-800"
                    >
                        {label}
                    </button>
                }
                .into_any()
            })
            .collect_view();
        view! {
            <div class="mt-1 max-h-48 overflow-y-auto rounded-lg border border-slate-700 bg-slate-950">
                {rows}
            </div>
        }
        .into_any()
    };

    let input_class =
        "rounded-lg border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-100";

    // The whole "Case capabilities" block, rendered reactively so a single `Copy`
    // closure can flip between read-only and edit modes. In view mode it lists
    // each assigned case with its granted capabilities as badges; in edit mode it
    // exposes per-case capability checkboxes, per-row removal, and a case picker
    // to add assignments — all applied at once via `save_edit`.
    let capabilities_section = move || {
        let header_buttons = if editing.get() {
            view! {
                <button
                    on:click=save_edit
                    prop:disabled=move || saving.get()
                    class="rounded-lg bg-primary-500 px-3 py-1.5 text-sm font-semibold text-white hover:bg-primary-600 disabled:opacity-50"
                >
                    {move || if saving.get() {
                        if can_edit_capabilities_directly {
                            "Saving\u{2026}"
                        } else {
                            "Submitting\u{2026}"
                        }
                    } else if can_edit_capabilities_directly {
                        "Save"
                    } else {
                        "Submit requests"
                    }}
                </button>
                <button
                    on:click=cancel_edit
                    class="rounded-lg border border-slate-700 px-3 py-1.5 text-sm font-medium text-slate-300 hover:bg-slate-800"
                >
                    "Cancel"
                </button>
            }
            .into_any()
        } else {
            view! {
                <button
                    on:click=begin_edit
                    class="rounded-lg border border-slate-700 px-3 py-1.5 text-sm font-medium text-slate-200 hover:bg-slate-800"
                >
                    "Edit"
                </button>
            }
            .into_any()
        };

        let body = if editing.get() {
            let add_control = view! {
                <div class="mt-3 flex flex-wrap items-end gap-2">
                    <div class="relative w-full flex-1 sm:min-w-[16rem]">
                        <input
                            class=input_class
                            class:w-full=true
                            placeholder="Search cases by name or id\u{2026}"
                            prop:value=move || {
                                let label = new_case_label.get();
                                if label.is_empty() { case_query.get() } else { label }
                            }
                            on:focus=move |_| {
                                picker_open.set(true);
                                new_case.set(String::new());
                                new_case_label.set(String::new());
                            }
                            on:input=move |ev| {
                                new_case.set(String::new());
                                new_case_label.set(String::new());
                                picker_open.set(true);
                                case_query.set(event_target_value(&ev));
                            }
                        />
                        {case_result_list}
                    </div>
                    <select
                        class=input_class
                        prop:value=move || new_preset.get()
                        on:change=move |ev| new_preset.set(event_target_value(&ev))
                    >
                        {CasePreset::ALL
                            .into_iter()
                            .map(|p| view! { <option value=p.slug()>{p.label()}</option> })
                            .collect_view()}
                    </select>
                    <button
                        on:click=add_to_draft
                        prop:disabled=move || new_case.get().is_empty()
                        class="rounded-lg bg-primary-500 px-3 py-1.5 text-sm font-semibold text-white hover:bg-primary-600 disabled:opacity-50"
                    >
                        "Add"
                    </button>
                </div>
            };
            let request_note = (!can_edit_capabilities_directly).then(|| view! {
                <div class="mt-3">
                    <label
                        class="mb-1 block text-xs font-medium text-slate-400"
                        for=capability_note_id.get_value()
                    >
                        "Request reason (optional)"
                    </label>
                    <textarea
                        id=capability_note_id.get_value()
                        rows="2"
                        maxlength="1000"
                        class="w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500"
                        placeholder="Context for the site admin reviewing this request"
                        prop:value=move || capability_request_note.get()
                        on:input=move |event| capability_request_note.set(event_target_value(&event))
                    ></textarea>
                </div>
            });

            view! {
                <Show
                    when=move || !draft.get().is_empty()
                    fallback=|| view! {
                        <p class="text-sm text-slate-500">"No case assignments."</p>
                    }
                >
                    <div>
                        <For each=move || draft.get() key=|d| d.case_id.clone() let:row>
                            {
                                let row_caps = row.caps;
                                let row_removed = row.removed;
                                let name = row.name.clone();
                                let checkboxes = CaseCapability::ALL
                                    .into_iter()
                                    .map(|cap| {
                                        let unmet = move || {
                                            let held = row_caps.get();
                                            cap.requires().iter().any(|r| !held.contains(r))
                                        };
                                        let title = move || {
                                            let held = row_caps.get();
                                            match cap
                                                .requires()
                                                .iter()
                                                .find(|r| !held.contains(r))
                                            {
                                                Some(missing) => format!(
                                                    "Requires \"{}\" first.",
                                                    missing.label()
                                                ),
                                                None => cap.label().to_string(),
                                            }
                                        };
                                        view! {
                                            <label
                                                class="inline-flex items-center gap-1.5 text-xs"
                                                class=("text-slate-300", move || !unmet())
                                                class=("text-slate-600", unmet)
                                                title=title
                                            >
                                                <input
                                                    type="checkbox"
                                                    class="h-3.5 w-3.5 rounded border-slate-600 bg-slate-950"
                                                    prop:checked=move || row_caps.get().contains(&cap)
                                                    prop:disabled=move || row_removed.get() || unmet()
                                                    on:change=move |ev| {
                                                        let enabled = event_target_checked(&ev);
                                                        row_caps.update(|c| {
                                                            if enabled {
                                                                if !c.contains(&cap) {
                                                                    c.push(cap);
                                                                }
                                                            } else {
                                                                c.retain(|x| *x != cap);
                                                                loop {
                                                                    let held = c.clone();
                                                                    let before = c.len();
                                                                    c.retain(|x| {
                                                                        x.requires()
                                                                            .iter()
                                                                            .all(|r| held.contains(r))
                                                                    });
                                                                    if c.len() == before {
                                                                        break;
                                                                    }
                                                                }
                                                            }
                                                        });
                                                    }
                                                />
                                                {cap.label()}
                                            </label>
                                        }
                                    })
                                    .collect_view();
                                view! {
                                    <div class=move || {
                                        let base = "border-b border-slate-800 py-3";
                                        if row_removed.get() {
                                            format!("{base} opacity-50")
                                        } else {
                                            base.to_string()
                                        }
                                    }>
                                        <div class="flex items-center justify-between gap-2">
                                            <span class="text-sm font-medium text-slate-200">
                                                {name}
                                            </span>
                                            <button
                                                on:click=move |_| row_removed.update(|r| *r = !*r)
                                                class="rounded-lg border border-rose-500/40 px-2 py-1 text-xs font-medium text-rose-300 hover:bg-rose-500/10"
                                            >
                                                {move || if row_removed.get() { "Undo" } else { "Remove" }}
                                            </button>
                                        </div>
                                        <div class="mt-2 flex flex-wrap gap-x-4 gap-y-1.5">
                                            {checkboxes}
                                        </div>
                                    </div>
                                }
                            }
                        </For>
                    </div>
                </Show>
                {add_control}
                {request_note}
            }
            .into_any()
        } else {
            let assigns = originals.get_value();
            if assigns.is_empty() {
                view! { <p class="text-sm text-slate-500">"No case assignments."</p> }.into_any()
            } else {
                assigns
                    .into_iter()
                    .map(|a| {
                        let name = case_name(&a.case_id);
                        let caps = a.capabilities.clone();
                        let badges = if caps.is_empty() {
                            view! {
                                <span class="text-xs text-slate-500">"No capabilities"</span>
                            }
                            .into_any()
                        } else {
                            caps.into_iter()
                                .map(|c| {
                                    view! {
                                        <span class="rounded-full bg-slate-800 px-2 py-0.5 text-xs text-slate-300">
                                            {c.label()}
                                        </span>
                                    }
                                    .into_any()
                                })
                                .collect_view()
                                .into_any()
                        };
                        view! {
                            <div class="border-b border-slate-800 py-3">
                                <span class="text-sm font-medium text-slate-200">{name}</span>
                                <div class="mt-2 flex flex-wrap gap-1.5">{badges}</div>
                            </div>
                        }
                        .into_any()
                    })
                    .collect_view()
                    .into_any()
            }
        };

        view! {
            <div>
                <div class="flex items-center justify-between">
                    <h3 class="text-sm font-semibold text-slate-200">"Case capabilities"</h3>
                    <div class="flex items-center gap-2">{header_buttons}</div>
                </div>
                <Show when=move || !save_error.get().is_empty()>
                    <p class="mt-2 rounded-lg border border-rose-500/40 bg-rose-500/10 px-3 py-2 text-xs text-rose-300">
                        {move || save_error.get()}
                    </p>
                </Show>
                <div class="mt-2">{body}</div>
            </div>
        }
        .into_any()
    };

    // --- change log: its own data source, fetched on demand ---
    // Collapsed by default; opening it mounts the `ChangeLog` component, which
    // fetches independently (with its own loading state), paginates, and filters
    // by a start/end date range. Rendered reactively so it is `Fn`.
    let log_open = RwSignal::new(false);
    let log_user_id = StoredValue::new(user_id.clone());
    let log_section = move || {
        if !log_open.get() {
            return ().into_any();
        }
        view! { <ChangeLog scope=AuditScope::User entity_id=log_user_id.get_value() /> }.into_any()
    };

    view! {
        <div class="rounded-xl border border-slate-800 bg-slate-900 p-5">
            <div class="flex flex-wrap items-start justify-between gap-3">
                <div>
                    <p class="font-medium text-slate-100">{full_name}</p>
                    <p class="text-xs text-slate-500">{email}</p>
                </div>
                <div class="flex flex-wrap items-center justify-end gap-2">
                    <span class=badge(current_role.badge_classes())>{current_role.label()}</span>
                    <Show when=move || editing.get()>
                        {if is_site_admin {
                            view! {
                                <select class=input_class on:change=role_change>
                                    {AccountRole::ALL
                                        .into_iter()
                                        .map(|role| view! {
                                            <option value=role.slug() selected=role == current_role>
                                                {role.label()}
                                            </option>
                                        })
                                        .collect_view()}
                                </select>
                            }
                            .into_any()
                        } else if can_request_changes {
                            view! {
                                <div class="flex max-w-full flex-wrap items-center justify-end gap-2">
                                    <select
                                        class=input_class
                                        prop:value=move || requested_role.get()
                                        on:change=move |event| requested_role.set(event_target_value(&event))
                                    >
                                        {AccountRole::ALL
                                            .into_iter()
                                            .map(|role| view! {
                                                <option value=role.slug()>{role.label()}</option>
                                            })
                                            .collect_view()}
                                    </select>
                                    <input
                                        class="min-w-0 rounded-lg border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-100 placeholder:text-slate-500"
                                        placeholder="Reason (optional)"
                                        maxlength="1000"
                                        prop:value=move || role_request_note.get()
                                        on:input=move |event| role_request_note.set(event_target_value(&event))
                                    />
                                    <button
                                        type="button"
                                        on:click=submit_role_request
                                        prop:disabled=move || {
                                            role_requesting.get()
                                                || requested_role.get() == current_role.slug()
                                        }
                                        class="rounded-lg border border-primary-500/50 px-3 py-1.5 text-sm font-semibold text-primary-300 hover:bg-primary-500/10 disabled:opacity-50"
                                >
                                        {move || if role_requesting.get() {
                                            "Submitting\u{2026}"
                                        } else {
                                            "Request role"
                                        }}
                                    </button>
                                </div>
                            }
                            .into_any()
                        } else {
                            ().into_any()
                        }}
                    </Show>
                </div>
            </div>

            <Show when=move || role_request_feedback.get().is_some()>
                {move || role_request_feedback.get().map(|feedback| match feedback {
                    Ok(message) => view! {
                        <p class="mt-2 text-xs text-emerald-300">{message}</p>
                    }
                    .into_any(),
                    Err(message) => view! {
                        <p class="mt-2 text-xs text-rose-300">{message}</p>
                    }
                    .into_any(),
                })}
            </Show>

            <div class="mt-4">
                {capabilities_section}
            </div>

            {is_site_admin.then(|| view! {
                <div class="mt-4">
                    <div class="flex items-center justify-between">
                        <h3 class="text-sm font-semibold text-slate-200">"Change log"</h3>
                        <button
                            on:click=move |_| log_open.update(|o| *o = !*o)
                            class="rounded-lg border border-slate-700 px-2 py-1 text-xs font-medium text-slate-300 hover:bg-slate-800"
                        >
                            {move || if log_open.get() { "Hide" } else { "Open change log" }}
                        </button>
                    </div>
                    {log_section}
                </div>
            })}
        </div>
    }
    .into_any()
}
