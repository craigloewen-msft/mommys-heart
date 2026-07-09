use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;

use crate::state::AppState;

/// Standalone self-service registration screen. New accounts become pending
/// volunteers in the local demo store.
#[component]
pub fn RegisterPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    let navigate = use_navigate();

    let name = RwSignal::new(String::new());
    let email = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());

    let submit = {
        let navigate = navigate.clone();
        move || match state.register(&name.get(), &email.get(), &password.get()) {
            Ok(_) => {
                error.set(String::new());
                navigate("/volunteer", Default::default());
            }
            Err(e) => error.set(e),
        }
    };

    let fill_demo = move |_| {
        name.set("Jamie Rivera".into());
        email.set(format!("jamie{}@mommysheart.org", 100));
        password.set("demo1234".into());
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
                        "Sign up to volunteer. An admin will activate your account."
                    </p>

                    <form
                        class="mt-5 space-y-4"
                        on:submit=move |ev| {
                            ev.prevent_default();
                            submit();
                        }
                    >
                        <div>
                            <label class="mb-1 block text-sm text-slate-300">"Full name"</label>
                            <input
                                class=input_class
                                placeholder="Jamie Rivera"
                                prop:value=move || name.get()
                                on:input=move |ev| name.set(event_target_value(&ev))
                            />
                        </div>
                        <div>
                            <label class="mb-1 block text-sm text-slate-300">"Email"</label>
                            <input
                                class=input_class
                                r#type="email"
                                placeholder="you@mommysheart.org"
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

                    <button
                        r#type="button"
                        on:click=fill_demo
                        class="mt-3 w-full rounded-lg border border-dashed border-slate-700 px-3 py-2 text-xs font-medium text-slate-300 hover:bg-slate-800"
                    >
                        "Fill demo details"
                    </button>

                    <p class="mt-5 text-center text-sm text-slate-400">
                        "Already have an account? "
                        <A
                            href="/login"
                            attr:class="font-medium text-primary-400 hover:text-primary-300"
                        >
                            "Sign in"
                        </A>
                    </p>
                </div>
            </div>
        </div>
    }
}
