use leptos::prelude::*;
use leptos_router::components::Redirect;

use crate::components::layout::Layout;
use crate::state::AppState;
use crate::types::{CaseStatus, Role};

/// Org-wide service pathways analytics: where need is concentrated, which
/// service categories co-occur (referral pathways), outcomes, and where
/// capacity gaps are appearing across the taxonomy.
#[component]
pub fn InsightsPage() -> impl IntoView {
    let state = expect_context::<AppState>();

    match state.current_user.get_untracked() {
        Some(u) if u.role == Role::Admin => {}
        Some(_) => return view! { <Redirect path="/volunteer" /> }.into_any(),
        None => return view! { <Redirect path="/login" /> }.into_any(),
    }

    // Headline outcome stats.
    let outcomes = move || {
        let cases = state.cases.get();
        let total = cases.len();
        let closed = cases
            .iter()
            .filter(|c| c.status == CaseStatus::Closed)
            .count();
        let open = total - closed;
        let unassigned = cases
            .iter()
            .filter(|c| c.assigned_volunteer_ids.is_empty() && c.status != CaseStatus::Closed)
            .count();
        [
            ("Clients served", state.clients.get().len()),
            ("Open needs", open),
            ("Resolved (closed)", closed),
            ("Unassigned", unassigned),
        ]
        .into_iter()
        .map(|(label, value)| {
            view! {
                <div class="rounded-xl border border-slate-800 bg-slate-900 p-5">
                    <p class="text-3xl font-semibold text-white">{value}</p>
                    <p class="mt-1 text-sm text-slate-400">{label}</p>
                </div>
            }
        })
        .collect_view()
    };

    // Demand by category (with a simple proportional bar).
    let category_rows = move || {
        let counts = state.category_counts();
        let max = counts.iter().map(|(_, n)| *n).max().unwrap_or(0).max(1);
        counts
            .into_iter()
            .map(|(cat, n)| {
                let pct = (n * 100 / max).max(if n > 0 { 4 } else { 0 });
                let bar_style = format!("width:{pct}%");
                let bar_cls = format!("h-2 rounded-full {}", bar_fill(cat.badge_classes()));
                view! {
                    <div class="flex items-center gap-3">
                        <span class="w-40 shrink-0 text-sm text-slate-300">{cat.label()}</span>
                        <div class="h-2 flex-1 overflow-hidden rounded-full bg-slate-800">
                            <div class=bar_cls style=bar_style></div>
                        </div>
                        <span class="w-8 shrink-0 text-right text-sm text-slate-400">{n}</span>
                    </div>
                }
            })
            .collect_view()
    };

    // Top service types.
    let service_rows = move || {
        let mut counts = state.service_type_counts(true);
        counts.retain(|(_, n)| *n > 0);
        counts.sort_by_key(|c| std::cmp::Reverse(c.1));
        counts.truncate(8);
        if counts.is_empty() {
            return view! {
                <p class="text-sm text-slate-500">"No open service needs recorded."</p>
            }
            .into_any();
        }
        counts
            .into_iter()
            .map(|(st, n)| {
                let cls = format!(
                    "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {}",
                    st.badge_classes(),
                );
                view! {
                    <div class="flex items-center justify-between rounded-lg bg-slate-950/60 px-3 py-1.5">
                        <span class=cls>{st.label()}</span>
                        <span class="text-sm text-slate-300">{n}</span>
                    </div>
                }
            })
            .collect_view()
            .into_any()
    };

    // Referral pathways: category co-occurrence per client.
    let pathway_rows = move || {
        let pairs = state.category_cooccurrence();
        if pairs.is_empty() {
            return view! {
                <p class="text-sm text-slate-500">
                    "No cross-category pathways yet \u{2014} clients currently have needs in a single category."
                </p>
            }
            .into_any();
        }
        pairs
            .into_iter()
            .map(|(a, b, n)| {
                let a_cls = format!(
                    "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {}",
                    a.badge_classes(),
                );
                let b_cls = format!(
                    "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {}",
                    b.badge_classes(),
                );
                view! {
                    <div class="flex items-center justify-between rounded-lg bg-slate-950/60 px-3 py-1.5">
                        <div class="flex items-center gap-1.5">
                            <span class=a_cls>{a.label()}</span>
                            <span class="text-slate-500">"\u{2194}"</span>
                            <span class=b_cls>{b.label()}</span>
                        </div>
                        <span class="text-sm text-slate-300">
                            {format!("{n} client(s)")}
                        </span>
                    </div>
                }
            })
            .collect_view()
            .into_any()
    };

    // Service gaps: open, unassigned/on-hold needs by category.
    let gap_rows = move || {
        let gaps = state.service_gaps();
        if gaps.is_empty() {
            return view! {
                <p class="text-sm text-emerald-400">
                    "No capacity gaps \u{2014} every open need is assigned and active."
                </p>
            }
            .into_any();
        }
        gaps.into_iter()
            .map(|(cat, n)| {
                let cls = format!(
                    "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {}",
                    cat.badge_classes(),
                );
                view! {
                    <div class="flex items-center justify-between rounded-lg bg-slate-950/60 px-3 py-1.5">
                        <span class=cls>{cat.label()}</span>
                        <span class="text-sm text-amber-300">
                            {format!("{n} unmet")}
                        </span>
                    </div>
                }
            })
            .collect_view()
            .into_any()
    };

    view! {
        <Layout title="Service pathways & insights">
            <div class="grid grid-cols-2 gap-4 lg:grid-cols-4">{outcomes}</div>

            <div class="mt-8 grid gap-6 lg:grid-cols-2">
                <section class="rounded-xl border border-slate-800 bg-slate-900 p-5">
                    <h2 class="mb-4 text-lg font-semibold">"Demand by category"</h2>
                    <div class="space-y-3">{category_rows}</div>
                </section>

                <section class="rounded-xl border border-slate-800 bg-slate-900 p-5">
                    <h2 class="mb-4 text-lg font-semibold">"Top service needs"</h2>
                    <div class="space-y-2">{service_rows}</div>
                </section>

                <section class="rounded-xl border border-slate-800 bg-slate-900 p-5">
                    <div class="mb-1 flex items-center justify-between">
                        <h2 class="text-lg font-semibold">"Referral pathways"</h2>
                    </div>
                    <p class="mb-4 text-xs text-slate-500">
                        "Service categories that commonly appear together for the same client."
                    </p>
                    <div class="space-y-2">{pathway_rows}</div>
                </section>

                <section class="rounded-xl border border-slate-800 bg-slate-900 p-5">
                    <div class="mb-1 flex items-center justify-between">
                        <h2 class="text-lg font-semibold">"Service gaps"</h2>
                    </div>
                    <p class="mb-4 text-xs text-slate-500">
                        "Open needs that are unassigned or on hold, by category."
                    </p>
                    <div class="space-y-2">{gap_rows}</div>
                </section>
            </div>
        </Layout>
    }
    .into_any()
}

/// Derive a solid-ish bar fill class from a category badge class string by
/// reusing its text color token as the background.
fn bar_fill(badge: &'static str) -> &'static str {
    if badge.contains("violet") {
        "bg-violet-400"
    } else if badge.contains("amber") {
        "bg-amber-400"
    } else if badge.contains("sky") {
        "bg-sky-400"
    } else if badge.contains("emerald") {
        "bg-emerald-400"
    } else if badge.contains("rose") {
        "bg-rose-400"
    } else {
        "bg-primary-400"
    }
}
