use leptos::prelude::*;

use crate::components::case_card::CaseCard;
use crate::components::layout::Layout;
use crate::state::AppState;
use crate::types::{Case, Role};

/// Volunteer's personal dashboard: the cases assigned to them and any attached
/// documents. Admins get a read-only overview of every case.
#[component]
pub fn VolunteerDashboardPage() -> impl IntoView {
    let state = expect_context::<AppState>();

    let user = state.current_user.get_untracked();
    let (is_admin, volunteer_id, greeting) = match user {
        Some(u) => (
            u.role == Role::Admin,
            u.volunteer_id.clone(),
            format!("Hi {}", u.name),
        ),
        None => (false, None, String::new()),
    };

    let my_cases = move || -> Vec<Case> {
        if is_admin {
            state.cases.get()
        } else if let Some(vid) = volunteer_id.clone() {
            state.cases_for_volunteer(&vid)
        } else {
            Vec::new()
        }
    };

    let cards = move || {
        let cases = my_cases();
        if cases.is_empty() {
            return view! {
                <div class="rounded-xl border border-dashed border-slate-700 bg-slate-900 p-8 text-center">
                    <p class="text-slate-300">"No cases assigned to you yet."</p>
                    <p class="mt-1 text-sm text-slate-500">
                        "An admin will assign cases to you. Check back soon."
                    </p>
                </div>
            }
            .into_any();
        }
        cases
            .into_iter()
            .map(|c| view! { <CaseCard id=c.id.clone() editable=false /> })
            .collect_view()
            .into_any()
    };

    view! {
        <Layout title="Volunteer dashboard">
            <div class="mb-6 flex items-center justify-between">
                <p class="text-slate-400">{greeting}</p>
                <Show when=move || is_admin>
                    <span class="rounded-full bg-primary-500/15 px-3 py-1 text-xs font-medium text-primary-300 ring-1 ring-primary-500/30">
                        "Admin preview \u{2014} showing all cases"
                    </span>
                </Show>
            </div>
            <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">{cards}</div>
        </Layout>
    }
}
