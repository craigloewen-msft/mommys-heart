//! A single, live-editable case card shared by the admin dashboard and the
//! client pathway view. It surfaces everything the org asked to track for a
//! case: category, status, assigned volunteers, notes, documents, a timeline of
//! actions taken, and cross-links to the client's other (interconnected) cases.

use leptos::prelude::*;

use crate::state::{AppState, EvidenceDraft};
use crate::taxonomy::ServiceCategory;
use crate::types::{CaseStatus, EvidenceType, ReviewStatus};

const SELECT_CLASS: &str = "rounded-lg border border-slate-700 bg-slate-950 px-2 py-1.5 text-xs text-slate-100 focus:border-primary-500 focus:outline-none";
const INPUT_CLASS: &str = "flex-1 rounded-lg border border-slate-700 bg-slate-950 px-3 py-1.5 text-xs text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none";

/// Full case card. `editable` gates the status/assignment/related controls;
/// notes and documents can always be appended.
#[component]
pub fn CaseCard(id: String, #[prop(default = true)] editable: bool) -> impl IntoView {
    let state = expect_context::<AppState>();

    let lookup_id = id.clone();
    let case = Memo::new(move |_| state.cases.get().into_iter().find(|c| c.id == lookup_id));

    view! {
        <div class="rounded-xl border border-slate-800 bg-slate-900 p-5">
            {move || match case.get() {
                None => view! {
                    <p class="text-sm text-slate-500">"This case is no longer available."</p>
                }
                    .into_any(),
                Some(c) => {
                    let status_badge = format!(
                        "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {}",
                        c.status.badge_classes(),
                    );
                    let prio_badge = format!(
                        "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {}",
                        c.priority.badge_classes(),
                    );
                    let cat_badge = format!(
                        "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {}",
                        c.category.badge_classes(),
                    );
                    let tags = c
                        .service_types
                        .iter()
                        .map(|st| {
                            let cls = format!(
                                "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {}",
                                st.badge_classes(),
                            );
                            view! { <span class=cls>{st.label()}</span> }
                        })
                        .collect_view();
                    view! {
                        <div class="flex items-start justify-between gap-3">
                            <div>
                                <h3 class="font-semibold text-white">{c.title.clone()}</h3>
                                <p class="text-xs text-slate-500">
                                    {state.client_name(&c.client_id)}
                                </p>
                            </div>
                            <div class="flex shrink-0 flex-wrap justify-end gap-1">
                                <span class=cat_badge>{c.category.label()}</span>
                                <span class=status_badge>{c.status.label()}</span>
                                <span class=prio_badge>
                                    {format!("{} priority", c.priority.label())}
                                </span>
                            </div>
                        </div>
                        <p class="mt-2 text-sm text-slate-400">{c.summary.clone()}</p>
                        <div class="mt-2 flex flex-wrap gap-1">{tags}</div>
                    }
                        .into_any()
                }
            }}

            <CaseControls id=id.clone() editable=editable case=case />
            <RelatedCases id=id.clone() editable=editable case=case />
            <CaseNotes id=id.clone() case=case />
            <CaseDocuments id=id.clone() case=case />
            <CaseEvidence id=id.clone() case=case />
            <CaseTimeline case=case />
        </div>
    }
}

#[component]
fn CaseControls(
    id: String,
    editable: bool,
    case: Memo<Option<crate::types::Case>>,
) -> impl IntoView {
    let state = expect_context::<AppState>();
    let status_id = id.clone();
    let assignments_id = id.clone();
    let services_id = id.clone();

    // Editable service-type tags, grouped by category.
    let service_editor = move || {
        let selected = case.get().map(|c| c.service_types).unwrap_or_default();
        ServiceCategory::ALL
            .into_iter()
            .map(|cat| {
                let boxes = cat
                    .types()
                    .iter()
                    .map(|st| {
                        let st = *st;
                        let checked = selected.contains(&st);
                        let case_id = services_id.clone();
                        view! {
                            <label class="flex cursor-pointer items-center gap-1.5 rounded-md border border-slate-700 px-2 py-1 text-xs text-slate-300 hover:bg-slate-800">
                                <input
                                    r#type="checkbox"
                                    class="accent-primary-500"
                                    prop:checked=checked
                                    on:change=move |_| state.toggle_case_service_type(&case_id, st)
                                />
                                {st.label()}
                            </label>
                        }
                    })
                    .collect_view();
                view! {
                    <div class="mb-2">
                        <p class="mb-1 text-[10px] font-semibold uppercase tracking-wide text-slate-500">
                            {cat.label()}
                        </p>
                        <div class="flex flex-wrap gap-1.5">{boxes}</div>
                    </div>
                }
            })
            .collect_view()
    };

    let assignments = move || {
        let assigned = case
            .get()
            .map(|c| c.assigned_volunteer_ids)
            .unwrap_or_default();
        state
            .volunteers
            .get()
            .into_iter()
            .map(|v| {
                let is_assigned = assigned.iter().any(|a| a == &v.id);
                if editable {
                    let case_id = assignments_id.clone();
                    let vid = v.id.clone();
                    view! {
                        <label class="flex cursor-pointer items-center gap-2 rounded-lg border border-slate-700 px-2 py-1 text-xs text-slate-300 hover:bg-slate-800">
                            <input
                                r#type="checkbox"
                                class="accent-primary-500"
                                prop:checked=is_assigned
                                on:change=move |_| state.toggle_case_volunteer(&case_id, &vid)
                            />
                            {v.name.clone()}
                        </label>
                    }
                    .into_any()
                } else if is_assigned {
                    view! {
                        <span class="rounded-lg border border-slate-700 px-2 py-1 text-xs text-slate-300">
                            {v.name.clone()}
                        </span>
                    }
                    .into_any()
                } else {
                    ().into_any()
                }
            })
            .collect_view()
    };

    let service_section = editable.then(|| {
        view! {
            <div class="mt-4">
                <p class="mb-1.5 text-xs font-medium uppercase tracking-wide text-slate-400">
                    "Service types"
                </p>
                {service_editor}
            </div>
        }
    });

    view! {
        <div class="mt-4 flex items-center gap-2">
            <span class="text-xs text-slate-400">"Status"</span>
            <Show
                when=move || editable
                fallback=move || {
                    view! {
                        <span class="text-xs text-slate-300">
                            {move || case.get().map(|c| c.status.label()).unwrap_or("")}
                        </span>
                    }
                }
            >
                <select
                    class=SELECT_CLASS
                    prop:value=move || case.get().map(|c| c.status.slug()).unwrap_or("open")
                    on:change={
                        let status_id = status_id.clone();
                        move |ev| {
                            if let Some(s) = CaseStatus::from_slug(&event_target_value(&ev)) {
                                state.set_case_status(&status_id, s);
                            }
                        }
                    }
                >
                    {CaseStatus::ALL
                        .into_iter()
                        .map(|s| view! { <option value=s.slug()>{s.label()}</option> })
                        .collect_view()}
                </select>
            </Show>
        </div>

        <div class="mt-4">
            <p class="mb-1.5 text-xs font-medium uppercase tracking-wide text-slate-400">
                "Assigned volunteers"
            </p>
            <div class="flex flex-wrap gap-2">{assignments}</div>
        </div>

        {service_section}
    }
}

#[component]
fn RelatedCases(
    id: String,
    editable: bool,
    case: Memo<Option<crate::types::Case>>,
) -> impl IntoView {
    let state = expect_context::<AppState>();
    let self_id = id.clone();

    // Sibling cases: same client, different case. These are the candidate
    // interconnected needs that can be linked into a pathway.
    let siblings = move || {
        let current = match case.get() {
            Some(c) => c,
            None => return ().into_any(),
        };
        let linked = current.related_case_ids.clone();
        let others: Vec<_> = state
            .cases_for_client(&current.client_id)
            .into_iter()
            .filter(|c| c.id != current.id)
            .collect();
        if others.is_empty() {
            return view! {
                <p class="text-xs text-slate-500">"No other cases for this client."</p>
            }
            .into_any();
        }
        others
            .into_iter()
            .map(|other| {
                let is_linked = linked.iter().any(|r| r == &other.id);
                let cat_badge = format!(
                    "inline-flex items-center rounded-full px-2 py-0.5 text-[10px] font-medium {}",
                    other.category.badge_classes(),
                );
                let row = view! {
                    <span class=cat_badge>{other.category.label()}</span>
                    <span class="text-xs text-slate-200">{other.title.clone()}</span>
                };
                if editable {
                    let self_id = self_id.clone();
                    let other_id = other.id.clone();
                    view! {
                        <label class="flex cursor-pointer items-center gap-2 rounded-lg border border-slate-700 px-2 py-1 hover:bg-slate-800">
                            <input
                                r#type="checkbox"
                                class="accent-primary-500"
                                prop:checked=is_linked
                                on:change=move |_| state.toggle_related_case(&self_id, &other_id)
                            />
                            {row}
                        </label>
                    }
                    .into_any()
                } else if is_linked {
                    view! {
                        <span class="flex items-center gap-2 rounded-lg border border-slate-700 px-2 py-1">
                            {row}
                        </span>
                    }
                    .into_any()
                } else {
                    ().into_any()
                }
            })
            .collect_view()
            .into_any()
    };

    view! {
        <div class="mt-4">
            <p class="mb-1.5 text-xs font-medium uppercase tracking-wide text-slate-400">
                "Related cases"
            </p>
            <div class="flex flex-wrap gap-2">{siblings}</div>
        </div>
    }
}

#[component]
fn CaseNotes(id: String, case: Memo<Option<crate::types::Case>>) -> impl IntoView {
    let state = expect_context::<AppState>();
    let note_body = RwSignal::new(String::new());
    let add_id = id.clone();
    let add_note = move |_| {
        state.add_case_note(&add_id, &note_body.get());
        note_body.set(String::new());
    };

    let notes = move || match case.get() {
        Some(c) if !c.notes.is_empty() => c
            .notes
            .into_iter()
            .rev()
            .map(|n| {
                view! {
                    <li class="rounded-lg bg-slate-950/70 px-3 py-2 text-xs">
                        <div class="flex items-center justify-between text-slate-500">
                            <span class="font-medium text-slate-300">{n.author}</span>
                            <span>{n.created_at}</span>
                        </div>
                        <p class="mt-1 text-slate-200">{n.body}</p>
                    </li>
                }
            })
            .collect_view()
            .into_any(),
        _ => view! {
            <li class="rounded-lg bg-slate-950/70 px-3 py-1.5 text-xs text-slate-500">
                "No notes yet."
            </li>
        }
        .into_any(),
    };

    view! {
        <div class="mt-4">
            <p class="mb-1.5 text-xs font-medium uppercase tracking-wide text-slate-400">
                "Case notes"
            </p>
            <ul class="space-y-1.5">{notes}</ul>
            <div class="mt-2 flex gap-2">
                <input
                    class=INPUT_CLASS
                    placeholder="Add a note\u{2026}"
                    prop:value=move || note_body.get()
                    on:input=move |ev| note_body.set(event_target_value(&ev))
                />
                <button
                    on:click=add_note
                    class="shrink-0 rounded-lg border border-slate-700 px-3 py-1.5 text-xs font-medium text-slate-200 hover:bg-slate-800"
                >
                    "Add note"
                </button>
            </div>
        </div>
    }
}

#[component]
fn CaseDocuments(id: String, case: Memo<Option<crate::types::Case>>) -> impl IntoView {
    let state = expect_context::<AppState>();
    let doc_name = RwSignal::new(String::new());
    let add_id = id.clone();
    let add_doc = move |_| {
        state.add_case_document(&add_id, &doc_name.get());
        doc_name.set(String::new());
    };

    let documents = move || {
        match case.get() {
        Some(c) if !c.documents.is_empty() => c
            .documents
            .into_iter()
            .map(|d| {
                view! {
                    <li class="flex items-center justify-between rounded-lg bg-slate-950/70 px-3 py-1.5 text-xs">
                        <span class="text-slate-200">{d.name}</span>
                        <span class="text-slate-500">{d.uploaded_at}</span>
                    </li>
                }
            })
            .collect_view()
            .into_any(),
        _ => view! {
            <li class="rounded-lg bg-slate-950/70 px-3 py-1.5 text-xs text-slate-500">
                "No documents yet."
            </li>
        }
        .into_any(),
    }
    };

    view! {
        <div class="mt-4">
            <p class="mb-1.5 text-xs font-medium uppercase tracking-wide text-slate-400">
                "Documents"
            </p>
            <ul class="space-y-1.5">{documents}</ul>
            <div class="mt-2 flex gap-2">
                <input
                    class=INPUT_CLASS
                    placeholder="Document name\u{2026}"
                    prop:value=move || doc_name.get()
                    on:input=move |ev| doc_name.set(event_target_value(&ev))
                />
                <button
                    on:click=add_doc
                    class="shrink-0 rounded-lg border border-slate-700 px-3 py-1.5 text-xs font-medium text-slate-200 hover:bg-slate-800"
                >
                    "Add document"
                </button>
            </div>
        </div>
    }
}

#[component]
fn CaseEvidence(id: String, case: Memo<Option<crate::types::Case>>) -> impl IntoView {
    let state = expect_context::<AppState>();

    let ev_name = RwSignal::new(String::new());
    let ev_type = RwSignal::new(EvidenceType::TextMessage.slug().to_string());
    let ev_date = RwSignal::new(String::new());
    let ev_party = RwSignal::new(String::new());
    let ev_source = RwSignal::new(String::new());
    let ev_tags = RwSignal::new(String::new());
    let ev_desc = RwSignal::new(String::new());
    let add_ev_id = id.clone();
    let add_evidence = move |_| {
        let draft = EvidenceDraft {
            name: ev_name.get(),
            evidence_type: EvidenceType::from_slug(&ev_type.get()).unwrap_or_default(),
            description: ev_desc.get(),
            source: ev_source.get(),
            party: ev_party.get(),
            occurred_on: ev_date.get(),
            tags: ev_tags.get().split(',').map(|t| t.to_string()).collect(),
        };
        state.add_case_evidence(&add_ev_id, draft);
        ev_name.set(String::new());
        ev_date.set(String::new());
        ev_party.set(String::new());
        ev_source.set(String::new());
        ev_tags.set(String::new());
        ev_desc.set(String::new());
    };

    let list_id = id.clone();
    let items = move || match case.get() {
        Some(c) if !c.evidence.is_empty() => c
            .evidence
            .into_iter()
            .map(|d| {
                let type_badge = format!(
                    "inline-flex items-center rounded-full px-2 py-0.5 text-[10px] font-medium {}",
                    d.evidence_type.badge_classes(),
                );
                let review_badge = format!(
                    "inline-flex items-center rounded-full px-2 py-0.5 text-[10px] font-medium {}",
                    d.review_status.badge_classes(),
                );
                let case_id = list_id.clone();
                let ev_id = d.id.clone();
                view! {
                    <li class="rounded-lg bg-slate-950/70 px-3 py-2 text-xs">
                        <div class="flex items-start justify-between gap-2">
                            <div class="min-w-0">
                                <p class="truncate font-medium text-slate-100">{d.name}</p>
                                <p class="mt-0.5 text-[11px] text-slate-500">
                                    {format!("{} \u{2022} {}", d.party, d.occurred_on)}
                                </p>
                            </div>
                            <div class="flex shrink-0 flex-col items-end gap-1">
                                <span class=type_badge>{d.evidence_type.label()}</span>
                                <span class=review_badge>{d.review_status.label()}</span>
                            </div>
                        </div>
                        <div class="mt-2 flex items-center gap-2">
                            <span class="text-[10px] uppercase tracking-wide text-slate-500">
                                "Review"
                            </span>
                            <select
                                class="rounded-md border border-slate-700 bg-slate-950 px-1.5 py-1 text-[11px] text-slate-100 focus:border-primary-500 focus:outline-none"
                                prop:value=d.review_status.slug()
                                on:change=move |ev| {
                                    if let Some(s) = ReviewStatus::from_slug(
                                        &event_target_value(&ev),
                                    ) {
                                        state.set_evidence_review_status(&case_id, &ev_id, s);
                                    }
                                }
                            >
                                {ReviewStatus::ALL
                                    .into_iter()
                                    .map(|s| view! { <option value=s.slug()>{s.label()}</option> })
                                    .collect_view()}
                            </select>
                        </div>
                    </li>
                }
            })
            .collect_view()
            .into_any(),
        _ => view! {
            <li class="rounded-lg bg-slate-950/70 px-3 py-1.5 text-xs text-slate-500">
                "No evidence yet."
            </li>
        }
        .into_any(),
    };

    view! {
        <div class="mt-4">
            <p class="mb-1.5 text-xs font-medium uppercase tracking-wide text-slate-400">
                "Evidence"
            </p>
            <ul class="space-y-1.5">{items}</ul>
            <div class="mt-3 rounded-lg border border-slate-800 bg-slate-950/40 p-3">
                <p class="mb-2 text-[10px] font-medium uppercase tracking-wide text-slate-500">
                    "Add evidence"
                </p>
                <div class="grid gap-2 sm:grid-cols-2">
                    <input
                        class=INPUT_CLASS
                        placeholder="Title / description\u{2026}"
                        prop:value=move || ev_name.get()
                        on:input=move |ev| ev_name.set(event_target_value(&ev))
                    />
                    <select
                        class=SELECT_CLASS
                        prop:value=move || ev_type.get()
                        on:change=move |ev| ev_type.set(event_target_value(&ev))
                    >
                        {EvidenceType::ALL
                            .into_iter()
                            .map(|t| view! { <option value=t.slug()>{t.label()}</option> })
                            .collect_view()}
                    </select>
                    <input
                        r#type="date"
                        class=INPUT_CLASS
                        prop:value=move || ev_date.get()
                        on:input=move |ev| ev_date.set(event_target_value(&ev))
                    />
                    <input
                        class=INPUT_CLASS
                        placeholder="Party (who it's from/about)"
                        prop:value=move || ev_party.get()
                        on:input=move |ev| ev_party.set(event_target_value(&ev))
                    />
                    <input
                        class=INPUT_CLASS
                        placeholder="Source (device / platform)"
                        prop:value=move || ev_source.get()
                        on:input=move |ev| ev_source.set(event_target_value(&ev))
                    />
                    <input
                        class=INPUT_CLASS
                        placeholder="Tags (comma separated)"
                        prop:value=move || ev_tags.get()
                        on:input=move |ev| ev_tags.set(event_target_value(&ev))
                    />
                </div>
                <textarea
                    class=format!("{INPUT_CLASS} mt-2")
                    rows="2"
                    placeholder="Notes / context\u{2026}"
                    prop:value=move || ev_desc.get()
                    on:input=move |ev| ev_desc.set(event_target_value(&ev))
                ></textarea>
                <button
                    on:click=add_evidence
                    class="mt-2 rounded-lg border border-slate-700 px-3 py-1.5 text-xs font-medium text-slate-200 hover:bg-slate-800"
                >
                    "Add evidence"
                </button>
            </div>
        </div>
    }
}

#[component]
fn CaseTimeline(case: Memo<Option<crate::types::Case>>) -> impl IntoView {
    let timeline = move || match case.get() {
        Some(c) if !c.timeline.is_empty() => c
            .timeline
            .into_iter()
            .rev()
            .map(|e| {
                let dot = format!(
                    "mt-1 h-2 w-2 shrink-0 rounded-full {}",
                    e.kind.dot_classes()
                );
                view! {
                    <li class="flex gap-2">
                        <span class=dot></span>
                        <div>
                            <p class="text-xs text-slate-200">{e.summary}</p>
                            <p class="text-[10px] uppercase tracking-wide text-slate-500">
                                {e.kind.label()} " \u{00b7} " {e.at}
                            </p>
                        </div>
                    </li>
                }
            })
            .collect_view()
            .into_any(),
        _ => view! {
            <li class="text-xs text-slate-500">"No activity recorded yet."</li>
        }
        .into_any(),
    };

    view! {
        <div class="mt-4">
            <p class="mb-1.5 text-xs font-medium uppercase tracking-wide text-slate-400">
                "Timeline"
            </p>
            <ul class="space-y-2">{timeline}</ul>
        </div>
    }
}
