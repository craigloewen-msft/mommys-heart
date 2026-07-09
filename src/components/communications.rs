//! Reusable read-only communications timeline.
//!
//! Renders the org-owned conversations linked to a case or a contact, so
//! communication activity shows up automatically inside the client's record.
//! Staff work the threads themselves from the Inbox console.

use leptos::prelude::*;

use crate::api_client::{conversations_for_case, conversations_for_contact};
use crate::types::Conversation;

/// Which record the timeline belongs to.
#[derive(Clone, PartialEq)]
pub enum CommScope {
    Case(String),
    Contact(String),
}

fn badge(classes: &str) -> String {
    format!("inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {classes}")
}

#[component]
pub fn CommunicationsTimeline(scope: CommScope) -> impl IntoView {
    let scope_for_fetch = scope.clone();
    let conversations = Resource::new(
        move || scope_for_fetch.clone(),
        |scope| async move {
            match scope {
                CommScope::Case(id) => conversations_for_case(id).await,
                CommScope::Contact(id) => conversations_for_contact(id).await,
            }
        },
    );

    view! {
        <div class="mt-3">
            <p class="mb-1.5 text-xs font-medium uppercase tracking-wide text-slate-400">
                "Communications"
            </p>
            <Suspense fallback=|| {
                view! { <p class="text-xs text-slate-500">"Loading\u{2026}"</p> }
            }>
                {move || Suspend::new(async move {
                    match conversations.await {
                        Ok(list) if list.is_empty() => view! {
                            <p class="rounded-lg bg-slate-950/70 px-3 py-1.5 text-xs text-slate-500">
                                "No communications recorded yet."
                            </p>
                        }
                        .into_any(),
                        Ok(list) => view! {
                            <ul class="space-y-1.5">
                                {list.into_iter().map(conversation_item).collect_view()}
                            </ul>
                        }
                        .into_any(),
                        Err(e) => view! {
                            <p class="text-xs text-rose-400">"Failed to load: " {e}</p>
                        }
                        .into_any(),
                    }
                })}
            </Suspense>
        </div>
    }
}

fn conversation_item(c: Conversation) -> impl IntoView {
    let chan_badge = badge(c.channel.badge_classes());
    let status_badge = badge(c.status.badge_classes());
    let count = format!("{} message(s)", c.message_count);
    view! {
        <li class="rounded-lg bg-slate-950/70 px-3 py-2 text-xs">
            <div class="flex items-center justify-between gap-2">
                <span class="truncate font-medium text-slate-200">{c.subject}</span>
                <div class="flex shrink-0 gap-1">
                    <span class=chan_badge>{c.channel.label()}</span>
                    <span class=status_badge>{c.status.label()}</span>
                </div>
            </div>
            <p class="mt-1 truncate text-slate-500">{c.last_message_preview}</p>
            <p class="mt-0.5 text-[11px] text-slate-600">{count}</p>
        </li>
    }
}
