use leptos::prelude::*;
use leptos_router::components::A;

use crate::api_client::get_contacts;
use crate::components::layout::Layout;

#[component]
pub fn ContactsPage() -> impl IntoView {
    let contacts = Resource::new(|| (), |_| async move { get_contacts().await });

    view! {
        <Layout title="Contacts">
            <div class="overflow-hidden rounded-xl border border-slate-800 bg-slate-900">
                <Suspense fallback=|| {
                    view! { <p class="p-4 text-slate-400">"Loading\u{2026}"</p> }
                }>
                    {move || Suspend::new(async move {
                        match contacts.await {
                            Ok(list) => {
                                view! {
                                    <table class="w-full text-sm">
                                        <thead class="text-left text-slate-400 border-b border-slate-800">
                                            <tr>
                                                <th class="px-4 py-3 font-medium">"Name"</th>
                                                <th class="px-4 py-3 font-medium">"Company"</th>
                                                <th class="px-4 py-3 font-medium">"Email"</th>
                                                <th class="px-4 py-3 font-medium">"Status"</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {list
                                                .into_iter()
                                                .map(|c| {
                                                    let badge = format!(
                                                        "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {}",
                                                        c.status.badge_classes(),
                                                    );
                                                    view! {
                                                        <tr class="border-b border-slate-800 last:border-0 hover:bg-slate-800/40">
                                                            <td class="px-4 py-3 font-medium">
                                                                <A
                                                                    href=format!("/contacts/{}", c.id)
                                                                    attr:class="text-slate-100 hover:text-primary-400"
                                                                >
                                                                    {c.name}
                                                                </A>
                                                            </td>
                                                            <td class="px-4 py-3 text-slate-400">{c.company}</td>
                                                            <td class="px-4 py-3 text-slate-400">{c.email}</td>
                                                            <td class="px-4 py-3">
                                                                <span class=badge>{c.status.label()}</span>
                                                            </td>
                                                        </tr>
                                                    }
                                                })
                                                .collect_view()}
                                        </tbody>
                                    </table>
                                }
                                    .into_any()
                            }
                            Err(e) => {
                                view! { <p class="p-4 text-rose-400">"Failed to load contacts: " {e}</p> }
                                    .into_any()
                            }
                        }
                    })}
                </Suspense>
            </div>
        </Layout>
    }
}
