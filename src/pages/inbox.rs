use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::guard::require_login;
use crate::components::layout::Layout;
use crate::state::AppState;
use crate::types::CaseCapability;

/// Case Chat: one chat thread per case. Anyone assigned to a case (with
/// permission) can read it; posting requires the `SendMessages` capability.
#[component]
pub fn InboxPage() -> impl IntoView {
    let state = expect_context::<AppState>();

    let selected = RwSignal::new(None::<String>);
    let search = RwSignal::new(String::new());
    // Lazy-load: only render this many rows, growing on demand.
    const PAGE: usize = 15;
    let visible_count = RwSignal::new(PAGE);

    let input_class = "w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/40";

    require_login(state, move || {
        // Auto-select the first accessible case so the chat is populated on load.
        if selected.get_untracked().is_none() {
            if let Some(first) = state.visible_cases().first() {
                selected.set(Some(first.id.clone()));
            }
        }

        // Display title for a case: the case name with its owner appended, so the
        // search can match on either.
        let title_for =
            move |c: &crate::types::Case| format!("{} · {}", c.name, state.user_name(&c.owner_id));

        // Cases matching the current search, in display order.
        let filtered_cases = move || {
            let q = search.get().trim().to_lowercase();
            state
                .visible_cases()
                .into_iter()
                .filter(|c| q.is_empty() || title_for(c).to_lowercase().contains(&q))
                .collect::<Vec<_>>()
        };

        let case_list = move || {
            let all = filtered_cases();
            let total = all.len();
            if total == 0 {
                let msg = if search.get().trim().is_empty() {
                    "You have no case chats yet."
                } else {
                    "No cases match your search."
                };
                return view! { <p class="text-sm text-slate-400">{msg}</p> }.into_any();
            }
            let shown = visible_count.get().min(total);
            let rows = all
                .into_iter()
                .take(shown)
                .map(|c| {
                    let case_id = c.id.clone();
                    let is_selected = {
                        let case_id = case_id.clone();
                        move || selected.get().as_deref() == Some(case_id.as_str())
                    };
                    let count = c.message_count;
                    let name = title_for(&c);
                    let select = {
                        let case_id = case_id.clone();
                        move |_| selected.set(Some(case_id.clone()))
                    };
                    view! {
                        <button
                            on:click=select
                            class=move || {
                                let base = "w-full rounded-xl border p-3 text-left transition-colors";
                                if is_selected() {
                                    format!("{base} border-primary-500/50 bg-slate-800")
                                } else {
                                    format!("{base} border-slate-800 bg-slate-900 hover:bg-slate-800")
                                }
                            }
                        >
                            <div class="flex items-center justify-between gap-2">
                                <span class="text-sm font-medium text-slate-200">{name}</span>
                                <span class="text-xs text-slate-500">{count} " msgs"</span>
                            </div>
                        </button>
                    }
                    .into_any()
                })
                .collect_view();

            let load_more = if shown < total {
                view! {
                    <button
                        on:click=move |_| visible_count.update(|n| *n += PAGE)
                        class="w-full rounded-lg border border-slate-700 px-3 py-2 text-xs font-medium text-slate-300 hover:bg-slate-800"
                    >
                        "Load more (" {shown} " of " {total} ")"
                    </button>
                }
                .into_any()
            } else if total > PAGE {
                view! {
                    <p class="text-center text-xs text-slate-500">
                        "Showing all " {total} " cases"
                    </p>
                }
                .into_any()
            } else {
                ().into_any()
            };

            view! {
                <div class="space-y-2">{rows}</div>
                <div class="pt-1">{load_more}</div>
            }
            .into_any()
        };

        let thread = move || {
            match selected.get() {
                None => view! {
                    <div class="rounded-xl border border-dashed border-slate-700 p-8 text-center text-sm text-slate-500">
                        "Select a case to open its chat."
                    </div>
                }
                .into_any(),
                Some(id) => match state.cases.get().into_iter().find(|c| c.id == id) {
                    Some(c) => {
                        let title = title_for(&c);
                        view! { <CaseChat case_id=c.id case_name=title /> }.into_any()
                    }
                    None => {
                        view! { <p class="text-sm text-slate-400">"Case not found."</p> }.into_any()
                    }
                },
            }
        };

        view! {
            <Layout title="Case Chat".to_string()>
                <div class="grid gap-6 lg:grid-cols-[22rem_1fr]">
                    <div class="space-y-2">
                        <input
                            class=input_class
                            placeholder="Search cases…"
                            prop:value=move || search.get()
                            on:input=move |ev| {
                                search.set(event_target_value(&ev));
                                visible_count.set(PAGE);
                            }
                        />
                        {case_list}
                    </div>
                    <div>{thread}</div>
                </div>
            </Layout>
        }
        .into_any()
    })
}

/// The chat thread for a single case.
///
/// Messages are paginated newest-first: the thread opens scrolled to the latest
/// message and a "Load earlier messages" button at the top pages older messages
/// in on demand, so a long conversation never loads all at once.
#[component]
fn CaseChat(case_id: String, case_name: String) -> impl IntoView {
    let state = expect_context::<AppState>();
    let me = state
        .current_user
        .get_untracked()
        .map(|u| u.id)
        .unwrap_or_default();

    let can_send = state
        .cases
        .get_untracked()
        .into_iter()
        .find(|c| c.id == case_id)
        .map(|c| state.case_can(&c, CaseCapability::SendMessages))
        .unwrap_or(false);

    // How many of the most recent messages to request; grows on "Load more".
    const MSG_PAGE: i64 = 20;
    let limit = RwSignal::new(MSG_PAGE);
    let total = RwSignal::new(0i64);

    // Scroll container; used to jump to the latest message on load and after
    // sending. Growing `limit` (loading earlier messages) deliberately does not
    // scroll, so the user stays where they were reading.
    let scroll_ref = NodeRef::<leptos::html::Div>::new();
    let scroll_to_bottom = move || {
        if let Some(el) = scroll_ref.get() {
            request_animation_frame(move || el.set_scroll_top(el.scroll_height()));
        }
    };
    let did_initial_scroll = RwSignal::new(false);

    // (Re)load the newest `limit` messages whenever the window grows. Browser
    // only; the SSR branch errors and is ignored.
    {
        let case_id = case_id.clone();
        Effect::new(move |_| {
            let lim = limit.get();
            let case_id = case_id.clone();
            spawn_local(async move {
                if let Ok(t) = state.load_messages(&case_id, lim).await {
                    total.set(t);
                }
            });
        });
    }

    // Once messages first arrive, jump to the latest one.
    {
        let case_id = case_id.clone();
        Effect::new(move |_| {
            let has = !state.messages_for_case(&case_id).is_empty();
            if has && !did_initial_scroll.get_untracked() {
                did_initial_scroll.set(true);
                scroll_to_bottom();
            }
        });
    }

    let body = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());

    let send = {
        let case_id = case_id.clone();
        move |_| {
            let case_id = case_id.clone();
            let body_val = body.get_untracked();
            spawn_local(async move {
                match state.send_case_message(&case_id, &body_val).await {
                    Ok(()) => {
                        body.set(String::new());
                        error.set(String::new());
                        total.update(|t| *t += 1);
                        scroll_to_bottom();
                    }
                    Err(e) => error.set(e),
                }
            });
        }
    };

    let messages_view = {
        let case_id = case_id.clone();
        let me = me.clone();
        move || {
            let msgs = state.messages_for_case(&case_id);
            let shown = msgs.len() as i64;

            // "Load more" pages in earlier messages at the top of the thread.
            let load_more = if shown < total.get() {
                view! {
                    <button
                        on:click=move |_| limit.update(|l| *l += MSG_PAGE)
                        class="mx-auto block rounded-lg border border-slate-700 px-3 py-1.5 text-xs font-medium text-slate-300 hover:bg-slate-800"
                    >
                        "Load earlier messages (" {shown} " of " {move || total.get()} ")"
                    </button>
                }
                .into_any()
            } else {
                ().into_any()
            };

            if msgs.is_empty() {
                return view! {
                    <p class="text-sm text-slate-500">"No messages yet. Start the conversation."</p>
                }
                .into_any();
            }

            let rows = msgs
                .into_iter()
                .map(|m| {
                    let mine = m.author_id == me;
                    let row = if mine {
                        "flex justify-end"
                    } else {
                        "flex justify-start"
                    };
                    let bubble = if mine {
                        "max-w-[80%] rounded-2xl bg-primary-500/20 px-4 py-2 text-sm text-slate-100"
                    } else {
                        "max-w-[80%] rounded-2xl bg-slate-800 px-4 py-2 text-sm text-slate-100"
                    };
                    view! {
                        <div class=row>
                            <div class=bubble>
                                <p class="text-xs font-medium text-slate-400">
                                    {m.author} " · " {m.sent_at}
                                </p>
                                <p class="mt-0.5 whitespace-pre-wrap">{m.body}</p>
                            </div>
                        </div>
                    }
                    .into_any()
                })
                .collect_view();

            view! {
                {load_more}
                {rows}
            }
            .into_any()
        }
    };

    let input_class = "w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/40";

    view! {
        <div class="flex h-[calc(100vh-12rem)] flex-col rounded-xl border border-slate-800 bg-slate-900">
            <div class="border-b border-slate-800 p-4">
                <h2 class="text-lg font-semibold">{case_name}</h2>
                <p class="text-xs text-slate-500">"Case chat"</p>
            </div>
            <div node_ref=scroll_ref class="flex-1 space-y-3 overflow-y-auto p-4">
                {messages_view}
            </div>
            <div class="border-t border-slate-800 p-4">
                {if can_send {
                    view! {
                        <div class="space-y-2">
                            <Show when=move || !error.get().is_empty()>
                                <p class="text-xs text-rose-300">{move || error.get()}</p>
                            </Show>
                            <div class="flex gap-2">
                                <input
                                    class=input_class
                                    placeholder="Type a message"
                                    prop:value=move || body.get()
                                    on:input=move |ev| body.set(event_target_value(&ev))
                                />
                                <button
                                    on:click=send
                                    class="shrink-0 rounded-lg bg-primary-500 px-4 py-2 text-sm font-semibold text-white hover:bg-primary-600"
                                >
                                    "Send"
                                </button>
                            </div>
                        </div>
                    }
                        .into_any()
                } else {
                    view! {
                        <div class="space-y-2">
                            <div class="flex gap-2">
                                <input
                                    class=format!("{input_class} cursor-not-allowed opacity-50")
                                    placeholder="You do not have permission to send messages"
                                    disabled=true
                                />
                                <button
                                    disabled=true
                                    class="shrink-0 cursor-not-allowed rounded-lg bg-primary-500 px-4 py-2 text-sm font-semibold text-white opacity-50"
                                >
                                    "Send"
                                </button>
                            </div>
                            <p class="text-xs text-slate-500">
                                "You have read-only access to this chat."
                            </p>
                        </div>
                    }
                        .into_any()
                }}
            </div>
        </div>
    }
    .into_any()
}
