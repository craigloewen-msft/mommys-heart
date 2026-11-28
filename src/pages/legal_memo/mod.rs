mod approved;
mod editor;
mod editor_tables;
mod read;
mod read_tables;
mod review;

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_params_map;

use crate::components::case_overview::month_day;
use crate::components::case_tabs::CaseTab;
use crate::components::design::{
    DarkBanner, Icon, PageHeader, SectionCard, StatusPill, Tone, VolunteersOnlyBadge, BTN, LINK,
    MUTED, SERIF_H1,
};
use crate::components::guard::require_login;
use crate::components::layout::Layout;
use crate::components::legal_memos::{memo_status_tone, ERRTEXT};
use crate::components::loading::Loading;
pub(super) use crate::helpers::format::date_long;
use crate::helpers::format::{date_parts, matter_label};
use crate::server_fns::err_text;
use crate::server_fns::legal_memos::{
    legal_memo_versions, load_legal_memo, load_legal_memo_version, LegalMemo, MemoStatus,
    MemoVersionEntry, MemoVersionSnapshot, VersionEvent, SECTION_BRIEF, SECTION_EVIDENCE,
    SECTION_FACTS, SECTION_HEADER, SECTION_PROCEDURE, SECTION_QUESTIONS, SECTION_SAFETY,
};
use crate::state::AppState;

use approved::{ChecksCard, Filed, HistoryCard, LockedCard};
use editor::MemoEditor;
use read::{content_cards, header_card};
use review::{ReviewPanel, WithdrawButton};

pub(super) struct Section {
    pub anchor: &'static str,
    pub number: Option<&'static str>,
    pub title: &'static str,
    pub required: bool,
    pub problem_key: &'static str,
}

const fn section(
    anchor: &'static str,
    number: Option<&'static str>,
    title: &'static str,
    required: bool,
    problem_key: &'static str,
) -> Section {
    Section {
        anchor,
        number,
        title,
        required,
        problem_key,
    }
}

pub(super) const SECTIONS: &[Section] = &[
    section("mh", None, "Header", true, SECTION_HEADER),
    section("ms", None, "Safety", true, SECTION_SAFETY),
    section(
        "s1",
        Some("1"),
        "Questions presented",
        true,
        SECTION_QUESTIONS,
    ),
    section("s2", Some("2"), "Brief answer", true, SECTION_BRIEF),
    section("s3", Some("3"), "Relevant facts", true, SECTION_FACTS),
    section(
        "s4",
        Some("4"),
        "Procedural history",
        false,
        SECTION_PROCEDURE,
    ),
    section("s5", Some("5"), "Evidence log", false, SECTION_EVIDENCE),
    section("s6", Some("6"), "Authorities", false, ""),
    section("s7", Some("7"), "Analysis", false, ""),
    section("s8", Some("8"), "Outstanding questions", false, ""),
    section("s9", Some("9"), "Proposed next steps", false, ""),
];

pub(super) const FACT_HELP: &str = "Tag every fact with where it came from: Client reports (the client told us), Document (we have the paper), Third party (someone else told us), Verified (checked against an official record).";
pub(super) const PROCEDURE_HELP: &str =
    "A hearing date can't be marked Confirmed without a source document, such as a court notice.";
pub(super) const EVIDENCE_HELP: &str =
    "\"Reviewed\" can't be chosen until the item has a date received.";
pub(super) const AUTHORITY_HELP: &str = "Free databases don't track later rulings that overturn or limit a case, so check before relying on one.";

pub(super) fn section_card(anchor: &str, error: Signal<bool>, body: AnyView) -> AnyView {
    let Some(s) = SECTIONS.iter().find(|s| s.anchor == anchor) else {
        return body;
    };
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
    let Some((m, d, _)) = date_parts(stamp) else {
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

const ALERT: &str = "rounded-2xl border-2 border-danger-600 bg-danger-50 px-5 py-3 text-[14.5px] font-semibold text-danger-700";

#[component]
pub fn LegalMemoPage() -> impl IntoView {
    let state = expect_context::<AppState>();
    let params = use_params_map();
    let memo = RwSignal::new(None::<LegalMemo>);
    let versions = RwSignal::new(Some(Vec::<MemoVersionEntry>::new()));
    let loading = RwSignal::new(true);
    let load_error = RwSignal::new(String::new());
    let reload = RwSignal::new(0u32);
    let generation = RwSignal::new(0u32);

    Effect::new(move |_| {
        reload.track();
        let (case_id, memo_id) = params.with(|p| {
            (
                p.get("case_id").unwrap_or_default(),
                p.get("memo_id").unwrap_or_default(),
            )
        });
        if !state.is_authenticated() {
            return;
        }
        generation.update(|g| *g += 1);
        let mine = generation.get_untracked();
        spawn_local(async move {
            let result = load_legal_memo(memo_id.clone()).await;
            let history = legal_memo_versions(memo_id).await.ok();
            if generation.try_get_untracked() != Some(mine) {
                return;
            }
            let _ = loading.try_set(false);
            match result {
                Ok(found) if found.case_id == case_id => {
                    let _ = load_error.try_set(String::new());
                    let _ = versions.try_set(history);
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
    let current = Memo::new(move |_| memo.get());

    require_login(state, move || {
        view! {
            <Layout title="Legal memo".to_string() hide_heading=true>
                <div class="flex flex-col gap-4">
                    <Show when=move || current.with(Option::is_some) && !load_error.get().is_empty()>
                        <p class=ALERT role="alert">
                            {move || format!("Couldn't refresh this memo: {}", load_error.get())}
                        </p>
                    </Show>
                    {move || {
                        if let Some(found) = current.get() {
                            return if found.can_edit {
                                view! { <MemoEditor memo=found versions=versions on_changed=on_changed /> }.into_any()
                            } else {
                                view! { <MemoView memo=found versions=versions on_changed=on_changed /> }.into_any()
                            };
                        }
                        if loading.get() {
                            return view! { <Loading label="Loading legal memo\u{2026}" /> }.into_any();
                        }
                        view! { <p class=ALERT role="alert">{move || load_error.get()}</p> }.into_any()
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
    versions: RwSignal<Option<Vec<MemoVersionEntry>>>,
    on_changed: Callback<()>,
) -> impl IntoView {
    let approved = memo.status == MemoStatus::Approved;
    let memo_id = StoredValue::new(memo.id.clone());
    let case_id = StoredValue::new(memo.case_id.clone());
    let viewing = RwSignal::new(None::<MemoVersionSnapshot>);
    let opening = RwSignal::new(false);
    let view_error = RwSignal::new(String::new());
    let filed = Filed::new(memo.id.clone(), memo.filing_may_be_running);
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

    let current_pills = if approved {
        let line = format!(
            "Approved by {}, {}",
            memo.approved_by_name,
            month_day(&memo.approved_at)
        );
        view! {
            <StatusPill label=line tone=Tone::Green icon_kind=Icon::Check />
            <StatusPill label=format!("Version {}", memo.version) tone=Tone::Sand />
            <VolunteersOnlyBadge />
        }
        .into_any()
    } else {
        view! { {status_pill(&memo)} <VolunteersOnlyBadge /> }.into_any()
    };
    let current_shown = move || viewing.with(Option::is_none);
    let pills = view! {
        <span class="contents" class:hidden=move || !current_shown()>{current_pills}</span>
        {move || viewing.with(|v| v.as_ref().map(|snap| view! {
            <StatusPill label=format!("Version {}", snap.version) tone=Tone::Sand />
            <VolunteersOnlyBadge />
        }))}
    };
    let sent_line = (memo.status == MemoStatus::InReview)
        .then_some(memo.last_sent.as_ref())
        .flatten()
        .map(|e| format!("Sent by {}, {}", e.actor_name, when(&e.at)));
    let banner = move || {
        let shows_approved = viewing.with(|v| match v {
            Some(snap) => snap.event == VersionEvent::Approved,
            None => approved,
        });
        if shows_approved {
            "Approved for internal use. Not for client distribution."
        } else {
            "Draft for attorney review. Not for client distribution."
        }
    };
    let approval_note = approved
        .then(|| memo.last_approval.clone())
        .flatten()
        .filter(|e| !e.comment.trim().is_empty());
    let withdraw = memo.can_withdraw.then(|| (memo.id.clone(), on_changed));
    let download = move || {
        if !current_shown() {
            return None;
        }
        filed.document.get().map(|doc| {
            view! { <a class=BTN href=filed.download_href(&case_id.get_value(), &doc)>"Download"</a> }
        })
    };

    let memo_sv = StoredValue::new(memo.clone());
    let title = move || {
        let shown = viewing.with(|v| v.as_ref().map(|snap| snap.title.clone()));
        format!(
            "Legal memo: {}",
            shown.unwrap_or_else(|| memo_sv.with_value(|m| m.title.clone()))
        )
    };
    let main = move || match viewing.get() {
        Some(snapshot) => {
            let label = format!(
                "You're viewing version {} as it was {}. ",
                snapshot.version,
                match snapshot.event {
                    VersionEvent::Sent => "sent",
                    VersionEvent::RevisionsRequested => "when revisions were requested",
                    VersionEvent::Approved => "approved",
                    VersionEvent::Withdrawn => "withdrawn",
                    VersionEvent::Started => "when started",
                }
            );
            let mut shown = memo_sv.get_value();
            shown.attorney_name = snapshot.attorney_name;
            shown.content = snapshot.content;
            shown.status = match snapshot.event {
                VersionEvent::Sent => MemoStatus::InReview,
                VersionEvent::RevisionsRequested => MemoStatus::RevisionsRequested,
                VersionEvent::Approved => MemoStatus::Approved,
                VersionEvent::Withdrawn | VersionEvent::Started => MemoStatus::Draft,
            };
            view! {
                <div class="flex flex-wrap items-center gap-3 rounded-[14px] border border-slate-700 bg-sand-50 px-[18px] py-3 text-[14.5px]" role="status">
                    <span class="font-semibold">{label}</span>
                    <button type="button" class=LINK on:click=move |_| viewing.set(None)>"Back to the current version"</button>
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

    let can_review = memo.can_review;
    let review_memo = memo.clone();
    let locked_memo = memo.clone();
    let checks = memo.review.clone();

    view! {
        <PageHeader back_href=back_href(&memo.case_id) back_label=back_label(&memo) actions=Box::new(move || view! {
            <span class="contents" class:hidden=move || !current_shown()>
                <span class=format!("{MUTED} text-[13.5px]")>{sent_line}</span>
                {withdraw.map(|(memo_id, on_changed)| view! { <WithdrawButton memo_id=memo_id on_changed=on_changed /> })}
            </span>
            {download}
        }.into_any())>
            <h1 class=SERIF_H1>{title}</h1>
            <div class="mt-2 flex flex-wrap gap-2">{pills}</div>
        </PageHeader>
        <DarkBanner>{banner}</DarkBanner>
        {approval_note.map(|e| view! {
            <div class="rounded-2xl border border-ok-200 bg-ok-50 px-5 py-3.5 text-ok-700" class:hidden=move || !current_shown() role="note">
                <p class="font-semibold">{format!("Comment from {}, {}", e.actor_name, when(&e.at))}</p>
                <p class="mt-1 whitespace-pre-line">{e.comment}</p>
            </div>
        })}
        <Show when=move || !view_error.get().is_empty()>
            <p class=ERRTEXT role="alert">{move || view_error.get()}</p>
        </Show>
        <div class="grid items-start gap-6 lg:grid-cols-[minmax(0,1fr)_360px]">
            <div class="flex min-w-0 flex-col gap-4">{main}</div>
            <aside class="flex flex-col gap-3.5 lg:sticky lg:top-4" aria-label="About this memo">
                <div class="flex flex-col gap-3.5" class:hidden=move || !current_shown()>
                    {can_review.then(|| view! { <ReviewPanel memo=review_memo on_changed=on_changed /> })}
                    {approved.then(|| view! {
                        <LockedCard memo=locked_memo filed=filed on_changed=on_changed />
                        <ChecksCard review=checks />
                    })}
                </div>
                {move || viewing.with(|v| {
                    v.as_ref()
                        .filter(|snap| !snap.review.marks.is_empty())
                        .map(|snap| view! { <ChecksCard review=snap.review.clone() heading_id="viewed-checks-h" /> })
                })}
                <HistoryCard versions=versions opening=opening on_view=open_version />
            </aside>
        </div>
    }
}
