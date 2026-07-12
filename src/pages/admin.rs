use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::guard::require_admin;
use crate::components::layout::Layout;
use crate::server_fns::audit::ChangeLogEntry;
use crate::server_fns::cases::CaseSummary;
use crate::server_fns::err_text;
use crate::server_fns::capabilities::{CaseCapability, CasePreset};
use crate::server_fns::users::AccountRole;
use crate::server_fns::users::User;
use crate::state::{today, AppState};

/// Cap on how many change-log rows are rendered at once (guards against huge
/// result sets).
const MAX_LOG_ROWS: usize = 100;
/// Widest date range the change-log filter will accept, in days.
const MAX_RANGE_DAYS: i64 = 366;
/// How many users the admin list loads per "page" (each "Load more" click grows
/// the visible window by this much).
const PAGE_SIZE: i64 = 10;

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

/// Days since 1970-01-01 for a civil date (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Inverse of [`days_from_civil`].
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Parse the `YYYY-MM-DD` prefix of a timestamp into a day ordinal.
fn date_ordinal(s: &str) -> Option<i64> {
    let d = s.get(..10).unwrap_or(s);
    let mut it = d.split('-');
    let y = it.next()?.parse().ok()?;
    let m = it.next()?.parse().ok()?;
    let d = it.next()?.parse().ok()?;
    Some(days_from_civil(y, m, d))
}

/// Shift a `YYYY-MM-DD` date by a number of days.
fn shift_days(date: &str, delta: i64) -> String {
    match date_ordinal(date) {
        Some(o) => {
            let (y, m, d) = civil_from_days(o + delta);
            format!("{y:04}-{m:02}-{d:02}")
        }
        None => date.to_string(),
    }
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

    // (Re)load the window whenever the debounced query, window size, or reload
    // tick changes — but only once a session is confirmed (server functions run
    // in the browser after hydration). We always fetch `[0, window)` so both
    // search changes and post-mutation refreshes are handled by one code path.
    Effect::new(move |_| {
        let count = window.get();
        let q = debounced_query.get();
        reload.track();
        if !state.is_authenticated() {
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

    require_admin(state, move || {
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
                .map(|u| view! { <UserCard user=u reload=reload /> }.into_any())
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
            <p class="mb-6 text-sm text-slate-400">
                "Manage every user's global role and per-case capabilities."
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
        </Layout>
    }
    .into_any()
    })
}

/// A management card for a single user. `reload` is bumped after any mutation so
/// the parent list refetches the current window and the card re-renders with
/// fresh data (assignments, role, audit log).
#[component]
fn UserCard(user: User, reload: RwSignal<u32>) -> impl IntoView {
    let user_id = user.id.clone();

    // --- global role ---
    let role_change = {
        let user_id = user_id.clone();
        move |ev| {
            if let Some(r) = AccountRole::from_slug(&event_target_value(&ev)) {
                let user_id = user_id.clone();
                spawn_local(async move {
                    if crate::server_fns::users::set_user_role(user_id, r)
                        .await
                        .is_ok()
                    {
                        reload.update(|n| *n += 1);
                    }
                });
            }
        }
    };

    // --- add assignment (case typeahead + preset) ---
    // `new_case` holds the *selected* case id; `new_case_label` its display name.
    // The picker searches all cases server-side (admin-only), so it scales to
    // tens of thousands of cases without ever loading them into the browser.
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

    let current_role = user.role;
    let full_name = user.full_name();
    let email = user.email.clone();
    let audit_data: Vec<ChangeLogEntry> = Vec::new();

    let originals = StoredValue::new(user.assigned_cases.clone());
    let editing = RwSignal::new(false);
    let draft: RwSignal<Vec<DraftAssignment>> = RwSignal::new(Vec::new());
    let save_error = RwSignal::new(String::new());
    let saving = RwSignal::new(false);

    let begin_edit = move |_| {
        let rows = originals
            .get_value()
            .into_iter()
            .map(|a| make_draft(a.case_id.clone(), case_name(&a.case_id), a.capabilities))
            .collect::<Vec<_>>();
        draft.set(rows);
        save_error.set(String::new());
        picker_open.set(false);
        new_case.set(String::new());
        new_case_label.set(String::new());
        case_query.set(String::new());
        editing.set(true);
    };

    let cancel_edit = move |_| {
        editing.set(false);
        save_error.set(String::new());
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
            match crate::server_fns::users::save_case_capabilities(user_id, changes).await {
                Ok(()) => {
                    save_error.set(String::new());
                    editing.set(false);
                    reload.update(|n| *n += 1);
                }
                Err(e) => save_error.set(err_text(e)),
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
                    {move || if saving.get() { "Saving\u{2026}" } else { "Save" }}
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
                    <div class="relative min-w-[16rem] flex-1">
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
                                        view! {
                                            <label class="inline-flex items-center gap-1.5 text-xs text-slate-300">
                                                <input
                                                    type="checkbox"
                                                    class="h-3.5 w-3.5 rounded border-slate-600 bg-slate-950"
                                                    prop:checked=move || row_caps.get().contains(&cap)
                                                    prop:disabled=move || row_removed.get()
                                                    on:change=move |ev| {
                                                        let enabled = event_target_checked(&ev);
                                                        row_caps.update(|c| {
                                                            if enabled {
                                                                if !c.contains(&cap) {
                                                                    c.push(cap);
                                                                }
                                                            } else {
                                                                c.retain(|x| *x != cap);
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

    // --- change log: collapsed by default, filtered to a date range ---
    let log_open = RwSignal::new(false);
    let default_to = today();
    let default_from = shift_days(&default_to, -14);
    let from = RwSignal::new(default_from);
    let to = RwSignal::new(default_to);

    // The whole expandable region: date pickers + validated, capped results.
    // Collapsed by default; rendered reactively so it is `Fn` (works with the
    // reactive `{...}` slot instead of a `<Show>` that would move it out).
    let log_section = {
        move || {
            if !log_open.get() {
                return ().into_any();
            }
            let date_input =
                "mt-1 block rounded-lg border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-100";
            let controls = view! {
                <div class="flex flex-wrap items-end gap-3">
                    <label class="text-xs text-slate-400">
                        "From"
                        <input
                            type="date"
                            class=date_input
                            prop:value=move || from.get()
                            on:change=move |ev| from.set(event_target_value(&ev))
                        />
                    </label>
                    <label class="text-xs text-slate-400">
                        "To"
                        <input
                            type="date"
                            class=date_input
                            prop:value=move || to.get()
                            on:change=move |ev| to.set(event_target_value(&ev))
                        />
                    </label>
                    <button
                        on:click=move |_| {
                            let t = today();
                            from.set(shift_days(&t, -14));
                            to.set(t);
                        }
                        class="rounded-lg border border-slate-700 px-2 py-1.5 text-xs font-medium text-slate-300 hover:bg-slate-800"
                    >
                        "Last 2 weeks"
                    </button>
                </div>
            };

            let body = {
                let (fo, to_o) = match (date_ordinal(&from.get()), date_ordinal(&to.get())) {
                    (Some(a), Some(b)) => (a, b),
                    _ => {
                        return view! {
                            <div class="mt-3 space-y-3">
                                {controls}
                                <p class="text-xs text-rose-300">
                                    "Enter a valid From and To date."
                                </p>
                            </div>
                        }
                        .into_any();
                    }
                };
                if fo > to_o {
                    return view! {
                        <div class="mt-3 space-y-3">
                            {controls}
                            <p class="text-xs text-rose-300">
                                "From date must be on or before the To date."
                            </p>
                        </div>
                    }
                    .into_any();
                }
                if to_o - fo > MAX_RANGE_DAYS {
                    return view! {
                        <div class="mt-3 space-y-3">
                            {controls}
                            <p class="text-xs text-rose-300">
                                "Date range is too large — choose at most " {MAX_RANGE_DAYS}
                                " days."
                            </p>
                        </div>
                    }
                    .into_any();
                }
                let matched: Vec<ChangeLogEntry> = audit_data
                    .clone()
                    .into_iter()
                    .filter(|e| {
                        date_ordinal(&e.at)
                            .map(|o| o >= fo && o <= to_o)
                            .unwrap_or(false)
                    })
                    .collect();
                let total = matched.len();
                if total == 0 {
                    return view! {
                        <p class="text-xs text-slate-500">"No changes in this date range."</p>
                    }
                    .into_any();
                }
                let capped = total > MAX_LOG_ROWS;
                let list = matched
                    .into_iter()
                    .take(MAX_LOG_ROWS)
                    .map(|e| {
                        view! {
                            <div class="text-xs text-slate-400">
                                <span class="text-slate-300">{e.actor}</span> " changed "
                                <span class="text-slate-300">{e.field}</span> " from \""
                                {e.old_value} "\" to \"" {e.new_value} "\" · " {e.at}
                            </div>
                        }
                        .into_any()
                    })
                    .collect_view();
                let note = if capped {
                    view! {
                        <p class="text-xs text-amber-300">
                            "Showing the first " {MAX_LOG_ROWS} " of " {total}
                            " changes — narrow the range to see the rest."
                        </p>
                    }
                    .into_any()
                } else {
                    view! {
                        <p class="text-xs text-slate-500">{total} " change(s) in range"</p>
                    }
                    .into_any()
                };
                view! {
                    <div class="space-y-1">{list}</div>
                    {note}
                }
                .into_any()
            };

            view! {
                <div class="mt-3 space-y-3">
                    {controls}
                    {body}
                </div>
            }
            .into_any()
        }
    };

    view! {
        <div class="rounded-xl border border-slate-800 bg-slate-900 p-5">
            <div class="flex flex-wrap items-center justify-between gap-3">
                <div>
                    <p class="font-medium text-slate-100">{full_name}</p>
                    <p class="text-xs text-slate-500">{email}</p>
                </div>
                <div class="flex items-center gap-2">
                    <span class=badge(current_role.badge_classes())>{current_role.label()}</span>
                    <select
                        class=input_class
                        on:change=role_change
                    >
                        {AccountRole::ALL
                            .into_iter()
                            .map(|r| {
                                view! {
                                    <option value=r.slug() selected=r == current_role>
                                        {r.label()}
                                    </option>
                                }
                            })
                            .collect_view()}
                    </select>
                </div>
            </div>

            <div class="mt-4">
                {capabilities_section}
            </div>

            <div class="mt-4">
                <div class="flex items-center justify-between">
                    <h3 class="text-sm font-semibold text-slate-200">"Change log"</h3>
                    <button
                        on:click=move |_| log_open.update(|o| *o = !*o)
                        class="rounded-lg border border-slate-700 px-2 py-1 text-xs font-medium text-slate-300 hover:bg-slate-800"
                    >
                        {move || if log_open.get() { "Hide" } else { "Show" }}
                    </button>
                </div>
                {log_section}
            </div>
        </div>
    }
    .into_any()
}
