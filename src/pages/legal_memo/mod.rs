mod approved;
mod editor;
mod editor_tables;
mod read;
mod read_tables;
mod review;

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_params_map;

use crate::components::case_tabs::CaseTab;
use crate::components::design::{
    DarkBanner, Icon, PageHeader, SectionCard, StatusPill, Tone, VolunteersOnlyBadge, MUTED,
    SERIF_H1,
};
use crate::components::guard::require_login;
use crate::components::layout::Layout;
use crate::components::case_overview::month_day;
use crate::components::legal_memos::{matter_label, memo_status_tone, ERRTEXT};
use crate::components::loading::Loading;
use crate::server_fns::err_text;
use crate::server_fns::legal_memos::{
    legal_memo_versions, load_legal_memo, load_legal_memo_version, LegalMemo, MemoStatus,
    MemoVersionEntry, MemoVersionSnapshot, VersionEvent,
};
use crate::state::AppState;

use approved::{ApprovedAside, Filed};
use editor::MemoEditor;
use read::{content_cards, header_card};
use review::ReviewPanel;

pub(super) struct Section {
    pub anchor: &'static str,
    pub number: Option<&'static str>,
    pub title: &'static str,
    pub required: bool,
    pub problem_key: &'static str,
}

pub(super) const SECTIONS: &[Section] = &[
    Section {
        anchor: "mh",
        number: None,
        title: "Header",
        required: true,
        problem_key: "Header",
    },
    Section {
        anchor: "ms",
        number: None,
        title: "Safety",
        required: true,
        problem_key: "Safety",
    },
    Section {
        anchor: "s1",
        number: Some("1"),
        title: "Questions presented",
        required: true,
        problem_key: "1 Questions presented",
    },
    Section {
        anchor: "s2",
        number: Some("2"),
        title: "Brief answer",
        required: true,
        problem_key: "2 Brief answer",
    },
    Section {
        anchor: "s3",
        number: Some("3"),
        title: "Relevant facts",
        required: true,
        problem_key: "3 Relevant facts",
    },
    Section {
        anchor: "s4",
        number: Some("4"),
        title: "Procedural history",
        required: false,
        problem_key: "4 Procedural history",
    },
    Section {
        anchor: "s5",
        number: Some("5"),
        title: "Evidence log",
        required: false,
        problem_key: "5 Evidence log",
    },
    Section {
        anchor: "s6",
        number: Some("6"),
        title: "Authorities",
        required: false,
        problem_key: "6 Authorities",
    },
    Section {
        anchor: "s7",
        number: Some("7"),
        title: "Analysis",
        required: false,
        problem_key: "7 Analysis",
    },
    Section {
        anchor: "s8",
        number: Some("8"),
        title: "Outstanding questions",
        required: false,
        problem_key: "8 Outstanding questions",
    },
    Section {
        anchor: "s9",
        number: Some("9"),
        title: "Proposed next steps",
        required: false,
        problem_key: "9 Proposed next steps",
    },
];

pub(super) fn section_card(anchor: &str, error: Signal<bool>, body: AnyView) -> AnyView {
    let s = section(anchor);
    match s.number {
        Some(n) => view! {
            <SectionCard title=s.title anchor=s.anchor number=n required=s.required error=error>{body}</SectionCard>
        }
        .into_any(),
        None => view! {
            <SectionCard title=s.title anchor=s.anchor required=s.required error=error>{body}</SectionCard>
        }
        .into_any(),
    }
}

pub(super) fn section(anchor: &str) -> &'static Section {
    SECTIONS
        .iter()
        .find(|s| s.anchor == anchor)
        .unwrap_or(&SECTIONS[0])
}

pub(super) const FACT_HELP: &str = "Tag every fact with where it came from: Client reports (the client told us), Document (we have the paper), Third party (someone else told us), Verified (checked against an official record).";
pub(super) const PROCEDURE_HELP: &str =
    "A hearing date can't be marked Confirmed without a source document, such as a court notice.";
pub(super) const EVIDENCE_HELP: &str =
    "\"Reviewed\" can't be chosen until the item has a date received.";
pub(super) const AUTHORITY_HELP: &str = "Bracketed values show the format only. Free databases don't track later rulings that overturn or limit a case, so check before relying on one.";

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn parts(stamp: &str) -> Option<(&'static str, u32, &str)> {
    let month = stamp
        .get(5..7)?
        .parse::<usize>()
        .ok()
        .filter(|m| (1..=12).contains(m))?;
    let day = stamp.get(8..10)?.parse::<u32>().ok()?;
    Some((MONTHS[month - 1], day, stamp.get(..4)?))
}

pub(super) fn date_long(stamp: &str) -> String {
    parts(stamp)
        .map(|(m, d, y)| format!("{m} {d}, {y}"))
        .unwrap_or_else(|| stamp.to_string())
}

pub(super) fn clock(hhmm: &str) -> Option<String> {
    let (h, min) = hhmm.get(..5)?.split_once(':')?;
    let h = h.parse::<u32>().ok()?;
    let suffix = if h >= 12 { "PM" } else { "AM" };
    let h12 = match h % 12 {
        0 => 12,
        other => other,
    };
    Some(format!("{h12}:{min} {suffix}"))
}

pub(super) fn when(stamp: &str) -> String {
    let Some((m, d, _)) = parts(stamp) else {
        return stamp.to_string();
    };
    match stamp.get(11..).and_then(clock) {
        Some(t) => format!("{m} {d}, {t}"),
        None => format!("{m} {d}"),
    }
}

pub(super) fn back_href(case_id: &str) -> String {
    CaseTab::Legal.href(case_id)
}

pub(super) fn back_label(memo: &LegalMemo) -> String {
    format!("{} \u{b7} Legal", memo.case_name)
}

pub(super) fn matter(memo: &LegalMemo) -> String {
    matter_label(&memo.case_name, &memo.docket)
}

pub(super) fn status_pill(memo: &LegalMemo) -> AnyView {
    let label = format!("{} \u{b7} version {}", memo.status.label(), memo.version);
    view! { <StatusPill label=label tone=memo_status_tone(memo.status) /> }.into_any()
}

#[component]
pub fn LegalMemoPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    let params = use_params_map();
    let case_id = StoredValue::new(params.read_untracked().get("case_id").unwrap_or_default());
    let memo_id = StoredValue::new(params.read_untracked().get("memo_id").unwrap_or_default());
    let memo = RwSignal::new(None::<LegalMemo>);
    let versions = RwSignal::new(Vec::<MemoVersionEntry>::new());
    let versions_error = RwSignal::new(String::new());
    let loading = RwSignal::new(true);
    let load_error = RwSignal::new(String::new());
    let reload = RwSignal::new(0u32);
    let generation = RwSignal::new(0u32);
    let viewing = RwSignal::new(None::<MemoVersionSnapshot>);

    Effect::new(move |_| {
        reload.track();
        if !state.is_authenticated() {
            return;
        }
        let id = memo_id.get_value();
        let expected_case = case_id.get_value();
        generation.update(|g| *g += 1);
        let mine = generation.get_untracked();
        spawn_local(async move {
            let result = load_legal_memo(id.clone()).await;
            let history = legal_memo_versions(id).await;
            if generation.try_get_untracked() != Some(mine) {
                return;
            }
            let _ = loading.try_set(false);
            match history {
                Ok(list) => {
                    let _ = versions.try_set(list);
                    let _ = versions_error.try_set(String::new());
                }
                Err(e) => {
                    let _ = versions_error.try_set(err_text(e));
                }
            }
            match result {
                Ok(found) if found.case_id == expected_case => {
                    let _ = load_error.try_set(String::new());
                    let _ = viewing.try_set(None);
                    let _ = memo.try_set(Some(found));
                }
                Ok(_) => {
                    let _ = load_error.try_set("Legal memo not found.".to_string());
                }
                Err(e) => {
                    let _ = load_error.try_set(err_text(e));
                }
            }
        });
    });

    let on_changed = Callback::new(move |_: ()| reload.update(|r| *r += 1));

    require_login(state, move || {
        view! {
            <Layout title="Legal memo".to_string() hide_heading=true>
                <div class="flex flex-col gap-4">
                    <Show when=move || memo.with(Option::is_some) && !load_error.get().is_empty()>
                        <p class="rounded-2xl border-2 border-danger-600 bg-danger-50 px-5 py-3 text-[14.5px] font-semibold text-danger-700" role="alert">
                            {move || format!("Couldn't refresh this memo: {}", load_error.get())}
                        </p>
                    </Show>
                    <Show when=move || !versions_error.get().is_empty()>
                        <p class=ERRTEXT role="alert">
                            {move || format!("Couldn't load the version history: {}", versions_error.get())}
                        </p>
                    </Show>
                    {move || {
                        if let Some(found) = memo.get() {
                            return if found.can_edit {
                                view! { <MemoEditor memo=found versions=versions on_changed=on_changed /> }.into_any()
                            } else {
                                view! { <MemoView memo=found versions=versions viewing=viewing on_changed=on_changed /> }.into_any()
                            };
                        }
                        if loading.get() {
                            return view! { <Loading label="Loading legal memo\u{2026}" /> }.into_any();
                        }
                        view! {
                            <p class="rounded-2xl border-2 border-danger-600 bg-danger-50 px-5 py-3 text-[14.5px] font-semibold text-danger-700" role="alert">
                                {move || load_error.get()}
                            </p>
                        }
                        .into_any()
                    }}
                </div>
            </Layout>
        }
        .into_any()
    })
}

#[component]
fn MemoView(
    memo: LegalMemo,
    versions: RwSignal<Vec<MemoVersionEntry>>,
    viewing: RwSignal<Option<MemoVersionSnapshot>>,
    on_changed: Callback<()>,
) -> impl IntoView {
    let approved = memo.status == MemoStatus::Approved;
    let memo_id = StoredValue::new(memo.id.clone());
    let case_id = StoredValue::new(memo.case_id.clone());
    let opening = RwSignal::new(false);
    let view_error = RwSignal::new(String::new());
    let filed = Filed::new(memo.id.clone());
    if approved {
        filed.fetch();
    }

    let open_version = Callback::new(move |version_id: String| {
        if opening.get_untracked() {
            return;
        }
        opening.set(true);
        view_error.set(String::new());
        let id = memo_id.get_value();
        spawn_local(async move {
            let result = load_legal_memo_version(id, version_id).await;
            let _ = opening.try_set(false);
            match result {
                Ok(snapshot) => {
                    let _ = viewing.try_set(Some(snapshot));
                }
                Err(e) => {
                    let _ = view_error.try_set(err_text(e));
                }
            }
        });
    });

    let pills = {
        let version = memo.version;
        if approved {
            let line = format!(
                "Approved by {}, {}",
                memo.approved_by_name,
                month_day(&memo.approved_at)
            );
            view! {
                <StatusPill label=line tone=Tone::Green icon_kind=Icon::Check />
                <StatusPill label=format!("Version {version}") tone=Tone::Sand />
                <VolunteersOnlyBadge />
            }
            .into_any()
        } else {
            view! { {status_pill(&memo)} <VolunteersOnlyBadge /> }.into_any()
        }
    };
    let sent_line = move || {
        versions.with(|list| {
            list.iter()
                .find(|e| e.event == VersionEvent::Sent)
                .map(|e| format!("Sent by {}, {}", e.actor_name, when(&e.at)))
        })
    };
    let actions = if approved {
        view! {
            {move || match filed.document.get() {
                Some(doc) => view! { <a class=crate::components::design::BTN href=filed.download_href(&case_id.get_value(), &doc)>"Download"</a> }.into_any(),
                None => view! { <button type="button" class=crate::components::design::BTN disabled=true>"Download"</button> }.into_any(),
            }}
        }
        .into_any()
    } else {
        view! { <span class=format!("{MUTED} text-[13.5px]")>{sent_line}</span> }.into_any()
    };

    let memo_title = memo.title.clone();
    let title = move || {
        let shown = viewing.with(|v| v.as_ref().map(|snap| snap.title.clone()));
        format!("Legal memo: {}", shown.unwrap_or_else(|| memo_title.clone()))
    };
    let banner = if approved {
        "Approved for internal use. Not for client distribution."
    } else {
        "Draft for attorney review. Not for client distribution."
    };
    let aside_width = if approved {
        "lg:grid-cols-[minmax(0,1fr)_360px]"
    } else {
        "lg:grid-cols-[minmax(0,1fr)_380px]"
    };
    let has_aside = approved || memo.can_review;
    let memo_sv = StoredValue::new(memo.clone());

    let main = move || match viewing.get() {
        Some(snapshot) => {
            let label = format!("You're viewing version {}. ", snapshot.version);
            let mut shown = memo_sv.get_value();
            shown.attorney_name = snapshot.attorney_name;
            shown.content = snapshot.content;
            shown.status = match snapshot.event {
                VersionEvent::Sent => MemoStatus::InReview,
                VersionEvent::RevisionsRequested => MemoStatus::RevisionsRequested,
                VersionEvent::Approved => MemoStatus::Approved,
            };
            view! {
                <div class="flex flex-wrap items-center gap-3 rounded-[14px] border border-slate-700 bg-sand-50 px-[18px] py-3 text-[14.5px]" role="status">
                    <span class="font-semibold">{label}</span>
                    <button type="button" class=crate::components::design::LINK on:click=move |_| viewing.set(None)>"Back to the current version"</button>
                </div>
                {header_card(&shown)}
                {content_cards(&shown.content, &shown.author_name)}
            }
            .into_any()
        }
        None => memo_sv
            .with_value(|m| view! { {header_card(m)} {content_cards(&m.content, &m.author_name)} })
            .into_any(),
    };

    view! {
        <PageHeader back_href=back_href(&memo.case_id) back_label=back_label(&memo) actions=Box::new(move || actions)>
            <h1 class=SERIF_H1>{title}</h1>
            <div class="mt-2 flex flex-wrap gap-2">{pills}</div>
        </PageHeader>
        <DarkBanner>{banner}</DarkBanner>
        <Show when=move || !view_error.get().is_empty()>
            <p class=ERRTEXT role="alert">{move || view_error.get()}</p>
        </Show>
        <div class=format!("grid items-start gap-6 {}", if has_aside { aside_width } else { "" })>
            <div class="flex min-w-0 flex-col gap-4">{main}</div>
            {has_aside.then(|| {
                let memo = memo_sv.get_value();
                view! {
                    <aside class="flex flex-col gap-3.5 lg:sticky lg:top-4" aria-label="About this memo">
                        {if approved {
                            view! {
                                <ApprovedAside memo=memo filed=filed versions=versions opening=opening on_view=open_version on_changed=on_changed />
                            }
                            .into_any()
                        } else {
                            view! { <ReviewPanel memo=memo on_changed=on_changed /> }.into_any()
                        }}
                    </aside>
                }
            })}
        </div>
    }
}
