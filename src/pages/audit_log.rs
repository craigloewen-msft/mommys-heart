use leptos::prelude::*;

use crate::components::guard::deny_redirect;
use crate::components::layout::Layout;
use crate::state::AppState;
use crate::types::Permission;

/// Admin/Staff-only audit trail: a chronological record of who did what, to
/// which record, and when. Demonstrates accountability for confidential data.
#[component]
pub fn AuditLogPage() -> impl IntoView {
    let state = expect_context::<AppState>();

    if let Some(redirect) = deny_redirect(&state, Permission::ViewAuditLog) {
        return redirect;
    }

    // Optional filter: show only security-relevant denials.
    let denials_only = RwSignal::new(false);

    let rows = move || {
        state
            .audit_log
            .get()
            .into_iter()
            .filter(|e| !denials_only.get() || e.action.is_denial())
            .map(|e| {
                let role_badge = format!(
                    "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {}",
                    e.actor_role.badge_classes(),
                );
                let action_class = if e.action.is_denial() {
                    "font-medium text-rose-300"
                } else {
                    "font-medium text-slate-100"
                };
                view! {
                    <tr class="border-b border-slate-800 last:border-0 hover:bg-slate-800/40">
                        <td class="px-4 py-3 text-slate-400 whitespace-nowrap">{e.at}</td>
                        <td class="px-4 py-3">
                            <p class="font-medium text-slate-100">{e.actor}</p>
                            <span class=role_badge>{e.actor_role.label()}</span>
                        </td>
                        <td class=format!("px-4 py-3 {action_class}")>{e.action.label()}</td>
                        <td class="px-4 py-3 text-slate-300">{e.target}</td>
                    </tr>
                }
            })
            .collect_view()
    };

    view! {
        <Layout title="Audit log">
            <p class="mb-4 max-w-3xl text-sm text-slate-400">
                "Every sign-in, record change, and document access is recorded here so the
                organization can demonstrate accountability for confidential survivor data.
                This demo keeps the trail in memory; production should persist it to
                append-only, tamper-evident storage."
            </p>

            <label class="mb-4 inline-flex cursor-pointer items-center gap-2 text-sm text-slate-300">
                <input
                    r#type="checkbox"
                    class="accent-primary-500"
                    prop:checked=move || denials_only.get()
                    on:change=move |ev| denials_only.set(event_target_checked(&ev))
                />
                "Show access denials only"
            </label>

            <div class="overflow-hidden rounded-xl border border-slate-800 bg-slate-900">
                <table class="w-full text-sm">
                    <thead class="border-b border-slate-800 text-left text-slate-400">
                        <tr>
                            <th class="px-4 py-3 font-medium">"When"</th>
                            <th class="px-4 py-3 font-medium">"Actor"</th>
                            <th class="px-4 py-3 font-medium">"Action"</th>
                            <th class="px-4 py-3 font-medium">"Target"</th>
                        </tr>
                    </thead>
                    <tbody>{rows}</tbody>
                </table>
            </div>
        </Layout>
    }
    .into_any()
}
