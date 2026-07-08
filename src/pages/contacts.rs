use leptos::prelude::*;
use leptos_router::components::A;

use crate::api_client::get_contacts;
use crate::components::layout::Layout;

#[component]
pub fn ContactsPage() -> impl IntoView {
    let contacts = Resource::new(|| (), |_| async move { get_contacts().await });

    view! {
        <Layout title="Contacts">
            <div class="bg-white rounded-lg border border-gray-200 overflow-hidden">
                <Suspense fallback=|| {
                    view! { <p class="p-4 text-gray-400">"Loading\u{2026}"</p> }
                }>
                    {move || Suspend::new(async move {
                        match contacts.await {
                            Ok(list) => {
                                view! {
                                    <table class="w-full text-sm">
                                        <thead class="text-left text-gray-500 border-b border-gray-200">
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
                                                        "text-xs px-2 py-1 rounded capitalize {}",
                                                        c.status.badge_classes(),
                                                    );
                                                    view! {
                                                        <tr class="border-b border-gray-100 last:border-0 hover:bg-gray-50">
                                                            <td class="px-4 py-3 font-medium">
                                                                <A
                                                                    href=format!("/contacts/{}", c.id)
                                                                    attr:class="hover:text-primary-600"
                                                                >
                                                                    {c.name}
                                                                </A>
                                                            </td>
                                                            <td class="px-4 py-3 text-gray-600">{c.company}</td>
                                                            <td class="px-4 py-3 text-gray-600">{c.email}</td>
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
                                view! { <p class="p-4 text-red-500">"Failed to load contacts: " {e}</p> }
                                    .into_any()
                            }
                        }
                    })}
                </Suspense>
            </div>
        </Layout>
    }
}
