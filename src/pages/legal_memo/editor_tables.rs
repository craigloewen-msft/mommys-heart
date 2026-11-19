use leptos::prelude::*;

use super::editor::add_row;
use super::{AUTHORITY_HELP, EVIDENCE_HELP, FACT_HELP, PROCEDURE_HELP};
use crate::components::design::{
    icon, EdgeTable, Icon, BTN_SM, ERRMSG, HINT, INPUT, INPUT_ERR, TD,
};
use crate::server_fns::legal_memos::{
    Authority, EventStatus, EvidenceItem, EvidenceStatus, FactSource, LegalMemoContent, MemoFact,
    NextStep, ProcedureEvent, MAX_LONG, MAX_ROWS, MAX_SHORT,
};

const CELL: &str = "border-b border-sand-300 px-3.5 py-3 align-top min-w-[150px]";

#[derive(Clone, Copy)]
pub(super) struct Tables {
    pub content: RwSignal<LegalMemoContent>,
    pub touch: Callback<()>,
    pub attempted: RwSignal<bool>,
}

type Rows<T> = fn(&LegalMemoContent) -> &Vec<T>;
type RowsMut<T> = fn(&mut LegalMemoContent) -> &mut Vec<T>;

impl Tables {
    #[allow(clippy::too_many_arguments)]
    fn input<T: 'static>(
        self,
        rows: Rows<T>,
        rows_mut: RowsMut<T>,
        i: usize,
        get: fn(&T) -> &String,
        set: fn(&mut T) -> &mut String,
        label: String,
        placeholder: &'static str,
        max: usize,
    ) -> AnyView {
        let content = self.content;
        let touch = self.touch;
        view! {
            <input
                class=INPUT
                maxlength=max
                aria-label=label
                placeholder=placeholder
                prop:value=move || content.with(|c| rows(c).get(i).map(|r| get(r).clone()).unwrap_or_default())
                on:input=move |ev| {
                    let v = event_target_value(&ev);
                    content.update(|c| if let Some(r) = rows_mut(c).get_mut(i) { *set(r) = v; });
                    touch.run(());
                }
            />
        }
        .into_any()
    }

    fn remove<T: 'static>(self, rows_mut: RowsMut<T>, i: usize, noun: &'static str) -> AnyView {
        let content = self.content;
        let touch = self.touch;
        view! {
            <td class=format!("{TD} w-12")>
                <button
                    type="button"
                    class=format!("{BTN_SM} px-2.5")
                    aria-label=format!("Remove {noun} {}", i + 1)
                    on:click=move |_| {
                        content.update(|c| { let r = rows_mut(c); if i < r.len() { r.remove(i); } });
                        touch.run(());
                    }
                >
                    {icon(Icon::Cross, "h-4 w-4")}
                </button>
            </td>
        }
        .into_any()
    }

    fn add<T: Default + 'static>(
        self,
        rows: Rows<T>,
        rows_mut: RowsMut<T>,
        label: &'static str,
    ) -> AnyView {
        let content = self.content;
        let touch = self.touch;
        add_row(
            label,
            Signal::derive(move || content.with(|c| rows(c).len() >= MAX_ROWS)),
            Callback::new(move |_| {
                content.update(|c| {
                    if rows(c).len() < MAX_ROWS {
                        rows_mut(c).push(T::default());
                    }
                });
                touch.run(());
            }),
        )
    }

    fn count<T: 'static>(self, rows: Rows<T>) -> impl Fn() -> std::ops::Range<usize> + Copy {
        let content = self.content;
        move || 0..content.with(|c| rows(c).len())
    }

    pub(super) fn facts(self) -> AnyView {
        let t = self;
        let content = self.content;
        let rows: Rows<MemoFact> = |c| &c.facts;
        let rows_mut: RowsMut<MemoFact> = |c| &mut c.facts;
        view! {
            <p class=format!("{HINT} mb-3")>{FACT_HELP}</p>
            <EdgeTable headers=vec![("Fact", None), ("Source", Some("190px")), ("", Some("48px"))]>
                <For each=t.count(rows) key=|i| *i let:i>
                    <tr>
                        <td class=CELL>{t.input(rows, rows_mut, i, |f| &f.text, |f| &mut f.text, format!("Fact {}", i + 1), "", MAX_LONG)}</td>
                        <td class=CELL>
                            <select
                                class=INPUT
                                aria-label=format!("Source for fact {}", i + 1)
                                prop:value=move || content.with(|c| c.facts.get(i).and_then(|f| f.source).map(|s| s.slug()).unwrap_or_default())
                                on:change=move |ev| {
                                    let v = event_target_value(&ev);
                                    content.update(|c| if let Some(f) = c.facts.get_mut(i) { f.source = FactSource::from_slug(&v); });
                                    t.touch.run(());
                                }
                            >
                                <Show when=move || content.with(|c| c.facts.get(i).is_some_and(|f| f.source.is_none()))>
                                    <option value="" selected=true>"Choose a source"</option>
                                </Show>
                                {FactSource::ALL
                                    .iter()
                                    .map(|s| {
                                        let s = *s;
                                        view! {
                                            <option value=s.slug() selected=move || content.with(|c| c.facts.get(i).and_then(|f| f.source) == Some(s))>{s.label()}</option>
                                        }
                                    })
                                    .collect_view()}
                            </select>
                        </td>
                        {t.remove(rows_mut, i, "fact")}
                    </tr>
                </For>
            </EdgeTable>
            {t.add(rows, rows_mut, "Add a fact")}
        }
        .into_any()
    }

    pub(super) fn procedure(self) -> AnyView {
        let t = self;
        let content = self.content;
        let rows: Rows<ProcedureEvent> = |c| &c.procedure;
        let rows_mut: RowsMut<ProcedureEvent> = |c| &mut c.procedure;
        view! {
            <p class=format!("{HINT} mb-3")>{PROCEDURE_HELP}</p>
            <EdgeTable headers=vec![("Date", Some("140px")), ("Event", None), ("Source", Some("210px")), ("Status", Some("170px")), ("", Some("48px"))]>
                <For each=t.count(rows) key=|i| *i let:i>
                    {
                        let no_source = move || content.with(|c| c.procedure.get(i).is_some_and(|p| p.source.trim().is_empty()));
                        let confirmed = move || content.with(|c| c.procedure.get(i).is_some_and(|p| p.status == EventStatus::Confirmed));
                        let bad = move || t.attempted.get() && no_source() && confirmed();
                        view! {
                            <tr class=move || if bad() { "bg-danger-50" } else { "" }>
                                <td class=CELL>{t.input(rows, rows_mut, i, |p| &p.date, |p| &mut p.date, format!("Event {} date", i + 1), "Jun 2026", MAX_SHORT)}</td>
                                <td class=CELL>{t.input(rows, rows_mut, i, |p| &p.event, |p| &mut p.event, format!("Event {}", i + 1), "", MAX_LONG)}</td>
                                <td class=CELL>{t.input(rows, rows_mut, i, |p| &p.source, |p| &mut p.source, format!("Event {} source", i + 1), "Court notice (Document)", MAX_SHORT)}</td>
                                <td class=CELL>
                                    <select
                                        class=move || if bad() { INPUT_ERR } else { INPUT }
                                        aria-label=format!("Event {} status", i + 1)
                                        prop:value=move || if confirmed() { EventStatus::Confirmed.slug() } else { EventStatus::Unconfirmed.slug() }
                                        on:change=move |ev| {
                                            let v = event_target_value(&ev);
                                            content.update(|c| if let Some(p) = c.procedure.get_mut(i) { p.status = EventStatus::from_slug(&v).unwrap_or(EventStatus::Unconfirmed); });
                                            t.touch.run(());
                                        }
                                    >
                                        <option value="unconfirmed" selected=move || !confirmed()>"Unconfirmed"</option>
                                        <option value="confirmed" selected=confirmed disabled=move || no_source() && !confirmed()>"Confirmed"</option>
                                    </select>
                                    <Show when=no_source>
                                        <span class=format!("{HINT} mt-1 block")>"Add a source document to confirm"</span>
                                    </Show>
                                </td>
                                {t.remove(rows_mut, i, "event")}
                            </tr>
                        }
                    }
                </For>
            </EdgeTable>
            {t.add(rows, rows_mut, "Add an event")}
        }
        .into_any()
    }

    pub(super) fn evidence(self) -> AnyView {
        let t = self;
        let content = self.content;
        let rows: Rows<EvidenceItem> = |c| &c.evidence;
        let rows_mut: RowsMut<EvidenceItem> = |c| &mut c.evidence;
        view! {
            <p class=format!("{HINT} mb-3")>{EVIDENCE_HELP}</p>
            <EdgeTable headers=vec![("Item", None), ("Source", Some("140px")), ("Status", Some("170px")), ("Date received", Some("140px")), ("Follow-up", None), ("", Some("48px"))]>
                <For each=t.count(rows) key=|i| *i let:i>
                    {
                        let status = move || content.with(|c| c.evidence.get(i).map(|e| e.status).unwrap_or(EvidenceStatus::NotReceived));
                        let no_date = move || content.with(|c| c.evidence.get(i).is_some_and(|e| e.date_received.trim().is_empty()));
                        let bad = move || t.attempted.get() && status() == EvidenceStatus::Reviewed && no_date();
                        view! {
                            <tr class=move || if bad() { "bg-danger-50" } else { "" }>
                                <td class=CELL>{t.input(rows, rows_mut, i, |e| &e.item, |e| &mut e.item, format!("Evidence item {}", i + 1), "", MAX_LONG)}</td>
                                <td class=CELL>{t.input(rows, rows_mut, i, |e| &e.source, |e| &mut e.source, format!("Evidence item {} source", i + 1), "", MAX_SHORT)}</td>
                                <td class=CELL>
                                    <select
                                        class=move || if bad() { INPUT_ERR } else { INPUT }
                                        aria-label=format!("Evidence item {} status", i + 1)
                                        prop:value=move || status().slug()
                                        on:change=move |ev| {
                                            let v = event_target_value(&ev);
                                            content.update(|c| if let Some(e) = c.evidence.get_mut(i) { e.status = EvidenceStatus::from_slug(&v).unwrap_or(EvidenceStatus::NotReceived); });
                                            t.touch.run(());
                                        }
                                    >
                                        {EvidenceStatus::ALL
                                            .iter()
                                            .map(|s| {
                                                let s = *s;
                                                view! {
                                                    <option
                                                        value=s.slug()
                                                        selected=move || status() == s
                                                        disabled=move || s == EvidenceStatus::Reviewed && status() != s && no_date()
                                                    >
                                                        {s.label()}
                                                    </option>
                                                }
                                            })
                                            .collect_view()}
                                    </select>
                                    <Show when=bad>
                                        <p class=format!("{ERRMSG} mt-1.5")>
                                            {icon(Icon::Warning, "mt-0.5 h-4 w-4 flex-none")}
                                            "Can't be Reviewed: it hasn't been received. Add the date received or change the status."
                                        </p>
                                    </Show>
                                </td>
                                <td class=CELL>
                                    <input
                                        class=move || if bad() { INPUT_ERR } else { INPUT }
                                        aria-label=format!("Evidence item {} date received", i + 1)
                                        placeholder="Not yet"
                                        maxlength=MAX_SHORT
                                        prop:value=move || content.with(|c| c.evidence.get(i).map(|e| e.date_received.clone()).unwrap_or_default())
                                        on:input=move |ev| {
                                            let v = event_target_value(&ev);
                                            content.update(|c| if let Some(e) = c.evidence.get_mut(i) { e.date_received = v; });
                                            t.touch.run(());
                                        }
                                    />
                                </td>
                                <td class=CELL>{t.input(rows, rows_mut, i, |e| &e.follow_up, |e| &mut e.follow_up, format!("Evidence item {} follow-up", i + 1), "", MAX_LONG)}</td>
                                {t.remove(rows_mut, i, "item")}
                            </tr>
                        }
                    }
                </For>
            </EdgeTable>
            {t.add(rows, rows_mut, "Add an item")}
        }
        .into_any()
    }

    pub(super) fn authorities(self) -> AnyView {
        let t = self;
        let content = self.content;
        let rows: Rows<Authority> = |c| &c.authorities;
        let rows_mut: RowsMut<Authority> = |c| &mut c.authorities;
        view! {
            <EdgeTable headers=vec![("Citation", None), ("Court level", Some("200px")), ("Link", Some("170px")), ("Date checked", Some("110px")), ("Subsequent treatment", Some("130px")), ("", Some("48px"))]>
                <For each=t.count(rows) key=|i| *i let:i>
                    <tr>
                        <td class=CELL>{t.input(rows, rows_mut, i, |a| &a.citation, |a| &mut a.citation, format!("Authority {} citation", i + 1), "[Case name] v. [Case name], [reporter citation]", MAX_LONG)}</td>
                        <td class=CELL>{t.input(rows, rows_mut, i, |a| &a.court_level, |a| &mut a.court_level, format!("Authority {} court level", i + 1), "Appellate, binding", MAX_SHORT)}</td>
                        <td class=CELL>{t.input(rows, rows_mut, i, |a| &a.link, |a| &mut a.link, format!("Authority {} link", i + 1), "https://", MAX_LONG)}</td>
                        <td class=CELL>{t.input(rows, rows_mut, i, |a| &a.date_checked, |a| &mut a.date_checked, format!("Authority {} date checked", i + 1), "Oct 21", MAX_SHORT)}</td>
                        <td class=CELL>
                            <label class="flex items-center gap-1.5 text-[13.5px]">
                                <input
                                    type="checkbox"
                                    class="h-[18px] w-[18px] accent-primary-600"
                                    aria-label=format!("Authority {} checked for subsequent treatment", i + 1)
                                    prop:checked=move || content.with(|c| c.authorities.get(i).is_some_and(|a| a.treatment_checked))
                                    on:change=move |ev| {
                                        let on = event_target_checked(&ev);
                                        content.update(|c| if let Some(a) = c.authorities.get_mut(i) { a.treatment_checked = on; });
                                        t.touch.run(());
                                    }
                                />
                                "Checked"
                            </label>
                        </td>
                        {t.remove(rows_mut, i, "authority")}
                    </tr>
                </For>
            </EdgeTable>
            {t.add(rows, rows_mut, "Add an authority")}
            <p class=format!("{HINT} mt-2")>{AUTHORITY_HELP}</p>
        }
        .into_any()
    }

    pub(super) fn next_steps(self) -> AnyView {
        let t = self;
        let rows: Rows<NextStep> = |c| &c.next_steps;
        let rows_mut: RowsMut<NextStep> = |c| &mut c.next_steps;
        view! {
            <EdgeTable headers=vec![("Action", None), ("Proposed owner", Some("230px")), ("Timing", Some("220px")), ("", Some("48px"))]>
                <For each=t.count(rows) key=|i| *i let:i>
                    <tr>
                        <td class=CELL>{t.input(rows, rows_mut, i, |n| &n.action, |n| &mut n.action, format!("Step {} action", i + 1), "", MAX_LONG)}</td>
                        <td class=CELL>{t.input(rows, rows_mut, i, |n| &n.owner, |n| &mut n.owner, format!("Step {} owner", i + 1), "", MAX_SHORT)}</td>
                        <td class=CELL>{t.input(rows, rows_mut, i, |n| &n.timing, |n| &mut n.timing, format!("Step {} timing", i + 1), "", MAX_SHORT)}</td>
                        {t.remove(rows_mut, i, "step")}
                    </tr>
                </For>
            </EdgeTable>
            {t.add(rows, rows_mut, "Add a step")}
        }
        .into_any()
    }
}
