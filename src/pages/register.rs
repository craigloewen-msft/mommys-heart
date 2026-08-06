use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;

use crate::state::AppState;

/// Standalone self-service registration screen. New accounts become clients in
/// the local demo store; an admin can promote them later.
#[component]
pub fn RegisterPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    let navigate = use_navigate();

    let first_name = RwSignal::new(String::new());
    let last_name = RwSignal::new(String::new());
    let email = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let confirm_password = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());

    let submit = {
        let navigate = navigate.clone();
        move || {
            let navigate = navigate.clone();
            let first_name = first_name.get_untracked();
            let last_name = last_name.get_untracked();
            let email = email.get_untracked();
            let password = password.get_untracked();
            let confirm_password = confirm_password.get_untracked();
            if password != confirm_password {
                error.set("Passwords do not match.".to_string());
                return;
            }
            spawn_local(async move {
                match state
                    .register(&first_name, &last_name, &email, &password)
                    .await
                {
                    Ok(()) => {
                        error.set(String::new());
                        navigate("/verify-email", Default::default());
                    }
                    Err(e) => error.set(e),
                }
            });
        }
    };

    let input_class = "w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/40";

    view! {
        <div class="min-h-screen bg-slate-950 text-slate-100 flex items-center justify-center px-4">
            <div class="w-full max-w-md">
                <div class="mb-6 flex items-center justify-center gap-2">
                    <span class="grid h-10 w-10 place-items-center rounded-xl bg-primary-500/20 text-2xl text-primary-400">
                        "\u{2665}"
                    </span>
                    <span class="text-xl font-semibold tracking-tight">"Mommy's Heart"</span>
                </div>

                <div class="rounded-2xl border border-slate-800 bg-slate-900 p-6 shadow-xl shadow-black/30">
                    <h1 class="text-lg font-semibold">"Create your account"</h1>
                    <p class="mt-1 text-sm text-slate-400">
                        "Sign up to get started. An admin can adjust your access later."
                    </p>

                    <form
                        class="mt-5 space-y-4"
                        on:submit=move |ev| {
                            ev.prevent_default();
                            submit();
                        }
                    >
                        <div class="grid grid-cols-2 gap-3">
                            <div>
                                <label class="mb-1 block text-sm text-slate-300">"First name"</label>
                                <input
                                    class=input_class
                                    placeholder="Jamie"
                                    prop:value=move || first_name.get()
                                    on:input=move |ev| first_name.set(event_target_value(&ev))
                                />
                            </div>
                            <div>
                                <label class="mb-1 block text-sm text-slate-300">"Last name"</label>
                                <input
                                    class=input_class
                                    placeholder="Nguyen"
                                    prop:value=move || last_name.get()
                                    on:input=move |ev| last_name.set(event_target_value(&ev))
                                />
                            </div>
                        </div>
                        <div>
                            <label class="mb-1 block text-sm text-slate-300">"Email"</label>
                            <input
                                class=input_class
                                r#type="email"
                                placeholder="you@example.com"
                                prop:value=move || email.get()
                                on:input=move |ev| email.set(event_target_value(&ev))
                            />
                        </div>
                        <div>
                            <label class="mb-1 block text-sm text-slate-300">"Password"</label>
                            <input
                                class=input_class
                                r#type="password"
                                placeholder="Choose a password"
                                prop:value=move || password.get()
                                on:input=move |ev| password.set(event_target_value(&ev))
                            />
                        </div>
                        <div>
                            <label class="mb-1 block text-sm text-slate-300">"Re-enter password"</label>
                            <input
                                class=input_class
                                r#type="password"
                                placeholder="Re-enter your password"
                                prop:value=move || confirm_password.get()
                                on:input=move |ev| confirm_password.set(event_target_value(&ev))
                            />
                        </div>

                        <Show when=move || !error.get().is_empty()>
                            <p class="rounded-lg bg-rose-500/10 px-3 py-2 text-sm text-rose-300 ring-1 ring-rose-500/30">
                                {move || error.get()}
                            </p>
                        </Show>

                        <button
                            r#type="submit"
                            class="w-full rounded-lg bg-primary-500 px-4 py-2.5 text-sm font-semibold text-white transition-colors hover:bg-primary-600"
                        >
                            "Create account"
                        </button>
                    </form>

                    <p class="mt-5 text-center text-sm text-slate-400">
                        "Already have an account? "
                        <A
                            href="/login"
                            attr:class="font-medium text-primary-400 hover:text-primary-300"
                        >
                            "Sign in"
                        </A>
                    </p>
                    <p class="mt-3 text-center text-sm text-slate-400">
                        "Opening a new case? "
                        <A
                            href="/case-signup"
                            attr:class="font-medium text-primary-400 hover:text-primary-300"
                        >
                            "Start client case signup"
                        </A>
                    </p>
                </div>
            </div>
        </div>
    }
}
