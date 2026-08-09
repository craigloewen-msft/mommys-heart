//! The admin dashboard shell: the guard, the tab bar, and one component per tab.

pub mod case_access;
pub mod case_requests;
pub mod requests;
pub mod user_card;
pub mod volunteers;

use leptos::prelude::*;

use crate::components::guard::require_operations_admin;
use crate::components::layout::Layout;
use crate::pages::admin::case_access::CaseAccessTab;
use crate::pages::admin::case_requests::CaseRequestsTab;
use crate::pages::admin::requests::RequestsTab;
use crate::pages::admin::volunteers::VolunteersTab;
use crate::state::AppState;

/// Which section of the admin dashboard is showing.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AdminTab {
    CaseRequests,
    Volunteers,
    CaseAccess,
    Requests,
}

/// Admin dashboard: review case requests, browse volunteers, and manage users'
/// roles and per-case capabilities.
#[component]
pub fn AdminDashboardPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    // Bumped after a mutation so tabs refetch their current window.
    let reload = RwSignal::new(0u32);
    // Case requests lead: an unanswered intake is somebody waiting for help.
    let tab = RwSignal::new(AdminTab::CaseRequests);

    require_operations_admin(state, move || {
        let actor = state
            .current_user_summary
            .get()
            .expect("admin guard requires a current user");
        let is_site_admin = actor.role.is_site_admin();
        let actor_user_id = actor.id;

        view! {
            <Layout title="Admin".to_string()>
                <div class="mb-6 border-b border-slate-800" role="tablist" aria-label="Admin sections">
                    <div class="flex gap-6">
                        <TabButton
                            tab=tab
                            this_tab=AdminTab::CaseRequests
                            label="Case Requests"
                            // Any admin can review a case, so not site-admin gated.
                            badge=Signal::derive(move || state.cases_pending_review.get())
                        />
                        <TabButton
                            tab=tab
                            this_tab=AdminTab::Volunteers
                            label="Volunteers"
                            badge=Signal::derive(move || state.volunteer_requests_pending.get())
                        />
                        <TabButton tab=tab this_tab=AdminTab::CaseAccess label="Case access" />
                        <TabButton
                            tab=tab
                            this_tab=AdminTab::Requests
                            label="Requests"
                            badge=Signal::derive(move || {
                                if is_site_admin { state.admin_request_pending.get() } else { 0 }
                            })
                        />
                    </div>
                </div>

                <div role="tabpanel" class:hidden=move || tab.get() != AdminTab::CaseRequests>
                    <CaseRequestsTab reload=reload />
                </div>

                <div role="tabpanel" class:hidden=move || tab.get() != AdminTab::Volunteers>
                    <VolunteersTab
                        active=Signal::derive(move || tab.get() == AdminTab::Volunteers)
                        is_site_admin=is_site_admin
                    />
                </div>

                <div role="tabpanel" class:hidden=move || tab.get() != AdminTab::CaseAccess>
                    <CaseAccessTab
                        active=Signal::derive(move || tab.get() == AdminTab::CaseAccess)
                        is_site_admin=is_site_admin
                        actor_user_id=actor_user_id
                        reload=reload
                    />
                </div>

                <div role="tabpanel" class:hidden=move || tab.get() != AdminTab::Requests>
                    <RequestsTab is_site_admin=is_site_admin reload=reload />
                </div>
            </Layout>
        }
        .into_any()
    })
}

/// One tab in the dashboard's tab bar, with an optional "needs attention" count.
#[component]
fn TabButton(
    tab: RwSignal<AdminTab>,
    this_tab: AdminTab,
    label: &'static str,
    #[prop(optional, into)] badge: Option<Signal<i64>>,
) -> impl IntoView {
    view! {
        <button
            type="button"
            role="tab"
            aria-selected=move || (tab.get() == this_tab).to_string()
            on:click=move |_| tab.set(this_tab)
            class=move || if tab.get() == this_tab {
                "border-b-2 border-primary-400 px-1 pb-3 text-sm font-medium text-primary-300"
            } else {
                "border-b-2 border-transparent px-1 pb-3 text-sm font-medium text-slate-400 hover:text-slate-200"
            }
        >
            <span class="inline-flex items-center gap-2">
                {label}
                {move || {
                    let count = badge.map(|b| b.get()).unwrap_or(0);
                    (count > 0).then(|| view! {
                        <span class="inline-flex min-w-5 items-center justify-center rounded-full bg-primary-500 px-1.5 py-0.5 text-[0.65rem] font-semibold leading-none text-white">
                            {count}
                        </span>
                    })
                }}
            </span>
        </button>
    }
}
