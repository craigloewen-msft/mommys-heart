use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api_client::send_chat;
use crate::components::layout::Layout;
use crate::types::{ChatRequest, SourceInfo};

#[derive(Clone)]
struct Message {
    role: &'static str,
    text: String,
    sources: Vec<SourceInfo>,
}

#[component]
pub fn ChatPage() -> impl IntoView {
    let messages = RwSignal::new(Vec::<Message>::new());
    let input = RwSignal::new(String::new());
    let pending = RwSignal::new(false);
    // Retained server-side conversation id, echoed back so the whole thread
    // stays together (and a human can pick it up from the inbox).
    let conversation_id = RwSignal::new(None::<String>);

    let send = move || {
        let text = input.get().trim().to_string();
        if text.is_empty() || pending.get() {
            return;
        }
        messages.update(|m| {
            m.push(Message {
                role: "user",
                text: text.clone(),
                sources: Vec::new(),
            })
        });
        input.set(String::new());
        pending.set(true);

        spawn_local(async move {
            let reply = match send_chat(ChatRequest {
                message: text,
                captcha_token: None,
                conversation_id: conversation_id.get_untracked(),
            })
            .await
            {
                Ok(r) => {
                    if r.conversation_id.is_some() {
                        conversation_id.set(r.conversation_id.clone());
                    }
                    Message {
                        role: "assistant",
                        text: r.answer,
                        sources: r.sources,
                    }
                }
                Err(_) => Message {
                    role: "assistant",
                    text: "Sorry \u{2014} something went wrong talking to the chat API.".into(),
                    sources: Vec::new(),
                },
            };
            messages.update(|m| m.push(reply));
            pending.set(false);
        });
    };

    // Kept out of the `view!` macro: the `::<Vec<_>>` turbofish would otherwise
    // be misparsed as HTML tags by the macro.
    let rows = move || {
        let list = messages.get();
        list.into_iter()
            .enumerate()
            .collect::<Vec<(usize, Message)>>()
    };

    view! {
        <Layout title="Chat assistant">
            <div class="max-w-3xl mx-auto rounded-xl border border-slate-800 bg-slate-900 flex flex-col">
                <div class="p-4 space-y-4 min-h-[50vh]">
                    <Show when=move || messages.read().is_empty()>
                        <p class="text-sm text-slate-500 text-center py-12">
                            "Ask a question to try the dedicated /api/chat endpoint."
                        </p>
                    </Show>
                    <For
                        each=rows
                        key=|(i, _)| *i
                        children=move |(_, msg)| {
                            let is_user = msg.role == "user";
                            let bubble = if is_user {
                                "max-w-[80%] rounded-lg px-4 py-2 text-sm bg-primary-500 text-white"
                            } else {
                                "max-w-[80%] rounded-lg px-4 py-2 text-sm bg-slate-800 text-slate-100"
                            };
                            let row = if is_user {
                                "flex justify-end"
                            } else {
                                "flex justify-start"
                            };
                            view! {
                                <div class=row>
                                    <div class=bubble>
                                        <p class="whitespace-pre-wrap">{msg.text}</p>
                                        <Show when={
                                            let sources = msg.sources.clone();
                                            move || !sources.is_empty()
                                        }>
                                            <ul class="mt-2 text-xs opacity-80 list-disc pl-4">
                                                {msg
                                                    .sources
                                                    .iter()
                                                    .map(|s| {
                                                        view! {
                                                            <li>{format!("{} \u{2014} {}", s.filename, s.heading)}</li>
                                                        }
                                                    })
                                                    .collect_view()}
                                            </ul>
                                        </Show>
                                    </div>
                                </div>
                            }
                        }
                    />
                </div>

                <form
                    class="border-t border-slate-800 p-3 flex gap-2"
                    on:submit=move |ev| {
                        ev.prevent_default();
                        send();
                    }
                >
                    <input
                        class="flex-1 rounded-md border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:outline-none focus:ring-2 focus:ring-primary-500/40"
                        placeholder="Type a message\u{2026}"
                        prop:value=move || input.get()
                        on:input=move |ev| input.set(event_target_value(&ev))
                        prop:disabled=move || pending.get()
                    />
                    <button
                        type="submit"
                        class="rounded-md bg-primary-500 px-4 py-2 text-sm font-medium text-white hover:bg-primary-600 disabled:opacity-50"
                        prop:disabled=move || pending.get()
                    >
                        "Send"
                    </button>
                </form>
            </div>
        </Layout>
    }
}
