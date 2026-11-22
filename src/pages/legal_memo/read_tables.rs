use leptos::prelude::*;

use super::read::{fact_tone, none, pill};
use super::{AUTHORITY_HELP, EVIDENCE_HELP, FACT_HELP, PROCEDURE_HELP};
use crate::components::design::{EdgeTable, Icon, StatusPill, Tone, HINT, LINK, MUTED, TD};
use crate::server_fns::legal_memos::{EventStatus, EvidenceStatus, LegalMemoContent};

fn hint(text: &'static str) -> AnyView {
    view! { <p class=format!("{HINT} mb-3")>{text}</p> }.into_any()
}

pub(super) fn facts(c: &LegalMemoContent) -> AnyView {
    let facts: Vec<_> = c
        .facts
        .iter()
        .filter(|f| !f.text.trim().is_empty())
        .cloned()
        .collect();
    let facts = view! {
        {hint(FACT_HELP)}
        {if facts.is_empty() {
            none()
        } else {
            view! {
                <EdgeTable headers=vec![("Fact", None), ("Source", None)]>
                    {facts
                        .into_iter()
                        .map(|f| view! {
                            <tr>
                                <td class=TD>{f.text}</td>
                                <td class=TD>
                                    {match f.source {
                                        Some(s) => pill(s.label(), fact_tone(s)),
                                        None => view! { <span class=MUTED>"Not recorded"</span> }.into_any(),
                                    }}
                                </td>
                            </tr>
                        })
                        .collect_view()}
                </EdgeTable>
            }
            .into_any()
        }}
    }
    .into_any();
    facts
}

pub(super) fn procedure(c: &LegalMemoContent) -> AnyView {
    let procedure: Vec<_> = c
        .procedure
        .iter()
        .filter(|p| !p.event.trim().is_empty() || !p.date.trim().is_empty())
        .cloned()
        .collect();
    let procedure = view! {
        {hint(PROCEDURE_HELP)}
        {if procedure.is_empty() {
            none()
        } else {
            view! {
                <EdgeTable headers=vec![("Date", None), ("Event", None), ("Source", None), ("Status", None)]>
                    {procedure
                        .into_iter()
                        .map(|p| view! {
                            <tr>
                                <td class=format!("{TD} whitespace-nowrap")>{p.date}</td>
                                <td class=TD>{p.event}</td>
                                <td class=TD>{p.source}</td>
                                <td class=TD>
                                    {match p.status {
                                        EventStatus::Confirmed => pill("Confirmed", Tone::Green),
                                        EventStatus::Unconfirmed => pill("Unconfirmed", Tone::Yellow),
                                    }}
                                </td>
                            </tr>
                        })
                        .collect_view()}
                </EdgeTable>
            }
            .into_any()
        }}
    }
    .into_any();
    procedure
}

pub(super) fn evidence(c: &LegalMemoContent) -> AnyView {
    let evidence: Vec<_> = c
        .evidence
        .iter()
        .filter(|e| !e.item.trim().is_empty())
        .cloned()
        .collect();
    let evidence = view! {
        {hint(EVIDENCE_HELP)}
        {if evidence.is_empty() {
            none()
        } else {
            view! {
                <EdgeTable headers=vec![("Item", None), ("Source", None), ("Status", None), ("Date received", None), ("Follow-up", None)]>
                    {evidence
                        .into_iter()
                        .map(|e| view! {
                            <tr>
                                <td class=TD>{e.item}</td>
                                <td class=TD>{e.source}</td>
                                <td class=TD>
                                    {match e.status {
                                        EvidenceStatus::NotReceived => pill("Not received", Tone::Yellow),
                                        EvidenceStatus::Received => pill("Received", Tone::Sand),
                                        EvidenceStatus::Reviewed => pill("Reviewed", Tone::Green),
                                    }}
                                </td>
                                <td class=TD>
                                    {if e.date_received.trim().is_empty() {
                                        view! { <span class=MUTED>"Not yet"</span> }.into_any()
                                    } else {
                                        view! { <span>{e.date_received}</span> }.into_any()
                                    }}
                                </td>
                                <td class=TD>{e.follow_up}</td>
                            </tr>
                        })
                        .collect_view()}
                </EdgeTable>
            }
            .into_any()
        }}
    }
    .into_any();
    evidence
}

pub(super) fn authorities(c: &LegalMemoContent) -> AnyView {
    let authorities: Vec<_> = c
        .authorities
        .iter()
        .filter(|a| !a.citation.trim().is_empty())
        .cloned()
        .collect();
    let authorities = view! {
        {if authorities.is_empty() {
            none()
        } else {
            view! {
                <EdgeTable headers=vec![("Citation", None), ("Court level", None), ("Link", None), ("Date checked", None), ("Subsequent treatment", None)]>
                    {authorities
                        .into_iter()
                        .map(|a| {
                            let link = a.link.trim().to_string();
                            view! {
                                <tr>
                                    <td class=TD>{a.citation}</td>
                                    <td class=TD>{a.court_level}</td>
                                    <td class=format!("{TD} max-w-[220px]")>
                                        {if link.starts_with("http://") || link.starts_with("https://") {
                                            view! { <a class=format!("{LINK} block truncate") title=link.clone() href=link.clone() target="_blank" rel="noopener">{link.clone()}</a> }.into_any()
                                        } else {
                                            view! { <span>{link}</span> }.into_any()
                                        }}
                                    </td>
                                    <td class=TD>{a.date_checked}</td>
                                    <td class=TD>
                                        {if a.treatment_checked {
                                            view! { <StatusPill label="Checked" tone=Tone::Green icon_kind=Icon::Check small=true /> }.into_any()
                                        } else {
                                            view! { <StatusPill label="Not checked" tone=Tone::Red icon_kind=Icon::Warning small=true /> }.into_any()
                                        }}
                                    </td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </EdgeTable>
            }
            .into_any()
        }}
        <p class=format!("{HINT} mt-2")>{AUTHORITY_HELP}</p>
    }
    .into_any();
    authorities
}

pub(super) fn next_steps(c: &LegalMemoContent) -> AnyView {
    let steps: Vec<_> = c
        .next_steps
        .iter()
        .filter(|n| !n.action.trim().is_empty())
        .cloned()
        .collect();
    let steps = if steps.is_empty() {
        none()
    } else {
        view! {
            <EdgeTable headers=vec![("Action", None), ("Proposed owner", None), ("Timing", None)]>
                {steps
                    .into_iter()
                    .map(|n| view! {
                        <tr><td class=TD>{n.action}</td><td class=TD>{n.owner}</td><td class=TD>{n.timing}</td></tr>
                    })
                    .collect_view()}
            </EdgeTable>
        }
        .into_any()
    };
    steps
}
