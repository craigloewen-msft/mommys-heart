use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;

use crate::components::case_intake::{CaseIntakeFields, CaseIntakeState};
use crate::helpers::case_intake::CaseIntake;
#[cfg(feature = "hydrate")]
use crate::helpers::terms::TERMS_VERSION;
use crate::helpers::terms::{TERMS_ATTESTATION, TERMS_MINOR_NOTICE, TERMS_SECTIONS};
#[cfg(feature = "hydrate")]
use crate::server_fns::{auth, err_text};

/// Where the browser remembers that the terms were accepted, so the details form
/// knows it was reached the long way round. This is a convenience for the user,
/// not a security boundary — the server independently refuses any submission
/// that does not carry the current terms version.
#[cfg(feature = "hydrate")]
const ACCEPTED_KEY: &str = "case-signup-terms-accepted";

/// Record acceptance of the current terms for this browser session.
fn remember_acceptance() {
    #[cfg(feature = "hydrate")]
    if let Some(storage) = session_storage() {
        let _ = storage.set_item(ACCEPTED_KEY, TERMS_VERSION);
    }
}

/// Whether this browser session has accepted the terms currently in force.
fn has_accepted() -> bool {
    #[cfg(feature = "hydrate")]
    {
        session_storage()
            .and_then(|storage| storage.get_item(ACCEPTED_KEY).ok().flatten())
            .is_some_and(|version| version == TERMS_VERSION)
    }
    // Server-rendered markup is identical either way; the check runs once the
    // page hydrates, and redirects then if the terms were skipped.
    #[cfg(not(feature = "hydrate"))]
    true
}

#[cfg(feature = "hydrate")]
fn session_storage() -> Option<web_sys::Storage> {
    web_sys::window().and_then(|window| window.session_storage().ok().flatten())
}

/// The page header shared by both signup steps.
#[component]
fn SignupHeader(#[prop(into)] title: String) -> impl IntoView {
    view! {
        <header class="mb-7 flex items-center justify-between gap-4">
            <div class="flex items-center gap-3">
                <span class="grid h-10 w-10 place-items-center rounded-lg bg-primary-500/15 text-2xl text-primary-500">
                    "\u{2665}"
                </span>
                <div>
                    <p class="text-sm font-medium text-slate-400">"Mommy's Heart"</p>
                    <h1 class="text-2xl font-semibold text-slate-100">{title}</h1>
                </div>
            </div>
            <A href="/login" attr:class="text-sm font-medium text-primary-500 hover:text-primary-600">
                "Sign in"
            </A>
        </header>
    }
}

/// Step one of a prospective client signup: read the Terms and Conditions and
/// accept them. Nothing is sent to the server here — acceptance is carried
/// forward and recorded only if the signup itself completes, so a visitor who
/// reads the terms and leaves creates no record at all.
#[component]
pub fn CaseSignupTermsPage() -> impl IntoView {
    let navigate = use_navigate();
    let accepted = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    // The acceptance control stays locked until the terms have been scrolled to
    // the bottom, so "I have read" sits under text the reader was at least shown
    // all of rather than under text they never moved past.
    let read_to_end = RwSignal::new(false);
    let terms_ref = NodeRef::<leptos::html::Div>::new();

    // Whether the terms pane is scrolled to (or within a pixel of) its end. Also
    // true when the content is short enough not to scroll at all: a pane with no
    // scrollbar can never fire a scroll event, and would otherwise lock the user
    // out of a form they have already read in full.
    let check_scrolled = move || {
        if let Some(pane) = terms_ref.get_untracked() {
            let scrolled = pane.scroll_top() as f64 + pane.client_height() as f64;
            // A pixel of slack absorbs the fractional scroll positions that zoom
            // and high-DPI displays produce, which otherwise stop just short.
            if scrolled >= pane.scroll_height() as f64 - 1.0 {
                read_to_end.set(true);
            }
        }
    };

    // Runs once after mount to catch the no-scrollbar case described above.
    Effect::new(move |_| check_scrolled());

    let submit = move || {
        if !accepted.get_untracked() {
            error.set("Please tick the box to confirm you accept the terms.".to_string());
            return;
        }
        remember_acceptance();
        navigate("/case-signup/details", Default::default());
    };

    view! {
        <main class="min-h-screen bg-slate-950 px-4 py-8 text-slate-100 sm:py-12">
            <div class="mx-auto w-full max-w-4xl">
                <SignupHeader title="Terms and Conditions" />

                <form
                    class="overflow-hidden rounded-lg border border-slate-800 bg-slate-900 shadow-lg shadow-black/5"
                    on:submit=move |event| {
                        event.prevent_default();
                        submit();
                    }
                >
                    <section class="p-5 sm:p-7">
                        <div class="mb-6 flex items-start gap-3">
                            <span class="grid h-8 w-8 shrink-0 place-items-center rounded-full bg-primary-500 text-sm font-bold text-white">"1"</span>
                            <div>
                                <h2 class="text-lg font-semibold text-slate-100">"Read and accept the terms"</h2>
                                <p class="mt-1 text-sm text-slate-400">"Please read all the way to the end. You must accept these terms before you can tell us about your case."</p>
                            </div>
                        </div>

                        <div
                            node_ref=terms_ref
                            on:scroll=move |_| check_scrolled()
                            class="max-h-[28rem] space-y-6 overflow-y-auto rounded-lg border border-slate-800 bg-slate-950 p-5 text-sm leading-relaxed text-slate-300"
                        >
                            {TERMS_SECTIONS
                                .iter()
                                .map(|section| {
                                    view! {
                                        <div class="space-y-3">
                                            <Show when=move || !section.heading.is_empty()>
                                                <h3 class="text-base font-semibold text-slate-100">{section.heading}</h3>
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
                            <p class="pt-2 text-xs font-medium text-slate-500">"— End of Terms and Conditions —"</p>
                        </div>

                        <Show when=move || !read_to_end.get()>
                            <p class="mt-3 text-xs text-red-400/90">
                                "Scroll to the end of the terms to continue."
                            </p>
                        </Show>
                    </section>

                    <section class="border-t border-slate-800 p-5 sm:p-7">
                        <p class="text-sm text-slate-400">{TERMS_ATTESTATION}</p>

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
                            <span>"I have read and accept the Terms and Conditions."</span>
                        </label>
                        <p class="mt-3 text-xs text-slate-500">{TERMS_MINOR_NOTICE}</p>

                        <Show when=move || read_to_end.get() && !accepted.get()>
                            <p class="mt-3 text-xs text-slate-400">
                                "Tick the box above to continue."
                            </p>
                        </Show>

                        <Show when=move || !error.get().is_empty()>
                            <p class="mt-5 rounded-lg bg-rose-500/10 px-3 py-2 text-sm text-rose-300 ring-1 ring-rose-500/30">
                                {move || error.get()}
                            </p>
                        </Show>

                        <div class="mt-7 flex flex-wrap items-center justify-end gap-3">
                            <A
                                href="/login"
                                attr:class="min-h-10 rounded-lg border border-slate-700 px-5 py-2.5 text-sm font-semibold text-slate-300 hover:bg-slate-800"
                            >
                                "Decline"
                            </A>
                            <button
                                type="submit"
                                disabled=move || !accepted.get() || !read_to_end.get()
                                class="min-w-36 rounded-lg bg-primary-500 px-6 py-3 text-sm font-bold text-white hover:bg-primary-600 disabled:cursor-not-allowed disabled:opacity-60"
                            >
                                "ACCEPT AND CONTINUE"
                            </button>
                        </div>
                    </section>
                </form>
            </div>
        </main>
    }
}

struct AccountFields {
    first_name: String,
    last_name: String,
    email: String,
    password: String,
    password_confirmation: String,
}

async fn start_case_signup(account: AccountFields, intake: CaseIntake) -> Result<(), String> {
    #[cfg(feature = "hydrate")]
    {
        let intake_json = serde_json::to_string(&intake)
            .map_err(|_| "Could not prepare the case information.".to_string())?;
        return auth::register_case_signup(
            account.first_name,
            account.last_name,
            account.email,
            account.password,
            account.password_confirmation,
            intake_json,
            TERMS_VERSION.to_string(),
        )
        .await
        .map_err(err_text);
    }

    #[cfg(not(feature = "hydrate"))]
    {
        let _ = (
            account.first_name,
            account.last_name,
            account.email,
            account.password,
            account.password_confirmation,
            intake,
        );
        Ok(())
    }
}

/// Step two of a prospective client signup: the case details and the account to
/// sign in with. Reached only after the terms have been accepted; arriving here
/// directly bounces back to them.
#[component]
pub fn CaseSignupDetailsPage() -> impl IntoView {
    let navigate = use_navigate();
    let first_name = RwSignal::new(String::new());
    let last_name = RwSignal::new(String::new());
    let email = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let password_confirmation = RwSignal::new(String::new());
    let intake = CaseIntakeState::new();
    let error = RwSignal::new(String::new());
    let pending = RwSignal::new(false);

    // Send anybody who skipped the terms back to read them.
    Effect::new({
        let navigate = navigate.clone();
        move |_| {
            if !has_accepted() {
                navigate("/case-signup", Default::default());
            }
        }
    });

    let input_class = "mt-1 w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2.5 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/30";
    let label_class = "block text-sm font-medium text-slate-300";

    let submit = {
        let navigate = navigate.clone();
        move || {
            if pending.get_untracked() {
                return;
            }
            if password.get_untracked() != password_confirmation.get_untracked() {
                error.set("The passwords do not match.".to_string());
                return;
            }
            let account = AccountFields {
                first_name: first_name.get_untracked(),
                last_name: last_name.get_untracked(),
                email: email.get_untracked(),
                password: password.get_untracked(),
                password_confirmation: password_confirmation.get_untracked(),
            };
            let intake = intake.value();
            if let Err(message) = intake.validate() {
                error.set(message);
                return;
            }
            let navigate = navigate.clone();
            pending.set(true);
            error.set(String::new());
            spawn_local(async move {
                match start_case_signup(account, intake).await {
                    Ok(()) => navigate("/case-signup/verify", Default::default()),
                    Err(message) => {
                        error.set(message);
                        pending.set(false);
                    }
                }
            });
        }
    };

    view! {
        <main class="min-h-screen bg-slate-950 px-4 py-8 text-slate-100 sm:py-12">
            <div class="mx-auto w-full max-w-4xl">
                <SignupHeader title="Prospective client case signup" />

                <p class="mb-5 rounded-lg bg-emerald-500/10 px-3 py-2 text-sm text-emerald-300 ring-1 ring-emerald-500/30">
                    "Terms and Conditions accepted. Now tell us about your case."
                </p>

                <form
                    class="overflow-hidden rounded-lg border border-slate-800 bg-slate-900 shadow-lg shadow-black/5"
                    on:submit=move |event| {
                        event.prevent_default();
                        submit();
                    }
                >
                    <section class="p-5 sm:p-7">
                        <div class="mb-6 flex items-start gap-3">
                            <span class="grid h-8 w-8 shrink-0 place-items-center rounded-full bg-primary-500 text-sm font-bold text-white">"2"</span>
                            <div>
                                <h2 class="text-lg font-semibold text-slate-100">"Provide case info"</h2>
                                <p class="mt-1 text-sm text-slate-400">"Fields marked with * are required."</p>
                            </div>
                        </div>

                        <CaseIntakeFields state=intake />
                    </section>

                    <section class="border-t border-slate-800 p-5 sm:p-7">
                        <div class="mb-6 flex items-start gap-3">
                            <span class="grid h-8 w-8 shrink-0 place-items-center rounded-full bg-primary-500 text-sm font-bold text-white">"3"</span>
                            <div>
                                <h2 class="text-lg font-semibold text-slate-100">"Provide account info"</h2>
                                <p class="mt-1 text-sm text-slate-400">"Your email address will be your sign-in username."</p>
                            </div>
                        </div>
                        <div class="grid gap-5 sm:grid-cols-2">
                            <div>
                                <label class=label_class>
                                    "First name "
                                    <span class="text-rose-400" aria-hidden="true">"*"</span>
                                </label>
                                <input class=input_class autocomplete="given-name" required prop:value=move || first_name.get() on:input=move |event| first_name.set(event_target_value(&event)) />
                            </div>
                            <div>
                                <label class=label_class>
                                    "Last name "
                                    <span class="text-rose-400" aria-hidden="true">"*"</span>
                                </label>
                                <input class=input_class autocomplete="family-name" required prop:value=move || last_name.get() on:input=move |event| last_name.set(event_target_value(&event)) />
                            </div>
                            <div class="sm:col-span-2">
                                <label class=label_class>
                                    "Email (sign-in username) "
                                    <span class="text-rose-400" aria-hidden="true">"*"</span>
                                </label>
                                <input class=input_class type="email" autocomplete="email" required prop:value=move || email.get() on:input=move |event| email.set(event_target_value(&event)) />
                            </div>
                            <div>
                                <label class=label_class>
                                    "Password "
                                    <span class="text-rose-400" aria-hidden="true">"*"</span>
                                </label>
                                <input class=input_class type="password" autocomplete="new-password" required minlength="8" prop:value=move || password.get() on:input=move |event| password.set(event_target_value(&event)) />
                                <p class="mt-1 text-xs text-slate-500">"At least 8 characters."</p>
                            </div>
                            <div>
                                <label class=label_class>
                                    "Re-enter password "
                                    <span class="text-rose-400" aria-hidden="true">"*"</span>
                                </label>
                                <input class=input_class type="password" autocomplete="new-password" required minlength="8" prop:value=move || password_confirmation.get() on:input=move |event| password_confirmation.set(event_target_value(&event)) />
                            </div>
                        </div>

                        <Show when=move || !error.get().is_empty()>
                            <p class="mt-5 rounded-lg bg-rose-500/10 px-3 py-2 text-sm text-rose-300 ring-1 ring-rose-500/30">
                                {move || error.get()}
                            </p>
                        </Show>

                        <div class="mt-7 flex justify-end">
                            <button
                                type="submit"
                                disabled=move || pending.get()
                                class="min-w-36 rounded-lg bg-primary-500 px-6 py-3 text-sm font-bold text-white hover:bg-primary-600 disabled:cursor-not-allowed disabled:opacity-60"
                            >
                                {move || if pending.get() { "SUBMITTING..." } else { "SUBMIT" }}
                            </button>
                        </div>
                    </section>
                </form>
            </div>
        </main>
    }
}
