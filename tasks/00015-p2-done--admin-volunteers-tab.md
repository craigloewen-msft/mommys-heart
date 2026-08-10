# Admin "Volunteers" tab + split `pages/admin.rs` into per-tab components

Two things in one change: a new **Volunteers** tab on the admin dashboard, and a
refactor of the 1000-line `src/pages/admin.rs` so each tab is its own component
file.

## 1. Refactor: `src/pages/admin.rs` → `src/pages/admin/`

`src/pages/admin.rs` is currently the page shell, the tab bar, the whole "Case
access" tab (search + pagination + email-failure panel), *and* the 700-line
`UserCard`. Split it (no behaviour change) into:

| File | Contents |
| --- | --- |
| `src/pages/admin/mod.rs` | `AdminDashboardPage`: admin guard, `AdminTab` enum, tab bar, the shared `reload` signal, and one `<div role="tabpanel">` per tab. Keeps `TabButton`. |
| `src/pages/admin/case_requests.rs` | `CaseRequestsTab` — blurb + existing `components::admin_cases::CaseRequests`. |
| `src/pages/admin/case_access.rs` | `CaseAccessTab` — search input + debounce, the paged user list `Effect`, `PAGE_SIZE`, "Load more" footer, and the collapsible "Email delivery failures" panel. |
| `src/pages/admin/user_card.rs` | `UserCard`, `DraftAssignment`, `same_caps`, `badge` — moved verbatim. |
| `src/pages/admin/requests.rs` | `RequestsTab` — blurb + existing `AdminRequestCenter`. |
| `src/pages/admin/volunteers.rs` | New `VolunteersTab` (below). |

Mechanics:

- `src/lib.rs` keeps `pub mod admin;` inside the inline `pages` module; the file
  just becomes `src/pages/admin/mod.rs`.
- Each tab component takes what it needs as props (`reload: RwSignal<u32>`,
  `is_site_admin: bool`, `actor_user_id`) instead of reaching into parent locals.
- The existing lazy-load behaviour is preserved: the Case-access effect only
  fetches when its tab is selected, so each tab component should keep an
  `active: Signal<bool>` prop (or be mounted only when selected) rather than
  fetching on page load. Tabs stay hidden via `class:hidden` as today so tab
  switches don't lose state.
- Pure move: no styling or copy changes in the refactor commit.

## 2. New Volunteers tab

**Visibility:** same gate as the rest of the page — any user with
operations-admin permissions (site admins included). Tab sits between "Case
Requests" and "Case access".

**Contents:** every user with `AccountRole::Volunteer` (that role only — not
operations/site admins), ordered by last name, showing:

- full name, linked to `/profile/:id` via the existing
  `components::profile_link::ProfileLink` so an admin can click through to the
  full profile (that page already shows contact details to operations admins);
- email;
- an **Agreement** status badge.

Search box + "Load more" pagination, matching the Case-access tab's pattern
(reuse `Page<T>` from `server_fns::pagination`).

### Agreement status (seam only, for now)

There is no volunteer-agreement record in the codebase yet — `terms_acceptances`
is the *client* Terms & Conditions accepted at case signup, and the real
volunteer agreement is coming in a separate PR. So this change lands the column
and its plumbing but not the source of truth:

- `server_fns::users::VolunteerListItem { id, first_name, last_name, email,
  agreement: AgreementStatus }`.
- `enum AgreementStatus { Completed, Outstanding, NotTracked }`, with a doc
  comment naming `NotTracked` as the temporary value until the volunteer
  agreement lands.
- The DB layer resolves it in one clearly-marked helper
  (`fn agreement_status(...) -> AgreementStatus`) that currently returns
  `NotTracked`; the follow-up PR only has to change that function.
- UI badge: green "Completed", amber "Outstanding", slate "Not recorded yet" for
  `NotTracked`, so the finished states render correctly the moment data exists.

### Server work

- `src/server/db/users.rs`: `pub async fn volunteers_page(offset, limit, search)
  -> Result<Page<VolunteerListItem>, sqlx::Error>` — same search predicate as
  `page()` but with `WHERE role = 'volunteer'`, and no per-user case-assignment
  fan-out (not shown in this list).
- `src/server_fns/users.rs`: `#[server] list_volunteers_page(offset, limit,
  search)` guarded by `require_user` + `require_operations_admin`, same as
  `list_users_page`.

## Verification

- `cargo check --no-default-features --features ssr`.
- `etc/dev-run.sh`, sign in as a site admin and as an operations admin: all four
  tabs render and behave exactly as before, Volunteers lists seeded volunteers
  with the "Not recorded yet" badge, search + Load more work, and clicking a name
  opens that user's profile with contact details visible.
