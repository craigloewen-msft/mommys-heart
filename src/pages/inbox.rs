use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::guard::require_login;
use crate::components::layout::Layout;
use crate::components::loading::Loading;
use crate::server_fns::cases::{load_case_summaries_for_user, CaseSummary};
use crate::server_fns::err_text;
use crate::server_fns::message::Message;
use crate::server_fns::capabilities::CaseCapability;
use crate::state::AppState;

/// Case Chat: one chat thread per case. Reading and posting both require the
/// `SendMessages` capability on the case.
#[component]
pub fn InboxPage() -> impl IntoView {
    let state = expect_context::<AppState>();

    let cases = RwSignal::new(Vec::<CaseSummary>::new());
    let total = RwSignal::new(0i64);
    let load_error = RwSignal::new(None::<String>);

    let selected = RwSignal::new(None::<String>);
    // Mobile only: whether the user has opened a chat (collapses the list and
    // shows the thread full-screen). Desktop always shows both panes.
    let viewing = RwSignal::new(false);
    let search = RwSignal::new(String::new());
    let debounced_search = RwSignal::new(String::new());
    // How many rows the current window requests; grows on "Load more".
    const PAGE: i64 = 15;
    let window = RwSignal::new(PAGE);
    let loading = RwSignal::new(false);

    let input_class = "w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/40";

    // (Re)load the window whenever the debounced search or window size changes
    Effect::new(move |_| {
        let count = window.get();
        let q = debounced_search.get();
        if !state.is_authenticated() {
            return;
        }
        loading.set(true);
        spawn_local(async move {
            match load_case_summaries_for_user(0, count, q).await {
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
        // Auto-select the first accessible case so the chat is populated on load.
        if selected.get_untracked().is_none() {
            if let Some(first) = cases.get().first() {
                selected.set(Some(first.id.clone()));
            }
        }

        // Display title for a case: the case name with its owner appended.
        let title_for = move |c: &CaseSummary| format!("{} · {}", c.name, c.owner_full_name());

        let case_list = move || {
            if let Some(msg) = load_error.get() {
                return view! {
                    <p class="text-sm text-rose-300">"Could not load case chats: " {msg}</p>
                }
                .into_any();
            }
            let all = cases.get();
            if all.is_empty() {
                let msg = if loading.get() {
                    "Loading\u{2026}"
                } else if search.get().trim().is_empty() {
                    "You have no case chats yet."
                } else {
                    "No cases match your search."
                };
                return view! { <p class="text-sm text-slate-400">{msg}</p> }.into_any();
            }
            all
                .into_iter()
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
                        move |_| {
                            selected.set(Some(case_id.clone()));
                            viewing.set(true);
                        }
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
                                <span class="min-w-0 truncate text-sm font-medium text-slate-200">{name}</span>
                                <span class="shrink-0 text-xs text-slate-500">{count} " msgs"</span>
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
                <div class="mt-2 flex items-center justify-between">
                    <p class="text-xs text-slate-500">"Showing " {shown} " of " {tot}</p>
                    <Show when=move || more>
                        <button
                            on:click=move |_| window.update(|w| *w += PAGE)
                            prop:disabled=move || loading.get()
                            class="rounded-lg border border-slate-700 px-3 py-1.5 text-xs font-medium text-slate-300 hover:bg-slate-800 disabled:opacity-50"
                        >
                            {move || if loading.get() { "Loading\u{2026}" } else { "Load more" }}
                        </button>
                    </Show>
                </div>
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
                Some(id) => match cases.get().into_iter().find(|c| c.id == id) {
                    Some(c) => {
                        let title = title_for(&c);
                        let can_send = c.capabilities.contains(&CaseCapability::SendMessages);
                        view! { <CaseChat case_id=c.id case_name=title can_send=can_send /> }.into_any()
                    }
                    None => {
                        view! { <p class="text-sm text-slate-400">"Case not found."</p> }.into_any()
                    }
                },
            }
        };

        // Debounce the search
        let mut on_search = debounce(std::time::Duration::from_secs(1), move |val: String| {
            window.set(PAGE);
            debounced_search.set(val);
        });

        view! {
            <Layout title="Case Chat".to_string()>
                <div class="grid gap-6 lg:grid-cols-[22rem_1fr]">
                    <div class="space-y-2 lg:block" class:hidden=move || viewing.get()>
                        <input
                            class=input_class
                            placeholder="Search cases…"
                            prop:value=move || search.get()
                            on:input=move |ev| {
                                let val = event_target_value(&ev);
                                search.set(val.clone());
                                on_search(val);
                            }
                        />
                        {case_list}
                        {footer}
                    </div>
                    <div class="lg:block" class:hidden=move || !viewing.get()>
                        <Show when=move || viewing.get()>
                            <button
                                on:click=move |_| viewing.set(false)
                                class="mb-4 inline-flex items-center gap-1.5 rounded-lg border border-slate-700 px-3 py-1.5 text-sm font-medium text-slate-200 hover:bg-slate-800 lg:hidden"
                            >
                                "\u{2190} Back to chats"
                            </button>
                        </Show>
                        {thread}
                    </div>
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
fn CaseChat(case_id: String, case_name: String, can_send: bool) -> impl IntoView {
    let state = expect_context::<AppState>();
    let me = state
        .current_user_summary
        .get_untracked()
        .map(|u| u.id)
        .unwrap_or_default();

    // This case's chat messages live here — loaded on demand for the open case.
    let messages = RwSignal::new(Vec::<Message>::new());
    // Tracks the in-flight fetch of the newest messages so the thread can show a
    // loading indicator instead of a premature "no messages" state.
    let loading = RwSignal::new(true);

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
            loading.set(true);
            spawn_local(async move {
                if let Ok(page) = crate::server_fns::cases::list_messages_page(case_id, lim).await {
                    messages.set(page.items);
                    total.set(page.total);
                }
                loading.set(false);
            });
        });
    }

    // Once messages first arrive, jump to the latest one.
    Effect::new(move |_| {
        let has = !messages.get().is_empty();
        if has && !did_initial_scroll.get_untracked() {
            did_initial_scroll.set(true);
            scroll_to_bottom();
        }
    });

    let body = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());

    let send = {
        let case_id = case_id.clone();
        move |_| {
            let case_id = case_id.clone();
            let body_val = body.get_untracked();
            spawn_local(async move {
                match crate::server_fns::cases::send_message(case_id, body_val).await {
                    Ok(msg) => {
                        messages.update(|all| all.push(msg));
                        body.set(String::new());
                        error.set(String::new());
                        total.update(|t| *t += 1);
                        scroll_to_bottom();
                    }
                    Err(e) => error.set(err_text(e)),
                }
            });
        }
    };

    let messages_view = {
        let me = me.clone();
        move || {
            let msgs = messages.get();
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
                if loading.get() {
                    return view! { <Loading label="Loading messages\u{2026}" /> }.into_any();
                }
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
        <div class="flex h-[70dvh] flex-col rounded-xl border border-slate-800 bg-slate-900 lg:h-[calc(100dvh-12rem)]">
            <div class="border-b border-slate-800 p-4">
                <h2 class="truncate text-lg font-semibold">{case_name}</h2>
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
