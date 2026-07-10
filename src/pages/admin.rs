use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::guard::require_admin;
use crate::components::layout::Layout;
use crate::state::{today, AppState};
use crate::types::{AccountRole, CaseCapability, CasePreset, ChangeLogEntry, User};

/// Cap on how many change-log rows are rendered at once (guards against huge
/// result sets).
const MAX_LOG_ROWS: usize = 100;
/// Widest date range the change-log filter will accept, in days.
const MAX_RANGE_DAYS: i64 = 366;

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

/// Admin dashboard: view all users and manage their permissions.
#[component]
pub fn AdminDashboardPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    let query = RwSignal::new(String::new());

    require_admin(state, move || {
    let users = move || {
        let q = query.get().trim().to_lowercase();
        let matches: Vec<_> = state
            .users
            .get()
            .into_iter()
            .filter(|u| {
                if q.is_empty() {
                    return true;
                }
                let haystack = format!(
                    "{} {} {}",
                    u.full_name().to_lowercase(),
                    u.email.to_lowercase(),
                    u.phone.to_lowercase()
                );
                haystack.contains(&q)
            })
            .collect();
        if matches.is_empty() {
            return view! {
                <p class="text-sm text-slate-500">"No users match your search."</p>
            }
            .into_any();
        }
        matches
            .into_iter()
            .map(|u| view! { <UserCard user=u /> }.into_any())
            .collect_view()
            .into_any()
    };

    view! {
        <Layout title="Admin".to_string()>
            <p class="mb-6 text-sm text-slate-400">
                "Manage every user's global role and per-case permissions."
            </p>
            <input
                class="mb-4 w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/40"
                placeholder="Search users by name, email, or phone"
                prop:value=move || query.get()
                on:input=move |ev| query.set(event_target_value(&ev))
            />
            <div class="space-y-4">{users}</div>
        </Layout>
    }
    .into_any()
    })
}

/// A management card for a single user.
#[component]
fn UserCard(user: User) -> impl IntoView {
    let state = expect_context::<AppState>();
    let user_id = user.id.clone();

    let live_user = {
        let user_id = user_id.clone();
        move || state.users.get().into_iter().find(|u| u.id == user_id)
    };

    // --- global role ---
    let role_change = {
        let user_id = user_id.clone();
        move |ev| {
            if let Some(r) = AccountRole::from_slug(&event_target_value(&ev)) {
                let user_id = user_id.clone();
                spawn_local(async move {
                    let _ = state.set_user_role(&user_id, r).await;
                });
            }
        }
    };

    // --- add assignment (seeded from a preset) ---
    let new_case = RwSignal::new(String::new());
    let new_preset = RwSignal::new(CasePreset::Viewer.slug().to_string());
    let assign = {
        let user_id = user_id.clone();
        move |_| {
            let case_id = new_case.get_untracked();
            if case_id.is_empty() {
                return;
            }
            let preset =
                CasePreset::from_slug(&new_preset.get_untracked()).unwrap_or(CasePreset::Viewer);
            let user_id = user_id.clone();
            spawn_local(async move {
                if state
                    .assign_user_to_case(&user_id, &case_id, preset.capabilities())
                    .await
                    .is_ok()
                {
                    new_case.set(String::new());
                }
            });
        }
    };

    let case_name = move |case_id: &str| -> String {
        state
            .cases
            .get()
            .into_iter()
            .find(|c| c.id == case_id)
            .map(|c| c.name)
            .unwrap_or_else(|| case_id.to_string())
    };

    let current_role = live_user().map(|u| u.role).unwrap_or(AccountRole::Client);
    let full_name = user.full_name();
    let email = user.email.clone();

    let assignments_view = {
        let user_id = user_id.clone();
        let live_user = live_user.clone();
        move || {
            let assigns = live_user().map(|u| u.assigned_cases).unwrap_or_default();
            if assigns.is_empty() {
                return view! { <p class="text-sm text-slate-500">"No case assignments."</p> }
                    .into_any();
            }
            assigns
                .into_iter()
                .map(|a| {
                    let case_id = a.case_id.clone();
                    let name = case_name(&case_id);
                    let remove = {
                        let user_id = user_id.clone();
                        let case_id = case_id.clone();
                        move |_| {
                            let user_id = user_id.clone();
                            let case_id = case_id.clone();
                            spawn_local(async move {
                                let _ =
                                    state.unassign_user_from_case(&user_id, &case_id).await;
                            });
                        }
                    };
                    let caps = a.capabilities.clone();
                    let checkboxes = CaseCapability::ALL
                        .into_iter()
                        .map(|cap| {
                            let checked = caps.contains(&cap);
                            let toggle = {
                                let user_id = user_id.clone();
                                let case_id = case_id.clone();
                                move |ev| {
                                    let user_id = user_id.clone();
                                    let case_id = case_id.clone();
                                    let enabled = event_target_checked(&ev);
                                    spawn_local(async move {
                                        let _ = state
                                            .toggle_case_capability(
                                                &user_id, &case_id, cap, enabled,
                                            )
                                            .await;
                                    });
                                }
                            };
                            view! {
                                <label class="inline-flex items-center gap-1.5 text-xs text-slate-300">
                                    <input
                                        type="checkbox"
                                        class="h-3.5 w-3.5 rounded border-slate-600 bg-slate-950"
                                        prop:checked=checked
                                        on:change=toggle
                                    />
                                    {cap.label()}
                                </label>
                            }
                            .into_any()
                        })
                        .collect_view();
                    view! {
                        <div class="border-b border-slate-800 py-3">
                            <div class="flex items-center justify-between gap-2">
                                <span class="text-sm font-medium text-slate-200">{name}</span>
                                <button
                                    on:click=remove
                                    class="rounded-lg border border-rose-500/40 px-2 py-1 text-xs font-medium text-rose-300 hover:bg-rose-500/10"
                                >
                                    "Remove"
                                </button>
                            </div>
                            <div class="mt-2 flex flex-wrap gap-x-4 gap-y-1.5">{checkboxes}</div>
                        </div>
                    }
                    .into_any()
                })
                .collect_view()
                .into_any()
        }
    };

    let case_options = move || {
        state
            .cases
            .get()
            .into_iter()
            .map(|c| view! { <option value=c.id.clone()>{c.name}</option> })
            .collect_view()
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
        let live_user = live_user.clone();
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
                let matched: Vec<ChangeLogEntry> = live_user()
                    .map(|u| u.audit_log)
                    .unwrap_or_default()
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

    let input_class =
        "rounded-lg border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-100";

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
                <h3 class="text-sm font-semibold text-slate-200">"Case permissions"</h3>
                <div class="mt-2">{assignments_view}</div>
                <div class="mt-3 flex flex-wrap items-center gap-2">
                    <select
                        class=input_class
                        prop:value=move || new_case.get()
                        on:change=move |ev| new_case.set(event_target_value(&ev))
                    >
                        <option value="">"Select case…"</option>
                        {case_options}
                    </select>
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
                        on:click=assign
                        class="rounded-lg bg-primary-500 px-3 py-1.5 text-sm font-semibold text-white hover:bg-primary-600"
                    >
                        "Assign"
                    </button>
                </div>
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
