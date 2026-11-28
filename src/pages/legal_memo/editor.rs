use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;

use super::approved::HistoryCard;
use super::editor_tables::Tables;
use super::{back_href, back_label, clock, matter, section_card, status_pill, when, SECTIONS};
use crate::components::autosave::Autosaver;
use crate::components::design::{
    icon, DarkBanner, Icon, PageHeader, Segmented, VolunteersOnlyBadge, BTN, BTN_PRIMARY, BTN_SM,
    ERRMSG, FIELD, HINT, INPUT, INPUT_ERR, INPUT_READONLY, KICKER, LBL, MUTED, SAVED,
};
use crate::components::legal_memos::ERRTEXT;
use crate::server_fns::crm::MAX_SHORT_TEXT;
use crate::server_fns::err_text;
use crate::server_fns::legal_memos::{
    delete_legal_memo_draft, memo_attorney_choices, problems_before_review, save_legal_memo_draft,
    send_legal_memo_for_review, validate_draft, AnalysisSection, LegalMemo, LegalMemoContent,
    MemoAttorney, MemoFact, MemoLine, MemoProblem, MemoStatus, MemoVersionEntry, MAX_MEMO_TEXT,
    MAX_ROWS, SECTION_BRIEF, SECTION_HEADER, STALE_MEMO_ERROR,
};
use crate::state::now_stamp;

trait RowText: Default + Clone + Send + Sync + 'static {
    fn text(&self) -> &str;
    fn set_text(&mut self, value: String);
}

impl RowText for MemoLine {
    fn text(&self) -> &str {
        &self.text
    }
    fn set_text(&mut self, value: String) {
        self.text = value;
    }
}

fn section_done(anchor: &str, attorney: &str, c: &LegalMemoContent) -> bool {
    match anchor {
        "mh" => !attorney.trim().is_empty(),
        "ms" => match c.safety_flagged {
            None => false,
            Some(false) => true,
            Some(true) => {
                !c.safety_escalated_on.is_empty() && !c.safety_escalated_to.trim().is_empty()
            }
        },
        "s1" => c.questions.iter().any(|q| !q.text.trim().is_empty()),
        "s2" => !c.brief_answer.trim().is_empty(),
        "s3" => {
            let facts: Vec<&MemoFact> = c
                .facts
                .iter()
                .filter(|f| !f.text.trim().is_empty())
                .collect();
            !facts.is_empty() && facts.iter().all(|f| f.source.is_some())
        }
        "s6" => c
            .authorities
            .iter()
            .filter(|a| !a.citation.trim().is_empty())
            .all(|a| a.treatment_checked),
        _ => true,
    }
}

pub(super) fn add_row(label: &'static str, full: Signal<bool>, on_click: Callback<()>) -> AnyView {
    view! {
        <div class="mt-3 flex flex-wrap items-center gap-2">
            <button type="button" class=BTN_SM prop:disabled=move || full.get() on:click=move |_| on_click.run(())>
                {icon(Icon::Plus, "h-4 w-4 flex-none")}
                {label}
            </button>
            <Show when=move || full.get()>
                <span class=HINT>{format!("{MAX_ROWS} rows is the limit.")}</span>
            </Show>
        </div>
    }
    .into_any()
}

#[component]
pub(super) fn MemoEditor(
    memo: LegalMemo,
    versions: RwSignal<Option<Vec<MemoVersionEntry>>>,
    on_changed: Callback<()>,
) -> impl IntoView {
    let memo_id = StoredValue::new(memo.id.clone());
    let case_sv = StoredValue::new(memo.case_id.clone());
    let title = RwSignal::new(memo.title.clone());
    let attorney = RwSignal::new(memo.attorney_id.clone());
    let latest_revision = Rc::new(RefCell::new(memo.revision.clone()));
    let revision = StoredValue::new_local(latest_revision.clone());
    let mut initial = memo.content.clone();
    if initial.memo_date.is_empty() {
        initial.memo_date = now_stamp()[..10].to_string();
    }
    if initial.questions.is_empty() {
        initial.questions.push(MemoLine::default());
    }
    if initial.facts.is_empty() {
        initial.facts.push(MemoFact::default());
    }
    let content = RwSignal::new(initial);
    let attorneys = RwSignal::new(Vec::<MemoAttorney>::new());
    let attorneys_error = RwSignal::new(String::new());
    let stale_attorney = RwSignal::new(String::new());
    let sending = RwSignal::new(false);
    let sent = RwSignal::new(false);
    let problems = RwSignal::new(Vec::<MemoProblem>::new());
    let message = RwSignal::new(String::new());
    let deleting = RwSignal::new(false);
    let confirm_delete = RwSignal::new(false);
    let navigate = StoredValue::new_local(use_navigate());

    let autosave = Autosaver::new(sending, move || {
        let (Some(id), Some(t), Some(a), Some(c)) = (
            memo_id.try_get_value(),
            title.try_get_untracked(),
            attorney.try_get_untracked(),
            content.try_get_untracked(),
        ) else {
            return None;
        };
        let t = if t.trim().is_empty() {
            "Untitled memo".to_string()
        } else {
            t
        };
        let latest = latest_revision.clone();
        Some(Box::pin(async move {
            validate_draft(&t, &c)?;
            let r = latest.borrow().clone();
            let next = save_legal_memo_draft(id, t, a, c, r)
                .await
                .map_err(err_text)?;
            *latest.borrow_mut() = next;
            Ok(())
        }))
    })
    .no_retry_on(STALE_MEMO_ERROR);
    let touch = Callback::new(move |_: ()| {
        if !problems.with_untracked(Vec::is_empty) {
            let a = attorney.get_untracked();
            let found = content.with_untracked(|c| problems_before_review(&a, c));
            if problems.with_untracked(|p| *p != found) {
                problems.set(found);
            }
        }
        autosave.changed();
    });
    let has_problem =
        move |key: &'static str| problems.with(|p| p.iter().any(|x| x.section == key));
    let draft_error = Memo::new(move |_| {
        let t = title.get();
        let t = if t.trim().is_empty() {
            "Untitled memo"
        } else {
            t.as_str()
        };
        content.with(|c| validate_draft(t, c).err())
    });

    let choices_case = memo.case_id.clone();
    let saved_attorney_name = memo.attorney_name.clone();
    spawn_local(async move {
        match memo_attorney_choices(choices_case).await {
            Ok(list) => {
                let current = attorney.try_get_untracked().unwrap_or_default();
                if !current.is_empty() && !list.iter().any(|a| a.id == current) {
                    let _ = attorney.try_set(String::new());
                    let _ = stale_attorney.try_set(saved_attorney_name);
                    let _ = touch.try_run(());
                }
                let _ = attorneys.try_set(list);
            }
            Err(e) => {
                let _ = attorneys_error.try_set(err_text(e));
            }
        }
    });

    let save_now = move |_| {
        if sending.get_untracked() {
            return;
        }
        message.set(String::new());
        autosave.save_now();
    };
    let send = move |_| {
        if sending.get_untracked() || autosave.saving.get_untracked() {
            return;
        }
        message.set(String::new());
        let (Some(id), t, a, c) = (
            memo_id.try_get_value(),
            title.get_untracked(),
            attorney.get_untracked(),
            content.get_untracked(),
        ) else {
            return;
        };
        let t = if t.trim().is_empty() {
            "Untitled memo".to_string()
        } else {
            t
        };
        if let Err(e) = validate_draft(&t, &c) {
            message.set(format!("Not sent: {e}"));
            return;
        }
        let found = problems_before_review(&a, &c);
        if !found.is_empty() {
            problems.set(found);
            autosave.save_now();
            return;
        }
        problems.set(Vec::new());
        let snapshot = autosave.snapshot();
        let r = revision.with_value(|r| r.borrow().clone());
        sending.set(true);
        spawn_local(async move {
            match send_legal_memo_for_review(id, t, a, c, r).await {
                Ok(()) => {
                    autosave.saved_through(snapshot);
                    let _ = sent.try_set(true);
                    let _ = on_changed.try_run(());
                }
                Err(e) => {
                    let _ = message.try_set(format!("Not sent: {}", err_text(e)));
                    let _ = sending.try_set(false);
                }
            }
        });
    };

    let reload = move |_| {
        autosave.discard();
        let _ = on_changed.try_run(());
    };
    let delete = move |_| {
        if deleting.get_untracked() {
            return;
        }
        let (Some(id), Some(case_id)) = (memo_id.try_get_value(), case_sv.try_get_value()) else {
            return;
        };
        deleting.set(true);
        message.set(String::new());
        spawn_local(async move {
            match delete_legal_memo_draft(id).await {
                Ok(()) => {
                    autosave.discard();
                    let href = back_href(&case_id);
                    navigate.try_with_value(|go| go(&href, Default::default()));
                }
                Err(e) => {
                    let _ = message.try_set(format!("Not deleted: {}", err_text(e)));
                    let _ = deleting.try_set(false);
                    let _ = confirm_delete.try_set(false);
                }
            }
        });
    };
    let never_sent = memo.status == MemoStatus::Draft && memo.last_sent.is_none();

    let saved_class = move || {
        autosave.status.with(|status| {
            if status.starts_with("Saved") {
                SAVED.to_string()
            } else if status.starts_with("Not saved") {
                ERRTEXT.to_string()
            } else {
                format!("{MUTED} text-[13.5px]")
            }
        })
    };
    let saved_text = move || {
        let status = autosave.status.get();
        let saved = status.starts_with("Saved");
        let label = match status.strip_prefix("Saved ") {
            Some(t) => format!("Saved {}", clock(t).unwrap_or_else(|| t.to_string())),
            None => status,
        };
        view! { {saved.then(|| icon(Icon::Check, "h-4 w-4 flex-none"))} {label} }
    };
    let actions = move |live: bool| {
        view! {
            <span class=saved_class role=live.then_some("status")>{saved_text}</span>
            <button type="button" class=BTN on:click=save_now prop:disabled=move || autosave.saving.get() || sending.get() || deleting.get()>
                "Save draft"
            </button>
            <button type="button" class=BTN_PRIMARY on:click=send prop:disabled=move || sending.get() || autosave.saving.get() || deleting.get()>
                {move || if sent.get() { "Sent" } else if sending.get() { "Sending\u{2026}" } else { "Send for attorney review" }}
                {icon(Icon::ArrowRight, "h-4 w-4 flex-none")}
            </button>
        }
    };

    let nav = view! {
        <nav aria-label="Memo sections" class="sticky top-4 hidden rounded-[20px] border border-slate-800 bg-white p-3 lg:block">
            <p class=format!("{KICKER} px-2.5 pt-1 pb-2")>"Sections"</p>
            <ol>
                {SECTIONS
                    .iter()
                    .map(|s| {
                        let flagged = move || problems.with(|p| p.iter().any(|x| x.section == s.problem_key));
                        let done = move || {
                            let a = attorney.get();
                            !flagged() && content.with(|c| section_done(s.anchor, &a, c))
                        };
                        let label = match s.number {
                            Some(n) => format!("{n} {}", s.title),
                            None => s.title.to_string(),
                        };
                        view! {
                            <li>
                                <a href=format!("#{}", s.anchor) class="flex items-center gap-2 rounded-[10px] px-2.5 py-[7px] text-[14px] text-slate-200 no-underline hover:bg-sand-200">
                                    {move || if done() {
                                        view! { <span class="text-ok-700">{icon(Icon::Check, "h-4 w-4 flex-none")}</span> }.into_any()
                                    } else if flagged() {
                                        view! { <span class="text-danger-600">{icon(Icon::Warning, "h-4 w-4 flex-none")}</span> }.into_any()
                                    } else {
                                        view! { <span class="text-primary-600">{icon(Icon::Warning, "h-4 w-4 flex-none")}</span> }.into_any()
                                    }}
                                    <span class="flex-1">{label}</span>
                                    {s.required.then(|| view! { <span class=HINT aria-hidden="true">"req."</span><span class="sr-only">"Required"</span> })}
                                    <span class="sr-only">{move || if done() { "Done" } else { "Not finished" }}</span>
                                </a>
                            </li>
                        }
                    })
                    .collect_view()}
            </ol>
        </nav>
    };

    let header = {
        let author = memo.author_name.clone();
        let matter = matter(&memo);
        let status = memo.status;
        view! {
            <div class="grid gap-x-4 gap-y-3 sm:grid-cols-2 lg:grid-cols-3">
                <div class=FIELD>
                    <label class=LBL for="memo-to">"Reviewer"</label>
                    <select
                        id="memo-to"
                        class=move || if has_problem(SECTION_HEADER) { INPUT_ERR } else { INPUT }
                        prop:value=move || attorney.get()
                        on:change=move |ev| {
                            attorney.set(event_target_value(&ev));
                            stale_attorney.set(String::new());
                            touch.run(());
                        }
                    >
                        <option value="" selected=move || attorney.get().is_empty()>"Choose a reviewer"</option>
                        {move || attorneys
                            .get()
                            .into_iter()
                            .map(|a| {
                                let id = a.id.clone();
                                view! { <option value=a.id selected=move || attorney.get() == id>{a.name}</option> }
                            })
                            .collect_view()}
                    </select>
                    <span class=HINT>"Anyone on the case who can see its documents. Choose an attorney when you can."</span>
                    <Show when=move || !stale_attorney.get().is_empty()>
                        <span class=ERRMSG>{move || format!("{} is no longer on this case. Choose another reviewer.", stale_attorney.get())}</span>
                    </Show>
                    <Show when=move || !attorneys_error.get().is_empty()>
                        <span class=ERRMSG>{move || format!("Could not load reviewers: {}", attorneys_error.get())}</span>
                    </Show>
                </div>
                <div class=FIELD>
                    <label class=LBL for="memo-from">"From"</label>
                    <input id="memo-from" class=INPUT_READONLY readonly=true value=author />
                </div>
                <div class=FIELD>
                    <label class=LBL for="memo-date">"Date"</label>
                    <input
                        id="memo-date"
                        type="date"
                        class=INPUT
                        prop:value=move || content.with(|c| c.memo_date.clone())
                        on:input=move |ev| { let v = event_target_value(&ev); content.update(|c| c.memo_date = v); touch.run(()); }
                    />
                </div>
                <div class=FIELD>
                    <label class=LBL for="memo-matter">"Matter"</label>
                    <input id="memo-matter" class=INPUT_READONLY readonly=true value=matter />
                </div>
                <div class=FIELD>
                    <label class=LBL for="memo-status">"Status"</label>
                    <input id="memo-status" class=INPUT_READONLY readonly=true value=status.label() />
                </div>
            </div>
        }
        .into_any()
    };

    fn list_rows<T: RowText>(
        content: RwSignal<LegalMemoContent>,
        touch: Callback<()>,
        get: fn(&LegalMemoContent) -> &Vec<T>,
        get_mut: fn(&mut LegalMemoContent) -> &mut Vec<T>,
        noun: &'static str,
    ) -> AnyView {
        view! {
            <ul class="flex flex-col gap-2">
                <For each=move || 0..content.with(|c| get(c).len()) key=|i| *i let:i>
                    <li class="flex items-center gap-2">
                        <span class=format!("{MUTED} w-5 flex-none text-[13.5px]")>{format!("{}.", i + 1)}</span>
                        <input
                            class=INPUT
                            maxlength=MAX_MEMO_TEXT
                            aria-label=format!("{noun} {}", i + 1)
                            prop:value=move || content.with(|c| get(c).get(i).map(|r| r.text().to_string()).unwrap_or_default())
                            on:input=move |ev| {
                                let v = event_target_value(&ev);
                                content.update(|c| if let Some(row) = get_mut(c).get_mut(i) { row.set_text(v); });
                                touch.run(());
                            }
                        />
                        <button
                            type="button"
                            class=format!("{BTN_SM} flex-none px-2.5")
                            aria-label=format!("Remove {} {}", noun.to_lowercase(), i + 1)
                            on:click=move |_| {
                                content.update(|c| { let rows = get_mut(c); if i < rows.len() { rows.remove(i); } });
                                touch.run(());
                            }
                        >
                            {icon(Icon::Cross, "h-4 w-4")}
                        </button>
                    </li>
                </For>
            </ul>
            {add_row(
                "Add another",
                Signal::derive(move || content.with(|c| get(c).len() >= MAX_ROWS)),
                Callback::new(move |_| {
                    content.update(|c| if get(c).len() < MAX_ROWS { get_mut(c).push(T::default()); });
                    touch.run(());
                }),
            )}
        }
        .into_any()
    }

    let brief = view! {
        <textarea
            class=move || if has_problem(SECTION_BRIEF) { INPUT_ERR } else { INPUT }
            rows="3"
            maxlength=MAX_MEMO_TEXT
            aria-label="Brief answer"
            placeholder="Answer each question in one or two sentences."
            prop:value=move || content.with(|c| c.brief_answer.clone())
            on:input=move |ev| { let v = event_target_value(&ev); content.update(|c| c.brief_answer = v); touch.run(()); }
        ></textarea>
    }
    .into_any();

    let analysis = view! {
        <div class="flex flex-col gap-4">
            <For each=move || 0..content.with(|c| c.analysis.len()) key=|i| *i let:i>
                <div class="flex flex-col gap-2 border-t border-sand-300 pt-3 first:border-t-0 first:pt-0">
                    <div class="flex items-center gap-2">
                        <input
                            class=INPUT
                            maxlength=MAX_SHORT_TEXT
                            aria-label=format!("Analysis heading {}", i + 1)
                            placeholder="Heading, for example Best interests"
                            prop:value=move || content.with(|c| c.analysis.get(i).map(|a| a.heading.clone()).unwrap_or_default())
                            on:input=move |ev| { let v = event_target_value(&ev); content.update(|c| if let Some(a) = c.analysis.get_mut(i) { a.heading = v; }); touch.run(()); }
                        />
                        <button
                            type="button"
                            class=format!("{BTN_SM} flex-none px-2.5")
                            aria-label=format!("Remove analysis section {}", i + 1)
                            on:click=move |_| {
                                content.update(|c| if i < c.analysis.len() { c.analysis.remove(i); });
                                touch.run(());
                            }
                        >
                            {icon(Icon::Cross, "h-4 w-4")}
                        </button>
                    </div>
                    <textarea
                        class=INPUT
                        rows="3"
                        maxlength=MAX_MEMO_TEXT
                        aria-label=format!("Analysis section {}", i + 1)
                        prop:value=move || content.with(|c| c.analysis.get(i).map(|a| a.body.clone()).unwrap_or_default())
                        on:input=move |ev| { let v = event_target_value(&ev); content.update(|c| if let Some(a) = c.analysis.get_mut(i) { a.body = v; }); touch.run(()); }
                    ></textarea>
                </div>
            </For>
        </div>
        {add_row(
            "Add a section",
            Signal::derive(move || content.with(|c| c.analysis.len() >= MAX_ROWS)),
            Callback::new(move |_| {
                content.update(|c| if c.analysis.len() < MAX_ROWS { c.analysis.push(AnalysisSection::default()); });
                touch.run(());
            }),
        )}
    }
    .into_any();

    let tables = Tables { content, touch };
    let card = move |anchor: &'static str, body: AnyView| {
        let key = SECTIONS
            .iter()
            .find(|s| s.anchor == anchor)
            .map(|s| s.problem_key)
            .unwrap_or("");
        section_card(
            anchor,
            Signal::derive(move || !key.is_empty() && has_problem(key)),
            body,
        )
    };
    let safety = view! {
        <Segmented
            name="memo-safety"
            legend="Safety concern flagged?"
            options=vec![("yes".to_string(), "Yes".to_string()), ("no".to_string(), "No".to_string())]
            value=Signal::derive(move || content.with(|c| match c.safety_flagged {
                Some(true) => "yes".to_string(),
                Some(false) => "no".to_string(),
                None => String::new(),
            }))
            on_change=Callback::new(move |v: String| {
                content.update(|c| c.safety_flagged = Some(v == "yes"));
                touch.run(());
            })
        />
        <p class=format!("{HINT} mt-2")>
            "If Yes, add when it was escalated and to whom. A safety concern goes to a supervisor right away, not only in this memo."
        </p>
        <Show when=move || content.with(|c| c.safety_flagged == Some(true))>
            <div class="mt-3 grid gap-x-4 gap-y-3 sm:grid-cols-2">
                <div class=FIELD>
                    <label class=LBL for="memo-escalated-on">"Date escalated"</label>
                    <input id="memo-escalated-on" type="date" class=INPUT
                        prop:value=move || content.with(|c| c.safety_escalated_on.clone())
                        on:input=move |ev| { let v = event_target_value(&ev); content.update(|c| c.safety_escalated_on = v); touch.run(()); } />
                </div>
                <div class=FIELD>
                    <label class=LBL for="memo-escalated-to">"Escalated to"</label>
                    <input id="memo-escalated-to" class=INPUT maxlength=MAX_SHORT_TEXT
                        prop:value=move || content.with(|c| c.safety_escalated_to.clone())
                        on:input=move |ev| { let v = event_target_value(&ev); content.update(|c| c.safety_escalated_to = v); touch.run(()); } />
                </div>
            </div>
        </Show>
    }
    .into_any();
    let title_input = view! {
        <label for="memo-title" class="sr-only">"Memo title"</label>
        <h1 class="font-serif text-[30px] font-semibold leading-tight text-slate-200">
            "Legal memo: "
            <input
                id="memo-title"
                maxlength=MAX_SHORT_TEXT
                class="min-w-[10ch] max-w-full rounded-md border-0 border-b-2 border-dashed border-sand-500 bg-transparent p-0 font-serif text-[30px] font-semibold text-slate-200 [field-sizing:content] focus:border-primary-600 focus:outline-none"
                prop:value=move || title.get()
                prop:disabled=move || sending.get()
                on:input=move |ev| { title.set(event_target_value(&ev)); touch.run(()); }
                on:blur=move |_| {
                    if title.get_untracked().trim().is_empty() {
                        title.set("Untitled memo".to_string());
                        touch.run(());
                    }
                }
            />
        </h1>
    };
    let revision_note = (memo.status == MemoStatus::RevisionsRequested)
        .then(|| memo.last_revisions_request.clone())
        .flatten();

    view! {
        <PageHeader back_href=back_href(&memo.case_id) back_label=back_label(&memo) actions=Box::new(move || actions(true).into_any())>
            {title_input}
            <div class="mt-2 flex flex-wrap gap-2">{status_pill(&memo)} <VolunteersOnlyBadge /></div>
        </PageHeader>
        <DarkBanner>"Draft for attorney review. Not for client distribution."</DarkBanner>
        <Show when=move || autosave.failed.with(Option::is_some)>
            <div class="flex flex-wrap items-center gap-3 rounded-2xl border-2 border-danger-600 bg-danger-50 px-5 py-3 text-danger-700" role="alert">
                <p class="font-semibold">{move || format!("Not saved: {}", autosave.failed.get().unwrap_or_default())}</p>
                {move || if autosave.failed.with(|f| f.as_deref() == Some(STALE_MEMO_ERROR)) {
                    view! { <button type="button" class=BTN_SM on:click=reload>"Reload"</button> }.into_any()
                } else if draft_error.with(Option::is_none) {
                    view! { <button type="button" class=BTN_SM on:click=move |_| autosave.save_now()>"Try again"</button> }.into_any()
                } else {
                    ().into_any()
                }}
            </div>
        </Show>
        {revision_note.map(|entry| view! {
            <div class="rounded-2xl border border-caution-300 bg-caution-50 px-5 py-3.5 text-caution-800" role="note">
                <p class="font-semibold">{format!("Revisions requested by {}, {}", entry.actor_name, when(&entry.at))}</p>
                <p class="mt-1 whitespace-pre-line">{entry.comment}</p>
            </div>
        })}
        <Show when=move || !problems.with(Vec::is_empty)>
            <div class="rounded-2xl border-2 border-danger-600 bg-danger-50 px-5 py-4" role="alert">
                <p class="flex items-center gap-2 font-bold text-danger-700">
                    {icon(Icon::Warning, "h-4 w-4 flex-none")}
                    "Can't send for attorney review yet"
                </p>
                <ul class="mt-2 flex list-disc flex-col gap-1 pl-[26px] text-danger-700">
                    {move || problems
                        .get()
                        .into_iter()
                        .map(|p| {
                            let anchor = SECTIONS
                                .iter()
                                .find(|s| s.problem_key == p.section)
                                .map(|s| s.anchor)
                                .unwrap_or("mh");
                            view! { <li><a class="underline underline-offset-[3px]" href=format!("#{anchor}")>{format!("{}: {}", p.section, p.message)}</a></li> }
                        })
                        .collect_view()}
                </ul>
            </div>
        </Show>
        <Show when=move || !message.get().is_empty()>
            <div class="flex flex-wrap items-center gap-3" role="alert">
                <p class="whitespace-pre-line text-[14.5px] font-semibold text-danger-700">{move || message.get()}</p>
                <Show when=move || message.with(|m| m.contains(STALE_MEMO_ERROR))>
                    <button type="button" class=BTN_SM on:click=reload>"Reload"</button>
                </Show>
            </div>
        </Show>
        <p class=format!("{MUTED} text-[13.5px]")>"Sections marked Required must be filled in before sending. The rest can be skipped on a simple matter."</p>
        <div class="grid items-start gap-6 lg:grid-cols-[250px_minmax(0,1fr)]">
            {nav}
            <fieldset class="flex min-w-0 flex-col gap-4" prop:disabled=move || sending.get() || deleting.get()>
                {card("mh", header)}
                {card("ms", safety)}
                {card("s1", list_rows(content, touch, |c| &c.questions, |c| &mut c.questions, "Question"))}
                {card("s2", brief)}
                {card("s3", tables.facts())}
                {card("s4", tables.procedure())}
                {card("s5", tables.evidence())}
                {card("s6", tables.authorities())}
                {card("s7", analysis)}
                {card("s8", list_rows(content, touch, |c| &c.outstanding, |c| &mut c.outstanding, "Outstanding question"))}
                {card("s9", tables.next_steps())}
                <div class="flex flex-wrap items-center justify-end gap-2">
                    {never_sent.then(|| view! {
                        <div class="mr-auto flex flex-wrap items-center gap-2">
                            {move || if confirm_delete.get() {
                                view! {
                                    <span class=format!("{MUTED} text-[13.5px]")>"Delete this draft for good?"</span>
                                    <button type="button" class=BTN_SM on:click=delete prop:disabled=move || deleting.get()>"Yes, delete it"</button>
                                    <button type="button" class=BTN_SM on:click=move |_| confirm_delete.set(false)>"Cancel"</button>
                                }
                                .into_any()
                            } else {
                                view! { <button type="button" class=BTN_SM on:click=move |_| confirm_delete.set(true)>"Delete draft"</button> }.into_any()
                            }}
                        </div>
                    })}
                    {actions(false)}
                </div>
            </fieldset>
            <Show when=move || versions.with(|v| v.as_ref().is_none_or(|v| !v.is_empty()))>
                <div class="lg:col-start-2">
                    <HistoryCard versions=versions opening=RwSignal::new(false) />
                </div>
            </Show>
        </div>
    }
}
