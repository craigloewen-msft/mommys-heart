//! The profile experience: `/profile` for your own profile and `/profile/:id`
//! for someone you work a case with.
//!
//! Your own profile is editable in place (name, phone, home address); someone
//! else's is read-only. A colleague sees only your name and role — the contact
//! details are withheld by the server unless the viewer is you or an
//! administrator, so this page simply renders whichever fields it is given.
//! Whether a profile may be opened at all is likewise decided server-side by
//! [`load_profile`]; the page shows the server's explanation when access is
//! refused.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use crate::components::guard::require_login;
use crate::components::layout::Layout;
use crate::components::loading::Loading;
use crate::components::volunteer_hours::VolunteerHoursPanel;
use crate::helpers::volunteer_terms::{VOLUNTEER_AGREEMENT_SECTIONS, VOLUNTEER_AGREEMENT_VERSION};
use crate::server_fns::err_text;
use crate::server_fns::profile::{load_profile, save_my_profile, ProfileEdit, UserProfile};
use crate::server_fns::volunteers::VolunteerStatus;
use crate::state::AppState;

const INPUT_CLASS: &str = "w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/40";
const LABEL_CLASS: &str = "text-xs font-medium text-slate-400";
const SECTION_CLASS: &str = "rounded-xl border border-slate-800 bg-slate-900 p-4";

/// One read-only "label + value" row, with a muted placeholder when the user
/// has not filled the field in.
#[component]
fn DetailRow(label: &'static str, #[prop(into)] value: String) -> impl IntoView {
    let empty = value.trim().is_empty();
    let text = if empty {
        "Not provided".to_string()
    } else {
        value
    };
    view! {
        <div class="flex flex-col gap-0.5 border-b border-slate-800 py-2 last:border-b-0 sm:flex-row sm:items-baseline sm:justify-between sm:gap-4">
            <span class=LABEL_CLASS>{label}</span>
            <span class=move || {
                if empty { "text-sm text-slate-500 italic" } else { "text-sm text-slate-200" }
            }>{text}</span>
        </div>
    }
}

#[component]
pub fn ProfilePage() -> impl IntoView {
    let state = expect_context::<AppState>();
    let params = use_params_map();
    let current_user_id =
        Memo::new(move |_| state.current_user_summary.get().map(|current| current.id));

    // `/profile` shows the signed-in user; `/profile/:id` shows that user.
    let target_id = move || {
        params
            .read()
            .get("id")
            .filter(|id| !id.trim().is_empty())
            .or_else(|| current_user_id.get())
            .unwrap_or_default()
    };

    let profile = RwSignal::new(None::<UserProfile>);
    let load_error = RwSignal::new(None::<String>);

    // Edit mode (own profile only).
    let editing = RwSignal::new(false);
    let draft = RwSignal::new(ProfileEdit::default());
    let save_error = RwSignal::new(None::<String>);
    let saving = RwSignal::new(false);
    let saved = RwSignal::new(false);

    Effect::new(move |_| {
        let id = target_id();
        if current_user_id.get().is_none() || id.is_empty() {
            return;
        }
        profile.set(None);
        load_error.set(None);
        editing.set(false);
        saved.set(false);
        spawn_local(async move {
            match load_profile(id).await {
                Ok(p) => {
                    profile.set(Some(p));
                    load_error.set(None);
                }
                Err(e) => load_error.set(Some(err_text(e))),
            }
        });
    });

    require_login(state, move || {
        let begin_edit = move |_| {
            if let Some(p) = profile.get_untracked() {
                draft.set(p.to_edit());
                save_error.set(None);
                saved.set(false);
                editing.set(true);
            }
        };

        let cancel_edit = move |_| {
            save_error.set(None);
            editing.set(false);
        };

        let save = move |_| {
            if saving.get_untracked() {
                return;
            }
            let edit = draft.get_untracked();
            saving.set(true);
            save_error.set(None);
            spawn_local(async move {
                match save_my_profile(edit).await {
                    Ok(saved_edit) => {
                        // Keep the navbar (and anything else reading the session
                        // user) in step with the freshly saved name.
                        state.current_user_summary.update(|current| {
                            if let Some(u) = current {
                                u.first_name = saved_edit.first_name.clone();
                                u.last_name = saved_edit.last_name.clone();
                            }
                        });
                        profile.update(|current| {
                            if let Some(profile) = current {
                                profile.first_name = saved_edit.first_name.clone();
                                profile.last_name = saved_edit.last_name.clone();
                                if let Some(contact) = &mut profile.contact {
                                    contact.phone = saved_edit.phone.clone();
                                    contact.home_address = saved_edit.home_address.clone();
                                }
                            }
                        });
                        editing.set(false);
                        saved.set(true);
                    }
                    Err(e) => save_error.set(Some(err_text(e))),
                }
                saving.set(false);
            });
        };

        let header = move || {
            let Some(p) = profile.get() else {
                return ().into_any();
            };
            let self_badge = if p.is_self {
                view! {
                    <span class="rounded-full bg-slate-800 px-2 py-0.5 text-xs font-medium text-slate-300">
                        "This is you"
                    </span>
                }
                .into_any()
            } else {
                ().into_any()
            };
            let is_self = p.is_self;
            let edit_btn = move || {
                if !is_self || editing.get() {
                    return ().into_any();
                }
                view! {
                    <button
                        on:click=begin_edit
                        class="shrink-0 rounded-lg border border-slate-700 px-3 py-1.5 text-sm font-medium text-slate-200 hover:bg-slate-800"
                    >
                        "Edit profile"
                    </button>
                }
                .into_any()
            };
            let role_badge = format!(
                "rounded-full px-2 py-0.5 text-xs font-medium {}",
                p.role.badge_classes(),
            );
            view! {
                <div class=SECTION_CLASS>
                    <div class="flex items-start justify-between gap-3">
                        <div class="flex min-w-0 items-center gap-3">
                            <span class="grid h-12 w-12 shrink-0 place-items-center rounded-full bg-primary-500/20 text-lg font-semibold text-primary-300">
                                {p.initials()}
                            </span>
                            <div class="min-w-0">
                                <h2 class="truncate text-lg font-semibold">{p.full_name()}</h2>
                                <div class="mt-1 flex flex-wrap items-center gap-2">
                                    <span class=role_badge>{p.role.label()}</span>
                                    {self_badge}
                                </div>
                            </div>
                        </div>
                        {edit_btn}
                    </div>
                </div>
            }
            .into_any()
        };

        let details = move || {
            let Some(p) = profile.get() else {
                return ().into_any();
            };
            // Contact details arrive only for the profile's owner and viewers
            // with operations-admin permissions.
            let Some(contact) = p.contact.clone() else {
                return view! {
                    <div class=SECTION_CLASS>
                        <h3 class="text-sm font-semibold text-slate-200">"Contact details"</h3>
                        <p class="mt-2 text-sm text-slate-500 italic">
                            "Contact details are only visible to " {p.full_name()}
                            " and to users with operations-admin permissions."
                        </p>
                    </div>
                }
                .into_any();
            };
            let scope_note = if p.is_self {
                "Only you and users with operations-admin permissions can see these details."
            } else {
                "Visible to you because you have operations-admin permissions."
            };
            view! {
                <div class=SECTION_CLASS>
                    <h3 class="text-sm font-semibold text-slate-200">"Contact details"</h3>
                    <p class="mt-1 text-xs text-slate-500">{scope_note}</p>
                    <div class="mt-2">
                        <DetailRow label="Email" value=contact.email />
                        <DetailRow label="Phone" value=contact.phone />
                        <DetailRow label="Home address" value=contact.home_address />
                    </div>
                </div>
            }
            .into_any()
        };

        let shared = move || {
            let Some(p) = profile.get() else {
                return ().into_any();
            };
            if p.is_self || !p.shares_case {
                return ().into_any();
            }
            view! {
                <p class="text-xs text-slate-500">
                    "You can see this profile because you work a case with " {p.full_name()} "."
                </p>
            }
            .into_any()
        };

        let volunteer_hours = move || {
            let Some(p) = profile.get() else {
                return ().into_any();
            };
            let display_name = p.full_name();
            let Some(hours) = p.volunteer_hours else {
                return ().into_any();
            };
            view! {
                <VolunteerHoursPanel
                    initial_hours=hours
                    is_self=p.is_self
                    display_name=display_name
                />
            }
            .into_any()
        };

        // Own profile only: the way in to becoming a volunteer. Hidden once the
        // account already has volunteer access, and replaced by a status note
        // while an application is being reviewed.
        let become_volunteer = move || {
            let Some(p) = profile.get() else {
                return ().into_any();
            };
            if !p.is_self || p.role.has_volunteer_privileges() {
                return ().into_any();
            }
            if p.volunteer
                .as_ref()
                .is_some_and(|v| v.status == VolunteerStatus::Pending)
            {
                return view! {
                    <div class=SECTION_CLASS>
                        <h3 class="text-sm font-semibold text-slate-200">"Volunteer application"</h3>
                        <p class="mt-2 text-sm text-slate-400">
                            "Your volunteer application is being reviewed. We'll email you when there's a decision."
                        </p>
                    </div>
                }
                .into_any();
            }
            // A previously declined applicant sees the invitation again: they are
            // free to accept the agreement and apply a second time.
            view! {
                <div class=SECTION_CLASS>
                    <h3 class="text-sm font-semibold text-slate-200">"Become a volunteer"</h3>
                    <p class="mt-2 text-sm text-slate-400">
                        "Volunteers work directly with the families the Foundation supports. Read the volunteer agreement and accept it to apply \u{2014} an administrator reviews every application."
                    </p>
                    <A
                        href="/volunteer-agreement"
                        attr:class="mt-3 inline-block rounded-lg bg-primary-500 px-3 py-1.5 text-sm font-semibold text-white hover:bg-primary-600"
                    >
                        "Read the volunteer agreement"
                    </A>
                </div>
            }
            .into_any()
        };

        // The volunteer agreement on file, for the owner and for admins. The
        // wording is only rendered when the accepted version matches this build:
        // showing the current text for an older acceptance would misrepresent
        // what the person actually agreed to.
        let agreement_open = RwSignal::new(false);
        let volunteer_agreement = move || {
            let Some(p) = profile.get() else {
                return ().into_any();
            };
            let Some(volunteer) = p.volunteer.clone() else {
                return ().into_any();
            };
            if !volunteer.has_agreement() {
                return ().into_any();
            }
            let status_badge = format!(
                "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {}",
                volunteer.status.badge_classes(),
            );
            let is_current = volunteer.agreement_version == VOLUNTEER_AGREEMENT_VERSION;
            let version = volunteer.agreement_version.clone();
            let body = move || {
                if !agreement_open.get() {
                    return ().into_any();
                }
                if !is_current {
                    return view! {
                        <p class="mt-3 text-sm text-slate-400 italic">
                            "This person accepted version " {version.clone()}
                            ", which is not the wording this version of the app carries. The text they agreed to is not shown rather than showing them wording they never saw."
                        </p>
                    }
                    .into_any();
                }
                view! {
                    <div class="mt-3 max-h-[28rem] space-y-6 overflow-y-auto rounded-lg border border-slate-800 bg-slate-950 p-5 text-sm leading-relaxed text-slate-300">
                        {VOLUNTEER_AGREEMENT_SECTIONS
                            .iter()
                            .map(|section| {
                                view! {
                                    <div class="space-y-3">
                                        <Show when=move || !section.heading.is_empty()>
                                            <h4 class="text-base font-semibold text-slate-100">
                                                {section.heading}
                                            </h4>
                                        </Show>
                                        {section
                                            .paragraphs
                                            .iter()
                                            .map(|paragraph| view! { <p>{*paragraph}</p> })
                                            .collect_view()}
                                    </div>
                                }
                            })
                            .collect_view()}
                    </div>
                }
                .into_any()
            };
            let decided = (!volunteer.decided_at.is_empty()).then(|| {
                let who = if volunteer.decided_by_name.is_empty() {
                    String::new()
                } else {
                    format!(" by {}", volunteer.decided_by_name)
                };
                format!("{}{}", volunteer.decided_at, who)
            });
            view! {
                <div class=SECTION_CLASS>
                    <div class="flex flex-wrap items-center justify-between gap-2">
                        <h3 class="text-sm font-semibold text-slate-200">"Volunteer agreement"</h3>
                        <div class="flex items-center gap-2">
                            <span class=status_badge>{volunteer.status.label()}</span>
                            <button
                                on:click=move |_| agreement_open.update(|open| *open = !*open)
                                class="rounded-lg border border-slate-700 px-2 py-1 text-xs font-medium text-slate-300 hover:bg-slate-800"
                            >
                                {move || if agreement_open.get() { "Hide" } else { "View agreement" }}
                            </button>
                        </div>
                    </div>
                    <div class="mt-2">
                        <DetailRow label="Accepted" value=volunteer.agreed_at.clone() />
                        <DetailRow label="Version" value=volunteer.agreement_version.clone() />
                        {decided.map(|decided| view! {
                            <DetailRow label="Decided" value=decided />
                        })}
                        {(!volunteer.decision_note.is_empty()).then(|| view! {
                            <DetailRow label="Decision note" value=volunteer.decision_note.clone() />
                        })}
                    </div>
                    {body}
                </div>
            }
            .into_any()
        };

        let edit_form = move || {
            if !editing.get() {
                return ().into_any();
            }
            view! {
                <div class=SECTION_CLASS>
                    <div class="flex items-center justify-between gap-3">
                        <h3 class="text-sm font-semibold text-slate-200">"Edit your profile"</h3>
                        <div class="flex items-center gap-2">
                            <button
                                on:click=save
                                prop:disabled=move || saving.get()
                                class="shrink-0 rounded-lg bg-primary-500 px-3 py-1.5 text-sm font-semibold text-white hover:bg-primary-600 disabled:opacity-60"
                            >
                                {move || if saving.get() { "Saving\u{2026}" } else { "Save" }}
                            </button>
                            <button
                                on:click=cancel_edit
                                class="shrink-0 rounded-lg border border-slate-700 px-3 py-1.5 text-sm font-medium text-slate-300 hover:bg-slate-800"
                            >
                                "Cancel"
                            </button>
                        </div>
                    </div>
                    <Show when=move || save_error.get().is_some()>
                        <p class="mt-3 rounded-lg border border-rose-500/40 bg-rose-500/10 px-3 py-2 text-sm text-rose-300">
                            {move || save_error.get().unwrap_or_default()}
                        </p>
                    </Show>
                    <div class="mt-4 space-y-4">
                        <div class="grid gap-4 sm:grid-cols-2">
                            <div>
                                <label class=LABEL_CLASS>"First name"</label>
                                <input
                                    class=INPUT_CLASS
                                    prop:value=move || draft.get().first_name
                                    on:input=move |ev| {
                                        let v = event_target_value(&ev);
                                        draft.update(|d| d.first_name = v);
                                    }
                                />
                            </div>
                            <div>
                                <label class=LABEL_CLASS>"Last name"</label>
                                <input
                                    class=INPUT_CLASS
                                    prop:value=move || draft.get().last_name
                                    on:input=move |ev| {
                                        let v = event_target_value(&ev);
                                        draft.update(|d| d.last_name = v);
                                    }
                                />
                            </div>
                        </div>
                        <div class="grid gap-4 sm:grid-cols-2">
                            <div>
                                <label class=LABEL_CLASS>"Phone"</label>
                                <input
                                    class=INPUT_CLASS
                                    placeholder="(555) 123-4567"
                                    prop:value=move || draft.get().phone
                                    on:input=move |ev| {
                                        let v = event_target_value(&ev);
                                        draft.update(|d| d.phone = v);
                                    }
                                />
                            </div>
                            <div>
                                <label class=LABEL_CLASS>"Home address"</label>
                                <input
                                    class=INPUT_CLASS
                                    placeholder="Street, city, state"
                                    prop:value=move || draft.get().home_address
                                    on:input=move |ev| {
                                        let v = event_target_value(&ev);
                                        draft.update(|d| d.home_address = v);
                                    }
                                />
                            </div>
                        </div>
                        <p class="text-xs text-slate-500">
                            "Your phone number and home address are only shown to you and to users with operations-admin permissions. Your email address and account role are managed by a site administrator."
                        </p>
                    </div>
                </div>
            }
            .into_any()
        };

        let body = move || {
            if let Some(msg) = load_error.get() {
                return view! {
                    <div class=SECTION_CLASS>
                        <p class="text-sm text-rose-300">{msg}</p>
                        <A
                            href="/cases"
                            attr:class="mt-3 inline-block rounded-lg border border-slate-700 px-3 py-1.5 text-sm font-medium text-slate-200 hover:bg-slate-800"
                        >
                            "Back to cases"
                        </A>
                    </div>
                }
                .into_any();
            }
            if profile.get().is_none() {
                return view! { <Loading label="Loading profile\u{2026}" /> }.into_any();
            }
            view! {
                <div class="space-y-6">
                    {header}
                    <Show when=move || saved.get()>
                        <p class="text-sm text-emerald-300">"Your profile has been saved."</p>
                    </Show>
                    {edit_form}
                    <Show when=move || !editing.get()>{details.clone()}</Show>
                    {volunteer_hours}
                    {become_volunteer}
                    {volunteer_agreement}
                    {shared}
                </div>
            }
            .into_any()
        };

        view! {
            <Layout title="Profile".to_string()>
                <div class="max-w-2xl">{body}</div>
            </Layout>
        }
        .into_any()
    })
}
