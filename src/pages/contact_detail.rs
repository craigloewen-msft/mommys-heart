use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use crate::api_client::get_contact;
use crate::components::communications::{CommScope, CommunicationsTimeline};
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
                attr:class="inline-block mb-4 text-sm text-slate-400 hover:text-slate-200"
            >
                "\u{2190} Back to contacts"
            </A>

            <Suspense fallback=|| view! { <p class="text-slate-400">"Loading\u{2026}"</p> }>
                {move || Suspend::new(async move {
                    match contact.await {
                        Ok(c) => {
                            let badge = format!(
                                "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {}",
                                c.status.badge_classes(),
                            );
                            let created = c.created_at.split('T').next().unwrap_or("").to_string();
                            let contact_id = c.id.clone();
                            view! {
                                <div class="rounded-xl border border-slate-800 bg-slate-900 p-6 max-w-2xl">
                                    <div class="flex items-center justify-between mb-4">
                                        <div>
                                            <h2 class="text-lg font-semibold text-white">{c.name}</h2>
                                            <p class="text-sm text-slate-500">{c.company}</p>
                                        </div>
                                        <span class=badge>{c.status.label()}</span>
                                    </div>
                                    <dl class="grid grid-cols-1 sm:grid-cols-2 gap-4 text-sm">
                                        <div>
                                            <dt class="text-slate-500">"Email"</dt>
                                            <dd class="text-slate-200">{c.email}</dd>
                                        </div>
                                        <div>
                                            <dt class="text-slate-500">"Phone"</dt>
                                            <dd class="text-slate-200">{c.phone}</dd>
                                        </div>
                                        <div class="sm:col-span-2">
                                            <dt class="text-slate-500">"Notes"</dt>
                                            <dd class="text-slate-200">{c.notes}</dd>
                                        </div>
                                        <div>
                                            <dt class="text-slate-500">"Created"</dt>
                                            <dd class="text-slate-200">{created}</dd>
                                        </div>
                                    </dl>
                                    <CommunicationsTimeline scope=CommScope::Contact(contact_id) />
                                </div>
                            }
                                .into_any()
                        }
                        Err(_) => {
                            view! {
                                <div class="rounded-xl border border-slate-800 bg-slate-900 p-6">
                                    <p class="text-sm text-rose-400">"Contact not found."</p>
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
