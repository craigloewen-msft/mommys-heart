//! Centralized contact directory with multi-category filtering and outreach history.

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::guard::require_volunteer_privileges;
use crate::components::layout::Layout;
use crate::server_fns::contact_directory::{
    add_contact_category, add_contact_communication, get_contact, list_contact_categories,
    save_contact, search_contacts, CommunicationKind, Contact, ContactCategory, ContactDetails,
    ContactInput,
};
use crate::server_fns::err_text;
use crate::state::AppState;

const INPUT: &str = "w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/40";
const LABEL: &str = "mb-1 block text-xs font-medium text-slate-400";

#[derive(Clone, Default)]
struct ContactDraft {
    id: Option<String>,
    input: ContactInput,
}

impl ContactDraft {
    fn from_contact(contact: &Contact) -> Self {
        Self {
            id: Some(contact.id.clone()),
            input: ContactInput {
                full_name: contact.full_name.clone(),
                title: contact.title.clone(),
                organization: contact.organization.clone(),
                email: contact.email.clone(),
                phone: contact.phone.clone(),
                address: contact.address.clone(),
                website: contact.website.clone(),
                category_ids: contact
                    .categories
                    .iter()
                    .map(|category| category.id.clone())
                    .collect(),
            },
        }
    }
}

fn toggle_id(ids: &mut Vec<String>, id: &str, checked: bool) {
    if checked {
        if !ids.iter().any(|selected| selected == id) {
            ids.push(id.to_string());
        }
    } else {
        ids.retain(|selected| selected != id);
    }
}

#[component]
pub fn ContactsPage() -> impl IntoView {
    let state = expect_context::<AppState>();

    require_volunteer_privileges(state, move || {
        let categories = RwSignal::new(Vec::<ContactCategory>::new());
        let contacts = RwSignal::new(Vec::<Contact>::new());
        let selected = RwSignal::new(None::<ContactDetails>);
        let query = RwSignal::new(String::new());
        let debounced_query = RwSignal::new(String::new());
        let filters = RwSignal::new(Vec::<String>::new());
        let search_generation = RwSignal::new(0u64);
        let reload = RwSignal::new(0u32);
        let loading = RwSignal::new(true);
        let error = RwSignal::new(None::<String>);
        let editor_open = RwSignal::new(false);
        let draft = RwSignal::new(ContactDraft::default());
        let saving = RwSignal::new(false);
        let communication_kind = RwSignal::new(CommunicationKind::Outreach.slug().to_string());
        let communication_body = RwSignal::new(String::new());
        let category_name = RwSignal::new(String::new());
        let category_parent = RwSignal::new(String::new());
        let category_saving = RwSignal::new(false);

        Effect::new(move |_| {
            spawn_local(async move {
                match list_contact_categories().await {
                    Ok(items) => categories.set(items),
                    Err(e) => error.set(Some(err_text(e))),
                }
            });
        });

        Effect::new(move |_| {
            let query = debounced_query.get();
            let filters = filters.get();
            reload.track();
            search_generation.update(|generation| *generation += 1);
            let generation = search_generation.get_untracked();
            loading.set(true);
            spawn_local(async move {
                match search_contacts(query, filters).await {
                    Ok(items) if search_generation.get_untracked() == generation => {
                        contacts.set(items);
                        error.set(None);
                    }
                    Err(e) if search_generation.get_untracked() == generation => {
                        error.set(Some(err_text(e)))
                    }
                    _ => return,
                }
                loading.set(false);
            });
        });

        let open_new = move |_| {
            draft.set(ContactDraft::default());
            editor_open.set(true);
            error.set(None);
        };

        let save = move |_| {
            if saving.get_untracked() {
                return;
            }
            let current = draft.get_untracked();
            saving.set(true);
            error.set(None);
            spawn_local(async move {
                match save_contact(current.id, current.input).await {
                    Ok(details) => {
                        selected.set(Some(details));
                        editor_open.set(false);
                        reload.update(|value| *value += 1);
                    }
                    Err(e) => error.set(Some(err_text(e))),
                }
                saving.set(false);
            });
        };

        let add_category = move |_| {
            if category_saving.get_untracked() {
                return;
            }
            category_saving.set(true);
            error.set(None);
            let name = category_name.get_untracked();
            let parent = category_parent.get_untracked();
            spawn_local(async move {
                let parent_id = (!parent.is_empty()).then_some(parent);
                match add_contact_category(name, parent_id).await {
                    Ok(items) => {
                        categories.set(items);
                        category_name.set(String::new());
                        category_parent.set(String::new());
                    }
                    Err(e) => error.set(Some(err_text(e))),
                }
                category_saving.set(false);
            });
        };

        let add_communication = move |_| {
            if saving.get_untracked() {
                return;
            }
            let Some(details) = selected.get_untracked() else {
                return;
            };
            let kind = CommunicationKind::from_slug(&communication_kind.get_untracked())
                .unwrap_or(CommunicationKind::Note);
            let body = communication_body.get_untracked();
            saving.set(true);
            error.set(None);
            spawn_local(async move {
                match add_contact_communication(details.contact.id, kind, body).await {
                    Ok(details) => {
                        selected.set(Some(details));
                        communication_body.set(String::new());
                    }
                    Err(e) => error.set(Some(err_text(e))),
                }
                saving.set(false);
            });
        };

        let mut on_search = debounce(
            std::time::Duration::from_millis(300),
            move |value: String| debounced_query.set(value),
        );

        let filter_options = move || {
            categories
                .get()
                .into_iter()
                .map(|category| {
                    let id = category.id.clone();
                    let checked_id = id.clone();
                    let label = category.label();
                    view! {
                        <label class="flex cursor-pointer items-start gap-2 rounded-md px-2 py-1.5 text-xs text-slate-300 hover:bg-slate-800">
                            <input
                                type="checkbox"
                                class="mt-0.5 accent-primary-500"
                                prop:checked=move || filters.get().contains(&checked_id)
                                on:change=move |event| {
                                    filters.update(|ids| {
                                        toggle_id(ids, &id, event_target_checked(&event))
                                    });
                                }
                            />
                            <span>{label}</span>
                        </label>
                    }
                })
                .collect_view()
        };

        let contact_rows = move || {
            if loading.get() {
                return view! { <p class="p-4 text-sm text-slate-500">"Loading contacts\u{2026}"</p> }
                    .into_any();
            }
            let items = contacts.get();
            if items.is_empty() {
                return view! {
                    <p class="p-4 text-sm text-slate-500">"No contacts match every selected filter."</p>
                }
                .into_any();
            }
            items
                .into_iter()
                .map(|contact| {
                    let id = contact.id.clone();
                    let active_id = id.clone();
                    let subtitle = [contact.title.clone(), contact.organization.clone()]
                        .into_iter()
                        .filter(|value| !value.is_empty())
                        .collect::<Vec<_>>()
                        .join(" at ");
                    let tags = contact
                        .categories
                        .iter()
                        .take(4)
                        .map(|category| {
                            view! {
                                <span class="rounded-full bg-slate-800 px-2 py-0.5 text-[0.68rem] text-slate-300">
                                    {category.name.clone()}
                                </span>
                            }
                        })
                        .collect_view();
                    view! {
                        <button
                            type="button"
                            on:click=move |_| {
                                let id = id.clone();
                                error.set(None);
                                spawn_local(async move {
                                    match get_contact(id).await {
                                        Ok(details) => {
                                            selected.set(Some(details));
                                            editor_open.set(false);
                                        }
                                        Err(e) => error.set(Some(err_text(e))),
                                    }
                                });
                            }
                            class=move || {
                                let base = "w-full border-b border-slate-800 p-4 text-left last:border-b-0 hover:bg-slate-800/60";
                                if selected.get().as_ref().is_some_and(|details| details.contact.id == active_id) {
                                    format!("{base} bg-primary-500/10")
                                } else {
                                    base.to_string()
                                }
                            }
                        >
                            <p class="font-medium text-slate-100">{contact.full_name}</p>
                            {(!subtitle.is_empty()).then(|| view! {
                                <p class="mt-0.5 text-xs text-slate-400">{subtitle}</p>
                            })}
                            <div class="mt-2 flex flex-wrap gap-1">{tags}</div>
                        </button>
                    }
                })
                .collect_view()
                .into_any()
        };

        let detail_panel = move || {
            if editor_open.get() {
                let category_checks = categories
                    .get()
                    .into_iter()
                    .map(|category| {
                        let id = category.id.clone();
                        let checked_id = id.clone();
                        let label = category.label();
                        view! {
                            <label class="flex cursor-pointer items-start gap-2 rounded-md px-2 py-1.5 text-xs text-slate-300 hover:bg-slate-800">
                                <input
                                    type="checkbox"
                                    class="mt-0.5 accent-primary-500"
                                    prop:checked=move || draft.get().input.category_ids.contains(&checked_id)
                                    on:change=move |event| {
                                        draft.update(|draft| {
                                            toggle_id(
                                                &mut draft.input.category_ids,
                                                &id,
                                                event_target_checked(&event),
                                            )
                                        });
                                    }
                                />
                                <span>{label}</span>
                            </label>
                        }
                    })
                    .collect_view();
                let heading = if draft.get().id.is_some() {
                    "Edit contact"
                } else {
                    "New contact"
                };
                return view! {
                    <section class="rounded-xl border border-slate-800 bg-slate-900 p-5">
                        <div class="mb-5 flex items-center justify-between gap-3">
                            <h2 class="text-lg font-semibold">{heading}</h2>
                            <button
                                type="button"
                                on:click=move |_| editor_open.set(false)
                                class="text-sm text-slate-400 hover:text-slate-200"
                            >
                                "Cancel"
                            </button>
                        </div>
                        <div class="grid gap-4 sm:grid-cols-2">
                            <label class="sm:col-span-2">
                                <span class=LABEL>"Full name *"</span>
                                <input class=INPUT prop:value=move || draft.get().input.full_name
                                    on:input=move |event| draft.update(|d| d.input.full_name = event_target_value(&event)) />
                            </label>
                            <label>
                                <span class=LABEL>"Title / position"</span>
                                <input class=INPUT prop:value=move || draft.get().input.title
                                    on:input=move |event| draft.update(|d| d.input.title = event_target_value(&event)) />
                            </label>
                            <label>
                                <span class=LABEL>"Organization"</span>
                                <input class=INPUT prop:value=move || draft.get().input.organization
                                    on:input=move |event| draft.update(|d| d.input.organization = event_target_value(&event)) />
                            </label>
                            <label>
                                <span class=LABEL>"Email"</span>
                                <input type="email" class=INPUT prop:value=move || draft.get().input.email
                                    on:input=move |event| draft.update(|d| d.input.email = event_target_value(&event)) />
                            </label>
                            <label>
                                <span class=LABEL>"Phone number"</span>
                                <input class=INPUT prop:value=move || draft.get().input.phone
                                    on:input=move |event| draft.update(|d| d.input.phone = event_target_value(&event)) />
                            </label>
                            <label class="sm:col-span-2">
                                <span class=LABEL>"Address / location"</span>
                                <input class=INPUT prop:value=move || draft.get().input.address
                                    on:input=move |event| draft.update(|d| d.input.address = event_target_value(&event)) />
                            </label>
                            <label class="sm:col-span-2">
                                <span class=LABEL>"Website"</span>
                                <input class=INPUT placeholder="https://" prop:value=move || draft.get().input.website
                                    on:input=move |event| draft.update(|d| d.input.website = event_target_value(&event)) />
                            </label>
                        </div>
                        <div class="mt-5">
                            <p class=LABEL>"Categories, subcategories, and tags (select multiple)"</p>
                            <div class="max-h-64 overflow-y-auto rounded-lg border border-slate-800 bg-slate-950 p-2">
                                {category_checks}
                            </div>
                        </div>
                        <button
                            type="button"
                            on:click=save
                            prop:disabled=move || saving.get()
                            class="mt-5 rounded-lg bg-primary-500 px-4 py-2 text-sm font-semibold text-white hover:bg-primary-600 disabled:opacity-50"
                        >
                            {move || if saving.get() { "Saving\u{2026}" } else { "Save contact" }}
                        </button>
                    </section>
                }
                .into_any();
            }

            let Some(details) = selected.get() else {
                return view! {
                    <section class="grid min-h-80 place-items-center rounded-xl border border-dashed border-slate-700 bg-slate-900/40 p-8 text-center">
                        <div>
                            <p class="font-medium text-slate-300">"Select a contact"</p>
                            <p class="mt-1 text-sm text-slate-500">"View details, edit categories, and record outreach history."</p>
                        </div>
                    </section>
                }
                .into_any();
            };
            let contact = details.contact.clone();
            let edit_contact = contact.clone();
            let category_badges = contact
                .categories
                .iter()
                .map(|category| view! {
                    <span class="rounded-full bg-primary-500/15 px-2.5 py-1 text-xs text-primary-300 ring-1 ring-primary-500/30">
                        {category.label()}
                    </span>
                })
                .collect_view();
            let communications = if details.communications.is_empty() {
                view! { <p class="text-sm text-slate-500">"No communications logged yet."</p> }
                    .into_any()
            } else {
                details
                    .communications
                    .into_iter()
                    .map(|entry| view! {
                        <article class="border-b border-slate-800 py-4 last:border-b-0">
                            <div class="flex flex-wrap items-center gap-x-2 gap-y-1">
                                <span class="text-xs font-semibold text-primary-300">{entry.kind.label()}</span>
                                <span class="text-xs text-slate-500">{entry.occurred_at} " \u{2022} " {entry.author_name}</span>
                            </div>
                            <p class="mt-2 whitespace-pre-wrap text-sm text-slate-300">{entry.body}</p>
                        </article>
                    })
                    .collect_view()
                    .into_any()
            };

            view! {
                <div class="space-y-5">
                    <section class="rounded-xl border border-slate-800 bg-slate-900 p-5">
                        <div class="flex flex-wrap items-start justify-between gap-3">
                            <div>
                                <h2 class="text-xl font-semibold">{contact.full_name}</h2>
                                <p class="mt-1 text-sm text-slate-400">
                                    {[contact.title.clone(), contact.organization.clone()]
                                        .into_iter().filter(|value| !value.is_empty())
                                        .collect::<Vec<_>>().join(" at ")}
                                </p>
                            </div>
                            <button
                                type="button"
                                on:click=move |_| {
                                    draft.set(ContactDraft::from_contact(&edit_contact));
                                    editor_open.set(true);
                                }
                                class="rounded-lg border border-slate-700 px-3 py-1.5 text-sm font-medium text-slate-200 hover:bg-slate-800"
                            >
                                "Edit"
                            </button>
                        </div>
                        <dl class="mt-5 grid gap-4 text-sm sm:grid-cols-2">
                            <div><dt class="text-xs text-slate-500">"Email"</dt><dd class="mt-1 text-slate-200">{contact.email}</dd></div>
                            <div><dt class="text-xs text-slate-500">"Phone"</dt><dd class="mt-1 text-slate-200">{contact.phone}</dd></div>
                            <div class="sm:col-span-2"><dt class="text-xs text-slate-500">"Address / location"</dt><dd class="mt-1 text-slate-200">{contact.address}</dd></div>
                            <div class="sm:col-span-2"><dt class="text-xs text-slate-500">"Website"</dt><dd class="mt-1 text-slate-200">{contact.website}</dd></div>
                        </dl>
                        <div class="mt-5 flex flex-wrap gap-2">{category_badges}</div>
                        <p class="mt-4 text-xs text-slate-600">"Last updated " {contact.updated_at}</p>
                    </section>

                    <section class="rounded-xl border border-slate-800 bg-slate-900 p-5">
                        <h3 class="font-semibold">"Notes & communications"</h3>
                        <p class="mt-1 text-xs text-slate-500">
                            "Record every outreach attempt, conversation, referral, follow-up, and relationship update."
                        </p>
                        <div class="mt-4 grid gap-3 sm:grid-cols-[12rem_1fr]">
                            <select
                                class=INPUT
                                on:change=move |event| communication_kind.set(event_target_value(&event))
                            >
                                {CommunicationKind::ALL.into_iter().map(|kind| view! {
                                    <option value=kind.slug() selected=move || communication_kind.get() == kind.slug()>
                                        {kind.label()}
                                    </option>
                                }).collect_view()}
                            </select>
                            <textarea
                                class=INPUT
                                rows="4"
                                maxlength="10000"
                                placeholder="What happened, what was discussed, and what should happen next?"
                                prop:value=move || communication_body.get()
                                on:input=move |event| communication_body.set(event_target_value(&event))
                            ></textarea>
                        </div>
                        <button
                            type="button"
                            on:click=add_communication
                            prop:disabled=move || saving.get()
                            class="mt-3 rounded-lg bg-primary-500 px-4 py-2 text-sm font-semibold text-white hover:bg-primary-600 disabled:opacity-50"
                        >
                            "Add to communication log"
                        </button>
                        <div class="mt-5 border-t border-slate-800">{communications}</div>
                    </section>
                </div>
            }
            .into_any()
        };

        view! {
            <Layout title="Contact Directory".to_string()>
                <div class="mb-5 flex flex-wrap items-start justify-between gap-3">
                    <p class="max-w-3xl text-sm text-slate-400">
                        "Search outreach, referral, fundraising, partnership, and volunteer-recruitment contacts. Multiple selected categories are combined, so every result matches all of them."
                    </p>
                    <button
                        type="button"
                        on:click=open_new
                        class="rounded-lg bg-primary-500 px-4 py-2 text-sm font-semibold text-white hover:bg-primary-600"
                    >
                        "Add contact"
                    </button>
                </div>

                <Show when=move || error.get().is_some()>
                    <p class="mb-4 rounded-lg border border-rose-500/40 bg-rose-500/10 px-3 py-2 text-sm text-rose-300">
                        {move || error.get().unwrap_or_default()}
                    </p>
                </Show>

                <details class="mb-5 rounded-xl border border-slate-800 bg-slate-900">
                    <summary class="cursor-pointer px-4 py-3 text-sm font-medium text-slate-300">
                        "Manage categories and tags"
                    </summary>
                    <div class="grid gap-3 border-t border-slate-800 p-4 sm:grid-cols-[1fr_1fr_auto]">
                        <input
                            class=INPUT
                            placeholder="New category or tag name"
                            prop:value=move || category_name.get()
                            on:input=move |event| category_name.set(event_target_value(&event))
                        />
                        <select
                            class=INPUT
                            prop:value=move || category_parent.get()
                            on:change=move |event| category_parent.set(event_target_value(&event))
                        >
                            <option value="">"Top-level category / standalone tag"</option>
                            {move || categories.get().into_iter().filter(|category| category.parent_id.is_none()).map(|category| view! {
                                <option value=category.id>{category.name}</option>
                            }).collect_view()}
                        </select>
                        <button
                            type="button"
                            on:click=add_category
                            prop:disabled=move || category_saving.get()
                            class="rounded-lg border border-primary-500/50 px-4 py-2 text-sm font-medium text-primary-300 hover:bg-primary-500/10 disabled:opacity-50"
                        >
                            "Add"
                        </button>
                    </div>
                </details>

                <div class="grid gap-5 lg:grid-cols-[22rem_minmax(0,1fr)]">
                    <aside class="space-y-4">
                        <input
                            class=INPUT
                            placeholder="Search name, organization, email, location\u{2026}"
                            prop:value=move || query.get()
                            on:input=move |event| {
                                let value = event_target_value(&event);
                                query.set(value.clone());
                                on_search(value);
                            }
                        />
                        <div class="rounded-xl border border-slate-800 bg-slate-900">
                            <div class="flex items-center justify-between border-b border-slate-800 px-4 py-3">
                                <div>
                                    <h2 class="text-sm font-semibold">"Filter categories"</h2>
                                    <p class="text-[0.68rem] text-slate-500">"Matches all selected"</p>
                                </div>
                                <button
                                    type="button"
                                    on:click=move |_| filters.set(Vec::new())
                                    class="text-xs text-primary-300 hover:text-primary-200"
                                >
                                    "Clear"
                                </button>
                            </div>
                            <div class="max-h-72 overflow-y-auto p-2">{filter_options}</div>
                        </div>
                        <div class="max-h-[42rem] overflow-y-auto rounded-xl border border-slate-800 bg-slate-900">
                            {contact_rows}
                        </div>
                    </aside>
                    <main>{detail_panel}</main>
                </div>
            </Layout>
        }
        .into_any()
    })
}
