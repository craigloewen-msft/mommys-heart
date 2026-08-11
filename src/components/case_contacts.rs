//! The "People on this case" panel.
//!
//! Rendered only for staff. Reading follows `ViewCase`; adding and removing
//! follow `EditCase`, both enforced on the server.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;

use crate::helpers::format::badge_pill;
use crate::server_fns::case_contacts::{
    add_case_contact, list_case_contacts, remove_case_contact, CaseContact, CaseContactRole,
};
use crate::server_fns::contacts::{list_contacts, ContactFilters};
use crate::server_fns::err_text;

const PANEL: &str = "rounded-xl border border-slate-800 bg-slate-900 p-5";
const INPUT: &str =
    "w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-600 focus:border-primary-500 focus:outline-none";
const LABEL: &str = "text-xs font-medium text-slate-400";

#[component]
pub fn CaseContactsPanel(case_id: String, can_edit: bool) -> impl IntoView {
    let id = StoredValue::new(case_id);
    let people = RwSignal::new(Vec::<CaseContact>::new());
    let directory = RwSignal::new(Vec::<(String, String)>::new());
    let loading = RwSignal::new(true);
    let error = RwSignal::new(String::new());
    let reload = RwSignal::new(0u32);
    let adding = RwSignal::new(false);

    let contact_id = RwSignal::new(String::new());
    let role = RwSignal::new(CaseContactRole::default().slug().to_string());
    let note = RwSignal::new(String::new());
    let is_primary = RwSignal::new(false);
    let busy = RwSignal::new(false);

    Effect::new(move |_| {
        reload.track();
        loading.set(true);
        spawn_local(async move {
            match list_case_contacts(id.get_value()).await {
                Ok(list) => {
                    people.set(list);
                    error.set(String::new());
                }
                Err(e) => error.set(err_text(e)),
            }
            loading.set(false);
        });
    });

    // Only loaded when the add form opens: the picker is useless to a reader.
    Effect::new(move |_| {
        if !adding.get() || !directory.get_untracked().is_empty() {
            return;
        }
        spawn_local(async move {
            if let Ok(page) = list_contacts(ContactFilters::default(), 0, 200).await {
                directory.set(
                    page.items
                        .into_iter()
                        .map(|c| {
                            let label = if c.organization_name.is_empty() {
                                c.display_name()
                            } else {
                                format!("{} ({})", c.display_name(), c.organization_name)
                            };
                            (c.id, label)
                        })
                        .collect(),
                );
            }
        });
    });

    let submit = move |_| {
        if busy.get_untracked() {
            return;
        }
        let chosen = contact_id.get_untracked();
        if chosen.is_empty() {
            error.set("Choose a person to add.".into());
            return;
        }
        busy.set(true);
        error.set(String::new());
        spawn_local(async move {
            let result = add_case_contact(
                id.get_value(),
                chosen,
                CaseContactRole::from_slug(&role.get_untracked()).unwrap_or_default(),
                note.get_untracked(),
                is_primary.get_untracked(),
            )
            .await;
            match result {
                Ok(()) => {
                    adding.set(false);
                    contact_id.set(String::new());
                    note.set(String::new());
                    is_primary.set(false);
                    reload.update(|r| *r += 1);
                }
                Err(e) => error.set(err_text(e)),
            }
            busy.set(false);
        });
    };

    let rows = move || {
        let list = people.get();
        if list.is_empty() {
            let message = if loading.get() {
                "Loading people\u{2026}"
            } else {
                "Nobody has been linked to this case yet."
            };
            return view! { <p class="text-sm text-slate-500">{message}</p> }.into_any();
        }
        list.into_iter()
            .map(|person| {
                let link_id = person.id.clone();
                let href = format!("/admin/people/{}", person.contact_id);
                let role = person.role;
                let primary = person.is_primary;
                let archived = person.contact_archived;
                let note = person.note.clone();
                let has_note = !note.is_empty();
                let org = person.organization_name.clone();
                let email = person.email.clone();
                let remove = move |_| {
                    let link_id = link_id.clone();
                    spawn_local(async move {
                        match remove_case_contact(link_id).await {
                            Ok(()) => reload.update(|r| *r += 1),
                            Err(e) => error.set(err_text(e)),
                        }
                    });
                };
                view! {
                    <div class="rounded-lg border border-slate-800 bg-slate-950 p-3">
                        <div class="flex flex-wrap items-start justify-between gap-2">
                            <div class="min-w-0">
                                <div class="flex flex-wrap items-center gap-2">
                                    <A href=href attr:class="text-sm font-semibold text-slate-100 hover:text-primary-300">
                                        {person.contact_name.clone()}
                                    </A>
                                    <span class=badge_pill(role.badge_classes())>{role.label()}</span>
                                    <Show when=move || primary>
                                        <span class=badge_pill("bg-primary-500/15 text-primary-300 ring-1 ring-primary-500/30")>"Primary"</span>
                                    </Show>
                                    <Show when=move || archived>
                                        <span class=badge_pill("bg-slate-700/40 text-slate-300 ring-1 ring-slate-600")>"Archived"</span>
                                    </Show>
                                </div>
                                <p class="mt-1 text-xs text-slate-500">
                                    {if org.is_empty() { "No organization".to_string() } else { org }}
                                    {if email.is_empty() { String::new() } else { format!(" \u{b7} {email}") }}
                                </p>
                                <Show when=move || has_note>
                                    <p class="mt-1 text-xs text-slate-400">{note.clone()}</p>
                                </Show>
                            </div>
                            <Show when=move || can_edit>
                                <button
                                    type="button"
                                    on:click=remove.clone()
                                    class="shrink-0 rounded-lg border border-slate-700 px-2 py-1 text-xs font-medium text-slate-400 hover:bg-slate-800"
                                >
                                    "Remove"
                                </button>
                            </Show>
                        </div>
                    </div>
                }
            })
            .collect_view()
            .into_any()
    };

    view! {
        <div class=PANEL>
            <div class="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                <div>
                    <h2 class="text-lg font-semibold text-slate-100">"People on this case"</h2>
                    <p class="mt-1 text-sm text-slate-500">
                        "Attorneys, caseworkers, emergency contacts, and anyone else involved."
                    </p>
                </div>
                <Show when=move || can_edit>
                    <button
                        type="button"
                        on:click=move |_| adding.update(|a| *a = !*a)
                        class="shrink-0 rounded-lg bg-primary-500 px-4 py-2 text-sm font-semibold text-white hover:bg-primary-600"
                    >
                        {move || if adding.get() { "Cancel" } else { "+ Add person" }}
                    </button>
                </Show>
            </div>

            <Show when=move || adding.get()>
                <div class="mt-4 space-y-3 border-t border-slate-800 pt-4">
                    <div class="grid gap-3 sm:grid-cols-2">
                        <label class="block">
                            <span class=LABEL>"Person"</span>
                            <select
                                class=INPUT
                                prop:value=move || contact_id.get()
                                on:change=move |e| contact_id.set(event_target_value(&e))
                            >
                                <option value="">"Choose someone\u{2026}"</option>
                                {move || directory
                                    .get()
                                    .into_iter()
                                    .map(|(id, label)| view! { <option value=id>{label}</option> })
                                    .collect_view()}
                            </select>
                        </label>
                        <label class="block">
                            <span class=LABEL>"Role on this case"</span>
                            <select
                                class=INPUT
                                prop:value=move || role.get()
                                on:change=move |e| role.set(event_target_value(&e))
                            >
                                {CaseContactRole::ALL
                                    .iter()
                                    .map(|r| view! { <option value=r.slug()>{r.label()}</option> })
                                    .collect_view()}
                            </select>
                        </label>
                    </div>
                    <label class="block">
                        <span class=LABEL>"Note"</span>
                        <input
                            class=INPUT
                            placeholder="Optional context"
                            prop:value=move || note.get()
                            on:input=move |e| note.set(event_target_value(&e))
                        />
                    </label>
                    <label class="flex items-center gap-2 text-sm text-slate-300">
                        <input
                            type="checkbox"
                            class="h-4 w-4 rounded border-slate-700 bg-slate-950"
                            prop:checked=move || is_primary.get()
                            on:change=move |e| is_primary.set(event_target_checked(&e))
                        />
                        "Primary contact for this case"
                    </label>
                    <p class="text-xs text-slate-500">
                        "Not in the list? An administrator adds people under Admin \u{2192} People."
                    </p>
                    <button
                        type="button"
                        on:click=submit
                        prop:disabled=move || busy.get()
                        class="rounded-lg bg-primary-500 px-4 py-2 text-sm font-semibold text-white hover:bg-primary-600 disabled:opacity-50"
                    >
                        {move || if busy.get() { "Adding\u{2026}" } else { "Add to case" }}
                    </button>
                </div>
            </Show>

            <Show when=move || !error.get().is_empty()>
                <p class="mt-3 text-sm text-rose-300" role="alert">{move || error.get()}</p>
            </Show>

            <div class="mt-4 space-y-2">{rows}</div>
        </div>
    }
}
