use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

use crate::pages::{
    chat::ChatPage, contact_detail::ContactDetailPage, contacts::ContactsPage,
    dashboard::DashboardPage,
};

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

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/mommys-heart-crm.css" />
        <Title text="Mommy's Heart CRM" />

        <Router>
            <Routes fallback=|| view! { <p class="p-6">"Page not found"</p> }>
                <Route path=path!("/") view=DashboardPage />
                <Route path=path!("/contacts") view=ContactsPage />
                <Route path=path!("/contacts/:id") view=ContactDetailPage />
                <Route path=path!("/chat") view=ChatPage />
            </Routes>
        </Router>
    }
}
