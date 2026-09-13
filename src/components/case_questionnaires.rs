use std::collections::BTreeMap;

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;

use crate::helpers::case_questionnaires::{
    answers_for, followups_for_answers, is_complete, required_followups, validate_answers,
    CaseQuestionnaire, QuestionnaireInput, QuestionnaireQuestion, GENERAL_QUESTIONNAIRE,
};
use crate::server_fns::case_properties::{self, CaseProperty};
use crate::server_fns::err_text;

const INPUT_CLASS: &str = "mt-1 w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2.5 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/30";

#[component]
pub fn CaseQuestionnaires(
    case_id: String,
    properties: Vec<CaseProperty>,
    can_complete: bool,
    on_saved: Callback<()>,
) -> impl IntoView {
    let general_complete = is_complete(&properties, &GENERAL_QUESTIONNAIRE);
    let followups = required_followups(&properties);
    let active_followups = followups
        .iter()
        .filter(|required| !is_complete(&properties, required.questionnaire))
        .copied()
        .collect::<Vec<_>>();
    let completed_followups = followups
        .iter()
        .filter(|required| is_complete(&properties, required.questionnaire))
        .copied()
        .collect::<Vec<_>>();

    let active_views = active_followups
        .into_iter()
        .map(|required| {
            questionnaire_card(
                case_id.clone(),
                required.questionnaire,
                Some(required.reason),
                &properties,
                can_complete,
                on_saved,
            )
        })
        .collect_view();

    let active_count = if general_complete {
        followups
            .iter()
            .filter(|required| !is_complete(&properties, required.questionnaire))
            .count()
    } else {
        1
    };

    let active_tasks = if general_complete {
        if active_count == 0 {
            view! {
                <div class="rounded-lg border border-emerald-500/30 bg-emerald-500/10 px-4 py-3">
                    <p class="text-sm font-medium text-emerald-300">"No active intake tasks"</p>
                    <p class="mt-1 text-xs text-emerald-200/70">
                        "The currently required questionnaires are complete."
                    </p>
                </div>
            }
            .into_any()
        } else {
            view! { <div class="space-y-4">{active_views}</div> }.into_any()
        }
    } else {
        questionnaire_card(
            case_id.clone(),
            &GENERAL_QUESTIONNAIRE,
            None,
            &properties,
            can_complete,
            on_saved,
        )
    };

    let mut completed_views = Vec::new();
    if general_complete {
        completed_views.push(questionnaire_card(
            case_id.clone(),
            &GENERAL_QUESTIONNAIRE,
            None,
            &properties,
            can_complete,
            on_saved,
        ));
    }
    completed_views.extend(completed_followups.into_iter().map(|required| {
        questionnaire_card(
            case_id.clone(),
            required.questionnaire,
            Some(required.reason),
            &properties,
            can_complete,
            on_saved,
        )
    }));
    let has_completed = !completed_views.is_empty();
    let completed_section = if has_completed {
        view! {
            <section>
                <div class="mb-3">
                    <h2 class="text-lg font-semibold text-slate-100">"Completed questionnaires"</h2>
                    <p class="mt-1 text-sm text-slate-500">
                        "Review or update previously submitted intake answers."
                    </p>
                </div>
                <div class="space-y-4 rounded-xl border border-slate-800 bg-slate-900 p-4">
                    {completed_views}
                </div>
            </section>
        }
        .into_any()
    } else {
        ().into_any()
    };

    view! {
        <div class="space-y-6">
            <section>
                <div class="mb-3 flex flex-wrap items-center justify-between gap-2">
                    <div>
                        <h2 class="text-lg font-semibold text-slate-100">"Active intake tasks"</h2>
                        <p class="mt-1 text-sm text-slate-500">
                            "Complete these forms to move the intake forward."
                        </p>
                    </div>
                    <span class=if active_count == 0 {
                        "rounded-full bg-emerald-500/15 px-2.5 py-1 text-xs font-medium text-emerald-300 ring-1 ring-emerald-500/30"
                    } else {
                        "rounded-full bg-amber-500/15 px-2.5 py-1 text-xs font-medium text-amber-300 ring-1 ring-amber-500/30"
                    }>
                        {format!("{active_count} active")}
                    </span>
                </div>
                <div class="rounded-xl border border-amber-500/30 bg-slate-900 p-4">
                    {active_tasks}
                </div>
            </section>

            {completed_section}
        </div>
    }
}

#[component]
pub fn CaseIntakeTaskSummary(case_id: String, properties: Vec<CaseProperty>) -> impl IntoView {
    let general_complete = is_complete(&properties, &GENERAL_QUESTIONNAIRE);
    let followups = required_followups(&properties);
    let active_followups = followups
        .iter()
        .filter(|required| !is_complete(&properties, required.questionnaire))
        .count();
    let active_count = if general_complete {
        active_followups
    } else {
        1
    };
    let summary = if !general_complete {
        "Complete the general intake questionnaire to identify any follow-up work.".to_string()
    } else if active_count == 0 {
        "All currently required intake questionnaires are complete.".to_string()
    } else if active_count == 1 {
        "One follow-up questionnaire needs attention.".to_string()
    } else {
        format!("{active_count} follow-up questionnaires need attention.")
    };
    let href = format!("/cases/{case_id}/intake");

    view! {
        <section>
            <div class="mb-3">
                <h2 class="text-lg font-semibold text-slate-100">"Active tasks"</h2>
                <p class="mt-1 text-sm text-slate-500">"Work that needs attention on this case."</p>
            </div>
            <div class="rounded-xl border border-amber-500/30 bg-slate-900 p-4">
                <div class="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
                    <div>
                        <div class="flex flex-wrap items-center gap-2">
                            <h3 class="text-sm font-semibold text-slate-100">"Case intake"</h3>
                            <span class=if active_count == 0 {
                                "rounded-full bg-emerald-500/15 px-2 py-0.5 text-xs font-medium text-emerald-300"
                            } else {
                                "rounded-full bg-amber-500/15 px-2 py-0.5 text-xs font-medium text-amber-300"
                            }>
                                {if active_count == 0 {
                                    "Complete".to_string()
                                } else {
                                    format!("{active_count} active")
                                }}
                            </span>
                        </div>
                        <p class="mt-1 text-sm text-slate-400">{summary}</p>
                    </div>
                    <A
                        href=href
                        attr:class="shrink-0 rounded-lg bg-primary-500 px-4 py-2 text-center text-sm font-semibold text-white hover:bg-primary-600"
                    >
                        "Open intake"
                    </A>
                </div>
            </div>
        </section>
    }
}

fn questionnaire_card(
    case_id: String,
    questionnaire: &'static CaseQuestionnaire,
    reason: Option<&'static str>,
    properties: &[CaseProperty],
    can_complete: bool,
    on_saved: Callback<()>,
) -> AnyView {
    let complete = is_complete(properties, questionnaire);
    let answers = RwSignal::new(answers_for(properties, questionnaire));
    let expanded = RwSignal::new(!complete);
    let pending = RwSignal::new(false);
    let error = RwSignal::new(String::new());

    let save = move || {
        if pending.get_untracked() {
            return;
        }
        let submitted = answers.get_untracked();
        if let Err(message) = validate_answers(questionnaire, &submitted) {
            error.set(message);
            return;
        }

        let case_id = case_id.clone();
        pending.set(true);
        error.set(String::new());
        spawn_local(async move {
            match case_properties::save_case_questionnaire(
                case_id,
                questionnaire.slug.to_string(),
                submitted,
            )
            .await
            {
                Ok(()) => {
                    pending.set(false);
                    expanded.set(false);
                    on_saved.run(());
                }
                Err(server_error) => {
                    pending.set(false);
                    error.set(err_text(server_error));
                }
            }
        });
    };

    let questions = questionnaire
        .questions
        .iter()
        .map(|question| question_control(*question, answers))
        .collect_view();

    let trigger_preview = (questionnaire.slug == GENERAL_QUESTIONNAIRE.slug).then(|| {
        view! {
            <div class="rounded-lg border border-sky-500/30 bg-sky-500/10 p-3">
                <h4 class="text-sm font-semibold text-sky-200">"This intake will create"</h4>
                {move || {
                    let triggered = followups_for_answers(&answers.get());
                    if triggered.is_empty() {
                        return view! {
                            <p class="mt-1 text-xs text-sky-200/70">
                                "No additional forms based on the answers selected so far."
                            </p>
                        }
                            .into_any();
                    }
                    view! {
                        <ul class="mt-2 space-y-2">
                            {triggered
                                .into_iter()
                                .map(|required| view! {
                                    <li class="text-xs text-sky-100">
                                        <span class="font-semibold">{required.questionnaire.title}</span>
                                        <span class="block text-sky-200/70">{required.reason}</span>
                                    </li>
                                })
                                .collect_view()}
                        </ul>
                    }
                        .into_any()
                }}
            </div>
        }
    });

    view! {
        <article class="rounded-lg border border-slate-800 bg-slate-950 p-4">
            <div class="flex items-start justify-between gap-3">
                <div>
                    <div class="flex flex-wrap items-center gap-2">
                        <h3 class="text-sm font-semibold text-slate-100">{questionnaire.title}</h3>
                        <span class=if complete {
                            "rounded-full bg-emerald-500/15 px-2 py-0.5 text-xs font-medium text-emerald-300"
                        } else {
                            "rounded-full bg-amber-500/15 px-2 py-0.5 text-xs font-medium text-amber-300"
                        }>
                            {if complete { "Complete" } else { "Needed" }}
                        </span>
                    </div>
                    <p class="mt-1 text-xs text-slate-400">{questionnaire.description}</p>
                    {reason.map(|text| view! {
                        <p class="mt-2 text-xs font-medium text-amber-300">{text}</p>
                    })}
                </div>
                {if complete && can_complete {
                    view! {
                        <button
                            type="button"
                            on:click=move |_| expanded.update(|open| *open = !*open)
                            class="shrink-0 rounded-lg border border-slate-700 px-2.5 py-1.5 text-xs font-medium text-slate-300 hover:bg-slate-800"
                        >
                            {move || if expanded.get() { "Close" } else { "Edit answers" }}
                        </button>
                    }
                        .into_any()
                } else {
                    ().into_any()
                }}
            </div>

            <form
                class=move || {
                    if expanded.get() && can_complete {
                        "mt-4 space-y-4 border-t border-slate-800 pt-4"
                    } else {
                        "hidden"
                    }
                }
                on:submit=move |event| {
                    event.prevent_default();
                    save();
                }
            >
                {questions}
                {trigger_preview}
                <Show when=move || !error.get().is_empty()>
                    <p class="text-sm text-rose-300" role="alert">{move || error.get()}</p>
                </Show>
                <button
                    type="submit"
                    prop:disabled=move || pending.get()
                    class="rounded-lg bg-primary-500 px-4 py-2 text-sm font-semibold text-white hover:bg-primary-600 disabled:cursor-not-allowed disabled:opacity-60"
                >
                    {move || if pending.get() { "Saving\u{2026}" } else { "Submit questionnaire" }}
                </button>
            </form>

            <Show when=move || !can_complete && !complete>
                <p class="mt-3 text-xs text-slate-500">
                    "Edit access is required to complete this questionnaire."
                </p>
            </Show>
        </article>
    }
    .into_any()
}

fn question_control(
    question: QuestionnaireQuestion,
    answers: RwSignal<BTreeMap<String, String>>,
) -> AnyView {
    let answer =
        move || answers.with(|values| values.get(question.key).cloned().unwrap_or_default());
    let set_answer = move |value: String| {
        answers.update(|values| {
            values.insert(question.key.to_string(), value);
        });
    };
    let control = match question.input {
        QuestionnaireInput::Select(options) => view! {
            <select
                class=INPUT_CLASS
                required=question.required
                on:change=move |event| set_answer(event_target_value(&event))
            >
                <option value="">"Select an answer"</option>
                {options
                    .iter()
                    .map(|option| {
                        let option = *option;
                        view! {
                            <option value=option selected=move || answer() == option>{option}</option>
                        }
                    })
                    .collect_view()}
            </select>
        }
        .into_any(),
        QuestionnaireInput::Text => view! {
            <input
                class=INPUT_CLASS
                required=question.required
                prop:value=answer
                on:input=move |event| set_answer(event_target_value(&event))
            />
        }
        .into_any(),
        QuestionnaireInput::TextArea => view! {
            <textarea
                class=INPUT_CLASS
                rows="3"
                required=question.required
                prop:value=answer
                on:input=move |event| set_answer(event_target_value(&event))
            ></textarea>
        }
        .into_any(),
    };

    view! {
        <div>
            <label class="block text-sm font-medium text-slate-300">
                {question.label}
                {if question.required {
                    view! { <span class="text-rose-400" aria-hidden="true">" *"</span> }.into_any()
                } else {
                    view! { <span class="font-normal text-slate-500">" (optional)"</span> }.into_any()
                }}
            </label>
            {control}
        </div>
    }
    .into_any()
}
