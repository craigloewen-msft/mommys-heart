//! Admin dashboard tab: cases waiting on an accept/decline decision.

use leptos::prelude::*;

use crate::components::admin_cases::CaseRequests;

#[component]
pub fn CaseRequestsTab(reload: RwSignal<u32>) -> impl IntoView {
    view! {
        <p class="mb-4 text-sm text-slate-400">
            "Cases people have submitted that are waiting on a decision. Accepting one lets you assign a volunteer from the Case access tab; declining tells the client why."
        </p>
        <CaseRequests reload=reload />
    }
}
