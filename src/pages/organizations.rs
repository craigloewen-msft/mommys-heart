//! The organization directory and detail views.
//!
//! These render inside the Admin workspace, so they carry no page shell or
//! guard of their own. Any staff account may *read* an organization server-side
//! (the contact form files people under one); managing them is administrative.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;

use crate::components::loading::Loading;
use crate::helpers::format::badge_pill;
use crate::server_fns::contacts::{list_contacts, Contact, ContactFilters};
use crate::server_fns::err_text;
use crate::server_fns::organizations::{
    create_organization, list_organizations, load_organization, set_organization_archived,
    update_organization, Organization, OrganizationFilters, OrganizationInput, OrganizationKind,
};
use crate::state::AppState;

const PANEL: &str = "rounded-xl border border-slate-800 bg-slate-900 p-5";
const INPUT: &str =
    "w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-600 focus:border-primary-500 focus:outline-none";
const LABEL: &str = "text-xs font-medium text-slate-400";

/// The Organizations workspace: the directory, or one organization.
#[component]
pub fn ManageOrganizations(selected_id: Option<String>) -> impl IntoView {
    match selected_id {
        Some(id) => view! { <OrganizationDetail organization_id=id /> }.into_any(),
        None => view! { <OrganizationDirectory /> }.into_any(),
    }
}

#[component]
fn OrganizationDirectory() -> impl IntoView {
    let state = expect_context::<AppState>();
    let is_admin = state.has_operations_admin_permissions();

    let keyword = RwSignal::new(String::new());
    let kind_filter = RwSignal::new(String::new());
    let include_archived = RwSignal::new(false);
    let applied = RwSignal::new(OrganizationFilters::default());
    let items = RwSignal::new(Vec::<Organization>::new());
    let loading = RwSignal::new(true);
    let error = RwSignal::new(String::new());
    let reload = RwSignal::new(0u32);
    let creating = RwSignal::new(false);

    Effect::new(move |_| {
        let filters = applied.get();
        reload.track();
        loading.set(true);
        spawn_local(async move {
            match list_organizations(filters, 0, 200).await {
                Ok(page) => {
                    items.set(page.items);
                    error.set(String::new());
                }
                Err(e) => error.set(err_text(e)),
            }
            loading.set(false);
        });
    });

    let apply = move |_| {
        applied.set(OrganizationFilters {
            keyword: keyword.get_untracked(),
            kind: OrganizationKind::from_slug(&kind_filter.get_untracked()),
            include_archived: include_archived.get_untracked(),
        });
    };

    let rows = move || {
        if !error.get().is_empty() {
            return view! { <p class="text-sm text-rose-300">{error.get()}</p> }.into_any();
        }
        let list = items.get();
        if list.is_empty() {
            let message = if loading.get() {
                "Loading organizations\u{2026}"
            } else {
                "No organizations match these filters."
            };
            return view! { <p class="text-sm text-slate-500">{message}</p> }.into_any();
        }
        list.into_iter()
            .map(|org| {
                let href = format!("/admin/organizations/{}", org.id);
                let kind = org.kind;
                let archived = org.archived;
                let count = org.contact_count;
                view! {
                    <A href=href attr:class="block rounded-lg border border-slate-800 bg-slate-950 p-3 hover:border-primary-500/40">
                        <div class="flex flex-wrap items-center gap-2">
                            <span class="text-sm font-semibold text-slate-100">{org.name.clone()}</span>
                            <span class=badge_pill(kind.badge_classes())>{kind.label()}</span>
                            <Show when=move || archived>
                                <span class=badge_pill("bg-slate-700/40 text-slate-300 ring-1 ring-slate-600")>"Archived"</span>
                            </Show>
                        </div>
                        <p class="mt-1 text-xs text-slate-500">
                            {count} {if count == 1 { " contact" } else { " contacts" }}
                            {if org.email.is_empty() { String::new() } else { format!(" \u{b7} {}", org.email) }}
                        </p>
                    </A>
                }
            })
            .collect_view()
            .into_any()
    };

    view! {
        <div class=PANEL>
            <div class="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                <div>
                    <h2 class="text-lg font-semibold text-slate-100">"Organizations"</h2>
                    <p class="mt-1 text-sm text-slate-500">
                        "Funders, partner agencies, providers, courts, and employers."
                    </p>
                </div>
                <Show when=move || is_admin>
                    <button
                        type="button"
                        on:click=move |_| creating.update(|c| *c = !*c)
                        class="shrink-0 rounded-lg bg-primary-500 px-4 py-2 text-sm font-semibold text-white hover:bg-primary-600"
                    >
                        {move || if creating.get() { "Cancel" } else { "+ New organization" }}
                    </button>
                </Show>
            </div>

            <Show when=move || creating.get()>
                <div class="mt-4 border-t border-slate-800 pt-4">
                    <OrganizationForm
                        organization=None
                        on_saved=Callback::new(move |_id: String| {
                            creating.set(false);
                            reload.update(|r| *r += 1);
                        })
                    />
                </div>
            </Show>

            <div class="mt-4 grid gap-3 sm:grid-cols-3">
                <label class="block">
                    <span class=LABEL>"Search"</span>
                    <input
                        class=INPUT
                        placeholder="Name or email"
                        prop:value=move || keyword.get()
                        on:input=move |e| keyword.set(event_target_value(&e))
                    />
                </label>
                <label class="block">
                    <span class=LABEL>"Type"</span>
                    <select
                        class=INPUT
                        prop:value=move || kind_filter.get()
                        on:change=move |e| kind_filter.set(event_target_value(&e))
                    >
                        <option value="">"Any type"</option>
                        {OrganizationKind::ALL
                            .iter()
                            .map(|k| view! { <option value=k.slug()>{k.label()}</option> })
                            .collect_view()}
                    </select>
                </label>
                <label class="flex items-end gap-2 pb-2 text-sm text-slate-300">
                    <input
                        type="checkbox"
                        class="h-4 w-4 rounded border-slate-700 bg-slate-950"
                        prop:checked=move || include_archived.get()
                        on:change=move |e| include_archived.set(event_target_checked(&e))
                    />
                    "Include archived"
                </label>
            </div>
            <button
                type="button"
                on:click=apply
                class="mt-3 rounded-lg bg-primary-500 px-3 py-1.5 text-sm font-semibold text-white hover:bg-primary-600"
            >
                "Apply filters"
            </button>

            <div class="mt-4 space-y-2">{rows}</div>
        </div>
    }
}

#[component]
fn OrganizationDetail(organization_id: String) -> impl IntoView {
    let state = expect_context::<AppState>();
    let is_admin = state.has_operations_admin_permissions();
    let id = StoredValue::new(organization_id);

    let organization = RwSignal::new(None::<Organization>);
    let contacts = RwSignal::new(Vec::<Contact>::new());
    let loading = RwSignal::new(true);
    let error = RwSignal::new(String::new());
    let editing = RwSignal::new(false);
    let reload = RwSignal::new(0u32);

    Effect::new(move |_| {
        reload.track();
        loading.set(true);
        spawn_local(async move {
            match load_organization(id.get_value()).await {
                Ok(found) => {
                    organization.set(found);
                    error.set(String::new());
                }
                Err(e) => error.set(err_text(e)),
            }
            let filters = ContactFilters {
                organization_id: id.get_value(),
                ..Default::default()
            };
            if let Ok(page) = list_contacts(filters, 0, 100).await {
                contacts.set(page.items);
            }
            loading.set(false);
        });
    });

    let toggle_archive = move |_| {
        let Some(current) = organization.get_untracked() else {
            return;
        };
        let next = !current.archived;
        spawn_local(async move {
            match set_organization_archived(id.get_value(), next).await {
                Ok(()) => reload.update(|r| *r += 1),
                Err(e) => error.set(err_text(e)),
            }
        });
    };

    view! {
        <div class="space-y-6">
            <Show when=move || loading.get() && organization.get().is_none()>
                <div class=PANEL><Loading label="Loading organization\u{2026}" /></div>
            </Show>
            <Show when=move || !error.get().is_empty()>
                <p class="text-sm text-rose-300">{move || error.get()}</p>
            </Show>

            {move || {
                let Some(org) = organization.get() else {
                    return ().into_any();
                };
                let kind = org.kind;
                let archived = org.archived;
                let detail = org.clone();
                view! {
                    <div class=PANEL>
                        <div class="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                            <div class="flex flex-wrap items-center gap-2">
                                <h2 class="text-lg font-semibold text-slate-100">{org.name.clone()}</h2>
                                <span class=badge_pill(kind.badge_classes())>{kind.label()}</span>
                                <Show when=move || archived>
                                    <span class=badge_pill("bg-slate-700/40 text-slate-300 ring-1 ring-slate-600")>"Archived"</span>
                                </Show>
                            </div>
                            <Show when=move || is_admin>
                                <div class="flex shrink-0 gap-2">
                                    <button
                                        type="button"
                                        on:click=move |_| editing.update(|e| *e = !*e)
                                        class="rounded-lg border border-slate-700 px-3 py-1.5 text-sm font-medium text-slate-200 hover:bg-slate-800"
                                    >
                                        {move || if editing.get() { "Cancel" } else { "Edit" }}
                                    </button>
                                    <button
                                        type="button"
                                        on:click=toggle_archive
                                        class="rounded-lg border border-slate-700 px-3 py-1.5 text-sm font-medium text-slate-300 hover:bg-slate-800"
                                    >
                                        {if archived { "Restore" } else { "Archive" }}
                                    </button>
                                </div>
                            </Show>
                        </div>

                        <Show
                            when=move || editing.get()
                            fallback=move || {
                                let o = detail.clone();
                                view! {
                                    <dl class="mt-4 grid gap-x-6 sm:grid-cols-2">
                                        <OrgRow label="Website" value=o.website.clone() />
                                        <OrgRow label="Email" value=o.email.clone() />
                                        <OrgRow label="Phone" value=o.phone.clone() />
                                        <OrgRow label="Address" value=o.address.clone() />
                                        <OrgRow label="Notes" value=o.description.clone() />
                                    </dl>
                                }
                            }
                        >
                            <div class="mt-4 border-t border-slate-800 pt-4">
                                <OrganizationForm
                                    organization=organization.get()
                                    on_saved=Callback::new(move |_id: String| {
                                        editing.set(false);
                                        reload.update(|r| *r += 1);
                                    })
                                />
                            </div>
                        </Show>
                    </div>
                }
                .into_any()
            }}

            <div class=PANEL>
                <h3 class="text-sm font-semibold text-slate-200">"People here"</h3>
                <div class="mt-3 space-y-2">
                    {move || {
                        let list = contacts.get();
                        if list.is_empty() {
                            return view! {
                                <p class="text-sm text-slate-500">"No contacts are filed under this organization."</p>
                            }
                            .into_any();
                        }
                        list.into_iter()
                            .map(|c| {
                                let href = format!("/admin/people/{}", c.id);
                                let title = c.job_title.clone();
                                let has_title = !title.is_empty();
                                view! {
                                    <A href=href attr:class="block rounded-lg border border-slate-800 bg-slate-950 p-3 hover:border-primary-500/40">
                                        <span class="text-sm font-medium text-slate-100">{c.display_name()}</span>
                                        <Show when=move || has_title>
                                            <span class="ml-2 text-xs text-slate-500">{title.clone()}</span>
                                        </Show>
                                    </A>
                                }
                            })
                            .collect_view()
                            .into_any()
                    }}
                </div>
            </div>

            <A href="/admin/organizations" attr:class="inline-block text-sm text-primary-400 hover:text-primary-300">
                "\u{2190} Back to organizations"
            </A>
        </div>
    }
}

#[component]
fn OrgRow(label: &'static str, value: String) -> impl IntoView {
    let empty = value.trim().is_empty();
    view! {
        <div class="border-b border-slate-800 py-2">
            <dt class="text-xs font-medium text-slate-500">{label}</dt>
            <dd class=if empty {
                "mt-1 text-sm italic text-slate-500"
            } else {
                "mt-1 whitespace-pre-wrap text-sm text-slate-200"
            }>{if empty { "Not provided".to_string() } else { value }}</dd>
        </div>
    }
}

#[component]
fn OrganizationForm(
    organization: Option<Organization>,
    on_saved: Callback<String>,
) -> impl IntoView {
    let existing_id = organization.as_ref().map(|o| o.id.clone());
    let editing = StoredValue::new(existing_id.clone());
    let seed = organization.unwrap_or_default();

    let name = RwSignal::new(seed.name.clone());
    let kind = RwSignal::new(seed.kind.slug().to_string());
    let website = RwSignal::new(seed.website.clone());
    let phone = RwSignal::new(seed.phone.clone());
    let email = RwSignal::new(seed.email.clone());
    let address = RwSignal::new(seed.address.clone());
    let description = RwSignal::new(seed.description.clone());
    let error = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

    let submit = move |_| {
        if busy.get_untracked() {
            return;
        }
        let input = OrganizationInput {
            name: name.get_untracked(),
            kind: OrganizationKind::from_slug(&kind.get_untracked()).unwrap_or_default(),
            website: website.get_untracked(),
            phone: phone.get_untracked(),
            email: email.get_untracked(),
            address: address.get_untracked(),
            description: description.get_untracked(),
        };
        if let Err(message) = input.validate() {
            error.set(message);
            return;
        }
        busy.set(true);
        error.set(String::new());
        spawn_local(async move {
            let result = match editing.get_value() {
                Some(id) => update_organization(id.clone(), input).await.map(|()| id),
                None => create_organization(input).await,
            };
            match result {
                Ok(id) => on_saved.run(id),
                Err(e) => error.set(err_text(e)),
            }
            busy.set(false);
        });
    };

    let field = move |label: &'static str, signal: RwSignal<String>| {
        view! {
            <label class="block">
                <span class=LABEL>{label}</span>
                <input
                    class=INPUT
                    prop:value=move || signal.get()
                    on:input=move |e| signal.set(event_target_value(&e))
                />
            </label>
        }
    };

    view! {
        <div class="space-y-4">
            <div class="grid gap-3 sm:grid-cols-2">
                {field("Name", name)}
                <label class="block">
                    <span class=LABEL>"Type"</span>
                    <select
                        class=INPUT
                        prop:value=move || kind.get()
                        on:change=move |e| kind.set(event_target_value(&e))
                    >
                        {OrganizationKind::ALL
                            .iter()
                            .map(|k| view! { <option value=k.slug()>{k.label()}</option> })
                            .collect_view()}
                    </select>
                </label>
                {field("Website", website)}
                {field("Email", email)}
                {field("Phone", phone)}
                {field("Address", address)}
            </div>
            <label class="block">
                <span class=LABEL>"Notes"</span>
                <textarea
                    class=INPUT
                    rows="3"
                    prop:value=move || description.get()
                    on:input=move |e| description.set(event_target_value(&e))
                />
            </label>
            <Show when=move || !error.get().is_empty()>
                <p class="text-sm text-rose-300" role="alert">{move || error.get()}</p>
            </Show>
            <button
                type="button"
                on:click=submit
                prop:disabled=move || busy.get()
                class="rounded-lg bg-primary-500 px-4 py-2 text-sm font-semibold text-white hover:bg-primary-600 disabled:opacity-50"
            >
                {move || {
                    if busy.get() {
                        "Saving\u{2026}"
                    } else if existing_id.is_some() {
                        "Save changes"
                    } else {
                        "Create organization"
                    }
                }}
            </button>
        </div>
    }
}
