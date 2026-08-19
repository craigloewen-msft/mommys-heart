//! Contacts: the shared staff directory for outreach and CRM detail.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_params_map, use_query_map};
use leptos_router::NavigateOptions;

use crate::components::contact_form::ContactTypeSelector;
use crate::components::guard::require_information_management_access;
use crate::components::layout::Layout;
use crate::components::property_filters::{MatchedProperties, PropertyFilterBar};
use crate::pages::people::ContactDetail;
use crate::server_fns::contact_directory::{
    add_contact_category, add_contact_communication, get_contact, list_contact_categories,
    save_contact, search_contacts, set_contact_categories, CommunicationKind, Contact,
    ContactCategory, ContactDetails, ContactInput,
};
use crate::server_fns::contacts::{ContactType, MAX_TYPES};
use crate::server_fns::err_text;
use crate::server_fns::organizations::{list_organizations, OrganizationFilters};
use crate::server_fns::property_filters::{
    self, PropertyFacetScope, PropertyFilter, PropertySubject,
};
use crate::state::AppState;

pub(crate) const INPUT: &str = "w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/40";
pub(crate) const LABEL: &str = "mb-1 block text-xs font-medium text-slate-400";

#[derive(Clone, Default)]
struct ContactDraft {
    pub id: Option<String>,
    pub input: ContactInput,
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
                types: contact.types.clone(),
                category_ids: contact
                    .categories
                    .iter()
                    .map(|category| category.id.clone())
                    .collect(),
            },
        }
    }
}

pub(crate) fn toggle_id(ids: &mut Vec<String>, id: &str, checked: bool) {
    if checked {
        if !ids.iter().any(|selected| selected == id) {
            ids.push(id.to_string());
        }
    } else {
        ids.retain(|selected| selected != id);
    }
}

/// Percent-encode a filter value for the query string.
///
/// Hand-rolled because the browser's `encodeURIComponent` is not available in
/// the SSR build, and the set of characters that matter here is small.
pub(crate) fn encode_query_value(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char)
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[component]
fn DirectoryContactEditor(
    draft: RwSignal<ContactDraft>,
    categories: RwSignal<Vec<ContactCategory>>,
    saving: RwSignal<bool>,
    on_save: Callback<()>,
    on_cancel: Callback<()>,
    show_categories: bool,
) -> impl IntoView {
    let category_checks = move || {
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
            .collect_view()
    };

    view! {
        <section class="rounded-xl border border-slate-800 bg-slate-900 p-5">
            <div class="mb-5 flex items-center justify-between gap-3">
                <h2 class="text-lg font-semibold">
                    {move || if draft.get().id.is_some() { "Edit contact" } else { "New contact" }}
                </h2>
                <button
                    type="button"
                    on:click=move |_| on_cancel.run(())
                    class="text-sm text-slate-400 hover:text-slate-200"
                >
                    "Cancel"
                </button>
            </div>
            <div class="grid gap-4 sm:grid-cols-2">
                <label class="sm:col-span-2">
                    <span class=LABEL>"Full name *"</span>
                    <input
                        class=INPUT
                        prop:value=move || draft.get().input.full_name
                        on:input=move |event| {
                            draft.update(|draft| draft.input.full_name = event_target_value(&event))
                        }
                    />
                </label>
                <label>
                    <span class=LABEL>"Title / position"</span>
                    <input
                        class=INPUT
                        prop:value=move || draft.get().input.title
                        on:input=move |event| {
                            draft.update(|draft| draft.input.title = event_target_value(&event))
                        }
                    />
                </label>
                <label>
                    <span class=LABEL>"Organization"</span>
                    <input
                        class=INPUT
                        prop:value=move || draft.get().input.organization
                        on:input=move |event| {
                            draft.update(|draft| draft.input.organization = event_target_value(&event))
                        }
                    />
                </label>
                <label>
                    <span class=LABEL>"Email"</span>
                    <input
                        type="email"
                        class=INPUT
                        prop:value=move || draft.get().input.email
                        on:input=move |event| {
                            draft.update(|draft| draft.input.email = event_target_value(&event))
                        }
                    />
                </label>
                <label>
                    <span class=LABEL>"Phone number"</span>
                    <input
                        class=INPUT
                        prop:value=move || draft.get().input.phone
                        on:input=move |event| {
                            draft.update(|draft| draft.input.phone = event_target_value(&event))
                        }
                    />
                </label>
                <label class="sm:col-span-2">
                    <span class=LABEL>"Address / location"</span>
                    <input
                        class=INPUT
                        prop:value=move || draft.get().input.address
                        on:input=move |event| {
                            draft.update(|draft| draft.input.address = event_target_value(&event))
                        }
                    />
                </label>
                <label class="sm:col-span-2">
                    <span class=LABEL>"Website"</span>
                    <input
                        class=INPUT
                        placeholder="https://"
                        prop:value=move || draft.get().input.website
                        on:input=move |event| {
                            draft.update(|draft| draft.input.website = event_target_value(&event))
                        }
                    />
                </label>
            </div>
            <div class="mt-5">
                <ContactTypeSelector
                    selected=Signal::derive(move || draft.get().input.types)
                    on_toggle=Callback::new(move |contact_type| {
                        draft.update(|draft| {
                            let types = &mut draft.input.types;
                            if let Some(index) = types.iter().position(|item| *item == contact_type) {
                                types.remove(index);
                            } else if types.len() < MAX_TYPES {
                                types.push(contact_type);
                            }
                        });
                    })
                />
            </div>
            <Show when=move || show_categories>
                <div class="mt-5">
                    <p class=LABEL>"Categories, subcategories, and tags (optional)"</p>
                    <p class="mb-2 text-xs text-slate-500">
                        "Use these organization-defined groupings for more specific classification."
                    </p>
                    <div class="max-h-64 overflow-y-auto rounded-lg border border-slate-800 bg-slate-950 p-2">
                        {category_checks}
                    </div>
                </div>
            </Show>
            <div class="mt-5 flex flex-wrap gap-2">
                <button
                    type="button"
                    on:click=move |_| on_save.run(())
                    prop:disabled=move || saving.get()
                    class="rounded-lg bg-primary-500 px-4 py-2 text-sm font-semibold text-white hover:bg-primary-600 disabled:opacity-50"
                >
                    {move || if saving.get() { "Saving…" } else { "Save contact" }}
                </button>
                <button
                    type="button"
                    on:click=move |_| on_cancel.run(())
                    class="rounded-lg border border-slate-700 px-4 py-2 text-sm font-medium text-slate-300 hover:bg-slate-800"
                >
                    "Cancel"
                </button>
            </div>
        </section>
    }
}

/// Category membership and outreach history layered over the structured contact.
#[component]
pub(crate) fn ContactOutreachPanel(
    contact_id: String,
    contact_changed: Callback<()>,
) -> impl IntoView {
    let id = StoredValue::new(contact_id);
    let state = expect_context::<AppState>();
    let can_edit_directory_fields = state.has_information_management_access();
    let details = RwSignal::new(None::<ContactDetails>);
    let categories = RwSignal::new(Vec::<ContactCategory>::new());
    let selected_categories = RwSignal::new(Vec::<String>::new());
    let editing_contact = RwSignal::new(false);
    let contact_draft = RwSignal::new(ContactDraft::default());
    let editing_categories = RwSignal::new(false);
    let kind = RwSignal::new(CommunicationKind::Outreach.slug().to_string());
    let body = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    let reload = RwSignal::new(0u32);
    let reset_contact_draft = move || {
        if let Some(current) = details.get_untracked() {
            contact_draft.set(ContactDraft::from_contact(&current.contact));
        }
    };
    let reset_categories = move || {
        if let Some(current) = details.get_untracked() {
            selected_categories.set(
                current
                    .contact
                    .categories
                    .iter()
                    .map(|category| category.id.clone())
                    .collect(),
            );
        }
    };

    Effect::new(move |_| {
        reload.track();
        spawn_local(async move {
            match get_contact(id.get_value()).await {
                Ok(found) => {
                    let Some(is_editing) = editing_contact.try_get_untracked() else {
                        return;
                    };
                    selected_categories.try_set(
                        found
                            .contact
                            .categories
                            .iter()
                            .map(|category| category.id.clone())
                            .collect(),
                    );
                    if !is_editing {
                        contact_draft.try_set(ContactDraft::from_contact(&found.contact));
                    }
                    details.try_set(Some(found));
                    error.try_set(String::new());
                }
                Err(err) => {
                    error.try_set(err_text(err));
                }
            }
            if categories
                .try_get_untracked()
                .is_some_and(|items| items.is_empty())
            {
                if let Ok(items) = list_contact_categories().await {
                    categories.try_set(items);
                }
            }
        });
    });

    let save_contact_fields = move |()| {
        let current = contact_draft.get_untracked();
        busy.set(true);
        error.set(String::new());
        spawn_local(async move {
            match save_contact(current.id, current.input).await {
                Ok(found) => {
                    details.set(Some(found));
                    editing_contact.set(false);
                    reload.update(|value| *value += 1);
                    contact_changed.run(());
                }
                Err(err) => error.set(err_text(err)),
            }
            busy.set(false);
        });
    };

    let save_categories = move |_| {
        let Some(current) = details.get_untracked() else {
            return;
        };
        let _ = current;
        busy.set(true);
        spawn_local(async move {
            match set_contact_categories(id.get_value(), selected_categories.get_untracked()).await
            {
                Ok(found) => {
                    details.set(Some(found));
                    editing_categories.set(false);
                    reload.update(|value| *value += 1);
                }
                Err(err) => error.set(err_text(err)),
            }
            busy.set(false);
        });
    };

    let add_communication = move |_| {
        let text = body.get_untracked();
        let communication_kind =
            CommunicationKind::from_slug(&kind.get_untracked()).unwrap_or(CommunicationKind::Note);
        busy.set(true);
        spawn_local(async move {
            match add_contact_communication(id.get_value(), communication_kind, text).await {
                Ok(found) => {
                    details.set(Some(found));
                    body.set(String::new());
                }
                Err(err) => error.set(err_text(err)),
            }
            busy.set(false);
        });
    };

    view! {
        <section class="rounded-xl border border-slate-800 bg-slate-900 p-5">
            <div class="flex flex-wrap items-start justify-between gap-3">
                <div>
                    <h3 class="text-sm font-semibold text-slate-200">"Categories & communications"</h3>
                    <p class="mt-1 text-xs text-slate-500">
                        "Organize this contact and record outreach, referrals, follow-ups, and relationship updates."
                    </p>
                </div>
                <div class="flex flex-wrap gap-2">
                    <Show when=move || {
                        can_edit_directory_fields
                            && details.get().is_some_and(|current| !current.contact.has_account)
                    }>
                        <button
                            type="button"
                            on:click=move |_| {
                                if editing_contact.get_untracked() {
                                    reset_contact_draft();
                                    editing_contact.set(false);
                                } else {
                                    reset_contact_draft();
                                    editing_contact.set(true);
                                }
                                reset_categories();
                                editing_categories.set(false);
                            }
                            class="rounded-lg border border-slate-700 px-3 py-1.5 text-xs font-medium text-slate-300 hover:bg-slate-800"
                        >
                            {move || if editing_contact.get() { "Cancel contact edit" } else { "Edit directory fields" }}
                        </button>
                    </Show>
                    <button
                        type="button"
                        on:click=move |_| {
                            if editing_categories.get_untracked() {
                                reset_categories();
                                editing_categories.set(false);
                            } else {
                                reset_categories();
                                editing_categories.set(true);
                            }
                            reset_contact_draft();
                            editing_contact.set(false);
                        }
                        class="rounded-lg border border-slate-700 px-3 py-1.5 text-xs font-medium text-slate-300 hover:bg-slate-800"
                    >
                        {move || if editing_categories.get() { "Cancel category edit" } else { "Edit categories" }}
                    </button>
                </div>
            </div>

            <Show when=move || !error.get().is_empty()>
                <p class="mt-3 text-sm text-rose-300" role="alert">{move || error.get()}</p>
            </Show>

            <Show when=move || editing_contact.get()>
                <div class="mt-4 border-t border-slate-800 pt-4">
                    <DirectoryContactEditor
                        draft=contact_draft
                        categories
                        saving=busy
                        on_save=Callback::new(save_contact_fields)
                        on_cancel=Callback::new(move |()| {
                            reset_contact_draft();
                            editing_contact.set(false);
                        })
                        show_categories=false
                    />
                </div>
            </Show>

            <Show
                when=move || editing_categories.get()
                fallback=move || {
                    let badges = details
                        .get()
                        .map(|details| details.contact.categories)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|category| view! {
                            <span class="rounded-full bg-primary-500/15 px-2.5 py-1 text-xs text-primary-300 ring-1 ring-primary-500/30">
                                {category.label()}
                            </span>
                        })
                        .collect_view();
                    view! { <div class="mt-4 flex flex-wrap gap-2">{badges}</div> }
                }
            >
                <div class="mt-4">
                    <div class="max-h-64 overflow-y-auto rounded-lg border border-slate-800 bg-slate-950 p-2">
                        {move || categories.get().into_iter().map(|category| {
                            let category_id = category.id.clone();
                            let checked_id = category_id.clone();
                            view! {
                                <label class="flex cursor-pointer items-start gap-2 rounded-md px-2 py-1.5 text-xs text-slate-300 hover:bg-slate-800">
                                    <input
                                        type="checkbox"
                                        class="mt-0.5 accent-primary-500"
                                        prop:checked=move || selected_categories.get().contains(&checked_id)
                                        on:change=move |event| selected_categories.update(|ids| {
                                            toggle_id(ids, &category_id, event_target_checked(&event))
                                        })
                                    />
                                    <span>{category.label()}</span>
                                </label>
                            }
                        }).collect_view()}
                    </div>
                    <button
                        type="button"
                        on:click=save_categories
                        prop:disabled=move || busy.get()
                        class="mt-3 rounded-lg bg-primary-500 px-3 py-1.5 text-sm font-semibold text-white hover:bg-primary-600 disabled:opacity-50"
                    >
                        {move || if busy.get() { "Saving…" } else { "Save categories" }}
                    </button>
                </div>
            </Show>

            <div class="mt-5 border-t border-slate-800 pt-5">
                <div class="grid gap-3 sm:grid-cols-[12rem_1fr]">
                    <select
                        class=INPUT
                        prop:value=move || kind.get()
                        on:change=move |event| kind.set(event_target_value(&event))
                    >
                        {CommunicationKind::ALL.into_iter().map(|item| view! {
                            <option value=item.slug()>{item.label()}</option>
                        }).collect_view()}
                    </select>
                    <textarea
                        class=INPUT
                        rows="4"
                        maxlength="10000"
                        placeholder="What happened, what was discussed, and what should happen next?"
                        prop:value=move || body.get()
                        on:input=move |event| body.set(event_target_value(&event))
                    ></textarea>
                </div>
                <button
                    type="button"
                    on:click=add_communication
                    prop:disabled=move || busy.get() || body.get().trim().is_empty()
                    class="mt-3 rounded-lg bg-primary-500 px-4 py-2 text-sm font-semibold text-white hover:bg-primary-600 disabled:opacity-50"
                >
                    "Add to communication log"
                </button>
                <div class="mt-5 border-t border-slate-800">
                    {move || {
                        let entries = details
                            .get()
                            .map(|details| details.communications)
                            .unwrap_or_default();
                        if entries.is_empty() {
                            view! { <p class="py-4 text-sm text-slate-500">"No communications logged yet."</p> }.into_any()
                        } else {
                            entries.into_iter().map(|entry| view! {
                                <article class="border-b border-slate-800 py-4 last:border-b-0">
                                    <div class="flex flex-wrap items-center gap-x-2 gap-y-1">
                                        <span class="text-xs font-semibold text-primary-300">{entry.kind.label()}</span>
                                        <span class="text-xs text-slate-500">{entry.occurred_at} " · " {entry.author_name}</span>
                                    </div>
                                    <p class="mt-2 whitespace-pre-wrap text-sm text-slate-300">{entry.body}</p>
                                </article>
                            }).collect_view().into_any()
                        }
                    }}
                </div>
            </div>
        </section>
    }
}

#[component]
pub fn ContactsPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    let params = use_params_map();

    require_information_management_access(state, move || {
        view! {
            <Layout title="Contacts".to_string()>
                {move || {
                    let selected_id = params.read().get("id").filter(|id| !id.trim().is_empty());
                    match selected_id {
                        Some(id) => view! { <ContactDetail contact_id=id /> }.into_any(),
                        None => view! { <ContactsDirectory /> }.into_any(),
                    }
                }}
            </Layout>
        }
        .into_any()
    })
}

#[component]
fn ContactsDirectory() -> impl IntoView {
    let state = expect_context::<AppState>();
    let can_send_mail =
        state.has_operations_admin_permissions() && state.has_information_management_access();
    let can_bulk_edit = state.has_information_management_access();

    // Seed every filter from the query string, so a shared link opens the view
    // its sender was looking at.
    let initial = use_query_map().get_untracked();
    let initial_query = initial.get("q").unwrap_or_default();
    let initial_type = initial.get("type").unwrap_or_default();
    let initial_organization = initial.get("org").unwrap_or_default();
    let initial_categories: Vec<String> = initial
        .get("cat")
        .unwrap_or_default()
        .split(',')
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect();
    let initial_archived = initial.get("archived").as_deref() == Some("1");
    let initial_properties = property_filters::decode(&initial.get("props").unwrap_or_default());
    drop(initial);

    let categories = RwSignal::new(Vec::<ContactCategory>::new());
    let contacts = RwSignal::new(Vec::<Contact>::new());
    let total = RwSignal::new(0i64);
    let query = RwSignal::new(initial_query.clone());
    let debounced_query = RwSignal::new(initial_query);
    let filters = RwSignal::new(initial_categories);
    let contact_type = RwSignal::new(initial_type);
    let organization_id = RwSignal::new(initial_organization);
    let include_archived = RwSignal::new(initial_archived);
    let property_filters = RwSignal::new(initial_properties);
    let organizations = RwSignal::new(Vec::<(String, String)>::new());
    let search_generation = RwSignal::new(0u64);
    let reload = RwSignal::new(0u32);
    let loading = RwSignal::new(true);
    let error = RwSignal::new(None::<String>);
    let creating = RwSignal::new(false);
    let draft = RwSignal::new(ContactDraft::default());
    let saving = RwSignal::new(false);
    let category_name = RwSignal::new(String::new());
    let category_parent = RwSignal::new(String::new());
    let category_saving = RwSignal::new(false);
    let offset = RwSignal::new(0i64);

    // What the facet counts are measured against, so the values on offer match
    // the list on screen.
    let facet_scope = Signal::derive(move || PropertyFacetScope {
        keyword: debounced_query.get(),
        include_archived: include_archived.get(),
        category_ids: filters.get(),
        contact_type: contact_type.get(),
        organization_id: organization_id.get(),
        ..Default::default()
    });

    Effect::new(move |_| {
        spawn_local(async move {
            match list_contact_categories().await {
                Ok(items) => categories.set(items),
                Err(e) => error.set(Some(err_text(e))),
            }
        });
    });

    Effect::new(move |_| {
        reload.track();
        spawn_local(async move {
            let filters = OrganizationFilters {
                include_archived: true,
                ..Default::default()
            };
            if let Ok(page) = list_organizations(filters, 0, 200).await {
                organizations.set(
                    page.items
                        .into_iter()
                        .map(|item| (item.id, item.name))
                        .collect(),
                );
            }
        });
    });

    // Mirror the applied filters into the URL so the view can be linked to.
    // `replace` keeps filter tweaks out of the back-button history.
    let navigate = use_navigate();
    Effect::new(move |previous: Option<()>| {
        let mut parts = Vec::<String>::new();
        let mut push = |key: &str, value: String| {
            if !value.is_empty() {
                parts.push(format!("{key}={}", encode_query_value(&value)));
            }
        };
        push("q", debounced_query.get());
        push("type", contact_type.get());
        push("org", organization_id.get());
        push("cat", filters.get().join(","));
        push("props", property_filters::encode(&property_filters.get()));
        if include_archived.get() {
            parts.push("archived=1".to_string());
        }
        // Skip the first run: it would only rewrite the URL we just read.
        if previous.is_some() {
            let target = if parts.is_empty() {
                "/contacts".to_string()
            } else {
                format!("/contacts?{}", parts.join("&"))
            };
            navigate(
                &target,
                NavigateOptions {
                    replace: true,
                    scroll: false,
                    ..Default::default()
                },
            );
        }
    });

    Effect::new(move |_| {
        let query = debounced_query.get();
        let filters = filters.get();
        let selected_type = ContactType::from_slug(&contact_type.get());
        let selected_organization = organization_id.get();
        let archived = include_archived.get();
        let properties = property_filters.get();
        let page_offset = offset.get();
        reload.track();
        search_generation.update(|generation| *generation += 1);
        let generation = search_generation.get_untracked();
        loading.set(true);
        spawn_local(async move {
            match search_contacts(
                query,
                filters,
                selected_type,
                selected_organization,
                archived,
                properties,
                page_offset,
                50,
            )
            .await
            {
                Ok(page) if search_generation.get_untracked() == generation => {
                    if page_offset == 0 {
                        contacts.set(page.items);
                    } else {
                        contacts.update(|items| items.extend(page.items));
                    }
                    total.set(page.total);
                    error.set(None);
                }
                Err(e) if search_generation.get_untracked() == generation => {
                    error.set(Some(err_text(e)));
                }
                _ => return,
            }
            loading.set(false);
        });
    });

    // A changed chip must start the result list again, not append to it.
    Effect::new(move |previous: Option<Vec<PropertyFilter>>| {
        let current = property_filters.get();
        if previous.is_some_and(|previous| previous != current) {
            offset.set(0);
        }
        current
    });

    let save = move |()| {
        if saving.get_untracked() {
            return;
        }
        let current = draft.get_untracked();
        saving.set(true);
        error.set(None);
        spawn_local(async move {
            match save_contact(current.id, current.input).await {
                Ok(_) => {
                    creating.set(false);
                    draft.set(ContactDraft::default());
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

    let mut on_search = debounce(
        std::time::Duration::from_millis(300),
        move |value: String| {
            offset.set(0);
            debounced_query.set(value);
        },
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
                                offset.set(0);
                                filters.update(|ids| toggle_id(ids, &id, event_target_checked(&event)));
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
            return view! { <p class="p-4 text-sm text-slate-500">"Loading contacts…"</p> }
                .into_any();
        }
        if contacts.get().is_empty() {
            return view! {
                <p class="p-4 text-sm text-slate-500">"No contacts match every selected filter."</p>
            }
            .into_any();
        }
        contacts
            .get()
            .into_iter()
            .map(|contact| {
                let href = format!("/contacts/{}", contact.id);
                let subtitle = [contact.title.clone(), contact.organization.clone()]
                    .into_iter()
                    .filter(|value| !value.is_empty())
                    .collect::<Vec<_>>()
                    .join(" at ");
                let archived = contact.archived;
                let has_account = contact.has_account;
                let type_badges = contact
                    .types
                    .iter()
                    .map(|contact_type| {
                        view! {
                            <span class=format!(
                                "rounded-full px-2 py-0.5 text-[0.68rem] {}",
                                contact_type.badge_classes(),
                            )>
                                {contact_type.label()}
                            </span>
                        }
                    })
                    .collect_view();
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
                let matched = contact.filtered_properties.clone();
                view! {
                    <A
                        href=href
                        attr:class="block rounded-xl border border-slate-800 bg-slate-900 p-4 hover:border-primary-500/40"
                    >
                        <p class="font-medium text-slate-100">{contact.full_name}</p>
                        <div class="mt-1 flex flex-wrap gap-1">
                            {archived.then(|| view! { <span class="rounded-full bg-slate-700/40 px-2 py-0.5 text-[0.68rem] text-slate-300">"Archived"</span> })}
                            {has_account.then(|| view! { <span class="rounded-full bg-primary-500/15 px-2 py-0.5 text-[0.68rem] text-primary-300">"Has account"</span> })}
                            {type_badges}
                        </div>

                        {(!subtitle.is_empty()).then(|| view! {
                            <p class="mt-0.5 text-xs text-slate-400">{subtitle}</p>
                        })}
                        <div class="mt-2 flex flex-wrap gap-1">{tags}</div>
                        <div class="mt-2 flex flex-wrap gap-1">
                            <MatchedProperties properties=matched />
                        </div>
                        <p class="mt-3 text-[0.68rem] text-slate-600">"Updated " {contact.updated_at}</p>
                    </A>
                }
            })
            .collect_view()
            .into_any()
    };

    view! {
        <div class="space-y-5">
            <div class="flex flex-wrap items-start justify-between gap-3">
                <div>
                    <h2 class="text-lg font-semibold text-slate-100">"Contacts"</h2>
                    <p class="mt-1 max-w-3xl text-sm text-slate-400">
                        "Search outreach, referral, fundraising, partnership, and volunteer-recruitment contacts. Open any contact for communications, categories, and structured CRM detail."
                    </p>
                </div>
                <div class="flex flex-wrap gap-2">
                    <Show when=move || can_bulk_edit>
                        <A
                            href="/properties/bulk?subject=contact"
                            attr:class="rounded-lg border border-slate-700 px-4 py-2 text-sm font-semibold text-slate-200 hover:bg-slate-800"
                        >
                            "Bulk edit properties"
                        </A>
                    </Show>
                    <Show when=move || can_send_mail>
                        <A
                            href="/contacts/mail"
                            attr:class="rounded-lg border border-primary-500/50 px-4 py-2 text-sm font-semibold text-primary-300 hover:bg-primary-500/10"
                        >
                            "Send mail"
                        </A>
                    </Show>
                    <button
                        type="button"
                        on:click=move |_| {
                            draft.set(ContactDraft::default());
                            creating.set(true);
                            error.set(None);
                        }
                        class="rounded-lg bg-primary-500 px-4 py-2 text-sm font-semibold text-white hover:bg-primary-600"
                    >
                        "Add contact"
                    </button>
                </div>
            </div>

            <Show when=move || error.get().is_some()>
                <p class="rounded-lg border border-rose-500/40 bg-rose-500/10 px-3 py-2 text-sm text-rose-300">
                    {move || error.get().unwrap_or_default()}
                </p>
            </Show>

            <Show when=move || creating.get()>
                <DirectoryContactEditor
                    draft
                    categories
                    saving
                    on_save=Callback::new(save)
                    on_cancel=Callback::new(move |()| {
                        creating.set(false);
                        draft.set(ContactDraft::default());
                    })
                    show_categories=true
                />
            </Show>

            <details class="rounded-xl border border-slate-800 bg-slate-900">
                <summary class="cursor-pointer px-4 py-3 text-sm font-medium text-slate-300">
                    "Manage categories and tags"
                </summary>
                <p class="border-t border-slate-800 px-4 pt-4 text-xs text-slate-500">
                    "Categories and tags are optional, organization-defined groupings. Required contact types are edited on each contact."
                </p>
                <div class="grid gap-3 border-slate-800 p-4 sm:grid-cols-[1fr_1fr_auto]">
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
                        {move || categories
                            .get()
                            .into_iter()
                            .filter(|category| category.parent_id.is_none())
                            .map(|category| view! { <option value=category.id>{category.name}</option> })
                            .collect_view()}
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
                        placeholder="Search name, organization, email, location…"
                        prop:value=move || query.get()
                        on:input=move |event| {
                            let value = event_target_value(&event);
                            query.set(value.clone());
                            on_search(value);
                        }
                    />
                    <PropertyFilterBar
                        subject=PropertySubject::Contact
                        filters=property_filters
                        scope=facet_scope
                    />
                    <label class="block">
                        <span class=LABEL>"Contact type"</span>
                        <select
                            class=INPUT
                            prop:value=move || contact_type.get()
                            on:change=move |event| {
                                offset.set(0);
                                contact_type.set(event_target_value(&event));
                            }
                        >
                            <option value="">"Any type"</option>
                            {ContactType::ALL.iter().map(|item| view! {
                                <option value=item.slug()>{item.label()}</option>
                            }).collect_view()}
                        </select>
                        <p class="mt-1 text-xs text-slate-500">
                            "Matches contacts that include the selected type."
                        </p>
                    </label>
                    <label class="block">
                        <span class=LABEL>"Organization"</span>
                        <select
                            class=INPUT
                            prop:value=move || organization_id.get()
                            on:change=move |event| {
                                offset.set(0);
                                organization_id.set(event_target_value(&event));
                            }
                        >
                            <option value="">"Any organization"</option>
                            {move || organizations.get().into_iter().map(|(id, name)| view! {
                                <option value=id>{name}</option>
                            }).collect_view()}
                        </select>
                    </label>
                    <label class="flex items-center gap-2 text-sm text-slate-300">
                        <input
                            type="checkbox"
                            class="h-4 w-4 rounded border-slate-700 bg-slate-950"
                            prop:checked=move || include_archived.get()
                            on:change=move |event| {
                                offset.set(0);
                                include_archived.set(event_target_checked(&event));
                            }
                        />
                        "Include archived"
                    </label>
                    <div class="rounded-xl border border-slate-800 bg-slate-900">
                        <div class="flex items-center justify-between border-b border-slate-800 px-4 py-3">
                            <div>
                                <h3 class="text-sm font-semibold">"Filter categories"</h3>
                                <p class="text-[0.68rem] text-slate-500">"Matches all selected"</p>
                            </div>
                            <button
                                type="button"
                                on:click=move |_| {
                                    offset.set(0);
                                    filters.set(Vec::new());
                                }
                                class="text-xs text-primary-300 hover:text-primary-200"
                            >
                                "Clear"
                            </button>
                        </div>
                        <div class="max-h-72 overflow-y-auto p-2">{filter_options}</div>
                    </div>
                    <p class="text-xs text-slate-500">
                        "Organizations have their own directory under "
                        <A href="/organizations" attr:class="text-primary-400 hover:text-primary-300">
                            "Organizations"
                        </A>
                        "."
                    </p>
                </aside>
                <section class="space-y-3">
                    <div class="flex flex-col gap-2 text-xs text-slate-500 sm:flex-row sm:items-center sm:justify-between">
                        <span>
                            "Showing " {move || contacts.get().len()} " of "
                            {move || total.get()} " contacts"
                        </span>
                        <Show when=move || (contacts.get().len() as i64) < total.get()>
                            <button
                                type="button"
                                on:click=move |_| offset.set(contacts.get_untracked().len() as i64)
                                class="rounded-lg border border-slate-700 px-3 py-1.5 text-sm font-medium text-slate-200 hover:bg-slate-800"
                            >
                                "Load more"
                            </button>
                        </Show>
                    </div>
                    <div class="space-y-3">{contact_rows}</div>
                </section>
            </div>
        </div>
    }
}
