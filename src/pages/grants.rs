use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::guard::require_admin;
use crate::components::layout::Layout;
use crate::server_fns::grants::{load_grants, GrantSummary};
use crate::server_fns::{err_text, grants};
use crate::state::AppState;

/// Grant Home: view and manage grant data (admin only).
#[component]
pub fn GrantHomePage() -> impl IntoView {
    let state = expect_context::<AppState>();

    let all_grants = RwSignal::new(Vec::<GrantSummary>::new());
    let load_error = RwSignal::new(None::<String>);
    // Bumped after add/rename/delete to force the list to reload.
    let reload = RwSignal::new(0u32);

    let new_name = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());

    let input_class = "w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/40";

    Effect::new(move |_| {
        reload.track();
        if !state.is_admin() {
            return;
        }
        spawn_local(async move {
            match load_grants().await {
                Ok(items) => {
                    all_grants.set(items);
                    load_error.set(None);
                }
                Err(e) => load_error.set(Some(err_text(e))),
            }
        });
    });

    require_admin(state, move || {
        let add_grant = move |_| {
            let name = new_name.get_untracked();
            spawn_local(async move {
                match grants::add_grant(name).await {
                    Ok(_) => {
                        new_name.set(String::new());
                        error.set(String::new());
                        reload.update(|n| *n += 1);
                    }
                    Err(e) => error.set(err_text(e)),
                }
            });
        };

        let list = move || {
            if let Some(msg) = load_error.get() {
                return view! {
                    <p class="text-sm text-rose-300">"Could not load grants: " {msg}</p>
                }
                .into_any();
            }
            let items = all_grants.get();
            if items.is_empty() {
                return view! { <p class="text-sm text-slate-400">"No grants yet."</p> }.into_any();
            }
            items
                .into_iter()
                .map(|g| {
                    let grant_id = g.id.clone();
                    let name = RwSignal::new(g.name.clone());
                    let save = {
                        let grant_id = grant_id.clone();
                        move |_| {
                            let grant_id = grant_id.clone();
                            let value = name.get_untracked();
                            spawn_local(async move {
                                match grants::rename_grant(grant_id, value).await {
                                    Ok(()) => reload.update(|n| *n += 1),
                                    Err(e) => error.set(err_text(e)),
                                }
                            });
                        }
                    };
                    let delete = {
                        let grant_id = grant_id.clone();
                        move |_| {
                            let grant_id = grant_id.clone();
                            spawn_local(async move {
                                match grants::delete_grant(grant_id).await {
                                    Ok(()) => reload.update(|n| *n += 1),
                                    Err(e) => error.set(err_text(e)),
                                }
                            });
                        }
                    };
                    view! {
                        <div class="flex items-center gap-2 rounded-xl border border-slate-800 bg-slate-900 p-3">
                            <input
                                class=input_class
                                prop:value=move || name.get()
                                on:input=move |ev| name.set(event_target_value(&ev))
                            />
                            <button
                                on:click=save
                                class="shrink-0 rounded-lg border border-slate-700 px-3 py-2 text-sm font-medium text-slate-200 hover:bg-slate-800"
                            >
                                "Save"
                            </button>
                            <button
                                on:click=delete
                                class="shrink-0 rounded-lg border border-rose-500/40 px-3 py-2 text-sm font-medium text-rose-300 hover:bg-rose-500/10"
                            >
                                "Delete"
                            </button>
                        </div>
                    }
                    .into_any()
                })
                .collect_view()
                .into_any()
        };

        view! {
            <Layout title="Grants".to_string()>
                <div class="max-w-2xl space-y-6">
                    <div class="rounded-xl border border-slate-800 bg-slate-900 p-4">
                        <h2 class="text-sm font-semibold text-slate-200">"New grant"</h2>
                        <div class="mt-3 flex gap-2">
                            <input
                                class=input_class
                                placeholder="Grant name"
                                prop:value=move || new_name.get()
                                on:input=move |ev| new_name.set(event_target_value(&ev))
                            />
                            <button
                                on:click=add_grant
                                class="shrink-0 rounded-lg bg-primary-500 px-3 py-2 text-sm font-semibold text-white hover:bg-primary-600"
                            >
                                "Add"
                            </button>
                        </div>
                        <Show when=move || !error.get().is_empty()>
                            <p class="mt-2 text-xs text-rose-300">{move || error.get()}</p>
                        </Show>
                    </div>

                    <div class="space-y-3">{list}</div>
                </div>
            </Layout>
        }
        .into_any()
    })
}
