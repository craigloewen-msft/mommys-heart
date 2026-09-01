//! The shared category controls: the general picker every classification list
//! uses, and the rule-driven groups that sit above it.
//!
//! Before this existed the contact editor, the outreach panel and the mail
//! campaign filters each rendered their own flat checkbox list, so they drifted
//! apart. One component keeps them consistent and gives the taxonomy's parent
//! headings and stored order a single place to be honoured.

use leptos::prelude::*;

use crate::server_fns::contact_directory::ContactCategory;
use crate::server_fns::contact_rules::RuleGroup;

/// Add or remove an id, leaving the rest of the selection untouched.
pub fn toggle_id(ids: &mut Vec<String>, id: &str, checked: bool) {
    if checked {
        if !ids.iter().any(|selected| selected == id) {
            ids.push(id.to_string());
        }
    } else {
        ids.retain(|selected| selected != id);
    }
}

/// Categories grouped under their parent, in the taxonomy's stored order.
///
/// `hidden` drops the categories a rule group already offers, so a vendor
/// subcategory is not presented twice on the same form.
#[component]
pub fn CategoryPicker(
    categories: Signal<Vec<ContactCategory>>,
    selected: RwSignal<Vec<String>>,
    /// Category ids to leave out, because a rule group covers them.
    #[prop(into, optional)]
    hidden: Signal<Vec<String>>,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let sections = move || {
        let hidden = hidden.get();
        let all = categories.get();
        // `list_categories` returns each root immediately before its own
        // children, already ordered, so one pass preserves the arrangement.
        // A root heads the section it belongs to and stays selectable within
        // it, rather than being stranded under a repeated "Top level".
        let mut sections: Vec<(String, Vec<ContactCategory>)> = Vec::new();
        for category in all {
            if hidden.iter().any(|id| id == &category.id) {
                continue;
            }
            let heading = if category.parent_name.is_empty() {
                category.name.clone()
            } else {
                category.parent_name.clone()
            };
            match sections.last_mut() {
                Some((existing, items)) if existing == &heading => items.push(category),
                _ => sections.push((heading, vec![category])),
            }
        }
        sections
    };

    view! {
        <div class=move || format!("max-h-64 overflow-y-auto rounded-lg border border-slate-800 bg-slate-950 p-2 {class}")>
            {move || sections()
                .into_iter()
                .map(|(heading, items)| {
                    let label = if heading.is_empty() {
                        "Other".to_string()
                    } else {
                        heading
                    };
                    view! {
                        <div class="mb-2 last:mb-0">
                            <p class="px-2 py-1 text-[0.65rem] font-semibold uppercase tracking-wide text-slate-500">
                                {label}
                            </p>
                            {items
                                .into_iter()
                                .map(|category| {
                                    let id = category.id.clone();
                                    let checked_id = id.clone();
                                    view! {
                                        <label class="flex cursor-pointer items-start gap-2 rounded-md px-2 py-1.5 text-xs text-slate-300 hover:bg-slate-800">
                                            <input
                                                type="checkbox"
                                                class="mt-0.5 accent-primary-500"
                                                prop:checked=move || selected.get().contains(&checked_id)
                                                on:change=move |event| {
                                                    let checked = event_target_checked(&event);
                                                    selected.update(|ids| toggle_id(ids, &id, checked));
                                                }
                                            />
                                            <span>{category.name.clone()}</span>
                                        </label>
                                    }
                                })
                                .collect_view()}
                        </div>
                    }
                })
                .collect_view()}
        </div>
    }
}

/// The category groups a rule offers for the current selection.
///
/// A group capped at one choice renders as radio buttons, which is what makes
/// "one location only" obvious before the server has to say so. Everything else
/// is a checkbox grid.
#[component]
pub fn RuleGroupSelector(
    groups: Signal<Vec<RuleGroup>>,
    selected: RwSignal<Vec<String>>,
) -> impl IntoView {
    view! {
        <Show when=move || !groups.get().is_empty()>
            <div class="space-y-3">
                {move || groups
                    .get()
                    .into_iter()
                    .map(|group| {
                        let single = group.is_single_select();
                        let limit = group.limit_label();
                        let group_name = format!("rule-group-{}", group.id);
                        let option_ids: Vec<String> = group
                            .options
                            .iter()
                            .map(|option| option.category_id.clone())
                            .collect();
                        view! {
                            <fieldset class="rounded-lg border border-primary-500/30 bg-primary-500/5 p-3">
                                <legend class="px-1 text-xs font-semibold text-primary-300">
                                    {group.label.clone()}
                                    <span class="ml-2 font-normal text-slate-500">{limit}</span>
                                </legend>
                                <Show when={
                                    let help = group.help_text.clone();
                                    move || !help.is_empty()
                                }>
                                    <p class="mb-2 px-1 text-xs text-slate-500">
                                        {group.help_text.clone()}
                                    </p>
                                </Show>
                                <div class="grid gap-x-3 sm:grid-cols-2">
                                    {group
                                        .options
                                        .iter()
                                        .map(|option| {
                                            let id = option.category_id.clone();
                                            let checked_id = id.clone();
                                            let siblings = option_ids.clone();
                                            view! {
                                                <label class="flex cursor-pointer items-start gap-2 rounded-md px-2 py-1.5 text-xs text-slate-300 hover:bg-slate-800">
                                                    <input
                                                        type=if single { "radio" } else { "checkbox" }
                                                        name=group_name.clone()
                                                        class="mt-0.5 accent-primary-500"
                                                        prop:checked=move || selected.get().contains(&checked_id)
                                                        on:change=move |event| {
                                                            let checked = event_target_checked(&event);
                                                            selected.update(|ids| {
                                                                // A single-select replaces its siblings, so the
                                                                // limit cannot be broken from the form at all.
                                                                if single && checked {
                                                                    ids.retain(|selected| !siblings.contains(selected));
                                                                }
                                                                toggle_id(ids, &id, checked);
                                                            });
                                                        }
                                                    />
                                                    <span>{option.category_name.clone()}</span>
                                                </label>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            </fieldset>
                        }
                    })
                    .collect_view()}
            </div>
        </Show>
    }
}
