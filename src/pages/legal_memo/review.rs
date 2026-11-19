use leptos::prelude::*;
use leptos::task::spawn_local;

use super::when;
use crate::components::design::{
    Icon, StatusPill, Tone, BTN, BTN_GREEN, CARD, FIELD, HINT, INPUT, LBL, SERIF_H2,
};
use crate::components::legal_memos::ERRTEXT;
use crate::server_fns::crm::MAX_LONG_TEXT;
use crate::server_fns::err_text;
use crate::server_fns::legal_memos::{
    approve_legal_memo, request_memo_revisions, set_memo_review_check, LegalMemo, ReviewCheck,
};

#[component]
pub(super) fn ReviewPanel(memo: LegalMemo, on_changed: Callback<()>) -> impl IntoView {
    let memo_id = StoredValue::new(memo.id.clone());
    let review = RwSignal::new(memo.review.clone());
    let busy = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    let comment = RwSignal::new(String::new());
    let total = memo
        .content
        .authorities
        .iter()
        .filter(|a| !a.citation.trim().is_empty())
        .count();
    let unchecked = memo
        .content
        .authorities
        .iter()
        .filter(|a| !a.citation.trim().is_empty() && !a.treatment_checked)
        .count();
    let ready =
        move || review.with(|r| ReviewCheck::REQUIRED_FOR_APPROVAL.iter().all(|c| r.has(*c)));

    let act = move |approve: bool| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        error.set(String::new());
        let id = memo_id.get_value();
        let note = comment.get_untracked();
        spawn_local(async move {
            let result = if approve {
                approve_legal_memo(id).await
            } else {
                request_memo_revisions(id, note).await
            };
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

    let toggle = move |check: ReviewCheck, checked: bool| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        error.set(String::new());
        let id = memo_id.get_value();
        spawn_local(async move {
            let result = set_memo_review_check(id, check, checked).await;
            let _ = busy.try_set(false);
            match result {
                Ok(updated) => {
                    let _ = review.try_set(updated);
                }
                Err(e) => {
                    let _ = error.try_set(err_text(e));
                    review.try_update(|_| ());
                }
            }
        });
    };

    view! {
        <section class=format!("{CARD} border-2! border-info-600! px-5 pt-[18px] pb-5") aria-labelledby="review-h">
            <div class="-mx-5 flex flex-wrap items-center justify-between gap-2 border-b border-slate-800 px-5 pb-3">
                <h2 id="review-h" class=SERIF_H2>"Attorney review"</h2>
                <StatusPill label="Supervising attorney only" tone=Tone::Dark icon_kind=Icon::Lock />
            </div>
            <ul class="mt-1.5">
                {ReviewCheck::ALL
                    .iter()
                    .copied()
                    .map(|check| {
                        let mark = move || review.with(|r| r.marks.iter().find(|m| m.check == check).cloned());
                        view! {
                            <li class="border-t border-sand-300 py-3 first:border-t-0">
                                <label class="flex items-start gap-2.5">
                                    <input
                                        type="checkbox"
                                        class="mt-px h-5 w-5 flex-none accent-ok-700"
                                        prop:checked=move || review.with(|r| r.has(check))
                                        prop:disabled=move || busy.get()
                                        on:change=move |ev| toggle(check, event_target_checked(&ev))
                                    />
                                    <span>
                                        <span class="block font-bold">{check.label()}</span>
                                        <span class=format!("block {HINT}")>
                                            {move || match mark() {
                                                Some(m) => format!("{} \u{b7} {}", m.by_name, when(&m.at)),
                                                None => "Not checked yet".to_string(),
                                            }}
                                        </span>
                                        {(check == ReviewCheck::ResearchChecked && unchecked > 0).then(|| view! {
                                            <span class="mt-1 block text-[13.5px] text-danger-700">
                                                {format!("{unchecked} of {total} authorities not checked for subsequent treatment")}
                                            </span>
                                        })}
                                    </span>
                                </label>
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
            <div class=format!("{FIELD} mt-3")>
                <label class=LBL for="review-comment">"Comment for the author"</label>
                <textarea
                    id="review-comment"
                    class=INPUT
                    rows="3"
                    maxlength=MAX_LONG_TEXT
                    prop:value=move || comment.get()
                    on:input=move |ev| comment.set(event_target_value(&ev))
                ></textarea>
                <span class=HINT>"Required when requesting revisions"</span>
            </div>
            <div class="mt-3 grid grid-cols-2 gap-2">
                <button type="button" class=BTN prop:disabled=move || busy.get() || comment.with(|c| c.trim().is_empty()) on:click=move |_| act(false)>
                    "Request revisions"
                </button>
                <button type="button" class=BTN_GREEN prop:disabled=move || busy.get() || !ready() on:click=move |_| act(true)>
                    "Approve"
                </button>
            </div>
            <p class=format!("{HINT} mt-2")>"Approve unlocks when facts and research are checked. Each check records who and when."</p>
            <Show when=move || !error.get().is_empty()>
                <p class=format!("{ERRTEXT} mt-2") role="alert">{move || error.get()}</p>
            </Show>
        </section>
    }
}
