use leptos::prelude::*;

use crate::api_client::{get_contacts, get_version};
use crate::components::layout::Layout;
use crate::types::ContactStatus;

#[component]
pub fn DashboardPage() -> impl IntoView {
    let data = Resource::new(
        || (),
        |_| async move {
            let contacts = get_contacts().await.unwrap_or_default();
            let version = get_version().await.ok();
            (contacts, version)
        },
    );

    view! {
        <Layout title="Dashboard">
            <Suspense fallback=|| view! { <p class="text-gray-400">"Loading\u{2026}"</p> }>
                {move || Suspend::new(async move {
                    let (contacts, version) = data.await;
                    let total = contacts.len();
                    let active = contacts.iter().filter(|c| c.status == ContactStatus::Active).count();
                    let leads = contacts.iter().filter(|c| c.status == ContactStatus::Lead).count();
                    let inactive = contacts.iter().filter(|c| c.status == ContactStatus::Inactive).count();
                    let stats = [
                        ("Total contacts", total),
                        ("Active", active),
                        ("Leads", leads),
                        ("Inactive", inactive),
                    ];
                    let version_label = version
                        .map(|v| format!("API v{}", v.version))
                        .unwrap_or_else(|| "API unavailable".into());

                    view! {
                        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
                            {stats
                                .into_iter()
                                .map(|(label, value)| {
                                    view! {
                                        <div class="bg-white rounded-lg border border-gray-200 p-5">
                                            <p class="text-2xl font-semibold">{value}</p>
                                            <p class="text-sm text-gray-500">{label}</p>
                                        </div>
                                    }
                                })
                                .collect_view()}
                        </div>

                        <div class="mt-6 bg-white rounded-lg border border-gray-200 p-5">
                            <div class="flex items-center justify-between mb-2">
                                <h2 class="font-semibold">"Welcome"</h2>
                                <span class="text-xs px-2 py-1 rounded bg-gray-100 text-gray-600">
                                    {version_label}
                                </span>
                            </div>
                            <p class="text-sm text-gray-600">
                                "This is the Mommy's Heart CRM, built with Leptos + Axum. Use the sidebar to browse contacts or talk to the assistant. The dashboard, data, and chat are wired to the dedicated API \u{2014} ready to augment."
                            </p>
                        </div>
                    }
                })}
            </Suspense>
        </Layout>
    }
}
