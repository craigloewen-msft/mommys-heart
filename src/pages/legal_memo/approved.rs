use leptos::prelude::*;
use leptos::task::spawn_local;

use super::when;
use crate::components::case_overview::month_day;
use crate::components::design::{
    icon, Icon, StatusPill, Tone, BTN_PRIMARY, BTN_SM, CARD, HINT, LINK, ROWLINE,
};
use crate::components::legal_memos::ERRTEXT;
use crate::helpers::format::encode_query;
use crate::server_fns::err_text;
use crate::server_fns::legal_memos::{
    file_legal_memo_document, legal_memo_document, start_new_memo_version, LegalMemo,
    MemoFiledDocument, MemoVersionEntry, VersionEvent,
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
    pub fn new(memo_id: String) -> Self {
        Self {
            memo_sv: StoredValue::new(memo_id),
            document: RwSignal::new(None),
            loading: RwSignal::new(true),
            error: RwSignal::new(String::new()),
            attempts: RwSignal::new(0),
        }
    }

    pub fn fetch(self) {
        let Some(id) = self.memo_sv.try_get_value() else {
            return;
        };
        spawn_local(async move {
            let result = legal_memo_document(id).await;
            let _ = self.loading.try_set(false);
            match result {
                Ok(None) => {
                    let tries = self.attempts.try_get_untracked().unwrap_or(FILING_CHECKS);
                    if tries < FILING_CHECKS {
                        let _ = self.attempts.try_set(tries + 1);
                        set_timeout(move || self.fetch(), FILING_CHECK_DELAY);
                    }
                }
                Ok(found) => {
                    let _ = self.document.try_set(found);
                }
                Err(e) => {
                    let _ = self.error.try_set(err_text(e));
                }
            }
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
pub(super) fn ApprovedAside(
    memo: LegalMemo,
    filed: Filed,
    versions: RwSignal<Vec<MemoVersionEntry>>,
    opening: RwSignal<bool>,
    on_view: Callback<String>,
    on_changed: Callback<()>,
) -> impl IntoView {
    let memo_id = StoredValue::new(memo.id.clone());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    let filing = RwSignal::new(false);

    let start_new = move |_| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        error.set(String::new());
        let id = memo_id.get_value();
        spawn_local(async move {
            let result = start_new_memo_version(id).await;
            let _ = busy.try_set(false);
            match result {
                Ok(()) => {
                    let _ = on_changed.try_run(());
                }
                Err(e) => {
                    let _ = error.try_set(err_text(e));
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
    let marks = memo.review.marks.clone();

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
                    <p class="mt-3 text-[13.5px]">"Not saved to SharePoint yet. Saving retries if SharePoint was unavailable."</p>
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
        <section class=format!("{CARD} px-5 pt-[18px] pb-3.5") aria-labelledby="history-h">
            <div class=SIDE_HEAD>
                <h2 id="history-h" class=SIDE_H2>"Version history"</h2>
            </div>
            <ul class="mt-1">{move || history_rows(&versions.get(), opening, on_view)}</ul>
        </section>
        <section class=format!("{CARD} px-5 pt-[18px] pb-3.5") aria-labelledby="checks-h">
            <div class=SIDE_HEAD>
                <h2 id="checks-h" class=SIDE_H2>"Review checks"</h2>
            </div>
            <ul class="mt-1">
                {if marks.is_empty() {
                    view! { <li class=ROWLINE><span class=HINT>"No checks recorded."</span></li> }.into_any()
                } else {
                    marks
                        .into_iter()
                        .map(|m| view! {
                            <li class=ROWLINE>
                                <span class="text-ok-700">{icon(Icon::Check, "h-4 w-4 flex-none")}</span>
                                <span class="flex-1">
                                    <span class="block font-semibold">{m.check.label()}</span>
                                    <span class=HINT>{format!("{} \u{b7} {}", m.by_name, month_day(&m.at))}</span>
                                </span>
                            </li>
                        })
                        .collect_view()
                        .into_any()
                }}
            </ul>
        </section>
    }
}

fn history_rows(
    entries: &[MemoVersionEntry],
    opening: RwSignal<bool>,
    on_view: Callback<String>,
) -> AnyView {
    let mut versions: Vec<i32> = entries.iter().map(|e| e.version).collect();
    versions.sort_unstable_by(|a, b| b.cmp(a));
    versions.dedup();
    if versions.is_empty() {
        return view! { <li class=ROWLINE><span class=HINT>"No versions recorded."</span></li> }
            .into_any();
    }
    versions
        .into_iter()
        .filter_map(|version| {
            let of_version: Vec<&MemoVersionEntry> = entries.iter().filter(|e| e.version == version).collect();
            let latest = of_version.first()?;
            let approval = of_version.iter().find(|e| e.event == VersionEvent::Approved);
            let sent = of_version.iter().find(|e| e.event == VersionEvent::Sent).unwrap_or(latest);
            let earlier_request = entries
                .iter()
                .find(|e| e.version == version - 1 && e.event == VersionEvent::RevisionsRequested);
            let (label, detail) = match (approval, earlier_request) {
                (Some(a), _) => ("Approved", format!("{} \u{b7} {}", a.actor_name, when(&a.at))),
                (None, Some(request)) => (
                    "Revised",
                    format!("{} \u{b7} {}, after revisions were requested on {}", sent.actor_name, month_day(&sent.at), month_day(&request.at)),
                ),
                (None, None) => ("Draft", format!("{} \u{b7} {}", sent.actor_name, month_day(&sent.at))),
            };
            let approved = approval.is_some();
            let id = latest.id.clone();
            let tone = if approved { Tone::Green } else { Tone::Sand };
            Some(view! {
                <li class=format!("{ROWLINE} items-start")>
                    <StatusPill label=format!("V{version}") tone=tone small=true />
                    <span class="flex-1">
                        <span class="block font-semibold">{label}</span>
                        <span class=HINT>{detail}</span>
                    </span>
                    <button type="button" class=format!("{LINK} text-[13.5px]") prop:disabled=move || opening.get() on:click=move |_| on_view.run(id.clone())>"View"</button>
                </li>
            })
        })
        .collect_view()
        .into_any()
}
