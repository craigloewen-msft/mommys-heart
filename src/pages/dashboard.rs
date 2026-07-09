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
        <Layout title="Overview">
            <Suspense fallback=|| view! { <p class="text-slate-400">"Loading\u{2026}"</p> }>
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
                                        <div class="rounded-xl border border-slate-800 bg-slate-900 p-5">
                                            <p class="text-3xl font-semibold text-white">{value}</p>
                                            <p class="mt-1 text-sm text-slate-400">{label}</p>
                                        </div>
                                    }
                                })
                                .collect_view()}
                        </div>

                        <div class="mt-6 rounded-xl border border-slate-800 bg-slate-900 p-5">
                            <div class="flex items-center justify-between mb-2">
                                <h2 class="font-semibold text-white">"Welcome"</h2>
                                <span class="text-xs px-2 py-1 rounded-full bg-slate-800 text-slate-300 ring-1 ring-slate-700">
                                    {version_label}
                                </span>
                            </div>
                            <p class="text-sm text-slate-400">
                                "This is the Mommy's Heart CRM, built with Leptos + Axum. Use the top navigation to manage volunteers and cases, browse contacts, or talk to the assistant. The dashboard, data, and chat are wired to the dedicated API \u{2014} ready to augment."
                            </p>
                        </div>
                    }
                })}
            </Suspense>
        </Layout>
    }
}
