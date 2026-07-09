use leptos::prelude::*;
use leptos_router::components::Redirect;

use crate::analytics;
use crate::components::charts::{Donut, HBarList, ProgressBar, StatCard, TrendLine};
use crate::components::layout::Layout;
use crate::state::AppState;
use crate::types::Role;

/// A titled panel that frames a chart or table.
#[component]
fn ChartCard(
    #[prop(into)] title: String,
    #[prop(into, optional)] subtitle: String,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="rounded-xl border border-slate-800 bg-slate-900 p-5">
            <div class="mb-4">
                <h3 class="font-semibold text-white">{title}</h3>
                <Show when={
                    let s = subtitle.clone();
                    move || !s.is_empty()
                }>
                    <p class="mt-0.5 text-xs text-slate-500">{subtitle.clone()}</p>
                </Show>
            </div>
            {children()}
        </div>
    }
}

/// A lightweight section heading.
#[component]
fn SectionTitle(#[prop(into)] title: String, #[prop(into)] blurb: String) -> impl IntoView {
    view! {
        <div class="mb-4 mt-10 first:mt-0">
            <h2 class="text-lg font-semibold text-white">{title}</h2>
            <p class="mt-0.5 text-sm text-slate-400">{blurb}</p>
        </div>
    }
}

/// Admin-only analytics, outcome-tracking, and program-reporting dashboard.
///
/// Everything is derived reactively from the in-memory demo store, so editing
/// volunteers/cases on the admin screens updates these metrics live.
#[component]
pub fn AnalyticsPage() -> impl IntoView {
    let state = expect_context::<AppState>();

    // Role guard — matches the other admin surfaces.
    match state.current_user.get_untracked() {
        Some(u) if u.role == Role::Admin => {}
        Some(_) => return view! { <Redirect path="/volunteer" /> }.into_any(),
        None => return view! { <Redirect path="/login" /> }.into_any(),
    }

    // --- Program overview stat cards -------------------------------------
    let overview_cards = move || {
        let cases = state.cases.get();
        let o = analytics::program_overview(&cases);
        let resolution = o
            .avg_days_to_resolution
            .map(|d| format!("{d:.0} days"))
            .unwrap_or_else(|| "\u{2014}".into());
        view! {
            <div class="grid grid-cols-2 gap-4 lg:grid-cols-4">
                <StatCard label="Clients served" value=o.clients_served.to_string() />
                <StatCard
                    label="Open matters"
                    value=o.open_matters.to_string()
                    sublabel=format!("{} closed", o.closed_matters)
                />
                <StatCard
                    label="Follow-up completion"
                    value=format!("{:.0}%", o.follow_up_completion_rate)
                />
                <StatCard
                    label="Avg. intake \u{2192} resolution"
                    value=resolution
                    sublabel="across resolved cases".to_string()
                />
            </div>
        }
    };

    // --- Programmatic metrics --------------------------------------------
    let matters_chart =
        move || view! { <HBarList data=analytics::matters_by_type(&state.cases.get()) /> };
    let trend_chart =
        move || view! { <TrendLine data=analytics::matters_opened_by_month(&state.cases.get()) /> };

    // --- Client & service metrics ----------------------------------------
    let outcome_chart =
        move || view! { <Donut data=analytics::cases_by_outcome(&state.cases.get()) /> };
    let status_chart =
        move || view! { <Donut data=analytics::cases_by_status(&state.cases.get()) /> };
    let service_cards = move || {
        let cases = state.cases.get();
        let o = analytics::program_overview(&cases);
        view! {
            <div class="space-y-4">
                <div class="grid grid-cols-2 gap-4">
                    <StatCard label="Referrals made" value=o.referrals_made.to_string() />
                    <StatCard label="Services provided" value=o.services_provided.to_string() />
                </div>
                <ProgressBar
                    label="Follow-up completion rate".to_string()
                    percent=o.follow_up_completion_rate
                />
            </div>
        }
    };

    // --- Volunteer metrics -----------------------------------------------
    let hours_chart =
        move || view! { <HBarList data=analytics::volunteer_hours(&state.volunteers.get()) /> };
    let workload_chart = move || {
        view! {
            <HBarList data=analytics::workload_distribution(&state.volunteers.get(), &state.cases.get()) />
        }
    };
    let training_bar = move || {
        let rate = analytics::training_participation_rate(&state.volunteers.get());
        view! {
            <ProgressBar
                label="Volunteers with completed training".to_string()
                percent=rate
                color="bg-primary-500"
            />
        }
    };
    let volunteer_table = move || {
        let rows = analytics::volunteer_metrics(&state.volunteers.get(), &state.cases.get());
        rows.into_iter()
            .map(|m| {
                view! {
                    <tr class="border-b border-slate-800 last:border-0">
                        <td class="px-4 py-3 font-medium text-slate-100">{m.name}</td>
                        <td class="px-4 py-3 text-right text-slate-300">
                            {format!("{:.1}", m.hours_logged)}
                        </td>
                        <td class="px-4 py-3 text-right text-slate-300">
                            {format!("{} / {}", m.open_case_load, m.total_case_load)}
                        </td>
                        <td class="px-4 py-3 text-right text-slate-300">
                            {m.client_contacts.to_string()}
                        </td>
                        <td class="px-4 py-3 text-right text-slate-300">
                            {format!("{:.0} hrs/wk", m.weekly_availability_hours)}
                        </td>
                        <td class="px-4 py-3 text-right text-slate-300">
                            {m.trainings_completed.to_string()}
                        </td>
                    </tr>
                }
            })
            .collect_view()
    };

    view! {
        <Layout title="Analytics & reporting">
            <p class="-mt-2 mb-6 max-w-3xl text-sm text-slate-400">
                "Program-wide outcomes, operational metrics, and service utilization \u{2014} \
                intended to surface trends, measure impact, and support grant reporting. All \
                figures update live from the current case and volunteer records."
            </p>

            {overview_cards}

            <SectionTitle
                title="Programmatic metrics"
                blurb="Matters by type and volume over time \u{2014} where community need is concentrated."
            />
            <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
                <ChartCard title="Matters by type" subtitle="Active + closed cases by category">
                    {matters_chart}
                </ChartCard>
                <ChartCard title="Matters opened over time" subtitle="New cases per month">
                    {trend_chart}
                </ChartCard>
            </div>

            <SectionTitle
                title="Client & service metrics"
                blurb="Service delivery, outcomes, and follow-through."
            />
            <div class="grid grid-cols-1 gap-4 lg:grid-cols-3">
                <ChartCard title="Case outcomes">{outcome_chart}</ChartCard>
                <ChartCard title="Case status">{status_chart}</ChartCard>
                <ChartCard title="Service delivery">{service_cards}</ChartCard>
            </div>

            <SectionTitle
                title="Volunteer metrics"
                blurb="Hours, workload distribution, contacts, availability, and training."
            />
            <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
                <ChartCard title="Volunteer hours" subtitle="Total service hours logged">
                    {hours_chart}
                </ChartCard>
                <ChartCard title="Case workload distribution" subtitle="Open cases per volunteer">
                    {workload_chart}
                </ChartCard>
            </div>
            <div class="mt-4">
                <ChartCard title="Training participation">{training_bar}</ChartCard>
            </div>
            <div class="mt-4 overflow-hidden rounded-xl border border-slate-800 bg-slate-900">
                <table class="w-full text-sm">
                    <thead class="border-b border-slate-800 text-slate-400">
                        <tr>
                            <th class="px-4 py-3 text-left font-medium">"Volunteer"</th>
                            <th class="px-4 py-3 text-right font-medium">"Hours"</th>
                            <th class="px-4 py-3 text-right font-medium">"Open / total cases"</th>
                            <th class="px-4 py-3 text-right font-medium">"Client contacts"</th>
                            <th class="px-4 py-3 text-right font-medium">"Availability"</th>
                            <th class="px-4 py-3 text-right font-medium">"Trainings"</th>
                        </tr>
                    </thead>
                    <tbody>{volunteer_table}</tbody>
                </table>
            </div>

            <p class="mt-8 rounded-lg border border-slate-800 bg-slate-900/60 p-4 text-xs text-slate-500">
                "Reporting note: these dashboards are a proof-of-concept over demo data. In a \
                production deployment they would be backed by persisted records and could be \
                exported (CSV/PDF) for grant and program-evaluation reporting."
            </p>
        </Layout>
    }
    .into_any()
}
