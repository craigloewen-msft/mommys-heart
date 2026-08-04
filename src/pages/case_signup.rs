use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;

use crate::components::case_intake::{CaseIntakeFields, CaseIntakeState};
use crate::helpers::case_intake::CaseIntake;
#[cfg(feature = "hydrate")]
use crate::server_fns::{auth, err_text};

struct AccountFields {
    first_name: String,
    last_name: String,
    email: String,
    password: String,
    password_confirmation: String,
}

async fn start_case_signup(
    agreement_ref: NodeRef<leptos::html::Input>,
    account: AccountFields,
    intake: CaseIntake,
) -> Result<(), String> {
    #[cfg(feature = "hydrate")]
    {
        use leptos::server_fn::codec::MultipartData;

        let input = agreement_ref
            .get_untracked()
            .ok_or_else(|| "Please upload the signed agreement.".to_string())?;
        let file = input
            .files()
            .and_then(|files| files.get(0))
            .ok_or_else(|| "Please upload the signed agreement.".to_string())?;
        if file.size() > crate::server_fns::evidence::MAX_SIZE_BYTES as f64 {
            return Err("The signed agreement is too large; the limit is 25 MB.".to_string());
        }
        if !file.name().to_ascii_lowercase().ends_with(".docx") {
            return Err("The signed agreement must be uploaded as a .docx file.".to_string());
        }

        let form = web_sys::FormData::new()
            .map_err(|_| "Could not prepare the signup form.".to_string())?;
        form.append_with_blob_and_filename("agreement", file.as_ref(), &file.name())
            .map_err(|_| "Could not attach the signed agreement.".to_string())?;
        for (key, value) in [
            ("first_name", account.first_name),
            ("last_name", account.last_name),
            ("email", account.email),
            ("password", account.password),
            ("password_confirmation", account.password_confirmation),
            (
                "intake_json",
                serde_json::to_string(&intake)
                    .map_err(|_| "Could not prepare the case information.".to_string())?,
            ),
        ] {
            form.append_with_str(key, &value)
                .map_err(|_| "Could not prepare the signup form.".to_string())?;
        }
        return auth::register_case_signup(MultipartData::from(form))
            .await
            .map_err(err_text);
    }

    #[cfg(not(feature = "hydrate"))]
    {
        let _ = (
            agreement_ref,
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

#[component]
pub fn CaseSignupPage() -> impl IntoView {
    let navigate = use_navigate();
    let first_name = RwSignal::new(String::new());
    let last_name = RwSignal::new(String::new());
    let email = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let password_confirmation = RwSignal::new(String::new());
    let intake = CaseIntakeState::new();
    let agreement_ref = NodeRef::<leptos::html::Input>::new();
    let error = RwSignal::new(String::new());
    let pending = RwSignal::new(false);

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
            let navigate = navigate.clone();
            pending.set(true);
            error.set(String::new());
            spawn_local(async move {
                match start_case_signup(agreement_ref, account, intake).await {
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
                <header class="mb-7 flex items-center justify-between gap-4">
                    <div class="flex items-center gap-3">
                        <span class="grid h-10 w-10 place-items-center rounded-lg bg-primary-500/15 text-2xl text-primary-500">
                            "\u{2665}"
                        </span>
                        <div>
                            <p class="text-sm font-medium text-slate-400">"Mommy's Heart"</p>
                            <h1 class="text-2xl font-semibold text-slate-100">"Customer case signup"</h1>
                        </div>
                    </div>
                    <A href="/login" attr:class="text-sm font-medium text-primary-500 hover:text-primary-600">
                        "Sign in"
                    </A>
                </header>

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
                                <h2 class="text-lg font-semibold text-slate-100">"Provide case info"</h2>
                                <p class="mt-1 text-sm text-slate-400">"Every field in this section is optional."</p>
                            </div>
                        </div>

                        <CaseIntakeFields state=intake />
                    </section>

                    <section class="border-t border-slate-800 p-5 sm:p-7">
                        <div class="mb-6 flex items-start gap-3">
                            <span class="grid h-8 w-8 shrink-0 place-items-center rounded-full bg-primary-500 text-sm font-bold text-white">"2"</span>
                            <div>
                                <h2 class="text-lg font-semibold text-slate-100">"Upload signed agreement"</h2>
                                <p class="mt-1 text-sm text-slate-400">"Download, sign, and upload the service agreement as a .docx file."</p>
                            </div>
                        </div>
                        <div class="space-y-4">
                            <a
                                href="/service-agreement-template.docx"
                                download
                                class="inline-flex min-h-10 items-center justify-center rounded-lg border border-primary-500/40 bg-primary-500/10 px-4 py-2 text-sm font-semibold text-primary-600 hover:bg-primary-500/15"
                            >
                                "Download agreement template"
                            </a>
                            <div>
                                <label class=label_class>
                                    "Signed service agreement "
                                    <span class="text-rose-400" aria-hidden="true">"*"</span>
                                </label>
                                <input
                                    node_ref=agreement_ref
                                    class=input_class
                                    type="file"
                                    accept=".docx,application/vnd.openxmlformats-officedocument.wordprocessingml.document"
                                    required
                                />
                            </div>
                        </div>
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
                                <input class=input_class type="password" autocomplete="new-password" required prop:value=move || password.get() on:input=move |event| password.set(event_target_value(&event)) />
                            </div>
                            <div>
                                <label class=label_class>
                                    "Re-enter password "
                                    <span class="text-rose-400" aria-hidden="true">"*"</span>
                                </label>
                                <input class=input_class type="password" autocomplete="new-password" required prop:value=move || password_confirmation.get() on:input=move |event| password_confirmation.set(event_target_value(&event)) />
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
