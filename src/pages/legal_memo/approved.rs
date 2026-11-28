use leptos::prelude::*;
use leptos::task::spawn_local;

use super::when;
use crate::components::design::{
    icon, Icon, StatusPill, Tone, BTN_PRIMARY, BTN_SM, CARD, HINT, LINK, ROWLINE,
};
use crate::components::legal_memos::ERRTEXT;
use crate::helpers::format::encode_query;
use crate::server_fns::err_text;
use crate::server_fns::legal_memos::{
    file_legal_memo_document, legal_memo_document, start_new_memo_version, LegalMemo,
    MemoFiledDocument, MemoReview, MemoVersionEntry, ReviewCheck, VersionEvent,
};

const FILING_CHECKS: u32 = 3;
const FILING_CHECK_DELAY: std::time::Duration = std::time::Duration::from_secs(2);
const SIDE_HEAD: &str =
    "-mx-5 flex flex-wrap items-center justify-between gap-2 border-b border-slate-800 px-5 pb-3";
const SIDE_H2: &str = "font-serif text-[18px] font-semibold text-slate-200";

#[derive(Clone, Copy)]
pub(super) struct Filed {
    memo_sv: StoredValue<String>,
    pub document: RwSignal<Option<MemoFiledDocument>>,
    loading: RwSignal<bool>,
    error: RwSignal<String>,
    attempts: RwSignal<u32>,
}

impl Filed {
    pub fn new(memo_id: String, may_be_running: bool) -> Self {
        Self {
            memo_sv: StoredValue::new(memo_id),
            document: RwSignal::new(None),
            loading: RwSignal::new(true),
            error: RwSignal::new(String::new()),
            attempts: RwSignal::new(if may_be_running { 0 } else { FILING_CHECKS }),
        }
    }

    pub fn fetch(self) {
        let Some(id) = self.memo_sv.try_get_value() else {
            return;
        };
        spawn_local(async move {
            let result = legal_memo_document(id).await;
            match result {
                Ok(None) => {
                    let tries = self.attempts.try_get_untracked().unwrap_or(FILING_CHECKS);
                    if tries < FILING_CHECKS {
                        let _ = self.attempts.try_set(tries + 1);
                        set_timeout(move || self.fetch(), FILING_CHECK_DELAY);
                        return;
                    }
                    let _ = self.error.try_set(String::new());
                }
                Ok(found) => {
                    let _ = self.error.try_set(String::new());
                    let _ = self.document.try_set(found);
                }
                Err(e) => {
                    let _ = self.error.try_set(err_text(e));
                }
            }
            let _ = self.loading.try_set(false);
        });
    }

    pub fn download_href(self, case_id: &str, doc: &MemoFiledDocument) -> String {
        format!(
            "/api/cases/{case_id}/documents/download?path={}",
            encode_query(&doc.relative_path)
        )
    }
}

#[component]
pub(super) fn LockedCard(memo: LegalMemo, filed: Filed, on_changed: Callback<()>) -> impl IntoView {
    let memo_id = StoredValue::new(memo.id.clone());
    let busy = RwSignal::new(false);
    let filing = RwSignal::new(false);
    let error = RwSignal::new(String::new());

    let start_new = move |_| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        error.set(String::new());
        let id = memo_id.get_value();
        spawn_local(async move {
            match start_new_memo_version(id).await {
                Ok(()) => {
                    let _ = on_changed.try_run(());
                }
                Err(e) => {
                    let _ = error.try_set(err_text(e));
                    let _ = busy.try_set(false);
                }
            }
        });
    };
    let file_now = move |_| {
        if filing.get_untracked() {
            return;
        }
        filing.set(true);
        error.set(String::new());
        let id = memo_id.get_value();
        spawn_local(async move {
            let result = file_legal_memo_document(id).await;
            let _ = filing.try_set(false);
            match result {
                Ok(()) => filed.fetch(),
                Err(e) => {
                    let _ = error.try_set(err_text(e));
                }
            }
        });
    };
    let can_start_new = memo.can_start_new;
    let can_file = memo.can_file;

    view! {
        <section class=format!("{CARD} px-5 pt-[18px] pb-[18px]") aria-labelledby="locked-h">
            <div class=SIDE_HEAD>
                <h2 id="locked-h" class=SIDE_H2>"Locked"</h2>
                <StatusPill label="Approved" tone=Tone::Green icon_kind=Icon::Lock />
            </div>
            {move || {
                if filed.loading.get() {
                    return view! { <p class=format!("{HINT} mt-3")>"Checking SharePoint\u{2026}"</p> }.into_any();
                }
                if filed.document.with(Option::is_some) {
                    return view! {
                        <p class="mt-3 flex items-center gap-2 text-[13.5px]">
                            <span class="text-ok-700">{icon(Icon::Check, "h-4 w-4 flex-none")}</span>
                            "Saved to SharePoint: Intake / Legal Memo"
                        </p>
                    }
                    .into_any();
                }
                view! {
                    <p class="mt-3 text-[13.5px]">"Not saved to SharePoint yet."</p>
                    {can_file.then(|| view! {
                        <button type="button" class=format!("{BTN_SM} mt-2") on:click=file_now prop:disabled=move || filing.get()>
                            {move || if filing.get() { "Saving\u{2026}" } else { "Save it now" }}
                        </button>
                    })}
                }
                .into_any()
            }}
            <Show when=move || !filed.error.get().is_empty()>
                <p class=format!("{ERRTEXT} mt-2") role="alert">{move || filed.error.get()}</p>
            </Show>
            {can_start_new.then(|| view! {
                <button type="button" class=format!("{BTN_PRIMARY} mt-3 w-full") on:click=start_new prop:disabled=move || busy.get()>
                    {move || if busy.get() { "Starting\u{2026}" } else { "Start a new version" }}
                </button>
            })}
            <p class=format!("{HINT} mt-1.5")>"Approved versions can't be edited. A new version starts as a draft copy."</p>
            <Show when=move || !error.get().is_empty()>
                <p class=format!("{ERRTEXT} mt-2") role="alert">{move || error.get()}</p>
            </Show>
        </section>
    }
}

#[component]
pub(super) fn ChecksCard(
    review: MemoReview,
    #[prop(default = "checks-h")] heading_id: &'static str,
) -> impl IntoView {
    view! {
        <section class=format!("{CARD} px-5 pt-[18px] pb-3.5") aria-labelledby=heading_id>
            <div class=SIDE_HEAD>
                <h2 id=heading_id class=SIDE_H2>"Review checks"</h2>
            </div>
            <ul class="mt-1">
                {ReviewCheck::ALL
                    .iter()
                    .map(|check| {
                        let mark = review.marks.iter().find(|m| m.check == *check).cloned();
                        view! {
                            <li class=ROWLINE>
                                {match &mark {
                                    Some(_) => view! { <span class="text-ok-700">{icon(Icon::Check, "h-4 w-4 flex-none")}</span> }.into_any(),
                                    None => view! { <span class="text-slate-500">{icon(Icon::Cross, "h-4 w-4 flex-none")}</span> }.into_any(),
                                }}
                                <span class="flex-1">
                                    <span class="block font-semibold">{check.label()}</span>
                                    <span class=HINT>
                                        {match mark {
                                            Some(m) => format!("{} \u{b7} {}", m.by_name, when(&m.at)),
                                            None => "Not checked".to_string(),
                                        }}
                                    </span>
                                </span>
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
        </section>
    }
}

#[component]
pub(super) fn HistoryCard(
    versions: RwSignal<Option<Vec<MemoVersionEntry>>>,
    opening: RwSignal<bool>,
    #[prop(optional)] on_view: Option<Callback<String>>,
) -> impl IntoView {
    let rows = move || {
        let Some(entries) = versions.get() else {
            return view! { <li class=ROWLINE><span class=ERRTEXT role="alert">"Couldn't load the version history. Reload the page to try again."</span></li> }
                .into_any();
        };
        if entries.is_empty() {
            return view! { <li class=ROWLINE><span class=HINT>"Not sent yet."</span></li> }
                .into_any();
        }
        entries
            .into_iter()
            .map(|e| {
                let tone = match e.event {
                    VersionEvent::Approved => Tone::Green,
                    VersionEvent::RevisionsRequested => Tone::Red,
                    VersionEvent::Withdrawn | VersionEvent::Started => Tone::Sand,
                    VersionEvent::Sent => Tone::Yellow,
                };
                let id = e.id.clone();
                view! {
                    <li class=format!("{ROWLINE} items-start")>
                        <StatusPill label=format!("V{}", e.version) tone=tone small=true />
                        <span class="min-w-0 flex-1">
                            <span class="block font-semibold">{e.event.label()}</span>
                            <span class=HINT>{format!("{} \u{b7} {}", e.actor_name, when(&e.at))}</span>
                            {(!e.comment.trim().is_empty()).then(|| view! {
                                <span class="mt-1 block whitespace-pre-line text-[13.5px]">{e.comment.clone()}</span>
                            })}
                        </span>
                        {on_view.map(|on_view| view! {
                            <button type="button" class=format!("{LINK} text-[13.5px]") prop:disabled=move || opening.get() on:click=move |_| on_view.run(id.clone())>"View"</button>
                        })}
                    </li>
                }
            })
            .collect_view()
            .into_any()
    };
    view! {
        <section class=format!("{CARD} px-5 pt-[18px] pb-3.5") aria-labelledby="history-h">
            <div class=SIDE_HEAD>
                <h2 id="history-h" class=SIDE_H2>"Version history"</h2>
            </div>
            <ul class="mt-1">{rows}</ul>
        </section>
    }
}
