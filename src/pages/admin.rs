//! Route-backed administrative workspaces for cases and users.

use leptos::prelude::*;
use leptos_router::components::{Redirect, A};
use leptos_router::hooks::use_params_map;

use crate::components::admin_manage_cases::ManageCases;
use crate::components::admin_manage_users::ManageUsers;
use crate::components::email_failures::EmailFailureLog;
use crate::components::guard::require_operations_admin;
use crate::components::layout::Layout;
use crate::state::AppState;

#[derive(Clone, Copy, PartialEq, Eq)]
enum AdminWorkspace {
    Cases,
    Users,
}

/// `/admin` has one obvious starting place.
#[component]
pub fn AdminDashboardPage() -> impl IntoView {
    view! { <Redirect path="/admin/cases" /> }
}

#[component]
pub fn AdminCasesPage() -> impl IntoView {
    admin_page(AdminWorkspace::Cases, None)
}

#[component]
pub fn AdminCaseDetailPage() -> impl IntoView {
    let params = use_params_map();
    move || {
        let case_id = params.read().get("id").filter(|id| !id.trim().is_empty());
        admin_page(AdminWorkspace::Cases, case_id)
    }
}

#[component]
pub fn AdminUsersPage() -> impl IntoView {
    admin_page(AdminWorkspace::Users, None)
}

#[component]
pub fn AdminUserDetailPage() -> impl IntoView {
    let params = use_params_map();
    move || {
        let user_id = params.read().get("id").filter(|id| !id.trim().is_empty());
        admin_page(AdminWorkspace::Users, user_id)
    }
}

fn admin_page(workspace: AdminWorkspace, selected_id: Option<String>) -> AnyView {
    let state = expect_context::<AppState>();
    let reload = RwSignal::new(0u32);
    let selected_id = StoredValue::new(selected_id);

    require_operations_admin(state, move || {
        let actor = state
            .current_user_summary
            .get()
            .expect("admin guard requires a current user");
        let is_site_admin = actor.role.is_site_admin();
        let actor_user_id = actor.id;
        let case_attention = Signal::derive(move || {
            state.cases_pending_review.get() + state.admin_case_request_pending.get()
        });
        let user_attention = Signal::derive(move || {
            state.volunteer_requests_pending.get() + state.admin_role_request_pending.get()
        });

        let content = match workspace {
            AdminWorkspace::Cases => view! {
                <ManageCases
                    is_site_admin=is_site_admin
                    reload=reload
                    selected_case_id=selected_id.get_value()
                />
            }
            .into_any(),
            AdminWorkspace::Users => view! {
                <ManageUsers
                    is_site_admin=is_site_admin
                    actor_user_id=actor_user_id
                    reload=reload
                    selected_user_id=selected_id.get_value()
                />
            }
            .into_any(),
        };

        view! {
            <Layout title="Admin".to_string()>
                <div class="mb-8">
                    <p class="text-sm text-slate-400">
                        "Review pending work, inspect records, and manage access from two focused workspaces."
                    </p>
                    <nav
                        class="mt-5 grid grid-cols-2 gap-2 rounded-xl border border-slate-800 bg-slate-900 p-1.5 sm:inline-grid sm:min-w-[30rem]"
                        aria-label="Admin workspaces"
                    >
                        <WorkspaceLink
                            href="/admin/cases"
                            label="Manage cases"
                            selected=workspace == AdminWorkspace::Cases
                            badge=case_attention
                        />
                        <WorkspaceLink
                            href="/admin/users"
                            label="Manage users"
                            selected=workspace == AdminWorkspace::Users
                            badge=user_attention
                        />
                    </nav>
                </div>

                {content}

                <AdminTools />
            </Layout>
        }
        .into_any()
    })
}

#[component]
fn WorkspaceLink(
    href: &'static str,
    label: &'static str,
    selected: bool,
    #[prop(into)] badge: Signal<i64>,
) -> impl IntoView {
    let classes = if selected {
        "flex items-center justify-center gap-2 rounded-lg bg-primary-500/15 px-4 py-2.5 text-sm font-semibold text-primary-200 ring-1 ring-primary-500/30"
    } else {
        "flex items-center justify-center gap-2 rounded-lg px-4 py-2.5 text-sm font-medium text-slate-400 hover:bg-slate-800 hover:text-slate-100"
    };
    view! {
        <A
            href=href
            attr:class=classes
            attr:aria-current=selected.then_some("page")
        >
            {label}
            {move || {
                let count = badge.get();
                (count > 0).then(|| view! {
                    <span class="inline-flex min-w-5 items-center justify-center rounded-full bg-primary-500 px-1.5 py-0.5 text-[0.65rem] font-semibold leading-none text-white">
                        {count}
                    </span>
                })
            }}
        </A>
    }
}

#[component]
fn AdminTools() -> impl IntoView {
    let open = RwSignal::new(false);
    view! {
        <section class="mt-10 border-t border-slate-800 pt-6">
            <h2>
                <button
                    type="button"
                    on:click=move |_| open.update(|value| *value = !*value)
                    aria-expanded=move || open.get().to_string()
                    aria-controls="admin-tools-content"
                    class="flex w-full items-center justify-between gap-3 text-left"
                >
                    <span>
                        <span class="block text-base font-semibold text-slate-100">"Admin tools"</span>
                        <span class="mt-1 block text-xs font-normal text-slate-500">
                            "Operational diagnostics kept separate from case and user management."
                        </span>
                    </span>
                    <span class="text-xs font-medium text-slate-400">
                        {move || if open.get() { "Hide" } else { "Show" }}
                    </span>
                </button>
            </h2>
            <Show when=move || open.get()>
                <div id="admin-tools-content" class="mt-4 rounded-xl border border-slate-800 bg-slate-900 p-5">
                    <h3 class="text-sm font-semibold text-slate-200">"Email delivery failures"</h3>
                    <p class="mt-1 text-xs text-slate-500">
                        "Outbound emails that failed to send, newest first."
                    </p>
                    <EmailFailureLog />
                </div>
            </Show>
        </section>
    }
}
