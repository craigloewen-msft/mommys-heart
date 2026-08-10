//! Admin dashboard tab: operations-admin approval requests.

use leptos::prelude::*;

use crate::components::admin_requests::AdminRequestCenter;

#[component]
pub fn RequestsTab(is_site_admin: bool, reload: RwSignal<u32>) -> impl IntoView {
    view! {
        <p class="mb-6 text-sm text-slate-400">
            {if is_site_admin {
                "Review active operations-admin requests and browse past decisions."
            } else {
                "Track active requests you submitted and browse their history."
            }}
        </p>
        <AdminRequestCenter is_site_admin=is_site_admin reload=reload />
    }
}
