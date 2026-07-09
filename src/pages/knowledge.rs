use leptos::prelude::*;

use crate::components::layout::Layout;
use crate::state::AppState;
use crate::types::{KnowledgeCategory, Permission};

/// Institutional knowledge base: templates, resources, best practices, and
/// retained case context owned by the organization. Ensures knowledge does not
/// leave with individual volunteers or interns.
#[component]
pub fn KnowledgeBasePage() -> impl IntoView {
    let state = expect_context::<AppState>();

    if state.current_user.get_untracked().is_none() {
        return view! { <leptos_router::components::Redirect path="/login" /> }.into_any();
    }

    // Only staff-level users may contribute/edit shared knowledge.
    let can_manage = state.can(Permission::ManageKnowledge);

    // --- new knowledge form (staff only) ---
    let nk_title = RwSignal::new(String::new());
    let nk_summary = RwSignal::new(String::new());
    let nk_category = RwSignal::new(KnowledgeCategory::Template.label().to_string());
    let nk_error = RwSignal::new(String::new());
    let add_item = move |_| {
        let category = KnowledgeCategory::ALL
            .into_iter()
            .find(|c| c.label() == nk_category.get())
            .unwrap_or(KnowledgeCategory::Template);
        match state.add_knowledge(&nk_title.get(), category, &nk_summary.get()) {
            Ok(()) => {
                nk_title.set(String::new());
                nk_summary.set(String::new());
                nk_error.set(String::new());
            }
            Err(e) => nk_error.set(e),
        }
    };

    let input_class = "w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/40";

    let cards = move || {
        state
            .knowledge
            .get()
            .into_iter()
            .map(|k| {
                let badge = format!(
                    "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {}",
                    k.category.badge_classes(),
                );
                view! {
                    <div class="rounded-xl border border-slate-800 bg-slate-900 p-5">
                        <div class="flex items-start justify-between gap-3">
                            <h3 class="font-semibold text-white">{k.title}</h3>
                            <span class=badge>{k.category.label()}</span>
                        </div>
                        <p class="mt-2 text-sm text-slate-400">{k.summary}</p>
                        <p class="mt-3 text-xs text-slate-500">
                            "Contributed by " {k.contributed_by} " \u{2022} updated " {k.updated_at}
                        </p>
                    </div>
                }
            })
            .collect_view()
    };

    view! {
        <Layout title="Knowledge base">
            <p class="mb-6 max-w-3xl text-sm text-slate-400">
                "Templates, resources, best practices, and retained case context — owned by
                the organization so historical knowledge stays even as volunteers, interns,
                and professionals rotate through. This complements the AI chat assistant,
                which answers questions over the organization's documents."
            </p>

            <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">{cards}</div>

            <Show when=move || can_manage>
                <div class="mt-6 rounded-xl border border-slate-800 bg-slate-900 p-4">
                    <p class="mb-2 text-xs font-medium uppercase tracking-wide text-slate-400">
                        "Add to the knowledge base"
                    </p>
                    <div class="grid gap-2 sm:grid-cols-2">
                        <input
                            class=input_class
                            placeholder="Title"
                            prop:value=move || nk_title.get()
                            on:input=move |ev| nk_title.set(event_target_value(&ev))
                        />
                        <select
                            class=input_class
                            on:change=move |ev| nk_category.set(event_target_value(&ev))
                        >
                            {KnowledgeCategory::ALL
                                .into_iter()
                                .map(|c| {
                                    view! { <option value=c.label()>{c.label()}</option> }
                                })
                                .collect_view()}
                        </select>
                    </div>
                    <textarea
                        class=format!("{input_class} mt-2")
                        rows="2"
                        placeholder="Short summary"
                        prop:value=move || nk_summary.get()
                        on:input=move |ev| nk_summary.set(event_target_value(&ev))
                    ></textarea>
                    <div class="mt-2 flex items-center gap-3">
                        <button
                            on:click=add_item
                            class="rounded-lg bg-primary-500 px-4 py-2 text-sm font-semibold text-white hover:bg-primary-600"
                        >
                            "Add resource"
                        </button>
                        <Show when=move || !nk_error.get().is_empty()>
                            <p class="text-xs text-rose-400">{move || nk_error.get()}</p>
                        </Show>
                    </div>
                </div>
            </Show>
        </Layout>
    }
    .into_any()
}
