use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::components::{Redirect, Route, Router, Routes};
use leptos_router::path;

use crate::pages::{
    admin::AdminDashboardPage, chat::ChatPage, clients::ClientDetailPage, clients::ClientsPage,
    contact_detail::ContactDetailPage, contacts::ContactsPage, dashboard::DashboardPage,
    login::LoginPage, register::RegisterPage, volunteer::VolunteerDashboardPage,
};
use crate::state::AppState;
use crate::types::Role;

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
    let path = match state.current_user.get_untracked() {
        Some(u) if u.role == Role::Admin => "/admin",
        Some(_) => "/volunteer",
        None => "/login",
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
                <Route path=path!("/admin") view=AdminDashboardPage />
                <Route path=path!("/clients") view=ClientsPage />
                <Route path=path!("/clients/:id") view=ClientDetailPage />
                <Route path=path!("/volunteer") view=VolunteerDashboardPage />
                <Route path=path!("/dashboard") view=DashboardPage />
                <Route path=path!("/contacts") view=ContactsPage />
                <Route path=path!("/contacts/:id") view=ContactDetailPage />
                <Route path=path!("/chat") view=ChatPage />
            </Routes>
        </Router>
    }
}
