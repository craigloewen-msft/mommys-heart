use leptos::prelude::*;

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
            .map(|c| {
                let status_badge = format!(
                    "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {}",
                    c.status.badge_classes(),
                );
                let prio_badge = format!(
                    "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {}",
                    c.priority.badge_classes(),
                );
                let docs = if c.documents.is_empty() {
                    view! {
                        <li class="rounded-lg bg-slate-950/70 px-3 py-1.5 text-xs text-slate-500">
                            "No documents attached."
                        </li>
                    }
                    .into_any()
                } else {
                    c.documents
                        .iter()
                        .map(|d| {
                            view! {
                                <li class="flex items-center justify-between rounded-lg bg-slate-950/70 px-3 py-1.5 text-xs">
                                    <span class="text-slate-200">{d.name.clone()}</span>
                                    <span class="text-slate-500">{d.uploaded_at.clone()}</span>
                                </li>
                            }
                        })
                        .collect_view()
                        .into_any()
                };
                view! {
                    <div class="rounded-xl border border-slate-800 bg-slate-900 p-5">
                        <div class="flex items-start justify-between gap-3">
                            <div>
                                <h3 class="font-semibold text-white">{c.title}</h3>
                                <p class="text-xs text-slate-500">{c.client_name}</p>
                            </div>
                            <div class="flex shrink-0 flex-wrap justify-end gap-1">
                                <span class=status_badge>{c.status.label()}</span>
                                <span class=prio_badge>
                                    {format!("{} priority", c.priority.label())}
                                </span>
                            </div>
                        </div>
                        <p class="mt-2 text-sm text-slate-400">{c.summary}</p>
                        <p class="mt-3 text-xs text-slate-500">
                            "Opened " {c.opened_at}
                        </p>
                        <div class="mt-3">
                            <p class="mb-1.5 text-xs font-medium uppercase tracking-wide text-slate-400">
                                "Documents"
                            </p>
                            <ul class="space-y-1.5">{docs}</ul>
                        </div>
                    </div>
                }
            })
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
