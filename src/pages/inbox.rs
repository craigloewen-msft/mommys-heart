//! Staff inbox console — the org-owned communications queue.
//!
//! Lists every retained conversation across channels, lets staff/volunteers
//! claim one, reply, move it through its lifecycle, and fold it into a client's
//! case record. Because the data lives in the server-side store (not a personal
//! device), records survive when the volunteer who handled them leaves.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::Redirect;

use crate::api_client::{
    assign_conversation, get_conversation, link_conversation_case, list_conversations,
    reply_conversation, set_conversation_status,
};
use crate::components::layout::Layout;
use crate::state::AppState;
use crate::types::{AuthorKind, Conversation, ConversationStatus, MessageDirection};

fn badge(classes: &str) -> String {
    format!("inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {classes}")
}

#[component]
pub fn InboxPage() -> impl IntoView {
    let state = expect_context::<AppState>();

    // Any signed-in staff/volunteer may work the shared inbox.
    if state.current_user.get_untracked().is_none() {
        return view! { <Redirect path="/login" /> }.into_any();
    }
    let user = state.current_user.get_untracked().unwrap();
    let me_id = StoredValue::new(user.volunteer_id.clone());
    let me_label = StoredValue::new(user.name.clone());

    let reload = RwSignal::new(0u32);
    let selected = RwSignal::new(None::<String>);
    let reply_text = RwSignal::new(String::new());

    let conversations = Resource::new(
        move || reload.get(),
        |_| async move { list_conversations().await },
    );

    let thread = Resource::new(
        move || (selected.get(), reload.get()),
        |(sel, _)| async move {
            match sel {
                Some(id) => get_conversation(id).await.map(Some),
                None => Ok(None),
            }
        },
    );

    // Resolve a volunteer id to a display name from the local demo store.
    let vol_name = move |id: &str| -> String {
        state
            .volunteers
            .get()
            .into_iter()
            .find(|v| v.id == id)
            .map(|v| v.name)
            .unwrap_or_else(|| id.to_string())
    };

    // --- conversation list (left column) ------------------------------------
    let list_view = move || {
        Suspend::new(async move {
            match conversations.await {
                Ok(list) if list.is_empty() => view! {
                    <p class="p-4 text-sm text-slate-500">"No conversations yet."</p>
                }
                .into_any(),
                Ok(list) => list
                    .into_iter()
                    .map(|c| conversation_row(c, selected, vol_name))
                    .collect_view()
                    .into_any(),
                Err(e) => view! {
                    <p class="p-4 text-sm text-rose-400">"Failed to load: " {e}</p>
                }
                .into_any(),
            }
        })
    };

    // --- reply submit -------------------------------------------------------
    let send_reply = move || {
        let Some(id) = selected.get() else { return };
        let text = reply_text.get().trim().to_string();
        if text.is_empty() {
            return;
        }
        reply_text.set(String::new());
        spawn_local(async move {
            let _ =
                reply_conversation(id, text, me_id.get_value(), Some(me_label.get_value())).await;
            reload.update(|n| *n += 1);
        });
    };

    // --- thread + actions (right column) ------------------------------------
    let thread_view = move || {
        Suspend::new(async move {
            match thread.await {
                Ok(None) => view! {
                    <div class="grid h-full place-items-center p-10 text-center text-sm text-slate-500">
                        "Select a conversation to view its full history."
                    </div>
                }
                .into_any(),
                Err(e) => view! {
                    <p class="p-4 text-sm text-rose-400">"Failed to load thread: " {e}</p>
                }
                .into_any(),
                Ok(Some(t)) => {
                    let conv = t.conversation.clone();
                    let conv_id = conv.id.clone();
                    let volunteers = state.volunteers.get();
                    let cases = state.cases.get();

                    let bubbles = t
                        .messages
                        .into_iter()
                        .map(|m| {
                            let inbound = m.direction == MessageDirection::Inbound;
                            let row = if inbound { "flex justify-start" } else { "flex justify-end" };
                            let bubble = if inbound {
                                "max-w-[80%] rounded-lg bg-slate-800 px-4 py-2 text-sm text-slate-100"
                            } else if m.author_kind == AuthorKind::Bot {
                                "max-w-[80%] rounded-lg bg-slate-700/60 px-4 py-2 text-sm text-slate-100"
                            } else {
                                "max-w-[80%] rounded-lg bg-primary-500 px-4 py-2 text-sm text-white"
                            };
                            let meta = format!("{} · {}", m.author_label, m.author_kind.label());
                            view! {
                                <div class=row>
                                    <div class=bubble>
                                        <p class="mb-0.5 text-[10px] uppercase tracking-wide opacity-70">
                                            {meta}
                                        </p>
                                        <p class="whitespace-pre-wrap">{m.body}</p>
                                    </div>
                                </div>
                            }
                        })
                        .collect_view();

                    // Assignment control.
                    let assign_id = conv_id.clone();
                    let assign_opts = volunteers
                        .into_iter()
                        .map(|v| {
                            let sel = conv.assigned_volunteer_id.as_deref() == Some(v.id.as_str());
                            view! { <option value=v.id.clone() selected=sel>{v.name}</option> }
                        })
                        .collect_view();

                    // Case-linking control.
                    let link_id = conv_id.clone();
                    let case_opts = cases
                        .into_iter()
                        .map(|c| {
                            let sel = conv.case_id.as_deref() == Some(c.id.as_str());
                            view! {
                                <option value=c.id.clone() selected=sel>
                                    {format!("{} ({})", c.title, c.id)}
                                </option>
                            }
                        })
                        .collect_view();

                    // Status control.
                    let status_id = conv_id.clone();
                    let status_opts = ConversationStatus::ALL
                        .into_iter()
                        .map(|s| {
                            let sel = s == conv.status;
                            view! { <option value=s.slug() selected=sel>{s.label()}</option> }
                        })
                        .collect_view();

                    let select_class = "rounded-lg border border-slate-700 bg-slate-950 px-2 py-1.5 text-xs text-slate-100 focus:border-primary-500 focus:outline-none";
                    let chan_badge = badge(conv.channel.badge_classes());
                    let status_badge = badge(conv.status.badge_classes());
                    let case_line = match &conv.case_id {
                        Some(c) => format!("Linked to case {c}"),
                        None => "Not linked to a case yet".to_string(),
                    };

                    view! {
                        <div class="flex h-full flex-col">
                            <div class="border-b border-slate-800 p-4">
                                <div class="flex items-center justify-between gap-2">
                                    <h3 class="font-semibold text-white">{conv.subject.clone()}</h3>
                                    <div class="flex shrink-0 gap-1">
                                        <span class=chan_badge>{conv.channel.label()}</span>
                                        <span class=status_badge>{conv.status.label()}</span>
                                    </div>
                                </div>
                                <p class="mt-1 text-xs text-primary-300">{case_line}</p>
                                <div class="mt-3 flex flex-wrap items-center gap-2">
                                    <label class="text-xs text-slate-400">"Owner"</label>
                                    <select
                                        class=select_class
                                        on:change=move |ev| {
                                            let v = event_target_value(&ev);
                                            let vol = if v.is_empty() { None } else { Some(v) };
                                            let id = assign_id.clone();
                                            spawn_local(async move {
                                                let _ = assign_conversation(id, vol).await;
                                                reload.update(|n| *n += 1);
                                            });
                                        }
                                    >
                                        <option value="">"Unassigned"</option>
                                        {assign_opts}
                                    </select>

                                    <label class="ml-2 text-xs text-slate-400">"Case"</label>
                                    <select
                                        class=select_class
                                        on:change=move |ev| {
                                            let v = event_target_value(&ev);
                                            if v.is_empty() {
                                                return;
                                            }
                                            let id = link_id.clone();
                                            spawn_local(async move {
                                                let _ = link_conversation_case(id, Some(v), None).await;
                                                reload.update(|n| *n += 1);
                                            });
                                        }
                                    >
                                        <option value="">"— link a case —"</option>
                                        {case_opts}
                                    </select>

                                    <label class="ml-2 text-xs text-slate-400">"Status"</label>
                                    <select
                                        class=select_class
                                        on:change=move |ev| {
                                            let v = event_target_value(&ev);
                                            let Some(status) = ConversationStatus::from_slug(&v) else {
                                                return;
                                            };
                                            let id = status_id.clone();
                                            spawn_local(async move {
                                                let _ = set_conversation_status(id, status).await;
                                                reload.update(|n| *n += 1);
                                            });
                                        }
                                    >
                                        {status_opts}
                                    </select>
                                </div>
                            </div>

                            <div class="flex-1 space-y-3 overflow-y-auto p-4">{bubbles}</div>

                            <form
                                class="flex gap-2 border-t border-slate-800 p-3"
                                on:submit=move |ev| {
                                    ev.prevent_default();
                                    send_reply();
                                }
                            >
                                <input
                                    class="flex-1 rounded-md border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:outline-none focus:ring-2 focus:ring-primary-500/40"
                                    placeholder="Reply as staff\u{2026}"
                                    prop:value=move || reply_text.get()
                                    on:input=move |ev| reply_text.set(event_target_value(&ev))
                                />
                                <button
                                    type="submit"
                                    class="rounded-md bg-primary-500 px-4 py-2 text-sm font-medium text-white hover:bg-primary-600"
                                >
                                    "Send"
                                </button>
                            </form>
                        </div>
                    }
                    .into_any()
                }
            }
        })
    };

    view! {
        <Layout title="Inbox">
            <p class="mb-4 text-sm text-slate-400">
                "Communications retained by the organization. Anything here stays with Mommy's Heart even if the volunteer who handled it leaves."
            </p>
            <div class="grid grid-cols-1 gap-4 lg:grid-cols-3">
                <div class="overflow-hidden rounded-xl border border-slate-800 bg-slate-900 lg:col-span-1">
                    <div class="border-b border-slate-800 px-4 py-3 text-xs font-medium uppercase tracking-wide text-slate-400">
                        "Conversations"
                    </div>
                    <div class="divide-y divide-slate-800">
                        <Suspense fallback=|| {
                            view! { <p class="p-4 text-sm text-slate-400">"Loading\u{2026}"</p> }
                        }>{list_view}</Suspense>
                    </div>
                </div>

                <div class="min-h-[60vh] overflow-hidden rounded-xl border border-slate-800 bg-slate-900 lg:col-span-2">
                    <Suspense fallback=|| {
                        view! { <p class="p-4 text-sm text-slate-400">"Loading\u{2026}"</p> }
                    }>{thread_view}</Suspense>
                </div>
            </div>
        </Layout>
    }
    .into_any()
}

/// One selectable row in the conversation list.
fn conversation_row(
    c: Conversation,
    selected: RwSignal<Option<String>>,
    vol_name: impl Fn(&str) -> String + 'static,
) -> impl IntoView {
    let id = c.id.clone();
    let is_selected = {
        let id = id.clone();
        move || selected.get().as_deref() == Some(id.as_str())
    };
    let chan_badge = badge(c.channel.badge_classes());
    let status_badge = badge(c.status.badge_classes());
    let owner = match &c.assigned_volunteer_id {
        Some(v) => format!("Owner: {}", vol_name(v)),
        None => "Unassigned".to_string(),
    };

    view! {
        <button
            class=move || {
                let base = "w-full px-4 py-3 text-left hover:bg-slate-800/50 transition-colors";
                if is_selected() {
                    format!("{base} bg-slate-800/70")
                } else {
                    base.to_string()
                }
            }
            on:click=move |_| selected.set(Some(id.clone()))
        >
            <div class="flex items-center justify-between gap-2">
                <span class="truncate text-sm font-medium text-slate-100">{c.subject}</span>
                <span class=chan_badge>{c.channel.label()}</span>
            </div>
            <p class="mt-1 truncate text-xs text-slate-400">{c.last_message_preview}</p>
            <div class="mt-1.5 flex items-center justify-between gap-2">
                <span class="text-[11px] text-slate-500">{owner}</span>
                <span class=status_badge>{c.status.label()}</span>
            </div>
        </button>
    }
}
