use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use crate::api_client::get_contact;
use crate::components::layout::Layout;

#[component]
pub fn ContactDetailPage() -> impl IntoView {
    let params = use_params_map();
    let id = move || params.read().get("id").unwrap_or_default();
    let contact = Resource::new(id, |id| async move { get_contact(id).await });

    view! {
        <Layout title="Contact">
            <A
                href="/contacts"
                attr:class="inline-block mb-4 text-sm text-gray-500 hover:text-gray-700"
            >
                "\u{2190} Back to contacts"
            </A>

            <Suspense fallback=|| view! { <p class="text-gray-400">"Loading\u{2026}"</p> }>
                {move || Suspend::new(async move {
                    match contact.await {
                        Ok(c) => {
                            let badge = format!(
                                "text-xs px-2 py-1 rounded capitalize {}",
                                c.status.badge_classes(),
                            );
                            let created = c.created_at.split('T').next().unwrap_or("").to_string();
                            view! {
                                <div class="bg-white rounded-lg border border-gray-200 p-6 max-w-2xl">
                                    <div class="flex items-center justify-between mb-4">
                                        <div>
                                            <h2 class="text-lg font-semibold">{c.name}</h2>
                                            <p class="text-sm text-gray-500">{c.company}</p>
                                        </div>
                                        <span class=badge>{c.status.label()}</span>
                                    </div>
                                    <dl class="grid grid-cols-1 sm:grid-cols-2 gap-4 text-sm">
                                        <div>
                                            <dt class="text-gray-500">"Email"</dt>
                                            <dd>{c.email}</dd>
                                        </div>
                                        <div>
                                            <dt class="text-gray-500">"Phone"</dt>
                                            <dd>{c.phone}</dd>
                                        </div>
                                        <div class="sm:col-span-2">
                                            <dt class="text-gray-500">"Notes"</dt>
                                            <dd>{c.notes}</dd>
                                        </div>
                                        <div>
                                            <dt class="text-gray-500">"Created"</dt>
                                            <dd>{created}</dd>
                                        </div>
                                    </dl>
                                </div>
                            }
                                .into_any()
                        }
                        Err(_) => {
                            view! {
                                <div class="bg-white rounded-lg border border-gray-200 p-6">
                                    <p class="text-sm text-red-500">"Contact not found."</p>
                                </div>
                            }
                                .into_any()
                        }
                    }
                })}
            </Suspense>
        </Layout>
    }
}
