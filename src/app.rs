use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::components::{Redirect, Route, Router, Routes};
use leptos_router::path;

use crate::pages::{
    admin::AdminDashboardPage,
    cases::{CaseHomePage, NewCasePage},
    grants::GrantHomePage,
    inbox::InboxPage,
    login::LoginPage,
    register::RegisterPage,
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

/// Sends the visitor to the right landing page based on their session.
#[component]
fn HomeRedirect() -> impl IntoView {
    let state = expect_context::<AppState>();
    let path = if state.current_user.get_untracked().is_some() {
        "/cases"
    } else {
        "/login"
    };
    view! { <Redirect path=path /> }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    provide_context(AppState::new());

    view! {
        <Stylesheet id="leptos" href="/pkg/mommys-heart-crm.css" />
        <Title text="Mommy's Heart CRM" />

        <Router>
            <Routes fallback=|| {
                view! { <p class="p-6 text-slate-300">"Page not found"</p> }
            }>
                <Route path=path!("/") view=HomeRedirect />
                <Route path=path!("/login") view=LoginPage />
                <Route path=path!("/register") view=RegisterPage />
                <Route path=path!("/cases") view=CaseHomePage />
                <Route path=path!("/cases/new") view=NewCasePage />
                <Route path=path!("/inbox") view=InboxPage />
                <Route path=path!("/grants") view=GrantHomePage />
                <Route path=path!("/admin") view=AdminDashboardPage />
            </Routes>
        </Router>
    }
}
