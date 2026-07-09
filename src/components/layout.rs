use leptos::prelude::*;
use leptos_router::components::{Redirect, A};
use leptos_router::hooks::{use_location, use_navigate};

use crate::state::AppState;

/// A top-navbar link that highlights when its route is active.
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
                let base = "px-3 py-2 rounded-lg text-sm font-medium transition-colors";
                if active() {
                    format!("{base} bg-primary-500/15 text-primary-300")
                } else {
                    format!("{base} text-slate-300 hover:bg-slate-800 hover:text-slate-100")
                }
            }
        >
            {label}
        </A>
    }
}

/// App shell: a dark top navbar plus the page content. Redirects to `/login`
/// when there is no signed-in user.
#[component]
pub fn Layout(#[prop(into)] title: String, children: Children) -> impl IntoView {
    let state = expect_context::<AppState>();
    let navigate = use_navigate();

    let user = state.current_user.get_untracked();
    if user.is_none() {
        return view! { <Redirect path="/login" /> }.into_any();
    }
    let user = user.unwrap();
    let role = user.role;
    let user_name = user.full_name();
    let role_label = role.label();
    let role_badge = format!(
        "rounded-full px-2 py-0.5 text-xs font-medium {}",
        role.badge_classes(),
    );

    let logout = move |_| {
        state.logout();
        navigate("/login", Default::default());
    };

    view! {
        <div class="min-h-screen bg-slate-950 text-slate-100">
            <header class="sticky top-0 z-20 border-b border-slate-800 bg-slate-900/80 backdrop-blur">
                <div class="mx-auto max-w-7xl px-4 sm:px-6">
                    <div class="flex h-16 items-center gap-4">
                        <A href="/" attr:class="flex items-center gap-2 shrink-0">
                            <span class="grid h-8 w-8 place-items-center rounded-lg bg-primary-500/20 text-lg text-primary-400">
                                "\u{2665}"
                            </span>
                            <span class="font-semibold tracking-tight">"Mommy's Heart"</span>
                        </A>

                        <nav class="hidden md:flex items-center gap-1">
                            <NavLink href="/cases" label="Cases" />
                            <NavLink href="/inbox" label="Case Chat" />
                            {if role.is_admin() {
                                view! { <NavLink href="/grants" label="Grants" /> }.into_any()
                            } else {
                                ().into_any()
                            }}
                            {if role.is_admin() {
                                view! { <NavLink href="/admin" label="Admin" /> }.into_any()
                            } else {
                                ().into_any()
                            }}
                        </nav>

                        <div class="ml-auto flex items-center gap-3">
                            <div class="hidden sm:flex flex-col items-end leading-tight">
                                <span class="text-sm font-medium">{user_name}</span>
                                <span class=role_badge>{role_label}</span>
                            </div>
                            <button
                                on:click=logout
                                class="rounded-lg border border-slate-700 px-3 py-1.5 text-sm font-medium text-slate-300 hover:bg-slate-800 hover:text-slate-100 transition-colors"
                            >
                                "Log out"
                            </button>
                        </div>
                    </div>
                </div>
            </header>

            <main class="mx-auto max-w-7xl px-4 sm:px-6 py-8">
                <h1 class="text-2xl font-semibold tracking-tight mb-6">{title}</h1>
                {children()}
            </main>
        </div>
    }
    .into_any()
}
