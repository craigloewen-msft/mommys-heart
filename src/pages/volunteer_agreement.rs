//! `/volunteer-agreement`: read the Volunteer Agreement and accept it, which
//! files an application for an administrator to review.
//!
//! The reading experience deliberately mirrors the client terms at
//! [`crate::pages::case_signup`]: the same scroll-to-the-end gate on the
//! acceptance control, so "I have read" sits under text the reader was at least
//! shown all of. Unlike that flow, acceptance posts immediately — there is no
//! second step to carry it forward to.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;

use crate::components::guard::require_login;
use crate::components::layout::Layout;
use crate::helpers::volunteer_terms::{
    VOLUNTEER_AGREEMENT_SECTIONS, VOLUNTEER_AGREEMENT_VERSION, VOLUNTEER_ATTESTATION,
};
use crate::server_fns::err_text;
use crate::server_fns::volunteers::apply_to_volunteer;
use crate::state::AppState;

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

    require_login(state, move || {
        let navigate = navigate.clone();
        let submit = move || {
            if !accepted.get_untracked() || submitting.get_untracked() {
                return;
            }
            submitting.set(true);
            error.set(String::new());
            let navigate = navigate.clone();
            spawn_local(async move {
                match apply_to_volunteer(VOLUNTEER_AGREEMENT_VERSION.to_string()).await {
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
                                <span>"I have read and accept the Volunteer Agreement."</span>
                            </label>

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
                                    href="/profile"
                                    attr:class="min-h-10 rounded-lg border border-slate-700 px-5 py-2.5 text-sm font-semibold text-slate-300 hover:bg-slate-800"
                                >
                                    "Cancel"
                                </A>
                                <button
                                    type="submit"
                                    disabled=move || {
                                        !accepted.get() || !read_to_end.get() || submitting.get()
                                    }
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
