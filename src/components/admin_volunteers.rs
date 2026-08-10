//! Admin dashboard tab: every volunteer account and whether they have completed
//! the volunteer agreement. Each name links to that user's profile.

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::profile_link::ProfileLink;
use crate::server_fns::err_text;
use crate::server_fns::users::{list_volunteers_page, VolunteerListItem};
use crate::server_fns::volunteers::{
    decide_volunteer_application, list_pending_volunteer_applications, Volunteer,
};
use crate::state::AppState;

/// How many volunteers each "Load more" click adds to the visible window.
const PAGE_SIZE: i64 = 20;

#[component]
pub fn VolunteersTab(
    /// Whether this tab is the one on screen; the fetch is deferred until it is.
    #[prop(into)]
    active: Signal<bool>,
    /// Only site admins may decide an application, because approving grants the
    /// Volunteer role. Everyone else sees the queue read-only.
    is_site_admin: bool,
) -> impl IntoView {
    let state = expect_context::<AppState>();
    // Bumped after a decision so both the pending queue and the volunteer list
    // below it refetch.
    let reload = RwSignal::new(0u32);
    // Instant-feedback search text plus its debounced mirror, which drives the
    // fetch so we don't hit the server on every keystroke.
    let query = RwSignal::new(String::new());
    let debounced_query = RwSignal::new(String::new());
    let results = RwSignal::new(Vec::<VolunteerListItem>::new());
    let total = RwSignal::new(0i64);
    let window = RwSignal::new(PAGE_SIZE);
    let loading = RwSignal::new(false);
    let load_error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        if !active.get() {
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
            match list_volunteers_page(0, count, q).await {
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

    let rows = move || {
        if let Some(msg) = load_error.get() {
            return view! {
                <p class="text-sm text-rose-300">"Could not load volunteers: " {msg}</p>
            }
            .into_any();
        }
        let items = results.get();
        if items.is_empty() {
            let text = if loading.get() {
                "Loading\u{2026}"
            } else {
                "No volunteers match your search."
            };
            return view! { <p class="text-sm text-slate-500">{text}</p> }.into_any();
        }
        items
            .into_iter()
            .map(|v| {
                let name = v.full_name();
                let badge_class = format!(
                    "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {}",
                    v.agreement.badge_classes(),
                );
                view! {
                    <div class="flex flex-wrap items-center justify-between gap-3 border-b border-slate-800 px-4 py-3 last:border-b-0">
                        <div class="min-w-0">
                            <ProfileLink user_id=v.id name=name />
                            <p class="truncate text-xs text-slate-500">{v.email}</p>
                        </div>
                        <span class=badge_class>{v.agreement.label()}</span>
                    </div>
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

    // Wait 1s of idle typing before firing the fetch (and resetting the window).
    let mut on_search = debounce(std::time::Duration::from_secs(1), move |val: String| {
        window.set(PAGE_SIZE);
        debounced_query.set(val);
    });

    view! {
        {is_site_admin.then(|| view! {
            <PendingApplications active=active reload=reload />
        })}
        <p class="mb-4 text-sm text-slate-400">
            "Everyone with a volunteer account, and whether they have completed the volunteer agreement. Click a name to open their profile."
        </p>
        <input
            class="mb-4 w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/40"
            placeholder="Search volunteers by name or email"
            prop:value=move || query.get()
            on:input=move |ev| {
                let val = event_target_value(&ev);
                query.set(val.clone());
                on_search(val);
            }
        />
        <div class="rounded-xl border border-slate-800 bg-slate-900">{rows}</div>
        {footer}
    }
}

/// The volunteer applications waiting on a decision, for site admins only.
/// Hidden entirely when the queue is empty, so the tab stays quiet when there is
/// nothing to do.
#[component]
fn PendingApplications(#[prop(into)] active: Signal<bool>, reload: RwSignal<u32>) -> impl IntoView {
    let state = expect_context::<AppState>();
    let items = RwSignal::new(Vec::<Volunteer>::new());
    let load_error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        if !active.get() {
            return;
        }
        reload.track();
        if !state.is_site_admin() {
            return;
        }
        spawn_local(async move {
            match list_pending_volunteer_applications().await {
                Ok(list) => {
                    // Keep the badge honest against what is on screen.
                    state.volunteer_requests_pending.set(list.len() as i64);
                    items.set(list);
                    load_error.set(None);
                }
                Err(e) => load_error.set(Some(err_text(e))),
            }
        });
    });

    move || {
        if let Some(message) = load_error.get() {
            return view! {
                <p class="mb-4 text-sm text-rose-300">
                    "Could not load volunteer applications: " {message}
                </p>
            }
            .into_any();
        }
        let pending = items.get();
        if pending.is_empty() {
            return ().into_any();
        }
        let count = pending.len();
        let cards = pending
            .into_iter()
            .map(|volunteer| {
                view! {
                    <ApplicationCard volunteer=volunteer reload=reload />
                }
                .into_any()
            })
            .collect_view();
        view! {
            <div class="mb-6 rounded-xl border border-slate-800 bg-slate-900 p-5">
                <div class="mb-3 flex items-center gap-2">
                    <h2 class="text-base font-semibold text-slate-100">
                        "Pending volunteer requests"
                    </h2>
                    <span class="inline-flex min-w-5 items-center justify-center rounded-full bg-primary-500 px-1.5 py-0.5 text-[0.65rem] font-semibold leading-none text-white">
                        {count}
                    </span>
                </div>
                <p class="mb-4 text-xs text-slate-500">
                    "People who accepted the volunteer agreement and are waiting to be approved. Approving grants them the Volunteer role; either decision emails them."
                </p>
                <div class="space-y-3">{cards}</div>
            </div>
        }
        .into_any()
    }
}

/// One pending application, with its approve/deny controls.
#[component]
fn ApplicationCard(volunteer: Volunteer, reload: RwSignal<u32>) -> impl IntoView {
    let state = expect_context::<AppState>();
    let user_id = StoredValue::new(volunteer.id.clone());
    let name = volunteer.full_name();
    let email = volunteer.email.clone();
    let agreed_at = volunteer.agreed_at.clone();
    let note = RwSignal::new(String::new());
    let deciding = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    let decide = move |approve: bool| {
        if deciding.get_untracked() {
            return;
        }
        deciding.set(true);
        error.set(None);
        let target = user_id.get_value();
        let decision_note = note.get_untracked();
        spawn_local(async move {
            match decide_volunteer_application(target, approve, decision_note).await {
                Ok(()) => {
                    reload.update(|value| *value += 1);
                    // The decided row leaves the queue, so keep the badge in step.
                    state.refresh_badges();
                }
                Err(e) => error.set(Some(err_text(e))),
            }
            deciding.set(false);
        });
    };

    view! {
        <div class="rounded-lg border border-slate-800 bg-slate-950 p-4">
            <div class="flex flex-wrap items-start justify-between gap-3">
                <div class="min-w-0">
                    <ProfileLink user_id=volunteer.id name=name />
                    <p class="truncate text-xs text-slate-500">{email}</p>
                    <p class="mt-1 text-xs text-slate-500">"Agreement accepted " {agreed_at}</p>
                </div>
                <span class="inline-flex items-center rounded-full bg-amber-500/15 px-2 py-0.5 text-xs font-medium text-amber-300 ring-1 ring-amber-500/30">
                    "Pending review"
                </span>
            </div>

            <Show when=move || error.get().is_some()>
                <p class="mt-3 rounded-lg border border-rose-500/40 bg-rose-500/10 px-3 py-2 text-xs text-rose-300">
                    {move || error.get().unwrap_or_default()}
                </p>
            </Show>

            <div class="mt-3 flex flex-wrap items-center gap-2">
                    <input
                        class="min-w-0 flex-1 rounded-lg border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-100 placeholder:text-slate-500"
                        placeholder="Decision note (optional, included in the email)"
                        maxlength="1000"
                        prop:value=move || note.get()
                        on:input=move |event| note.set(event_target_value(&event))
                    />
                    <button
                        on:click=move |_| decide(true)
                        prop:disabled=move || deciding.get()
                        class="rounded-lg bg-primary-500 px-3 py-1.5 text-sm font-semibold text-white hover:bg-primary-600 disabled:opacity-50"
                    >
                        "Approve"
                    </button>
                    <button
                        on:click=move |_| decide(false)
                        prop:disabled=move || deciding.get()
                        class="rounded-lg border border-rose-500/40 px-3 py-1.5 text-sm font-medium text-rose-300 hover:bg-rose-500/10 disabled:opacity-50"
                    >
                    "Deny"
                </button>
            </div>
        </div>
    }
}
