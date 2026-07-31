//! A person's display name rendered as a link to their profile.
//!
//! Used everywhere a case surfaces someone by name (the case's owner, the
//! author of a chat message) so a reader can jump straight to
//! `/profile/:id`. When no user id is known for the name — historical records
//! store only a display name — it degrades to plain text rather than a dead
//! link.

use leptos::prelude::*;
use leptos_router::components::A;

/// A person's name, linked to their profile when their user id is known.
///
/// `class` styles the plain-text fallback and is extended with the link accent
/// when the name is clickable.
#[component]
pub fn ProfileLink(
    #[prop(into)] user_id: String,
    #[prop(into)] name: String,
    /// Base classes applied in both the link and plain-text cases.
    #[prop(into, default = String::new())]
    class: String,
) -> impl IntoView {
    if user_id.trim().is_empty() {
        return view! { <span class=class>{name}</span> }.into_any();
    }
    let href = format!("/profile/{user_id}");
    let link_class = format!(
        "{class} rounded font-medium text-primary-300 underline decoration-dotted \
         underline-offset-2 hover:text-primary-200 hover:decoration-solid"
    );
    view! {
        <A href=href attr:class=link_class attr:title="View profile">
            {name}
        </A>
    }
    .into_any()
}
