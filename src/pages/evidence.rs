//! Cross-case Evidence Repository: search, filter, a chronological timeline,
//! and simple client-side "pattern" summaries over the in-memory evidence store.
//! Structured only — no AI. Admins see every case; volunteers see the evidence
//! from cases assigned to them.

use std::collections::BTreeMap;

use leptos::prelude::*;
use leptos_router::components::Redirect;

use crate::components::layout::Layout;
use crate::state::{AppState, EvidenceRow};
use crate::types::{EvidenceType, ReviewStatus, Role};

/// Month key (`YYYY-MM`) used to bucket the timeline and pattern summaries.
fn month_key(occurred_on: &str) -> String {
    if occurred_on.len() >= 7 {
        occurred_on[..7].to_string()
    } else if occurred_on.is_empty() {
        "Undated".to_string()
    } else {
        occurred_on.to_string()
    }
}

/// Does an evidence row match the free-text query? Matches title, description,
/// source, party, and tags (case-insensitive).
fn matches_query(row: &EvidenceRow, q: &str) -> bool {
    if q.is_empty() {
        return true;
    }
    let q = q.to_lowercase();
    let item = &row.item;
    let haystack = format!(
        "{} {} {} {} {} {}",
        item.name,
        item.description,
        item.source,
        item.party,
        item.tags.join(" "),
        row.case_title,
    )
    .to_lowercase();
    haystack.contains(&q)
}

#[component]
pub fn EvidenceRepositoryPage() -> impl IntoView {
    let state = expect_context::<AppState>();

    // Anyone signed in may view; scope is applied per-role below.
    let user = match state.current_user.get_untracked() {
        Some(u) => u,
        None => return view! { <Redirect path="/login" /> }.into_any(),
    };
    let is_admin = user.role == Role::Admin;
    let volunteer_id = user.volunteer_id.clone();

    // Filter state.
    let search = RwSignal::new(String::new());
    let filter_type = RwSignal::new("all".to_string());
    let filter_review = RwSignal::new("all".to_string());
    let filter_case = RwSignal::new("all".to_string());
    let sort_desc = RwSignal::new(true);

    // Cases available for the case filter (role-scoped).
    let case_options = {
        let volunteer_id = volunteer_id.clone();
        move || {
            let cases = if is_admin {
                state.cases.get()
            } else if let Some(vid) = volunteer_id.clone() {
                state.cases_for_volunteer(&vid)
            } else {
                Vec::new()
            };
            cases
                .into_iter()
                .map(|c| (c.id, c.title))
                .collect::<Vec<_>>()
        }
    };

    // The role-scoped, filtered, sorted rows powering every section below.
    let rows = {
        let volunteer_id = volunteer_id.clone();
        move || -> Vec<EvidenceRow> {
            let allowed: Option<Vec<String>> = if is_admin {
                None
            } else {
                Some(
                    volunteer_id
                        .clone()
                        .map(|vid| {
                            state
                                .cases_for_volunteer(&vid)
                                .into_iter()
                                .map(|c| c.id)
                                .collect()
                        })
                        .unwrap_or_default(),
                )
            };

            let q = search.get();
            let ft = filter_type.get();
            let fr = filter_review.get();
            let fc = filter_case.get();

            let mut out: Vec<EvidenceRow> = state
                .all_evidence()
                .into_iter()
                .filter(|r| {
                    allowed
                        .as_ref()
                        .map(|ids| ids.contains(&r.case_id))
                        .unwrap_or(true)
                })
                .filter(|r| ft == "all" || r.item.evidence_type.slug() == ft)
                .filter(|r| fr == "all" || r.item.review_status.slug() == fr)
                .filter(|r| fc == "all" || r.case_id == fc)
                .filter(|r| matches_query(r, &q))
                .collect();

            out.sort_by(|a, b| a.item.occurred_on.cmp(&b.item.occurred_on));
            if sort_desc.get() {
                out.reverse();
            }
            out
        }
    };

    let input_class = "w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/40";
    let select_class = "rounded-lg border border-slate-700 bg-slate-950 px-2 py-2 text-xs text-slate-100 focus:border-primary-500 focus:outline-none";

    // --- Pattern summaries (counts by type / month / party) -----------------
    let patterns = {
        let rows = rows.clone();
        move || {
            let data = rows();
            let mut by_type: BTreeMap<&'static str, usize> = BTreeMap::new();
            let mut by_month: BTreeMap<String, usize> = BTreeMap::new();
            let mut by_party: BTreeMap<String, usize> = BTreeMap::new();
            for r in &data {
                *by_type.entry(r.item.evidence_type.label()).or_insert(0) += 1;
                *by_month.entry(month_key(&r.item.occurred_on)).or_insert(0) += 1;
                let party = if r.item.party.trim().is_empty() {
                    "Unspecified".to_string()
                } else {
                    r.item.party.clone()
                };
                *by_party.entry(party).or_insert(0) += 1;
            }

            let panel = |title: &str, mut entries: Vec<(String, usize)>| {
                // Highest counts first.
                entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
                let title = title.to_string();
                let list = entries
                    .into_iter()
                    .map(|(label, count)| {
                        view! {
                            <li class="flex items-center justify-between gap-2 text-xs">
                                <span class="truncate text-slate-300">{label}</span>
                                <span class="shrink-0 rounded-full bg-slate-800 px-2 py-0.5 font-medium text-slate-200">
                                    {count}
                                </span>
                            </li>
                        }
                    })
                    .collect_view();
                view! {
                    <div class="rounded-xl border border-slate-800 bg-slate-900 p-4">
                        <p class="mb-2 text-xs font-medium uppercase tracking-wide text-slate-400">
                            {title}
                        </p>
                        <ul class="space-y-1.5">{list}</ul>
                    </div>
                }
            };

            view! {
                <div class="grid grid-cols-1 gap-4 md:grid-cols-3">
                    {panel("By type", by_type.into_iter().map(|(k, v)| (k.to_string(), v)).collect())}
                    {panel("By month", by_month.into_iter().collect())}
                    {panel("By party", by_party.into_iter().collect())}
                </div>
            }
        }
    };

    // --- Timeline (grouped by month) ----------------------------------------
    let timeline = {
        let rows = rows.clone();
        move || {
            let data = rows();
            if data.is_empty() {
                return view! {
                    <div class="rounded-xl border border-dashed border-slate-700 bg-slate-900 p-8 text-center text-slate-400">
                        "No evidence matches the current filters."
                    </div>
                }
                .into_any();
            }

            // Preserve the sorted order while grouping into month buckets.
            let mut groups: Vec<(String, Vec<EvidenceRow>)> = Vec::new();
            for r in data {
                let key = month_key(&r.item.occurred_on);
                match groups.last_mut() {
                    Some((k, items)) if *k == key => items.push(r),
                    _ => groups.push((key, vec![r])),
                }
            }

            groups
                .into_iter()
                .map(|(month, items)| {
                    let entries = items
                        .into_iter()
                        .map(|r| {
                            let type_badge = format!(
                                "inline-flex items-center rounded-full px-2 py-0.5 text-[10px] font-medium {}",
                                r.item.evidence_type.badge_classes(),
                            );
                            let review_badge = format!(
                                "inline-flex items-center rounded-full px-2 py-0.5 text-[10px] font-medium {}",
                                r.item.review_status.badge_classes(),
                            );
                            let tags = r
                                .item
                                .tags
                                .iter()
                                .map(|t| {
                                    view! {
                                        <span class="rounded bg-slate-800 px-1.5 py-0.5 text-[10px] text-slate-400">
                                            {format!("#{t}")}
                                        </span>
                                    }
                                })
                                .collect_view();
                            view! {
                                <li class="rounded-xl border border-slate-800 bg-slate-900 p-4">
                                    <div class="flex items-start justify-between gap-3">
                                        <div class="min-w-0">
                                            <p class="font-medium text-slate-100">{r.item.name}</p>
                                            <p class="mt-0.5 text-xs text-slate-500">
                                                {format!(
                                                    "{} \u{2022} {} \u{2022} {}",
                                                    r.item.occurred_on,
                                                    r.case_title,
                                                    r.client_name,
                                                )}
                                            </p>
                                        </div>
                                        <div class="flex shrink-0 flex-col items-end gap-1">
                                            <span class=type_badge>{r.item.evidence_type.label()}</span>
                                            <span class=review_badge>{r.item.review_status.label()}</span>
                                        </div>
                                    </div>
                                    <Show when={
                                        let d = r.item.description.clone();
                                        move || !d.trim().is_empty()
                                    }>
                                        <p class="mt-2 text-sm text-slate-400">
                                            {r.item.description.clone()}
                                        </p>
                                    </Show>
                                    <div class="mt-2 flex flex-wrap items-center gap-2">
                                        <Show when={
                                            let p = r.item.party.clone();
                                            move || !p.trim().is_empty()
                                        }>
                                            <span class="text-[11px] text-slate-500">
                                                {format!("From/about: {}", r.item.party.clone())}
                                            </span>
                                        </Show>
                                        <Show when={
                                            let s = r.item.source.clone();
                                            move || !s.trim().is_empty()
                                        }>
                                            <span class="text-[11px] text-slate-500">
                                                {format!("Source: {}", r.item.source.clone())}
                                            </span>
                                        </Show>
                                        {tags}
                                    </div>
                                </li>
                            }
                        })
                        .collect_view();
                    view! {
                        <div>
                            <h3 class="mb-2 text-sm font-semibold text-primary-300">{month}</h3>
                            <ul class="space-y-3">{entries}</ul>
                        </div>
                    }
                })
                .collect_view()
                .into_any()
        }
    };

    let total = {
        let rows = rows.clone();
        move || rows().len()
    };

    view! {
        <Layout title="Evidence repository">
            <p class="mb-6 text-sm text-slate-400">
                "Search, filter, and review evidence across "
                {if is_admin { "all cases" } else { "your assigned cases" }}
                ". Metadata-only in this preview \u{2014} secure file storage arrives with the real backend."
            </p>

            // Filter / search bar
            <div class="rounded-xl border border-slate-800 bg-slate-900 p-4">
                <input
                    class=input_class
                    placeholder="Search evidence (title, notes, source, party, tags)\u{2026}"
                    prop:value=move || search.get()
                    on:input=move |ev| search.set(event_target_value(&ev))
                />
                <div class="mt-3 flex flex-wrap gap-2">
                    <select
                        class=select_class
                        prop:value=move || filter_type.get()
                        on:change=move |ev| filter_type.set(event_target_value(&ev))
                    >
                        <option value="all">"All types"</option>
                        {EvidenceType::ALL
                            .into_iter()
                            .map(|t| view! { <option value=t.slug()>{t.label()}</option> })
                            .collect_view()}
                    </select>
                    <select
                        class=select_class
                        prop:value=move || filter_review.get()
                        on:change=move |ev| filter_review.set(event_target_value(&ev))
                    >
                        <option value="all">"All review states"</option>
                        {ReviewStatus::ALL
                            .into_iter()
                            .map(|s| view! { <option value=s.slug()>{s.label()}</option> })
                            .collect_view()}
                    </select>
                    <select
                        class=select_class
                        prop:value=move || filter_case.get()
                        on:change=move |ev| filter_case.set(event_target_value(&ev))
                    >
                        <option value="all">"All cases"</option>
                        {move || {
                            case_options()
                                .into_iter()
                                .map(|(id, title)| view! { <option value=id>{title}</option> })
                                .collect_view()
                        }}
                    </select>
                    <button
                        on:click=move |_| sort_desc.update(|d| *d = !*d)
                        class="rounded-lg border border-slate-700 px-3 py-2 text-xs font-medium text-slate-200 hover:bg-slate-800"
                    >
                        {move || {
                            if sort_desc.get() {
                                "Newest first \u{2193}"
                            } else {
                                "Oldest first \u{2191}"
                            }
                        }}
                    </button>
                    <span class="ml-auto self-center text-xs text-slate-500">
                        {move || format!("{} item(s)", total())}
                    </span>
                </div>
            </div>

            // Pattern summaries
            <section class="mt-8">
                <h2 class="mb-3 text-lg font-semibold">"Patterns"</h2>
                {patterns}
            </section>

            // Timeline
            <section class="mt-8">
                <h2 class="mb-3 text-lg font-semibold">"Timeline"</h2>
                <div class="space-y-6">{timeline}</div>
            </section>
        </Layout>
    }
    .into_any()
}
