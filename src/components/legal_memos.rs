use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::components::case_overview::avatar_classes;
use crate::components::design::{
    icon, Icon, Tone, BTN_SM, CARD, CARD_HEAD, HINT, KICKER, LINK, MUTED, SERIF_H2, SERIF_PAGE_H2,
};
use crate::helpers::format::initials;
use crate::helpers::new_case_folders::STRATEGY_NOTES;
use crate::helpers::visibility::Visibility;
use crate::server_fns::case_contacts::{list_case_contacts, CaseContact, CaseContactRole};
use crate::server_fns::case_properties::CaseProperty;
use crate::server_fns::err_text;
use crate::server_fns::legal_memos::{MemoStatus, COURT_KEYS, DOCKET_KEYS};

pub fn memo_status_tone(status: MemoStatus) -> Tone {
    match status {
        MemoStatus::Draft => Tone::Sand,
        MemoStatus::InReview => Tone::Yellow,
        MemoStatus::RevisionsRequested => Tone::Red,
        MemoStatus::Approved => Tone::Green,
    }
}

pub const ERRTEXT: &str = "text-[13.5px] font-semibold text-danger-700";

pub fn matter_label(case_name: &str, docket: &str) -> String {
    if docket.trim().is_empty() {
        case_name.to_string()
    } else {
        format!("{case_name} \u{b7} {}", docket.trim())
    }
}

fn eye_icon() -> impl IntoView {
    view! {
        <svg class="h-3.5 w-3.5 flex-none" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" aria-hidden="true">
            <path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12z"></path>
            <circle cx="12" cy="12" r="3"></circle>
        </svg>
    }
}

const CLIENT_SEES: &str = "inline-flex items-center gap-[5px] rounded-full border border-slate-700 bg-sand-300 px-2.5 py-0.5 text-[12.5px] font-bold text-slate-400";

#[component]
pub fn LegalTab(
    case_id: String,
    client_first_name: String,
    #[prop(into)] properties: Signal<Vec<CaseProperty>>,
    show_contacts: bool,
    can_edit_people: bool,
    on_open_strategy: Callback<()>,
    on_edit_fields: Callback<()>,
    on_manage_people: Callback<()>,
) -> impl IntoView {
    let client = if client_first_name.trim().is_empty() {
        "The client".to_string()
    } else {
        client_first_name
    };
    let shared = move |key: &'static str| {
        properties.with(|all| {
            all.iter()
                .find(|p| p.key == key && !p.value.trim().is_empty())
                .map(|p| (p.value.clone(), p.visibility == Visibility::Shared))
        })
    };
    let court = move || COURT_KEYS.iter().find_map(|key| shared(*key));
    let docket_entry = move || DOCKET_KEYS.iter().find_map(|key| shared(*key));
    let docket = move || docket_entry().map(|(v, _)| v).unwrap_or_default();
    let client_sees = StoredValue::new(client.clone());

    view! {
        <div class="space-y-[18px]">
            <h2 class=SERIF_PAGE_H2>"Legal"</h2>
            <p class=format!("{KICKER} mt-1 text-[12px]")>"Also on this case"</p>
            <div class="grid items-start gap-[22px] [grid-template-columns:repeat(auto-fit,minmax(min(300px,100%),1fr))]">
                <section class=format!("{CARD} px-[22px] pt-5 pb-[22px]") aria-labelledby="legal-folder-h">
                    <div class=CARD_HEAD>
                        <h3 id="legal-folder-h" class=SERIF_H2>"Strategy Notes folder"</h3>
                    </div>
                    <div class="mt-4 flex items-center gap-3.5">
                        <span class="inline-flex h-[42px] w-[42px] flex-none items-center justify-center rounded-xl bg-peach-100 text-primary-600">
                            {icon(Icon::Folder, "h-5 w-5")}
                        </span>
                        <div class="min-w-0 flex-1">
                            <p class="font-bold">{STRATEGY_NOTES}</p>
                            <p class=format!("{MUTED} text-[13.5px]")>"SharePoint folder"</p>
                        </div>
                        <button type="button" class=BTN_SM on:click=move |_| on_open_strategy.run(())>"Open folder"</button>
                    </div>
                    <p class=format!("{CLIENT_SEES} mt-3.5 whitespace-normal rounded-xl px-3 py-2")>
                        {eye_icon()}
                        {format!("{} can see every file in this folder.", client_sees.get_value())}
                    </p>
                </section>
                <section class=format!("{CARD} px-[22px] pt-5 pb-[18px]") aria-labelledby="legal-court-h">
                    <div class=CARD_HEAD>
                        <h3 id="legal-court-h" class=SERIF_H2>"Court details"</h3>
                        {move || {
                            let visible = court().is_some_and(|(_, s)| s) || docket_entry().is_some_and(|(_, s)| s);
                            visible.then(|| view! {
                                <span class=CLIENT_SEES>{eye_icon()}{format!("{} can see these", client_sees.get_value())}</span>
                            })
                        }}
                    </div>
                    <dl class="mt-1.5 flex flex-col text-[14.5px]">
                        <div class="flex justify-between gap-3 border-b border-sand-300 py-2.5 last:border-b-0">
                            <dt class=MUTED>"Court"</dt>
                            <dd class="text-right">{move || court().map(|(v, _)| v).unwrap_or_else(|| "Not filled in".into())}</dd>
                        </div>
                        <Show when=move || docket_entry().is_some()>
                            <div class="flex justify-between gap-3 py-2.5">
                                <dt class=MUTED>"Docket"</dt>
                                <dd class="text-right">{docket}</dd>
                            </div>
                        </Show>
                    </dl>
                    <p class=format!("{MUTED} mt-1.5 text-[13.5px]")>
                        "From Case information. "
                        <button type="button" class=LINK on:click=move |_| on_edit_fields.run(())>"Edit fields"</button>
                    </p>
                </section>
                {show_contacts.then(|| view! {
                    <LegalContactsCard case_id=case_id.clone() can_edit_people=can_edit_people on_manage_people=on_manage_people />
                })}
            </div>
        </div>
    }
}

#[component]
fn LegalContactsCard(
    case_id: String,
    can_edit_people: bool,
    on_manage_people: Callback<()>,
) -> impl IntoView {
    let contacts = RwSignal::new(Vec::<CaseContact>::new());
    let loading = RwSignal::new(true);
    let error = RwSignal::new(String::new());
    spawn_local(async move {
        let found = list_case_contacts(case_id).await;
        let _ = loading.try_set(false);
        match found {
            Ok(list) => {
                let _ = contacts.try_set(
                    list.into_iter()
                        .filter(|c| {
                            matches!(
                                c.role,
                                CaseContactRole::Attorney | CaseContactRole::CourtProfessional
                            )
                        })
                        .collect(),
                );
            }
            Err(e) => {
                let _ = error.try_set(err_text(e));
            }
        }
    });

    view! {
        <section class=format!("{CARD} px-[22px] pt-5 pb-[22px]") aria-labelledby="legal-people-h">
            <div class=CARD_HEAD>
                <h3 id="legal-people-h" class=SERIF_H2>"Legal contacts"</h3>
                {can_edit_people.then(|| view! {
                    <button type="button" class=BTN_SM on:click=move |_| on_manage_people.run(())>"Manage people"</button>
                })}
            </div>
            {move || {
                if loading.get() {
                    return view! { <p class=format!("{HINT} mt-4")>"Loading\u{2026}"</p> }.into_any();
                }
                if !error.get().is_empty() {
                    return view! {
                        <p class=format!("{ERRTEXT} mt-4") role="alert">{move || error.get()}</p>
                    }
                    .into_any();
                }
                let list = contacts.get();
                if list.is_empty() {
                    return view! { <p class=format!("{MUTED} mt-4 text-[13.5px]")>"No attorneys or court contacts on this case yet."</p> }.into_any();
                }
                view! {
                    <ul class="mt-4 flex flex-col gap-4">
                        {list
                            .into_iter()
                            .map(|c| {
                                let tone = avatar_classes(c.role);
                                let role_line = if c.organization_name.trim().is_empty() {
                                    c.role.label().to_string()
                                } else {
                                    format!("{} \u{b7} {}", c.role.label(), c.organization_name)
                                };
                                let reach: Vec<String> = [c.email.clone(), c.phone.clone()]
                                    .into_iter()
                                    .filter(|v| !v.trim().is_empty())
                                    .collect();
                                view! {
                                    <li class="flex items-start gap-3">
                                        <span aria-hidden="true" class=format!("inline-flex h-10 w-10 flex-none items-center justify-center rounded-full text-sm font-bold {tone}")>
                                            {initials(&c.contact_name)}
                                        </span>
                                        <div class="min-w-0">
                                            <p class="font-bold">{c.contact_name}</p>
                                            <p class=format!("{MUTED} text-[13.5px]")>{role_line}</p>
                                            {(!reach.is_empty()).then(|| view! { <p class=format!("{MUTED} text-[13.5px]")>{reach.join(" \u{b7} ")}</p> })}
                                        </div>
                                    </li>
                                }
                            })
                            .collect_view()}
                    </ul>
                }
                .into_any()
            }}
        </section>
    }
}
