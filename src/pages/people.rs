//! The people directory and person detail views — the CRM's front door.
//!
//! These render inside the Admin workspace (see [`crate::pages::admin`]), so
//! they carry no page shell or guard of their own. Reading a contact is allowed
//! for any staff account server-side — the case people-picker needs it — but the
//! directory itself is administrative, which is why it lives under `/admin`.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;

use crate::components::change_log::ChangeLog;
use crate::components::contact_cases::ContactCasesPanel;
use crate::components::contact_form::ContactForm;
use crate::components::contact_properties::ContactPropertiesPanel;
use crate::components::loading::Loading;
use crate::helpers::format::badge_pill;
use crate::server_fns::audit::AuditScope;
use crate::server_fns::contacts::{
    list_contacts, load_contact, set_contact_account, set_contact_archived, unlinked_accounts,
    Contact, ContactFilters, ContactType,
};
use crate::server_fns::err_text;
use crate::server_fns::organizations::{list_organizations, OrganizationFilters};
use crate::state::AppState;

const PAGE_SIZE: i64 = 20;
const PANEL: &str = "rounded-xl border border-slate-800 bg-slate-900 p-5";
const INPUT: &str =
    "w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-600 focus:border-primary-500 focus:outline-none";

/// The People workspace: the directory, or one person when `selected_id` is set.
#[component]
pub fn ManagePeople(selected_id: Option<String>) -> impl IntoView {
    match selected_id {
        Some(id) => view! { <PersonDetail contact_id=id /> }.into_any(),
        None => view! { <PeopleDirectory /> }.into_any(),
    }
}

#[component]
fn PeopleDirectory() -> impl IntoView {
    let keyword = RwSignal::new(String::new());
    let type_filter = RwSignal::new(String::new());
    let org_filter = RwSignal::new(String::new());
    let include_archived = RwSignal::new(false);
    let applied = RwSignal::new(ContactFilters::default());

    let items = RwSignal::new(Vec::<Contact>::new());
    let total = RwSignal::new(0i64);
    let window = RwSignal::new(PAGE_SIZE);
    let loading = RwSignal::new(true);
    let error = RwSignal::new(String::new());
    let reload = RwSignal::new(0u32);
    let creating = RwSignal::new(false);

    let organizations = RwSignal::new(Vec::<(String, String)>::new());
    Effect::new(move |_| {
        spawn_local(async move {
            // A generous limit: the picker lists every organization that can
            // still be chosen, and the directory is small.
            let filters = OrganizationFilters::default();
            if let Ok(page) = list_organizations(filters, 0, 200).await {
                organizations.set(page.items.into_iter().map(|o| (o.id, o.name)).collect());
            }
        });
    });

    Effect::new(move |_| {
        let filters = applied.get();
        let limit = window.get();
        reload.track();
        loading.set(true);
        spawn_local(async move {
            match list_contacts(filters, 0, limit).await {
                Ok(page) => {
                    items.set(page.items);
                    total.set(page.total);
                    error.set(String::new());
                }
                Err(e) => error.set(err_text(e)),
            }
            loading.set(false);
        });
    });

    let apply = move |_| {
        window.set(PAGE_SIZE);
        applied.set(ContactFilters {
            keyword: keyword.get_untracked(),
            contact_type: ContactType::from_slug(&type_filter.get_untracked()),
            organization_id: org_filter.get_untracked(),
            include_archived: include_archived.get_untracked(),
        });
    };
    let clear = move |_| {
        keyword.set(String::new());
        type_filter.set(String::new());
        org_filter.set(String::new());
        include_archived.set(false);
        window.set(PAGE_SIZE);
        applied.set(ContactFilters::default());
    };

    let rows = move || {
        if !error.get().is_empty() {
            return view! {
                <p class="text-sm text-rose-300">"Could not load people: " {error.get()}</p>
            }
            .into_any();
        }
        let list = items.get();
        if list.is_empty() {
            let message = if loading.get() {
                "Loading people\u{2026}"
            } else {
                "No people match these filters."
            };
            return view! { <p class="text-sm text-slate-500">{message}</p> }.into_any();
        }
        list.into_iter()
            .map(|contact| {
                let href = format!("/admin/people/{}", contact.id);
                let name = contact.display_name();
                let types = contact.types.clone();
                let org = contact.organization_name.clone();
                let email = contact.email.clone();
                let phone = contact.phone.clone();
                let archived = contact.archived;
                let has_account = contact.has_account();
                view! {
                    <A href=href attr:class="block rounded-lg border border-slate-800 bg-slate-950 p-3 hover:border-primary-500/40">
                        <div class="flex flex-wrap items-center gap-2">
                            <span class="text-sm font-semibold text-slate-100">{name}</span>
                            <Show when=move || archived>
                                <span class=badge_pill("bg-slate-700/40 text-slate-300 ring-1 ring-slate-600")>"Archived"</span>
                            </Show>
                            <Show when=move || has_account>
                                <span class=badge_pill("bg-primary-500/15 text-primary-300 ring-1 ring-primary-500/30")>"Has account"</span>
                            </Show>
                            {types
                                .iter()
                                .map(|t| view! {
                                    <span class=badge_pill(t.badge_classes())>{t.label()}</span>
                                })
                                .collect_view()}
                        </div>
                        <p class="mt-1 text-xs text-slate-500">
                            {if org.is_empty() { "No organization".to_string() } else { org }}
                            {if email.is_empty() { String::new() } else { format!(" \u{b7} {email}") }}
                            {if phone.is_empty() { String::new() } else { format!(" \u{b7} {phone}") }}
                        </p>
                    </A>
                }
            })
            .collect_view()
            .into_any()
    };

    view! {
        <div class="space-y-6">
            <div class=PANEL>
                <div class="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                    <div>
                        <h2 class="text-lg font-semibold text-slate-100">"Contact directory"</h2>
                        <p class="mt-1 text-sm text-slate-500">
                            "Everyone the foundation knows \u{2014} including people who have no login."
                        </p>
                    </div>
                    <button
                        type="button"
                        on:click=move |_| creating.update(|c| *c = !*c)
                        class="shrink-0 rounded-lg bg-primary-500 px-4 py-2 text-sm font-semibold text-white hover:bg-primary-600"
                    >
                        {move || if creating.get() { "Cancel" } else { "+ New contact" }}
                    </button>
                </div>

                <Show when=move || creating.get()>
                    <div class="mt-4 border-t border-slate-800 pt-4">
                        <ContactForm
                            contact=None
                            organizations=organizations
                            on_saved=Callback::new(move |_id: String| {
                                creating.set(false);
                                reload.update(|r| *r += 1);
                            })
                        />
                    </div>
                </Show>

                <div class="mt-4 grid gap-3 md:grid-cols-2 xl:grid-cols-4">
                    <label class="block">
                        <span class="text-xs font-medium text-slate-400">"Search"</span>
                        <input
                            class=INPUT
                            placeholder="Name, email, phone, organization"
                            prop:value=move || keyword.get()
                            on:input=move |e| keyword.set(event_target_value(&e))
                        />
                    </label>
                    <label class="block">
                        <span class="text-xs font-medium text-slate-400">"Type"</span>
                        <select
                            class=INPUT
                            prop:value=move || type_filter.get()
                            on:change=move |e| type_filter.set(event_target_value(&e))
                        >
                            <option value="">"Any type"</option>
                            {ContactType::ALL
                                .iter()
                                .map(|t| view! { <option value=t.slug()>{t.label()}</option> })
                                .collect_view()}
                        </select>
                    </label>
                    <label class="block">
                        <span class="text-xs font-medium text-slate-400">"Organization"</span>
                        <select
                            class=INPUT
                            prop:value=move || org_filter.get()
                            on:change=move |e| org_filter.set(event_target_value(&e))
                        >
                            <option value="">"Any organization"</option>
                            {move || organizations
                                .get()
                                .into_iter()
                                .map(|(id, name)| view! { <option value=id>{name}</option> })
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
                <div class="mt-3 flex gap-2">
                    <button
                        type="button"
                        on:click=apply
                        class="rounded-lg bg-primary-500 px-3 py-1.5 text-sm font-semibold text-white hover:bg-primary-600"
                    >
                        "Apply filters"
                    </button>
                    <button
                        type="button"
                        on:click=clear
                        class="rounded-lg border border-slate-700 px-3 py-1.5 text-sm font-medium text-slate-300 hover:bg-slate-800"
                    >
                        "Clear"
                    </button>
                </div>

                <div class="mt-4 space-y-2">{rows}</div>

                <div class="mt-4 flex flex-col gap-2 text-xs text-slate-500 sm:flex-row sm:items-center sm:justify-between">
                    <span>"Showing " {move || items.get().len()} " of " {move || total.get()} " people"</span>
                    <Show when=move || (items.get().len() as i64) < total.get()>
                        <button
                            type="button"
                            prop:disabled=move || loading.get()
                            on:click=move |_| window.update(|w| *w += PAGE_SIZE)
                            class="rounded-lg border border-slate-700 px-3 py-1.5 text-sm font-medium text-slate-200 hover:bg-slate-800 disabled:opacity-50"
                        >
                            {move || if loading.get() { "Loading\u{2026}" } else { "Load more" }}
                        </button>
                    </Show>
                </div>
            </div>

            <p class="text-xs text-slate-500">
                "Funders, partner agencies and other bodies live under "
                <A href="/admin/organizations" attr:class="text-primary-400 hover:text-primary-300">"Organizations"</A>
                "."
            </p>
        </div>
    }
}

#[component]
fn PersonDetail(contact_id: String) -> impl IntoView {
    let state = expect_context::<AppState>();
    let is_admin = state
        .current_user_summary
        .get_untracked()
        .is_some_and(|user| user.role.has_operations_admin_permissions());
    let id = StoredValue::new(contact_id);

    let contact = RwSignal::new(None::<Contact>);
    let loading = RwSignal::new(true);
    let error = RwSignal::new(String::new());
    let editing = RwSignal::new(false);
    let reload = RwSignal::new(0u32);
    let organizations = RwSignal::new(Vec::<(String, String)>::new());

    Effect::new(move |_| {
        spawn_local(async move {
            if let Ok(page) = list_organizations(OrganizationFilters::default(), 0, 200).await {
                organizations.set(page.items.into_iter().map(|o| (o.id, o.name)).collect());
            }
        });
    });

    Effect::new(move |_| {
        reload.track();
        loading.set(true);
        spawn_local(async move {
            match load_contact(id.get_value()).await {
                Ok(found) => {
                    contact.set(found);
                    error.set(String::new());
                }
                Err(e) => error.set(err_text(e)),
            }
            loading.set(false);
        });
    });

    let toggle_archive = move |_| {
        let Some(current) = contact.get_untracked() else {
            return;
        };
        let next = !current.archived;
        spawn_local(async move {
            match set_contact_archived(id.get_value(), next).await {
                Ok(()) => reload.update(|r| *r += 1),
                Err(e) => error.set(err_text(e)),
            }
        });
    };

    view! {
        <div class="space-y-6">
            <Show when=move || loading.get() && contact.get().is_none()>
                <div class=PANEL><Loading label="Loading person\u{2026}" /></div>
            </Show>
            <Show when=move || !error.get().is_empty()>
                <p class="text-sm text-rose-300">{move || error.get()}</p>
            </Show>

            {move || {
                let Some(person) = contact.get() else {
                    return ().into_any();
                };
                let name = person.display_name();
                let archived = person.archived;
                let do_not_contact = person.do_not_contact;
                let types = person.types.clone();
                let editable = person.clone();
                let linked = person.clone();
                view! {
                    <div class=PANEL>
                        <div class="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                            <div class="min-w-0">
                                <div class="flex flex-wrap items-center gap-2">
                                    <h2 class="text-lg font-semibold text-slate-100">{name}</h2>
                                    <Show when=move || archived>
                                        <span class=badge_pill("bg-slate-700/40 text-slate-300 ring-1 ring-slate-600")>"Archived"</span>
                                    </Show>
                                    <Show when=move || do_not_contact>
                                        <span class=badge_pill("bg-rose-500/15 text-rose-300 ring-1 ring-rose-500/30")>"Do not contact"</span>
                                    </Show>
                                    {types
                                        .iter()
                                        .map(|t| view! {
                                            <span class=badge_pill(t.badge_classes())>{t.label()}</span>
                                        })
                                        .collect_view()}
                                </div>
                            </div>
                            <div class="flex shrink-0 gap-2">
                                <button
                                    type="button"
                                    on:click=move |_| editing.update(|e| *e = !*e)
                                    class="rounded-lg border border-slate-700 px-3 py-1.5 text-sm font-medium text-slate-200 hover:bg-slate-800"
                                >
                                    {move || if editing.get() { "Cancel" } else { "Edit" }}
                                </button>
                                <Show when=move || is_admin>
                                    <button
                                        type="button"
                                        on:click=toggle_archive
                                        class="rounded-lg border border-slate-700 px-3 py-1.5 text-sm font-medium text-slate-300 hover:bg-slate-800"
                                    >
                                        {if archived { "Restore" } else { "Archive" }}
                                    </button>
                                </Show>
                            </div>
                        </div>

                        <Show
                            when=move || editing.get()
                            fallback=move || view! { <ContactSummary contact=editable.clone() /> }
                        >
                            <div class="mt-4 border-t border-slate-800 pt-4">
                                <ContactForm
                                    contact=contact.get()
                                    organizations=organizations
                                    on_saved=Callback::new(move |_id: String| {
                                        editing.set(false);
                                        reload.update(|r| *r += 1);
                                    })
                                />
                            </div>
                        </Show>
                    </div>

                    <ContactPropertiesPanel contact_id=id.get_value() />

                    <Show when=move || person.has_account_field_conflict>
                        <div class="rounded-xl border border-amber-500/30 bg-amber-500/10 p-4 text-sm text-amber-200">
                            "This linked person has preserved CRM identity values that differ from the account. The account-owned values shown above are authoritative; review the migration conflict record before unlinking."
                        </div>
                    </Show>

                    <ContactCasesPanel contact_id=id.get_value() />

                    <AccountLink
                        contact=linked.clone()
                        can_manage=is_admin
                        on_changed=Callback::new(move |_: ()| reload.update(|r| *r += 1))
                    />

                    <Show when=move || is_admin>
                        <div class=PANEL>
                            <h3 class="text-sm font-semibold text-slate-200">"Change log"</h3>
                            <div class="mt-3">
                                <ChangeLog scope=AuditScope::Contact entity_id=id.get_value() />
                            </div>
                        </div>
                    </Show>
                }
                .into_any()
            }}

            <A href="/admin/people" attr:class="inline-block text-sm text-primary-400 hover:text-primary-300">
                "\u{2190} Back to people"
            </A>
        </div>
    }
}

/// The link between this person and a sign-in account.
///
/// A contact and a user are separate records on purpose (ADR-0005), so this
/// states plainly which account — if any — belongs to this person, and links
/// through to their profile. Linking and unlinking never touch the account
/// itself: unlinking leaves them able to sign in exactly as before.
#[component]
fn AccountLink(contact: Contact, can_manage: bool, on_changed: Callback<()>) -> impl IntoView {
    let contact_id = StoredValue::new(contact.id.clone());
    let has_account = contact.has_account();
    let user_id = contact.user_id.clone();
    let email = contact.linked_email.clone();
    let role = contact.linked_role;
    let profile_href = format!("/profile/{user_id}");

    let choices = RwSignal::new(Vec::<(String, String, String)>::new());
    let chosen = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let picking = RwSignal::new(false);

    // Accounts that have no person record yet, so linking cannot create a
    // duplicate. Only loaded when an admin actually opens the picker.
    Effect::new(move |_| {
        if !picking.get() || !choices.get_untracked().is_empty() {
            return;
        }
        spawn_local(async move {
            match unlinked_accounts().await {
                Ok(list) => {
                    choices.set(list.into_iter().map(|a| (a.id, a.name, a.email)).collect())
                }
                Err(e) => error.set(err_text(e)),
            }
        });
    });

    let apply = move |target: Option<String>| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        error.set(String::new());
        spawn_local(async move {
            match set_contact_account(contact_id.get_value(), target.unwrap_or_default()).await {
                Ok(()) => {
                    picking.set(false);
                    on_changed.run(());
                }
                Err(e) => error.set(err_text(e)),
            }
            busy.set(false);
        });
    };

    view! {
        <div class=PANEL>
            <div class="flex flex-wrap items-start justify-between gap-3">
                <div>
                    <h3 class="text-sm font-semibold text-slate-200">"Sign-in account"</h3>
                    <p class="mt-1 text-xs text-slate-500">
                        "A person and an account are separate records. Most people never need a login."
                    </p>
                </div>
                <Show when=move || can_manage && has_account>
                    <button
                        type="button"
                        prop:disabled=move || busy.get()
                        on:click=move |_| apply(None)
                        class="rounded-lg border border-slate-700 px-3 py-1.5 text-xs font-medium text-slate-300 hover:bg-slate-800 disabled:opacity-50"
                    >
                        "Unlink"
                    </button>
                </Show>
            </div>

            <Show when=move || !error.get().is_empty()>
                <p class="mt-3 text-sm text-rose-300" role="alert">{move || error.get()}</p>
            </Show>

            {if has_account {
                let badge = role
                    .map(|r| view! {
                        <span class=badge_pill(r.badge_classes())>{r.label()}</span>
                    }.into_any())
                    .unwrap_or_else(|| ().into_any());
                view! {
                    <div class="mt-3 rounded-lg border border-slate-800 bg-slate-950 p-3">
                        <div class="flex flex-wrap items-center gap-2">
                            <A
                                href=profile_href.clone()
                                attr:class="text-sm font-medium text-primary-300 hover:text-primary-200"
                            >
                                "Open their profile"
                            </A>
                            {badge}
                        </div>
                        <p class="mt-1 text-xs text-slate-500">
                            "Signs in as " {email.clone()}
                            ". Email and role belong to the account and are changed under Manage users."
                        </p>
                    </div>
                }
                .into_any()
            } else {
                view! {
                    <p class="mt-3 text-sm text-slate-500">
                        "No account. This person cannot sign in."
                    </p>
                }
                .into_any()
            }}

            <Show when=move || can_manage && !has_account>
                <div class="mt-3">
                    <Show
                        when=move || picking.get()
                        fallback=move || view! {
                            <button
                                type="button"
                                on:click=move |_| picking.set(true)
                                class="rounded-lg border border-slate-700 px-3 py-1.5 text-sm font-medium text-slate-200 hover:bg-slate-800"
                            >
                                "Link an existing account"
                            </button>
                        }
                    >
                        <div class="flex flex-wrap gap-2">
                            <select
                                class=INPUT
                                prop:value=move || chosen.get()
                                on:change=move |e| chosen.set(event_target_value(&e))
                            >
                                <option value="">"Choose an account\u{2026}"</option>
                                {move || choices
                                    .get()
                                    .into_iter()
                                    .map(|(id, name, mail)| view! {
                                        <option value=id>{format!("{name} \u{2014} {mail}")}</option>
                                    })
                                    .collect_view()}
                            </select>
                            <button
                                type="button"
                                prop:disabled=move || busy.get()
                                on:click=move |_| {
                                    let id = chosen.get_untracked();
                                    if id.is_empty() {
                                        error.set("Choose an account to link.".into());
                                    } else {
                                        apply(Some(id));
                                    }
                                }
                                class="rounded-lg bg-primary-500 px-3 py-1.5 text-sm font-semibold text-white hover:bg-primary-600 disabled:opacity-50"
                            >
                                "Link"
                            </button>
                            <button
                                type="button"
                                on:click=move |_| picking.set(false)
                                class="rounded-lg border border-slate-700 px-3 py-1.5 text-sm font-medium text-slate-300 hover:bg-slate-800"
                            >
                                "Cancel"
                            </button>
                        </div>
                        <p class="mt-2 text-xs text-slate-500">
                            "Only accounts that have no person record yet are listed."
                        </p>
                    </Show>
                </div>
            </Show>
        </div>
    }
}

/// The read-only view of a contact's fields, including the linked account.
#[component]
fn ContactSummary(contact: Contact) -> impl IntoView {
    let row = |label: &'static str, value: String| {
        let empty = value.trim().is_empty();
        view! {
            <div class="border-b border-slate-800 py-2 last:border-b-0">
                <dt class="text-xs font-medium text-slate-500">{label}</dt>
                <dd class=if empty {
                    "mt-1 text-sm italic text-slate-500"
                } else {
                    "mt-1 whitespace-pre-wrap text-sm text-slate-200"
                }>{if empty { "Not provided".to_string() } else { value }}</dd>
            </div>
        }
    };

    let account = if contact.has_account() {
        let role = contact
            .linked_role
            .map(|r| r.label().to_string())
            .unwrap_or_default();
        format!("{} ({role})", contact.linked_email)
    } else {
        String::new()
    };

    view! {
        <dl class="mt-4 grid gap-x-6 sm:grid-cols-2">
            {row("Full name", format!("{} {}", contact.first_name, contact.last_name).trim().to_string())}
            {row("Preferred name", contact.preferred_name.clone())}
            <div class="border-b border-slate-800 py-2 last:border-b-0">
                <dt class="text-xs font-medium text-slate-500">"Organization"</dt>
                <dd class="mt-1 text-sm text-slate-200">
                    {if contact.organization_id.is_empty() {
                        view! { <span class="italic text-slate-500">"Not provided"</span> }.into_any()
                    } else {
                        view! {
                            <A
                                href=format!("/admin/organizations/{}", contact.organization_id)
                                attr:class="text-primary-300 hover:text-primary-200"
                            >
                                {contact.organization_name.clone()}
                            </A>
                        }
                        .into_any()
                    }}
                </dd>
            </div>
            {row("Job title", contact.job_title.clone())}
            {row("Email", contact.email.clone())}
            {row("Phone", contact.phone.clone())}
            {row("Mobile", contact.mobile.clone())}
            {row("Address", contact.address.clone())}
            {row("Source", contact.source.clone())}
            {row("Sign-in account", account)}
            {row("Notes", contact.description.clone())}
        </dl>
    }
}
