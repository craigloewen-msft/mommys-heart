//! `/volunteer-agreement`: read the Volunteer Agreement, give the details that
//! go with it, and accept, which files an application for review.
//!
//! Mirrors the client terms at [`crate::pages::case_signup`], including the
//! scroll-to-the-end gate. The information fields stay disabled until the
//! agreement is read to the end and the box is ticked.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;

use crate::components::guard::require_login;
use crate::components::layout::Layout;
use crate::helpers::dates;
use crate::helpers::volunteer_details::{VolunteerDetails, BACKGROUND_CHECK_CONSENT};
use crate::helpers::volunteer_terms::{
    CONSENT_CHECKBOX_LABEL, ELECTRONIC_CONSENT, ELECTRONIC_CONSENT_HEADING, ELECTRONIC_EXECUTION,
    ELECTRONIC_EXECUTION_HEADING, FOUNDATION_SIGNATORY, FOUNDATION_SIGNATORY_TITLE,
    VOLUNTEER_AGREEMENT_SECTIONS, VOLUNTEER_AGREEMENT_VERSION, VOLUNTEER_ATTESTATION,
};
use crate::server_fns::err_text;
use crate::server_fns::profile::load_profile;
use crate::server_fns::volunteers::apply_to_volunteer;
use crate::state::AppState;

const INPUT_CLASS: &str = "mt-1 w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2.5 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/30 disabled:cursor-not-allowed disabled:opacity-50";
const LABEL_CLASS: &str = "block text-sm font-medium text-slate-300";
const HEADING_CLASS: &str = "mt-7 text-sm font-semibold text-slate-200";

/// The red asterisk marking a required field.
#[component]
fn Required() -> impl IntoView {
    view! { <span class="text-rose-400" aria-hidden="true">"*"</span> }
}

#[component]
pub fn VolunteerAgreementPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    let navigate = use_navigate();
    let accepted = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    let submitting = RwSignal::new(false);
    // The acceptance control stays locked until the agreement has been scrolled
    // to the bottom.
    let read_to_end = RwSignal::new(false);
    let agreement_ref = NodeRef::<leptos::html::Div>::new();

    let details = RwSignal::new(VolunteerDetails::default());
    // Shown read-only: the account's email is the sign-in identity.
    let email = RwSignal::new(String::new());

    // Whether the pane is scrolled to (or within a pixel of) its end. Also true
    // when the content is short enough not to scroll at all: a pane with no
    // scrollbar can never fire a scroll event, and would otherwise lock the
    // reader out of a form they have already read in full.
    let check_scrolled = move || {
        if let Some(pane) = agreement_ref.get_untracked() {
            let scrolled = pane.scroll_top() as f64 + pane.client_height() as f64;
            if scrolled >= pane.scroll_height() as f64 - 1.0 {
                read_to_end.set(true);
            }
        }
    };

    // Runs once after mount to catch the no-scrollbar case described above.
    Effect::new(move |_| check_scrolled());

    // Prefill contact from the account, so the volunteer confirms what we hold
    // rather than retyping it.
    Effect::new(move |_| {
        let Some(current) = state.current_user_summary.get() else {
            return;
        };
        spawn_local(async move {
            if let Ok(profile) = load_profile(current.id).await {
                if let Some(contact) = profile.contact {
                    email.set(contact.email);
                    details.update(|d| {
                        if d.phone.is_empty() {
                            d.phone = contact.phone;
                        }
                    });
                }
            }
        });
    });

    // Only answerable once the agreement has been read and accepted.
    let unlocked = move || read_to_end.get() && accepted.get();

    // Drives the parent/guardian block, which the entered date of birth decides.
    let is_minor = move || dates::is_minor(&details.get().date_of_birth);

    require_login(state, move || {
        let navigate = navigate.clone();
        let submit = move || {
            if !accepted.get_untracked() || submitting.get_untracked() {
                return;
            }
            let mut submitted = details.get_untracked();
            // The tick box is the consent; date and time are generated server-side.
            submitted.electronic_consent = accepted.get_untracked();
            submitted.signer_is_guardian = dates::is_minor(&submitted.date_of_birth);
            let submitted = submitted.normalized();
            // Validate before the round trip; the server runs the same checks.
            if let Err(message) = submitted
                .validate_with(false)
                .and_then(|()| submitted.validate_signature())
            {
                error.set(message);
                return;
            }
            submitting.set(true);
            error.set(String::new());
            let navigate = navigate.clone();
            spawn_local(async move {
                match apply_to_volunteer(VOLUNTEER_AGREEMENT_VERSION.to_string(), submitted).await {
                    Ok(()) => navigate("/profile", Default::default()),
                    Err(e) => {
                        error.set(err_text(e));
                        submitting.set(false);
                    }
                }
            });
        };

        view! {
            <Layout title="Volunteer agreement".to_string()>
                <div class="mx-auto w-full max-w-4xl">
                    <form
                        class="overflow-hidden rounded-lg border border-slate-800 bg-slate-900 shadow-lg shadow-black/5"
                        on:submit=move |event| {
                            event.prevent_default();
                            submit();
                        }
                    >
                        <section class="p-5 sm:p-7">
                            <div class="mb-6">
                                <h2 class="text-lg font-semibold text-slate-100">
                                    "Read and accept the volunteer agreement"
                                </h2>
                                <p class="mt-1 text-sm text-slate-400">
                                    "Please read all the way to the end. Accepting submits an application to volunteer \u{2014} an administrator will review it and you'll hear back by email."
                                </p>
                            </div>

                            <div
                                node_ref=agreement_ref
                                on:scroll=move |_| check_scrolled()
                                class="max-h-[28rem] space-y-6 overflow-y-auto rounded-lg border border-slate-800 bg-slate-950 p-5 text-sm leading-relaxed text-slate-300"
                            >
                                {VOLUNTEER_AGREEMENT_SECTIONS
                                    .iter()
                                    .map(|section| {
                                        view! {
                                            <div class="space-y-3">
                                                <Show when=move || !section.heading.is_empty()>
                                                    <h3 class="text-base font-semibold text-slate-100">
                                                        {section.heading}
                                                    </h3>
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
                                <p class="pt-2 text-xs font-medium text-slate-500">
                                    "\u{2014} End of Volunteer Agreement \u{2014}"
                                </p>
                            </div>

                            <Show when=move || !read_to_end.get()>
                                <p class="mt-3 text-xs text-red-400/90">
                                    "Scroll to the end of the agreement to continue."
                                </p>
                            </Show>
                        </section>

                        <section class="border-t border-slate-800 p-5 sm:p-7">
                            <p class="text-sm text-slate-400">{VOLUNTEER_ATTESTATION}</p>

                            <h3 class="mt-6 text-sm font-semibold uppercase tracking-wide text-slate-200">
                                {ELECTRONIC_CONSENT_HEADING}
                            </h3>
                            <p class="mt-2 text-sm leading-relaxed text-slate-400">
                                {ELECTRONIC_CONSENT}
                            </p>

                            <label
                                class="mt-5 flex items-start gap-3 text-sm"
                                class=("text-slate-200", move || read_to_end.get())
                                class=("text-slate-500", move || !read_to_end.get())
                            >
                                <input
                                    class="mt-0.5 h-4 w-4 rounded border-slate-700 bg-slate-950 text-primary-500 focus:ring-2 focus:ring-primary-500/40 disabled:cursor-not-allowed disabled:opacity-50"
                                    type="checkbox"
                                    disabled=move || !read_to_end.get()
                                    prop:checked=move || accepted.get()
                                    on:change=move |event| {
                                        accepted.set(event_target_checked(&event));
                                        error.set(String::new());
                                    }
                                />
                                <span>{CONSENT_CHECKBOX_LABEL}</span>
                            </label>

                            <Show when=move || read_to_end.get() && !accepted.get()>
                                <p class="mt-3 text-xs text-slate-400">
                                    "Tick the box above to continue."
                                </p>
                            </Show>
                        </section>

                        <section class="border-t border-slate-800 p-5 sm:p-7">
                            <h2 class="text-lg font-semibold text-slate-100">
                                "Volunteer information and signature"
                            </h2>
                            <p class="mt-1 text-sm text-slate-400">
                                "The date and time of submission are recorded automatically."
                            </p>

                            <Show when=move || !unlocked()>
                                <p class="mb-5 mt-4 rounded-lg bg-slate-800/60 px-3 py-2 text-xs text-slate-400">
                                    "Read the agreement to the end and accept it above to fill this in."
                                </p>
                            </Show>

                            <div class="mt-4 grid gap-5 sm:grid-cols-2">
                                <div>
                                    <label class=LABEL_CLASS>
                                        "Volunteer\u{2019}s full legal name " <Required />
                                    </label>
                                    <input
                                        class=INPUT_CLASS
                                        prop:disabled=move || !unlocked()
                                        prop:value=move || details.get().legal_name
                                        on:input=move |event| {
                                            let value = event_target_value(&event);
                                            details.update(|d| d.legal_name = value);
                                        }
                                    />
                                </div>
                                <div>
                                    <label class=LABEL_CLASS>
                                        "Electronic signature (type full legal name) " <Required />
                                    </label>
                                    <input
                                        class=INPUT_CLASS
                                        prop:disabled=move || !unlocked()
                                        prop:value=move || details.get().signature_name
                                        on:input=move |event| {
                                            let value = event_target_value(&event);
                                            details.update(|d| d.signature_name = value);
                                        }
                                    />
                                    <p class="mt-1 text-xs text-slate-500">
                                        "If the volunteer is under 18, this is the parent or legal guardian\u{2019}s name."
                                    </p>
                                </div>
                                <div>
                                    <label class=LABEL_CLASS>"Email address"</label>
                                    <input
                                        class=INPUT_CLASS
                                        type="email"
                                        disabled=true
                                        prop:value=move || email.get()
                                    />
                                </div>
                                <div>
                                    <label class=LABEL_CLASS>"Date and time of submission"</label>
                                    <input
                                        class=INPUT_CLASS
                                        disabled=true
                                        prop:value="Recorded automatically when you submit"
                                    />
                                </div>
                            </div>

                            <Show when=move || is_minor()>
                                <h3 class=HEADING_CLASS>"If the volunteer is under 18"</h3>
                                <p class="mt-1 text-xs text-slate-500">
                                    "A parent or legal guardian must sign on the volunteer\u{2019}s behalf."
                                </p>
                                <div class="mt-4 grid gap-5 sm:grid-cols-2">
                                    <div>
                                        <label class=LABEL_CLASS>
                                            "Parent/legal guardian\u{2019}s full legal name " <Required />
                                        </label>
                                        <input
                                            class=INPUT_CLASS
                                            prop:disabled=move || !unlocked()
                                            prop:value=move || details.get().guardian_name
                                            on:input=move |event| {
                                                let value = event_target_value(&event);
                                                details.update(|d| d.guardian_name = value);
                                            }
                                        />
                                    </div>
                                    <div>
                                        <label class=LABEL_CLASS>
                                            "Relationship to minor volunteer " <Required />
                                        </label>
                                        <input
                                            class=INPUT_CLASS
                                            prop:disabled=move || !unlocked()
                                            prop:value=move || details.get().guardian_relationship
                                            on:input=move |event| {
                                                let value = event_target_value(&event);
                                                details.update(|d| d.guardian_relationship = value);
                                            }
                                        />
                                    </div>
                                    <div>
                                        <label class=LABEL_CLASS>
                                            "Parent/legal guardian\u{2019}s email address " <Required />
                                        </label>
                                        <input
                                            class=INPUT_CLASS
                                            type="email"
                                            prop:disabled=move || !unlocked()
                                            prop:value=move || details.get().guardian_email
                                            on:input=move |event| {
                                                let value = event_target_value(&event);
                                                details.update(|d| d.guardian_email = value);
                                            }
                                        />
                                    </div>
                                </div>
                            </Show>

                            <h3 class=HEADING_CLASS>"Mommy\u{2019}s Heart acceptance"</h3>
                            <div class="mt-4 grid gap-5 sm:grid-cols-2">
                                <div>
                                    <label class=LABEL_CLASS>
                                        "Authorized representative\u{2019}s full legal name"
                                    </label>
                                    <input
                                        class=INPUT_CLASS
                                        disabled=true
                                        prop:value=FOUNDATION_SIGNATORY
                                    />
                                </div>
                                <div>
                                    <label class=LABEL_CLASS>"Title"</label>
                                    <input
                                        class=INPUT_CLASS
                                        disabled=true
                                        prop:value=FOUNDATION_SIGNATORY_TITLE
                                    />
                                </div>
                                <div>
                                    <label class=LABEL_CLASS>
                                        "Electronic signature (type full legal name)"
                                    </label>
                                    <input
                                        class=INPUT_CLASS
                                        disabled=true
                                        prop:value=FOUNDATION_SIGNATORY
                                    />
                                </div>
                                <div>
                                    <label class=LABEL_CLASS>"Date"</label>
                                    <input
                                        class=INPUT_CLASS
                                        disabled=true
                                        prop:value="Same date as the volunteer\u{2019}s submission"
                                    />
                                </div>
                            </div>

                            <h3 class=HEADING_CLASS>{ELECTRONIC_EXECUTION_HEADING}</h3>
                            <p class="mt-2 text-sm leading-relaxed text-slate-400">
                                {ELECTRONIC_EXECUTION}
                            </p>
                        </section>

                        <section class="border-t border-slate-800 p-5 sm:p-7">
                            <div class="mb-6">
                                <h2 class="text-lg font-semibold text-slate-100">
                                    "Volunteer\u{2019}s contact information"
                                </h2>
                                <p class="mt-1 text-sm text-slate-400">
                                    "Fields marked with " <Required /> " are required."
                                </p>
                            </div>

                            <Show when=move || !unlocked()>
                                <p class="mb-5 rounded-lg bg-slate-800/60 px-3 py-2 text-xs text-slate-400">
                                    "Read the agreement to the end and accept it above to fill this in."
                                </p>
                            </Show>

                            <p class="text-xs text-slate-500">{BACKGROUND_CHECK_CONSENT}</p>

                            <div class="mt-4 grid gap-5 sm:grid-cols-2">
                                <div>
                                    <label class=LABEL_CLASS>
                                        "Date of birth " <Required />
                                    </label>
                                    <input
                                        class=INPUT_CLASS
                                        type="date"
                                        prop:disabled=move || !unlocked()
                                        prop:value=move || details.get().date_of_birth
                                        on:input=move |event| {
                                            let value = event_target_value(&event);
                                            details.update(|d| d.date_of_birth = value);
                                        }
                                    />
                                </div>
                                <div>
                                    <label class=LABEL_CLASS>
                                        "Social Security Number " <Required />
                                    </label>
                                    <input
                                        class=INPUT_CLASS
                                        placeholder="000-00-0000"
                                        prop:disabled=move || !unlocked()
                                        prop:value=move || details.get().ssn
                                        on:input=move |event| {
                                            let value = event_target_value(&event);
                                            details.update(|d| d.ssn = value);
                                        }
                                    />
                                    <p class="mt-1 text-xs text-slate-500">
                                        "Only a site administrator can view it, and every viewing is logged."
                                    </p>
                                </div>
                                <div>
                                    <label class=LABEL_CLASS>"Email"</label>
                                    <input
                                        class=INPUT_CLASS
                                        type="email"
                                        disabled=true
                                        prop:value=move || email.get()
                                    />
                                    <p class="mt-1 text-xs text-slate-500">
                                        "Your sign-in email address."
                                    </p>
                                </div>
                                <div>
                                    <label class=LABEL_CLASS>
                                        "Phone number " <Required />
                                    </label>
                                    <input
                                        class=INPUT_CLASS
                                        placeholder="(000) 000-0000"
                                        prop:disabled=move || !unlocked()
                                        prop:value=move || details.get().phone
                                        on:input=move |event| {
                                            let value = event_target_value(&event);
                                            details.update(|d| d.phone = value);
                                        }
                                    />
                                </div>
                                <div>
                                    <label class=LABEL_CLASS>"Volunteer role"</label>
                                    <input
                                        class=INPUT_CLASS
                                        prop:disabled=move || !unlocked()
                                        prop:value=move || details.get().volunteer_role
                                        on:input=move |event| {
                                            let value = event_target_value(&event);
                                            details.update(|d| d.volunteer_role = value);
                                        }
                                    />
                                </div>
                            </div>

                            <div class="mt-5">
                                <label class=LABEL_CLASS>
                                    "Volunteer special skills and area of focus " <Required />
                                </label>
                                <textarea
                                    class=INPUT_CLASS
                                    rows="3"
                                    prop:disabled=move || !unlocked()
                                    prop:value=move || details.get().skills_focus
                                    on:input=move |event| {
                                        let value = event_target_value(&event);
                                        details.update(|d| d.skills_focus = value);
                                    }
                                />
                            </div>

                            <h3 class=HEADING_CLASS>
                                "Emergency contact information"
                            </h3>

                            <div class="mt-4 grid gap-5 sm:grid-cols-2">
                                <div>
                                    <label class=LABEL_CLASS>
                                        "First name " <Required />
                                    </label>
                                    <input
                                        class=INPUT_CLASS
                                        prop:disabled=move || !unlocked()
                                        prop:value=move || details.get().emergency_first_name
                                        on:input=move |event| {
                                            let value = event_target_value(&event);
                                            details.update(|d| d.emergency_first_name = value);
                                        }
                                    />
                                </div>
                                <div>
                                    <label class=LABEL_CLASS>
                                        "Last name " <Required />
                                    </label>
                                    <input
                                        class=INPUT_CLASS
                                        prop:disabled=move || !unlocked()
                                        prop:value=move || details.get().emergency_last_name
                                        on:input=move |event| {
                                            let value = event_target_value(&event);
                                            details.update(|d| d.emergency_last_name = value);
                                        }
                                    />
                                </div>
                                <div>
                                    <label class=LABEL_CLASS>"Relationship to volunteer"</label>
                                    <input
                                        class=INPUT_CLASS
                                        prop:disabled=move || !unlocked()
                                        prop:value=move || details.get().emergency_relationship
                                        on:input=move |event| {
                                            let value = event_target_value(&event);
                                            details.update(|d| d.emergency_relationship = value);
                                        }
                                    />
                                </div>
                                <div>
                                    <label class=LABEL_CLASS>
                                        "Phone number " <Required />
                                    </label>
                                    <input
                                        class=INPUT_CLASS
                                        placeholder="(000) 000-0000"
                                        prop:disabled=move || !unlocked()
                                        prop:value=move || details.get().emergency_phone
                                        on:input=move |event| {
                                            let value = event_target_value(&event);
                                            details.update(|d| d.emergency_phone = value);
                                        }
                                    />
                                </div>
                            </div>

                            <Show when=move || !error.get().is_empty()>
                                <p class="mt-5 rounded-lg bg-rose-500/10 px-3 py-2 text-sm text-rose-300 ring-1 ring-rose-500/30">
                                    {move || error.get()}
                                </p>
                            </Show>

                            <div class="mt-7 flex flex-wrap items-center justify-end gap-3">
                                <A
                                    href="/profile"
                                    attr:class="min-h-10 rounded-lg border border-slate-700 px-5 py-2.5 text-sm font-semibold text-slate-300 hover:bg-slate-800"
                                >
                                    "Cancel"
                                </A>
                                <button
                                    type="submit"
                                    disabled=move || !unlocked() || submitting.get()
                                    class="min-w-36 rounded-lg bg-primary-500 px-6 py-3 text-sm font-bold text-white hover:bg-primary-600 disabled:cursor-not-allowed disabled:opacity-60"
                                >
                                    {move || if submitting.get() {
                                        "SUBMITTING\u{2026}"
                                    } else {
                                        "ACCEPT AND APPLY"
                                    }}
                                </button>
                            </div>
                        </section>
                    </form>
                </div>
            </Layout>
        }
        .into_any()
    })
}
