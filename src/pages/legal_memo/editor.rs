use leptos::prelude::*;
use leptos::task::spawn_local;

use super::editor_tables::Tables;
use super::{
    back_href, back_label, clock, matter, section, section_card, status_pill, when, Section,
    SECTIONS,
};
use crate::components::autosave::Autosaver;
use crate::components::design::{
    icon, DarkBanner, Icon, PageHeader, Segmented, VolunteersOnlyBadge, BTN, BTN_PRIMARY, BTN_SM,
    ERRMSG, FIELD, HINT, INPUT, INPUT_ERR, INPUT_READONLY, KICKER, LBL, MUTED, SAVED,
};
use crate::components::legal_memos::ERRTEXT;
use crate::server_fns::err_text;
use crate::server_fns::legal_memos::{
    memo_attorney_choices, problems_before_review, save_legal_memo_draft,
    send_legal_memo_for_review, validate_draft, EventStatus, EvidenceStatus, LegalMemo,
    LegalMemoContent, MemoAttorney, MemoFact, MemoProblem, MemoStatus, MemoVersionEntry,
    VersionEvent, ANALYSIS_HEADINGS, MAX_LONG, MAX_ROWS, MAX_SHORT,
};
use crate::state::now_stamp;

#[derive(Clone, Copy, PartialEq)]
enum Mark {
    Done,
    Warn,
    Error,
}

fn mark_of(s: &Section, attorney: &str, c: &LegalMemoContent) -> Mark {
    let ok = match s.anchor {
        "mh" => !attorney.trim().is_empty(),
        "ms" => match c.safety_flagged {
            None => false,
            Some(false) => true,
            Some(true) => {
                !c.safety_escalated_on.trim().is_empty() && !c.safety_escalated_to.trim().is_empty()
            }
        },
        "s1" => c.questions.iter().any(|q| !q.trim().is_empty()),
        "s2" => !c.brief_answer.trim().is_empty(),
        "s3" => {
            let facts: Vec<&MemoFact> = c
                .facts
                .iter()
                .filter(|f| !f.text.trim().is_empty())
                .collect();
            !facts.is_empty() && facts.iter().all(|f| f.source.is_some())
        }
        "s4" => !c
            .procedure
            .iter()
            .any(|p| p.status == EventStatus::Confirmed && p.source.trim().is_empty()),
        "s5" => !c
            .evidence
            .iter()
            .any(|e| e.status == EvidenceStatus::Reviewed && e.date_received.trim().is_empty()),
        "s6" => c
            .authorities
            .iter()
            .filter(|a| !a.citation.trim().is_empty())
            .all(|a| a.treatment_checked),
        _ => true,
    };
    if ok {
        Mark::Done
    } else {
        Mark::Warn
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
    versions: RwSignal<Vec<MemoVersionEntry>>,
    on_changed: Callback<()>,
) -> impl IntoView {
    let memo_id = StoredValue::new(memo.id.clone());
    let title = RwSignal::new(memo.title.clone());
    let attorney = RwSignal::new(memo.attorney_id.clone());
    let mut initial = memo.content.clone();
    if initial.memo_date.is_empty() {
        initial.memo_date = now_stamp()[..10].to_string();
    }
    if initial.questions.is_empty() {
        initial.questions.push(String::new());
    }
    if initial.facts.is_empty() {
        initial.facts.push(MemoFact::default());
    }
    let content = RwSignal::new(initial);
    let attorneys = RwSignal::new(Vec::<MemoAttorney>::new());
    let attorneys_error = RwSignal::new(String::new());
    let stale_attorney = RwSignal::new(String::new());
    let sending = RwSignal::new(false);
    let attempted = RwSignal::new(false);
    let message = RwSignal::new(String::new());

    let autosave = Autosaver::new(sending, move || {
        let (Some(id), Some(t), Some(a), Some(c)) = (
            memo_id.try_get_value(),
            title.try_get_untracked(),
            attorney.try_get_untracked(),
            content.try_get_untracked(),
        ) else {
            return None;
        };
        Some(Box::pin(async move {
            validate_draft(&t, &c)?;
            save_legal_memo_draft(id, t, a, c).await.map_err(err_text)
        }))
    });
    let touch = Callback::new(move |_: ()| autosave.changed());
    let problems = Memo::new(move |_| {
        if !attempted.get() {
            return Vec::<MemoProblem>::new();
        }
        let a = attorney.get();
        content.with(|c| problems_before_review(&a, c))
    });
    let has_problem =
        move |key: &'static str| problems.with(|p| p.iter().any(|x| x.section == key));
    let draft_error = Memo::new(move |_| {
        let t = title.get();
        content.with(|c| validate_draft(&t, c).err())
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
        if let Err(e) = validate_draft(&t, &c) {
            message.set(format!("Not sent: {e}"));
            return;
        }
        if !problems_before_review(&a, &c).is_empty() {
            attempted.set(true);
            autosave.save_now();
            return;
        }
        let snapshot = autosave.snapshot();
        sending.set(true);
        spawn_local(async move {
            match send_legal_memo_for_review(id, t, a, c).await {
                Ok(()) => {
                    autosave.saved_through(snapshot);
                    let _ = on_changed.try_run(());
                }
                Err(e) => {
                    let _ = message.try_set(format!("Not sent: {}", err_text(e)));
                    let _ = sending.try_set(false);
                }
            }
        });
    };

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
            <button type="button" class=BTN on:click=save_now prop:disabled=move || autosave.saving.get() || sending.get()>
                "Save draft"
            </button>
            <button type="button" class=BTN_PRIMARY on:click=send prop:disabled=move || sending.get() || autosave.saving.get()>
                {move || if sending.get() { "Sending\u{2026}" } else { "Send for attorney review" }}
                {icon(Icon::ArrowRight, "h-4 w-4 flex-none")}
            </button>
        }
    };
    let draft_line = move || {
        if autosave.saving.get() {
            "Saving your draft\u{2026}"
        } else if autosave.failed.with(Option::is_some) || autosave.dirty.get() {
            "Your latest changes aren't saved yet."
        } else {
            "Your draft is saved. Nothing you typed was lost."
        }
    };
    let revision_note = move || {
        versions.with(|list| {
            list.iter()
                .find(|e| e.event == VersionEvent::RevisionsRequested)
                .cloned()
        })
    };
    let is_revision = memo.status == MemoStatus::RevisionsRequested;

    let nav = move || {
        view! {
            <nav aria-label="Memo sections" class="sticky top-4 hidden rounded-[20px] border border-slate-800 bg-white p-3 lg:block">
                <p class=format!("{KICKER} px-2.5 pt-1 pb-2")>"Sections"</p>
                <ol>
                    {SECTIONS
                        .iter()
                        .map(|s| {
                            let state = move || {
                                if has_problem(s.problem_key) {
                                    Mark::Error
                                } else {
                                    let a = attorney.get();
                                    content.with(|c| mark_of(s, &a, c))
                                }
                            };
                            let label = match s.number {
                                Some(n) => format!("{n} {}", s.title),
                                None => s.title.to_string(),
                            };
                            view! {
                                <li>
                                    <a href=format!("#{}", s.anchor) class="flex items-center gap-2 rounded-[10px] px-2.5 py-[7px] text-[14px] text-slate-200 no-underline hover:bg-sand-200">
                                        {move || match state() {
                                            Mark::Done => view! { <span class="text-ok-700">{icon(Icon::Check, "h-4 w-4 flex-none")}</span> }.into_any(),
                                            Mark::Warn => view! { <span class="text-primary-600">{icon(Icon::Warning, "h-4 w-4 flex-none")}</span> }.into_any(),
                                            Mark::Error => view! { <span class="text-danger-600">{icon(Icon::Warning, "h-4 w-4 flex-none")}</span> }.into_any(),
                                        }}
                                        <span class="flex-1">{label}</span>
                                        {s.required.then(|| view! { <span class=HINT aria-hidden="true">"req."</span><span class="sr-only">"Required"</span> })}
                                        <span class="sr-only">
                                            {move || match state() {
                                                Mark::Done => "Done",
                                                Mark::Warn => "Not finished",
                                                Mark::Error => "Needs fixing",
                                            }}
                                        </span>
                                    </a>
                                </li>
                            }
                        })
                        .collect_view()}
                </ol>
            </nav>
        }
    };

    let card = move |anchor: &'static str, body: AnyView| {
        let key = section(anchor).problem_key;
        section_card(anchor, Signal::derive(move || has_problem(key)), body)
    };

    let header = {
        let author = memo.author_name.clone();
        let matter = matter(&memo);
        let status = memo.status;
        view! {
            <div class="grid gap-x-4 gap-y-3 sm:grid-cols-2 lg:grid-cols-3">
                <div class=FIELD>
                    <label class=LBL for="memo-to">"To"</label>
                    <select
                        id="memo-to"
                        class=move || if has_problem("Header") { INPUT_ERR } else { INPUT }
                        prop:value=move || attorney.get()
                        on:change=move |ev| {
                            attorney.set(event_target_value(&ev));
                            stale_attorney.set(String::new());
                            touch.run(());
                        }
                    >
                        <option value="" selected=move || attorney.get().is_empty()>"Choose an attorney"</option>
                        {move || attorneys
                            .get()
                            .into_iter()
                            .map(|a| {
                                let id = a.id.clone();
                                view! { <option value=a.id selected=move || attorney.get() == id>{a.name}</option> }
                            })
                            .collect_view()}
                    </select>
                    <Show when=move || !stale_attorney.get().is_empty()>
                        <span class=ERRMSG>{move || format!("{} is no longer on this case. Choose another attorney.", stale_attorney.get())}</span>
                    </Show>
                    <Show when=move || !attorneys_error.get().is_empty()>
                        <span class=ERRMSG>{move || format!("Could not load attorneys: {}", attorneys_error.get())}</span>
                    </Show>
                </div>
                <div class=FIELD>
                    <label class=LBL for="memo-from">"From"</label>
                    <input id="memo-from" class=INPUT_READONLY readonly=true value=author />
                    <span class=HINT>"Filled in automatically"</span>
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
                    <span class=HINT>"Filled in from the case"</span>
                </div>
                <div class=FIELD>
                    <label class=LBL for="memo-status">"Status"</label>
                    <input id="memo-status" class=INPUT_READONLY readonly=true value=status.label() />
                    <span class=HINT>"Changes when you send it or the attorney acts"</span>
                </div>
            </div>
        }
        .into_any()
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
                content.update(|c| {
                    c.safety_flagged = Some(v == "yes");
                    c.safety_answered_on = now_stamp();
                });
                touch.run(());
            })
        />
        <p class=format!("{HINT} mt-2")>
            "If Yes, you'll be asked for the date it was escalated and who it was escalated to. A safety concern goes to a supervisor right away, not only in this memo."
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
                    <input id="memo-escalated-to" class=INPUT maxlength=MAX_SHORT
                        prop:value=move || content.with(|c| c.safety_escalated_to.clone())
                        on:input=move |ev| { let v = event_target_value(&ev); content.update(|c| c.safety_escalated_to = v); touch.run(()); } />
                </div>
            </div>
        </Show>
    }
    .into_any();

    let list_rows = move |get: fn(&LegalMemoContent) -> &Vec<String>,
                          get_mut: fn(&mut LegalMemoContent) -> &mut Vec<String>,
                          noun: &'static str| {
        view! {
            <ul class="flex flex-col gap-2">
                <For each=move || 0..content.with(|c| get(c).len()) key=|i| *i let:i>
                    <li class="flex items-center gap-2">
                        <span class=format!("{MUTED} w-5 flex-none text-[13.5px]")>{format!("{}.", i + 1)}</span>
                        <input
                            class=INPUT
                            maxlength=MAX_LONG
                            aria-label=format!("{noun} {}", i + 1)
                            prop:value=move || content.with(|c| get(c).get(i).cloned().unwrap_or_default())
                            on:input=move |ev| {
                                let v = event_target_value(&ev);
                                content.update(|c| if let Some(row) = get_mut(c).get_mut(i) { *row = v; });
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
                    content.update(|c| if get(c).len() < MAX_ROWS { get_mut(c).push(String::new()); });
                    touch.run(());
                }),
            )}
        }
        .into_any()
    };

    let brief = view! {
        <textarea
            class=move || if has_problem("2 Brief answer") { INPUT_ERR } else { INPUT }
            rows="3"
            maxlength=MAX_LONG
            aria-label="Brief answer"
            placeholder=move || has_problem("2 Brief answer").then_some("Answer each question in one or two sentences.")
            prop:value=move || content.with(|c| c.brief_answer.clone())
            on:input=move |ev| { let v = event_target_value(&ev); content.update(|c| c.brief_answer = v); touch.run(()); }
        ></textarea>
        <Show when=move || has_problem("2 Brief answer")>
            <p class=format!("{ERRMSG} mt-2")>
                {icon(Icon::Warning, "mt-0.5 h-4 w-4 flex-none")}
                "Add a brief answer before sending. One or two sentences per question is enough."
            </p>
        </Show>
    }
    .into_any();

    let analysis = view! {
        <div class="flex flex-col gap-3">
            {ANALYSIS_HEADINGS
                .iter()
                .enumerate()
                .map(|(i, heading)| view! {
                    <div class=FIELD>
                        <label class=LBL for=format!("memo-analysis-{i}")>{*heading}</label>
                        <textarea
                            id=format!("memo-analysis-{i}")
                            class=INPUT
                            rows="2"
                            maxlength=MAX_LONG
                            prop:value=move || content.with(|c| c.analysis[i].clone())
                            on:input=move |ev| { let v = event_target_value(&ev); content.update(|c| c.analysis[i] = v); touch.run(()); }
                        ></textarea>
                    </div>
                })
                .collect_view()}
        </div>
    }
    .into_any();

    let tables = Tables {
        content,
        touch,
        attempted,
    };
    let title_input = view! {
        <label for="memo-title" class="sr-only">"Memo title"</label>
        <h1 class="font-serif text-[30px] font-semibold leading-tight text-slate-200">
            "Legal memo: "
            <input
                id="memo-title"
                maxlength=MAX_SHORT
                class="min-w-[10ch] max-w-full rounded-md border-0 border-b-2 border-dashed border-sand-500 bg-transparent p-0 font-serif text-[30px] font-semibold text-slate-200 [field-sizing:content] focus:border-primary-600 focus:outline-none"
                prop:value=move || title.get()
                on:input=move |ev| { title.set(event_target_value(&ev)); touch.run(()); }
            />
        </h1>
    };
    let pills = view! { {status_pill(&memo)} <VolunteersOnlyBadge /> };

    view! {
        <PageHeader back_href=back_href(&memo.case_id) back_label=back_label(&memo) actions=Box::new(move || actions(true).into_any())>
            {title_input}
            <div class="mt-2 flex flex-wrap gap-2">{pills}</div>
        </PageHeader>
        <DarkBanner>"Draft for attorney review. Not for client distribution."</DarkBanner>
        <Show when=move || autosave.failed.with(Option::is_some)>
            <div class="flex flex-wrap items-center gap-3 rounded-2xl border-2 border-danger-600 bg-danger-50 px-5 py-3 text-danger-700" role="alert">
                <p class="font-semibold">{move || format!("Not saved: {}", autosave.failed.get().unwrap_or_default())}</p>
                <Show when=move || draft_error.with(Option::is_none)>
                    <button type="button" class=BTN_SM on:click=move |_| { let _ = on_changed.try_run(()); }>"Reload"</button>
                </Show>
            </div>
        </Show>
        {is_revision.then(|| view! {
            {move || revision_note().map(|entry| view! {
                <div class="rounded-2xl border border-caution-300 bg-caution-50 px-5 py-3.5 text-caution-800" role="note">
                    <p class="font-semibold">{format!("Revisions requested by {}, {}", entry.actor_name, when(&entry.at))}</p>
                    <p class="mt-1 whitespace-pre-line">{entry.comment}</p>
                </div>
            })}
        })}
        <Show when=move || !problems.with(Vec::is_empty)>
            <div class="rounded-2xl border-2 border-danger-600 bg-danger-50 px-5 py-4" role="alert">
                <p class="flex items-center gap-2 font-bold text-danger-700">
                    {icon(Icon::Warning, "h-4 w-4 flex-none")}
                    {move || {
                        let n = problems.with(Vec::len);
                        format!("Can't send for attorney review yet: {n} {} to fix", if n == 1 { "thing" } else { "things" })
                    }}
                </p>
                <ul class="mt-2 flex list-disc flex-col gap-1 pl-[26px]">
                    {move || problems
                        .get()
                        .into_iter()
                        .map(|p| {
                            let anchor = SECTIONS.iter().find(|s| s.problem_key == p.section).map(|s| s.anchor).unwrap_or("mh");
                            view! { <li><a class="text-primary-300 underline-offset-[3px]" href=format!("#{anchor}")>{p.message}</a></li> }
                        })
                        .collect_view()}
                </ul>
                <p class=format!("{MUTED} mt-2 text-[13.5px]")>{draft_line}</p>
            </div>
        </Show>
        <Show when=move || !message.get().is_empty()>
            <p class="whitespace-pre-line text-[14.5px] font-semibold text-danger-700" role="alert">{move || message.get()}</p>
        </Show>
        <p class=format!("{MUTED} text-[13.5px]")>"Sections marked Required must be filled in before sending. The rest can be skipped on a simple matter."</p>
        <div class="grid items-start gap-6 lg:grid-cols-[250px_minmax(0,1fr)]">
            {nav}
            <div class="flex min-w-0 flex-col gap-4">
                {card("mh", header)}
                {card("ms", safety)}
                {card("s1", list_rows(|c| &c.questions, |c| &mut c.questions, "Question"))}
                {card("s2", brief)}
                {card("s3", tables.facts())}
                {card("s4", tables.procedure())}
                {card("s5", tables.evidence())}
                {card("s6", tables.authorities())}
                {card("s7", analysis)}
                {card("s8", list_rows(|c| &c.outstanding, |c| &mut c.outstanding, "Outstanding question"))}
                {card("s9", tables.next_steps())}
                <div class="flex flex-wrap items-center justify-end gap-2">{actions(false)}</div>
            </div>
        </div>
    }
}
