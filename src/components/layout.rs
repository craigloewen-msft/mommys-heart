use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_location;

/// A sidebar navigation link that highlights when its route is active.
#[component]
fn NavLink(href: &'static str, label: &'static str) -> impl IntoView {
    let location = use_location();
    let active = move || {
        let path = location.pathname.get();
        if href == "/" {
            path == "/"
        } else {
            path.starts_with(href)
        }
    };

    view! {
        <A
            href=href
            attr:class=move || {
                let base = "flex items-center gap-3 px-3 py-2 rounded-md text-sm font-medium transition-colors";
                if active() {
                    format!("{base} bg-primary-50 text-primary-700")
                } else {
                    format!("{base} text-gray-600 hover:bg-gray-100")
                }
            }
        >
            {label}
        </A>
    }
}

/// App shell: fixed sidebar + top bar, with the page rendered in the main area.
#[component]
pub fn Layout(#[prop(into)] title: String, children: Children) -> impl IntoView {
    view! {
        <div class="min-h-screen flex bg-gray-50 text-gray-900">
            <aside class="w-60 shrink-0 border-r border-gray-200 bg-white flex flex-col">
                <div class="h-16 flex items-center gap-2 px-4 border-b border-gray-200">
                    <span class="text-2xl text-primary-500">"\u{2665}"</span>
                    <span class="font-semibold">"Mommy's Heart CRM"</span>
                </div>
                <nav class="flex-1 p-3 space-y-1">
                    <NavLink href="/" label="Dashboard" />
                    <NavLink href="/contacts" label="Contacts" />
                    <NavLink href="/chat" label="Chat" />
                </nav>
                <div class="p-3 text-xs text-gray-400 border-t border-gray-200">
                    "v" {env!("CARGO_PKG_VERSION")}
                </div>
            </aside>

            <div class="flex-1 flex flex-col min-w-0">
                <header class="h-16 shrink-0 border-b border-gray-200 bg-white flex items-center justify-between px-6">
                    <h1 class="text-lg font-semibold">{title}</h1>
                    <A
                        href="/chat"
                        attr:class="text-sm font-medium text-primary-600 hover:text-primary-700"
                    >
                        "Ask the assistant"
                    </A>
                </header>
                <main class="flex-1 overflow-auto p-6">{children()}</main>
            </div>
        </div>
    }
}
