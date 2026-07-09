use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::Redirect;

use crate::api_client::offboard_volunteer;
use crate::components::case_card::CaseCard;
use crate::components::layout::Layout;
use crate::state::AppState;
use crate::taxonomy::{ServiceCategory, ServiceType};
use crate::types::{CaseStatus, MatterType, NeedCategory, VolunteerStatus};

/// Admin-only control center: manage volunteers, cases, documents, and
/// assignments over the local demo store.
#[component]
pub fn AdminDashboardPage() -> impl IntoView {
    let state = expect_context::<AppState>();

    // Role guard.
    match state.current_user.get_untracked() {
        Some(u) if u.role.is_staff_level() => {}
        Some(_) => return view! { <Redirect path="/volunteer" /> }.into_any(),
        None => return view! { <Redirect path="/login" /> }.into_any(),
    }

    // --- new volunteer form ---
    let nv_name = RwSignal::new(String::new());
    let nv_email = RwSignal::new(String::new());
    let nv_specialty = RwSignal::new(String::new());
    let nv_error = RwSignal::new(String::new());
    let add_volunteer =
        move |_| match state.add_volunteer(&nv_name.get(), &nv_email.get(), &nv_specialty.get()) {
            Ok(()) => {
                nv_name.set(String::new());
                nv_email.set(String::new());
                nv_specialty.set(String::new());
                nv_error.set(String::new());
            }
            Err(e) => nv_error.set(e),
        };

    // --- new case form ---
    let nc_title = RwSignal::new(String::new());
    let nc_client_id = RwSignal::new(String::new());
    let nc_category = RwSignal::new(NeedCategory::Housing.slug().to_string());
    let nc_matter = RwSignal::new(MatterType::Housing.slug().to_string());
    let nc_services = RwSignal::new(Vec::<ServiceType>::new());
    let nc_summary = RwSignal::new(String::new());
    let nc_error = RwSignal::new(String::new());
    let add_case = move |_| {
        let category = NeedCategory::from_slug(&nc_category.get()).unwrap_or(NeedCategory::Other);
        let matter_type = MatterType::from_slug(&nc_matter.get()).unwrap_or(MatterType::Other);
        match state.add_case(
            &nc_client_id.get(),
            &nc_title.get(),
            category,
            nc_services.get(),
            &nc_summary.get(),
            matter_type,
        ) {
            Ok(()) => {
                nc_title.set(String::new());
                nc_client_id.set(String::new());
                nc_category.set(NeedCategory::Housing.slug().to_string());
                nc_matter.set(MatterType::Housing.slug().to_string());
                nc_services.set(Vec::new());
                nc_summary.set(String::new());
                nc_error.set(String::new());
            }
            Err(e) => nc_error.set(e),
        }
    };

    let input_class = "w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/40";
    let select_class = "rounded-lg border border-slate-700 bg-slate-950 px-2 py-1.5 text-xs text-slate-100 focus:border-primary-500 focus:outline-none";

    // Feedback shown after offboarding a volunteer.
    let offboard_msg = RwSignal::new(String::new());

    // Stats.
    let stat_cards = move || {
        let volunteers = state.volunteers.get();
        let cases = state.cases.get();
        let clients = state.clients.get();
        let active = volunteers
            .iter()
            .filter(|v| v.status == VolunteerStatus::Active)
            .count();
        let open = cases
            .iter()
            .filter(|c| c.status != CaseStatus::Closed)
            .count();
        let unassigned = cases
            .iter()
            .filter(|c| c.assigned_volunteer_ids.is_empty())
            .count();
        [
            ("Clients served", clients.len()),
            ("Open cases", open),
            ("Unassigned cases", unassigned),
            ("Active volunteers", active),
        ]
        .into_iter()
        .map(|(label, value)| {
            view! {
                <div class="rounded-xl border border-slate-800 bg-slate-900 p-5">
                    <p class="text-3xl font-semibold text-white">{value}</p>
                    <p class="mt-1 text-sm text-slate-400">{label}</p>
                </div>
            }
        })
        .collect_view()
    };

    // Volunteer rows.
    let volunteer_rows = move || {
        state
            .volunteers
            .get()
            .into_iter()
            .map(|v| {
                let id = v.id.clone();
                let offboard_id = v.id.clone();
                let offboard_name = v.name.clone();
                let badge = format!(
                    "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {}",
                    v.status.badge_classes(),
                );
                view! {
                    <tr class="border-b border-slate-800 last:border-0 hover:bg-slate-800/40">
                        <td class="px-4 py-3">
                            <p class="font-medium text-slate-100">{v.name.clone()}</p>
                            <p class="text-xs text-slate-500">{v.email.clone()}</p>
                        </td>
                        <td class="px-4 py-3 text-slate-300">{v.specialty.clone()}</td>
                        <td class="px-4 py-3">
                            <span class=badge>{v.status.label()}</span>
                        </td>
                        <td class="px-4 py-3">
                            <div class="flex items-center gap-2">
                                <select
                                    class=select_class
                                    prop:value=v.status.slug()
                                    on:change=move |ev| {
                                        if let Some(s) = VolunteerStatus::from_slug(
                                            &event_target_value(&ev),
                                        ) {
                                            state.set_volunteer_status(&id, s);
                                        }
                                    }
                                >
                                    {VolunteerStatus::ALL
                                        .into_iter()
                                        .map(|s| {
                                            view! { <option value=s.slug()>{s.label()}</option> }
                                        })
                                        .collect_view()}
                                </select>
                                <button
                                    class="shrink-0 rounded-lg border border-slate-700 px-2 py-1.5 text-xs font-medium text-slate-300 hover:bg-slate-800 hover:text-white"
                                    title="Deactivate and release their conversations to the org (history preserved)"
                                    on:click=move |_| {
                                        let vid = offboard_id.clone();
                                        let vname = offboard_name.clone();
                                        state.set_volunteer_status(&vid, VolunteerStatus::Inactive);
                                        spawn_local(async move {
                                            match offboard_volunteer(vid).await {
                                                Ok(n) => offboard_msg.set(format!(
                                                    "Offboarded {vname}: {n} conversation(s) released to the org (history preserved).",
                                                )),
                                                Err(e) => offboard_msg.set(format!("Offboard failed: {e}")),
                                            }
                                        });
                                    }
                                >
                                    "Offboard"
                                </button>
                            </div>
                        </td>
                    </tr>
                }
            })
            .collect_view()
    };

    // Case cards.
    let case_cards = move || {
        state
            .cases
            .get()
            .into_iter()
            .map(|c| view! { <CaseCard id=c.id.clone() /> })
            .collect_view()
    };

    view! {
        <Layout title="Admin dashboard">
            <div class="grid grid-cols-2 gap-4 lg:grid-cols-4">{stat_cards}</div>

            // Volunteers management
            <section class="mt-8">
                <div class="mb-3 flex items-center justify-between">
                    <h2 class="text-lg font-semibold">"Volunteers"</h2>
                    <Show when=move || !offboard_msg.get().is_empty()>
                        <span class="rounded-lg bg-primary-500/15 px-3 py-1 text-xs font-medium text-primary-300 ring-1 ring-primary-500/30">
                            {move || offboard_msg.get()}
                        </span>
                    </Show>
                </div>
                <div class="overflow-hidden rounded-xl border border-slate-800 bg-slate-900">
                    <table class="w-full text-sm">
                        <thead class="border-b border-slate-800 text-left text-slate-400">
                            <tr>
                                <th class="px-4 py-3 font-medium">"Name"</th>
                                <th class="px-4 py-3 font-medium">"Specialty"</th>
                                <th class="px-4 py-3 font-medium">"Status"</th>
                                <th class="px-4 py-3 font-medium">"Change status"</th>
                            </tr>
                        </thead>
                        <tbody>{volunteer_rows}</tbody>
                    </table>
                    <div class="border-t border-slate-800 bg-slate-900/60 p-4">
                        <p class="mb-2 text-xs font-medium uppercase tracking-wide text-slate-400">
                            "Add volunteer"
                        </p>
                        <div class="flex flex-col gap-2 sm:flex-row">
                            <input
                                class=input_class
                                placeholder="Name"
                                prop:value=move || nv_name.get()
                                on:input=move |ev| nv_name.set(event_target_value(&ev))
                            />
                            <input
                                class=input_class
                                placeholder="Email"
                                prop:value=move || nv_email.get()
                                on:input=move |ev| nv_email.set(event_target_value(&ev))
                            />
                            <input
                                class=input_class
                                placeholder="Specialty"
                                prop:value=move || nv_specialty.get()
                                on:input=move |ev| nv_specialty.set(event_target_value(&ev))
                            />
                            <button
                                on:click=add_volunteer
                                class="shrink-0 rounded-lg bg-primary-500 px-4 py-2 text-sm font-semibold text-white hover:bg-primary-600"
                            >
                                "Add"
                            </button>
                        </div>
                        <Show when=move || !nv_error.get().is_empty()>
                            <p class="mt-2 text-xs text-rose-400">{move || nv_error.get()}</p>
                        </Show>
                    </div>
                </div>
            </section>

            // Cases management
            <section class="mt-8">
                <h2 class="mb-3 text-lg font-semibold">"Cases"</h2>
                <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">{case_cards}</div>

                <div class="mt-4 rounded-xl border border-slate-800 bg-slate-900 p-4">
                    <p class="mb-2 text-xs font-medium uppercase tracking-wide text-slate-400">
                        "Open a new case"
                    </p>
                    <div class="grid gap-2 sm:grid-cols-2">
                        <input
                            class=input_class
                            placeholder="Case title"
                            prop:value=move || nc_title.get()
                            on:input=move |ev| nc_title.set(event_target_value(&ev))
                        />
                        <select
                            class=input_class
                            prop:value=move || nc_client_id.get()
                            on:change=move |ev| nc_client_id.set(event_target_value(&ev))
                        >
                            <option value="">"Select client\u{2026}"</option>
                            {move || {
                                state
                                    .clients
                                    .get()
                                    .into_iter()
                                    .map(|cl| {
                                        view! {
                                            <option value=cl.id.clone()>{cl.display_name}</option>
                                        }
                                    })
                                    .collect_view()
                            }}
                        </select>
                    </div>
                    <div class="mt-2">
                        <select
                            class=input_class
                            prop:value=move || nc_category.get()
                            on:change=move |ev| nc_category.set(event_target_value(&ev))
                        >
                            {NeedCategory::ALL
                                .into_iter()
                                .map(|cat| {
                                    view! {
                                        <option value=cat.slug()>
                                            {format!("Need: {}", cat.label())}
                                        </option>
                                    }
                                })
                                .collect_view()}
                        </select>
                    </div>
                    <div class="mt-2">
                        <select
                            class=input_class
                            prop:value=move || nc_matter.get()
                            on:change=move |ev| nc_matter.set(event_target_value(&ev))
                        >
                            {MatterType::ALL
                                .into_iter()
                                .map(|mt| {
                                    view! {
                                        <option value=mt.slug()>
                                            {format!("Matter: {}", mt.label())}
                                        </option>
                                    }
                                })
                                .collect_view()}
                        </select>
                    </div>
                    <p class="mt-3 mb-1.5 text-xs font-medium uppercase tracking-wide text-slate-400">
                        "Service types"
                    </p>
                    <ServiceTypePicker selected=nc_services />
                    <textarea
                        class=format!("{input_class} mt-2")
                        rows="2"
                        placeholder="Short summary"
                        prop:value=move || nc_summary.get()
                        on:input=move |ev| nc_summary.set(event_target_value(&ev))
                    ></textarea>
                    <div class="mt-2 flex items-center gap-3">
                        <button
                            on:click=add_case
                            class="rounded-lg bg-primary-500 px-4 py-2 text-sm font-semibold text-white hover:bg-primary-600"
                        >
                            "Create case"
                        </button>
                        <Show when=move || !nc_error.get().is_empty()>
                            <p class="text-xs text-rose-400">{move || nc_error.get()}</p>
                        </Show>
                    </div>
                </div>
            </section>
        </Layout>
    }
    .into_any()
}

/// A category-grouped multi-select of taxonomy service types, bound to a local
/// `Vec<ServiceType>` signal. Used by the new-case form.
#[component]
fn ServiceTypePicker(selected: RwSignal<Vec<ServiceType>>) -> impl IntoView {
    move || {
        let current = selected.get();
        ServiceCategory::ALL
            .into_iter()
            .map(|cat| {
                let boxes = cat
                    .types()
                    .iter()
                    .map(|st| {
                        let st = *st;
                        let checked = current.contains(&st);
                        view! {
                            <label class="flex cursor-pointer items-center gap-1.5 rounded-md border border-slate-700 px-2 py-1 text-xs text-slate-300 hover:bg-slate-800">
                                <input
                                    r#type="checkbox"
                                    class="accent-primary-500"
                                    prop:checked=checked
                                    on:change=move |_| {
                                        selected
                                            .update(|list| {
                                                if let Some(pos) = list.iter().position(|s| *s == st) {
                                                    list.remove(pos);
                                                } else {
                                                    list.push(st);
                                                }
                                            });
                                    }
                                />
                                {st.label()}
                            </label>
                        }
                    })
                    .collect_view();
                view! {
                    <div class="mb-2">
                        <p class="mb-1 text-[10px] font-semibold uppercase tracking-wide text-slate-500">
                            {cat.label()}
                        </p>
                        <div class="flex flex-wrap gap-1.5">{boxes}</div>
                    </div>
                }
            })
            .collect_view()
    }
}
