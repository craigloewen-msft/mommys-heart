use leptos::prelude::*;

use super::{date_long, matter, section_card};
use crate::components::case_overview::month_day;
use crate::components::design::{Icon, StatusPill, Tone, HINT, MUTED};
use crate::server_fns::legal_memos::{LegalMemo, LegalMemoContent};

fn card(anchor: &'static str, body: AnyView) -> AnyView {
    section_card(anchor, Signal::from(false), body)
}

fn none() -> AnyView {
    view! { <p class=format!("{MUTED} text-[14.5px]")>"None recorded."</p> }.into_any()
}

pub(super) fn header_card(memo: &LegalMemo) -> AnyView {
    let rows = vec![
        (
            "To",
            if memo.attorney_name.trim().is_empty() {
                "Not chosen yet".to_string()
            } else {
                memo.attorney_name.clone()
            },
        ),
        ("From", memo.author_name.clone()),
        (
            "Date",
            if memo.content.memo_date.is_empty() {
                "Not set".to_string()
            } else {
                date_long(&memo.content.memo_date)
            },
        ),
        ("Matter", matter(memo)),
        ("Status", memo.status.label().to_string()),
    ];
    card(
        "mh",
        view! {
            <dl class="grid gap-x-5 gap-y-3 text-[15px] sm:grid-cols-2 lg:grid-cols-3">
                {rows
                    .into_iter()
                    .map(|(label, value)| view! {
                        <div><dt class=HINT>{label}</dt><dd class="font-semibold">{value}</dd></div>
                    })
                    .collect_view()}
            </dl>
        }
        .into_any(),
    )
}

pub(super) fn content_cards(c: &LegalMemoContent, author: &str) -> AnyView {
    let answered = if c.safety_answered_on.is_empty() {
        format!("Answered by {author}")
    } else {
        format!("Answered by {author}, {}", month_day(&c.safety_answered_on))
    };
    let safety = match c.safety_flagged {
        Some(false) => view! {
            <p class="flex flex-wrap items-center gap-2">
                <StatusPill label="No safety concern flagged" tone=Tone::Green icon_kind=Icon::Check />
                <span class=format!("{MUTED} text-[13.5px]")>{answered}</span>
            </p>
        }
        .into_any(),
        Some(true) => {
            let escalated = format!(
                "Escalated {} to {}.",
                if c.safety_escalated_on.is_empty() { "on an unrecorded date".to_string() } else { date_long(&c.safety_escalated_on) },
                if c.safety_escalated_to.trim().is_empty() { "someone not recorded" } else { c.safety_escalated_to.as_str() }
            );
            view! {
                <p class="flex flex-wrap items-center gap-2">
                    <StatusPill label="Safety concern flagged" tone=Tone::Red icon_kind=Icon::Warning />
                    <span class=format!("{MUTED} text-[13.5px]")>{answered}</span>
                </p>
                <p class="mt-2">{escalated}</p>
            }
            .into_any()
        }
        None => view! { <p class=MUTED>"Not answered yet."</p> }.into_any(),
    };

    let brief = if c.brief_answer.trim().is_empty() {
        none()
    } else {
        view! { <p class="whitespace-pre-line text-[15.5px]">{c.brief_answer.clone()}</p> }
            .into_any()
    };

    view! {
        {card("ms", safety)}
        {card("s2", brief)}
    }
    .into_any()
}
