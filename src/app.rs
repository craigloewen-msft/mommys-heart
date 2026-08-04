use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::components::{Redirect, Route, Router, Routes, A};
use leptos_router::path;

use crate::pages::{
    admin::AdminDashboardPage,
    cases::{CaseHomePage, NewCasePage},
    forgot_password::ForgotPasswordPage,
    inbox::InboxPage,
    login::LoginPage,
    mfa::MfaVerifyPage,
    profile::ProfilePage,
    register::RegisterPage,
    reset_password::ResetPasswordPage,
    settings::SettingsPage,
    verify_email::VerifyEmailPage,
};
use crate::state::AppState;

/// The HTML document shell rendered on the server around the hydrated app.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options=options.clone() />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

/// Sends the visitor to the right landing page based on their session. Reactive
/// on the auth phase, which resolves asynchronously in the browser.
#[component]
fn HomeRedirect() -> impl IntoView {
    let state = expect_context::<AppState>();
    move || {
        if state.is_authenticated() {
            view! { <Redirect path="/cases" /> }.into_any()
        } else if !state.auth_resolved.get() {
            // Render nothing while page loads
            ().into_any()
        } else {
            view! { <Redirect path="/login" /> }.into_any()
        }
    }
}

/// Friendly full-page 404 shown when no route matches.
#[component]
fn NotFoundPage() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-slate-950 text-slate-100 flex items-center justify-center px-4">
            <div class="w-full max-w-md text-center">
                <div class="mb-6 flex items-center justify-center gap-2">
                    <span class="grid h-10 w-10 place-items-center rounded-xl bg-primary-500/20 text-2xl text-primary-400">
                        "\u{2665}"
                    </span>
                    <span class="text-xl font-semibold tracking-tight">"Mommy's Heart"</span>
                </div>

                <div class="rounded-2xl border border-slate-800 bg-slate-900 p-8 shadow-xl shadow-black/30">
                    <p class="text-6xl font-bold tracking-tight text-primary-400">"404"</p>
                    <h1 class="mt-4 text-lg font-semibold text-slate-100">"Page not found"</h1>
                    <p class="mt-2 text-sm text-slate-400">
                        "The page you're looking for doesn't exist or may have moved."
                    </p>
                    <A
                        href="/"
                        attr:class="mt-6 inline-block rounded-lg border border-primary-500/40 bg-primary-500/10 px-4 py-2 text-sm font-medium text-primary-300 hover:bg-primary-500/20"
                    >
                        "Back to home"
                    </A>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    let state = AppState::new();
    provide_context(state);
    state.restore_session();

    // Refresh unread notifications every 5 minutes
    Effect::new(move |_| {
        set_interval(
            move || state.refresh_unread(),
            std::time::Duration::from_secs(300),
        );
    });

    view! {
        <Stylesheet id="leptos" href="/pkg/mommys-heart-crm.css" />
        <Title text="Mommy's Heart CRM" />

        <Router>
            <Routes fallback=|| view! { <NotFoundPage /> }>
                <Route path=path!("/") view=HomeRedirect />
                <Route path=path!("/login") view=LoginPage />
                <Route path=path!("/mfa") view=MfaVerifyPage />
                <Route path=path!("/forgot-password") view=ForgotPasswordPage />
                <Route path=path!("/reset-password") view=ResetPasswordPage />
                <Route path=path!("/register") view=RegisterPage />
                <Route path=path!("/verify-email") view=VerifyEmailPage />
                <Route path=path!("/cases") view=CaseHomePage />
                <Route path=path!("/cases/new") view=NewCasePage />
                <Route path=path!("/inbox") view=InboxPage />
                <Route path=path!("/settings") view=SettingsPage />
                <Route path=path!("/profile") view=ProfilePage />
                <Route path=path!("/profile/:id") view=ProfilePage />
                <Route path=path!("/admin") view=AdminDashboardPage />
            </Routes>
        </Router>
    }
}
