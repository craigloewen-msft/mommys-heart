use std::collections::{BTreeMap, HashMap};

use leptos::prelude::*;

use crate::helpers::case_intake::{
    court_label, docket_number_label, field_keys, judge_label, CaseIntake, CourtDocket,
    IntakeInput, IntakeItem, IntakeRequirement, EXTRA_NOTES_LABEL, INTAKE_ITEMS,
};

#[derive(Clone, Copy)]
pub struct CaseIntakeState {
    /// One text signal per scalar question, keyed by its storage key; see
    /// [`INTAKE_ITEMS`]. Held in a `StoredValue` so the state stays `Copy`.
    fields: StoredValue<HashMap<&'static str, RwSignal<String>>>,
    judges: RwSignal<Vec<(usize, String)>>,
    courts: RwSignal<Vec<(usize, CourtDocket)>>,
    extra_notes: RwSignal<String>,
}

impl CaseIntakeState {
    pub fn new() -> Self {
        let fields = field_keys()
            .map(|key| (key, RwSignal::new(String::new())))
            .collect::<HashMap<_, _>>();
        Self {
            fields: StoredValue::new(fields),
            judges: RwSignal::new(vec![(0, String::new()), (1, String::new())]),
            courts: RwSignal::new(vec![
                (0, CourtDocket::default()),
                (1, CourtDocket::default()),
            ]),
            extra_notes: RwSignal::new(String::new()),
        }
    }

    /// The signal backing one scalar question. Every [`INTAKE_ITEMS`] field is
    /// seeded in [`Self::new`], so a known key always resolves.
    fn field(&self, key: &'static str) -> RwSignal<String> {
        self.fields
            .with_value(|fields| fields.get(key).copied())
            .expect("a signal exists for every intake field key")
    }

    pub fn value(self) -> CaseIntake {
        let fields = self.fields.with_value(|fields| {
            fields
                .iter()
                .map(|(key, signal)| ((*key).to_string(), signal.get_untracked()))
                .collect::<BTreeMap<String, String>>()
        });
        CaseIntake {
            fields,
            judges: self
                .judges
                .get_untracked()
                .into_iter()
                .map(|(_, value)| value)
                .collect(),
            courts: self
                .courts
                .get_untracked()
                .into_iter()
                .map(|(_, value)| value)
                .collect(),
            extra_notes: self.extra_notes.get_untracked(),
        }
    }
}

const INPUT_CLASS: &str = "mt-1 w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2.5 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/30";
const LABEL_CLASS: &str = "block text-sm font-medium text-slate-300";

#[component]
pub fn CaseIntakeFields(state: CaseIntakeState) -> impl IntoView {
    // Render straight from `INTAKE_ITEMS`: consecutive scalar questions share
    // one responsive grid, while judges and courts each get their own section.
    // The first block sits flush; every later block gets a top divider.
    let mut blocks: Vec<AnyView> = Vec::new();
    let mut run: Vec<(
        &'static str,
        &'static str,
        &'static IntakeInput,
        IntakeRequirement,
    )> = Vec::new();

    for item in INTAKE_ITEMS {
        match item {
            IntakeItem::Field {
                key,
                label,
                input,
                requirement,
            } => run.push((*key, *label, input, *requirement)),
            IntakeItem::Judges { heading } | IntakeItem::Courts { heading } => {
                if !run.is_empty() {
                    let separated = !blocks.is_empty();
                    blocks.push(fields_grid(std::mem::take(&mut run), separated, state));
                }
                let separated = !blocks.is_empty();
                blocks.push(match item {
                    IntakeItem::Judges { .. } => judges_section(*heading, separated, state),
                    _ => courts_section(*heading, separated, state),
                });
            }
        }
    }
    if !run.is_empty() {
        let separated = !blocks.is_empty();
        blocks.push(fields_grid(run, separated, state));
    }

    blocks.push(notes_section(state));

    blocks.into_iter().collect_view()
}

/// A responsive grid of scalar questions.
fn fields_grid(
    fields: Vec<(
        &'static str,
        &'static str,
        &'static IntakeInput,
        IntakeRequirement,
    )>,
    separated: bool,
    state: CaseIntakeState,
) -> AnyView {
    let class = if separated {
        "mt-7 grid gap-5 border-t border-slate-800 pt-6 sm:grid-cols-2"
    } else {
        "grid gap-5 sm:grid-cols-2"
    };
    view! {
        <div class=class>
            {fields
                .into_iter()
                .map(|(key, label, input, requirement)| {
                    field_control(key, label, input, requirement, state)
                })
                .collect_view()}
        </div>
    }
    .into_any()
}

/// One labelled scalar question: a dropdown, a text line, or a rate line.
fn field_control(
    key: &'static str,
    label: &'static str,
    input: &'static IntakeInput,
    requirement: IntakeRequirement,
    state: CaseIntakeState,
) -> AnyView {
    let signal = state.field(key);
    let required = requirement.is_required();
    let control = match input {
        IntakeInput::Select(options) => {
            let options = *options;
            view! {
                <select
                    class=INPUT_CLASS
                    required=required
                    on:change=move |event| signal.set(event_target_value(&event))
                >
                    <option value="">"Select an answer"</option>
                    {options
                        .iter()
                        .map(|option| {
                            let option = *option;
                            view! { <option value=option>{option}</option> }
                        })
                        .collect_view()}
                </select>
            }
            .into_any()
        }
        IntakeInput::Text => view! {
            <input
                class=INPUT_CLASS
                required=required
                prop:value=move || signal.get()
                on:input=move |event| signal.set(event_target_value(&event))
            />
        }
        .into_any(),
        IntakeInput::Rate => view! {
            <input
                class=INPUT_CLASS
                inputmode="decimal"
                required=required
                prop:value=move || signal.get()
                on:input=move |event| signal.set(event_target_value(&event))
            />
        }
        .into_any(),
    };
    view! {
        <div>
            <label class=LABEL_CLASS>
                {label}
                {if required {
                    view! { <span class="text-rose-400" aria-hidden="true">" *"</span> }
                        .into_any()
                } else {
                    view! { <span class="font-normal text-slate-500">" (optional)"</span> }
                        .into_any()
                }}
            </label>
            {control}
        </div>
    }
    .into_any()
}

/// The optional free-text notes block, always full width and last.
fn notes_section(state: CaseIntakeState) -> AnyView {
    let signal = state.extra_notes;
    view! {
        <div class="mt-7 border-t border-slate-800 pt-6">
            <label class=LABEL_CLASS>{EXTRA_NOTES_LABEL}</label>
            <textarea
                class=INPUT_CLASS
                rows="4"
                prop:value=move || signal.get()
                on:input=move |event| signal.set(event_target_value(&event))
            ></textarea>
        </div>
    }
    .into_any()
}

fn judges_section(heading: &'static str, separated: bool, state: CaseIntakeState) -> AnyView {
    let class = if separated {
        "mt-7 border-t border-slate-800 pt-6"
    } else {
        ""
    };
    view! {
        <div class=class>
            <div class="mb-3">
                <h3 class="text-sm font-semibold text-slate-200">
                    {heading}
                    <span class="font-normal text-slate-500">" (optional)"</span>
                </h3>
            </div>
            <div class="grid gap-4 sm:grid-cols-2">
                <For
                    each=move || state.judges.get()
                    key=|(id, _)| *id
                    children=move |(id, value)| view! {
                        <div>
                            <label class=LABEL_CLASS>{judge_label(id)}</label>
                            <input
                                class=INPUT_CLASS
                                prop:value=value
                                on:input=move |event| {
                                    let value = event_target_value(&event);
                                    state.judges.update(|rows| {
                                        if let Some((_, current)) = rows.iter_mut().find(|(row_id, _)| *row_id == id) {
                                            *current = value;
                                        }
                                    });
                                }
                            />
                        </div>
                    }
                />
            </div>
        </div>
    }
    .into_any()
}

fn courts_section(heading: &'static str, separated: bool, state: CaseIntakeState) -> AnyView {
    let class = if separated {
        "mt-7 border-t border-slate-800 pt-6"
    } else {
        ""
    };
    view! {
        <div class=class>
            <div class="mb-3">
                <h3 class="text-sm font-semibold text-slate-200">
                    {heading}
                    <span class="font-normal text-slate-500">" (optional)"</span>
                </h3>
            </div>
            <div class="space-y-4">
                <For
                    each=move || state.courts.get()
                    key=|(id, _)| *id
                    children=move |(id, entry)| view! {
                        <div class="grid gap-4 sm:grid-cols-2">
                            <div>
                                <label class=LABEL_CLASS>{court_label(id)}</label>
                                <input
                                    class=INPUT_CLASS
                                    prop:value=entry.court
                                    on:input=move |event| {
                                        let value = event_target_value(&event);
                                        state.courts.update(|rows| {
                                            if let Some((_, current)) = rows.iter_mut().find(|(row_id, _)| *row_id == id) {
                                                current.court = value;
                                            }
                                        });
                                    }
                                />
                            </div>
                            <div>
                                <label class=LABEL_CLASS>{docket_number_label(id)}</label>
                                <input
                                    class=INPUT_CLASS
                                    prop:value=entry.docket_number
                                    on:input=move |event| {
                                        let value = event_target_value(&event);
                                        state.courts.update(|rows| {
                                            if let Some((_, current)) = rows.iter_mut().find(|(row_id, _)| *row_id == id) {
                                                current.docket_number = value;
                                            }
                                        });
                                    }
                                />
                            </div>
                        </div>
                    }
                />
            </div>
        </div>
    }
    .into_any()
}
