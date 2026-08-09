//! Admin dashboard tab: every volunteer account and whether they have completed
//! the volunteer agreement. Each name links to that user's profile.

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::profile_link::ProfileLink;
use crate::server_fns::err_text;
use crate::server_fns::users::{list_volunteers_page, VolunteerListItem};
use crate::state::AppState;

/// How many volunteers each "Load more" click adds to the visible window.
const PAGE_SIZE: i64 = 20;

#[component]
pub fn VolunteersTab(
    /// Whether this tab is the one on screen; the fetch is deferred until it is.
    #[prop(into)]
    active: Signal<bool>,
) -> impl IntoView {
    let state = expect_context::<AppState>();
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
