mod read;

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
    legal_memo_versions, load_legal_memo, LegalMemo, MemoStatus, MemoVersionEntry, VersionEvent,
};
use crate::state::AppState;

use read::{content_cards, header_card};

pub(super) struct Section {
    pub anchor: &'static str,
    pub number: Option<&'static str>,
    pub title: &'static str,
    pub required: bool,
}

pub(super) const SECTIONS: &[Section] = &[
    Section {
        anchor: "mh",
        number: None,
        title: "Header",
        required: true,
    },
    Section {
        anchor: "ms",
        number: None,
        title: "Safety",
        required: true,
    },
    Section {
        anchor: "s1",
        number: Some("1"),
        title: "Questions presented",
        required: true,
    },
    Section {
        anchor: "s2",
        number: Some("2"),
        title: "Brief answer",
        required: true,
    },
    Section {
        anchor: "s3",
        number: Some("3"),
        title: "Relevant facts",
        required: true,
    },
    Section {
        anchor: "s4",
        number: Some("4"),
        title: "Procedural history",
        required: false,
    },
    Section {
        anchor: "s5",
        number: Some("5"),
        title: "Evidence log",
        required: false,
    },
    Section {
        anchor: "s6",
        number: Some("6"),
        title: "Authorities",
        required: false,
    },
    Section {
        anchor: "s7",
        number: Some("7"),
        title: "Analysis",
        required: false,
    },
    Section {
        anchor: "s8",
        number: Some("8"),
        title: "Outstanding questions",
        required: false,
    },
    Section {
        anchor: "s9",
        number: Some("9"),
        title: "Proposed next steps",
        required: false,
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
                            return view! { <MemoView memo=found versions=versions on_changed=on_changed /> }.into_any();
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
    on_changed: Callback<()>,
) -> impl IntoView {
    let approved = memo.status == MemoStatus::Approved;
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
    let actions =
        view! { <span class=format!("{MUTED} text-[13.5px]")>{sent_line}</span> }.into_any();
    let title = format!("Legal memo: {}", memo.title);
    let banner = if approved {
        "Approved for internal use. Not for client distribution."
    } else {
        "Draft for attorney review. Not for client distribution."
    };
    let _ = on_changed;
    view! {
        <PageHeader back_href=back_href(&memo.case_id) back_label=back_label(&memo) actions=Box::new(move || actions)>
            <h1 class=SERIF_H1>{title}</h1>
            <div class="mt-2 flex flex-wrap gap-2">{pills}</div>
        </PageHeader>
        <DarkBanner>{banner}</DarkBanner>
        {header_card(&memo)}
        {content_cards(&memo.content, &memo.author_name)}
    }
}
