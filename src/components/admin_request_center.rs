//! Admin dashboard tab: separate administrative request queues.

use leptos::prelude::*;

use crate::components::admin_requests::AdminRequestCenter;
use crate::server_fns::admin_requests::AdminRequestKind;

#[component]
pub fn RequestsTab(is_site_admin: bool, reload: RwSignal<u32>) -> impl IntoView {
    view! {
        <p class="mb-6 text-sm text-slate-400">
            {if is_site_admin {
                "Review active case-permission, information-access, and role requests, then browse each history separately."
            } else {
                "Track your active case-permission, information-access, and role requests, then browse each history separately."
            }}
        </p>
        <div class="space-y-10">
            <AdminRequestCenter
                kind=AdminRequestKind::CaseCapabilities
                is_site_admin=is_site_admin
                reload=reload
            />
            <AdminRequestCenter
                kind=AdminRequestKind::InformationAccess
                is_site_admin=is_site_admin
                reload=reload
            />
            <AdminRequestCenter kind=AdminRequestKind::Role is_site_admin=is_site_admin reload=reload />
        </div>
    }
}
