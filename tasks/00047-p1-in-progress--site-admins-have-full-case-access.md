# Site admins have full case access; "Admin read-only" is removed

Case access was governed *only* by the `CaseCapability` rows stored in
`case_assignments` — owning a case granted nothing, and being an admin granted
nothing. Task 00024 then bolted on a second, weaker path so an unassigned admin
could still inspect a case: `require_case_view_or_admin_read` and
`require_case_evidence_read_or_admin`, surfaced as an "Admin read-only" badge in
the case directory and a blue banner on the case itself.

That middle state is gone. A **site admin now holds every capability on every
case**, so they get the ordinary case view with every control live. Nobody else
gains anything.

## Decisions

- **Site admin only.** `AccountRole::has_full_case_access()` returns
  `is_site_admin()`. It is a named predicate, not an inlined `is_site_admin()`
  call, so widening the policy later is a one-line change in one place.
- **Operations admins keep only their stored capabilities.** They lose the
  read-only inspection path along with everyone else. This costs them little:
  `require_case_access_management` already lets an operations admin grant
  *themselves* any capabilities on any case, directly and without approval. The
  workflow becomes "give yourself access, which is audited" rather than "look
  without leaving a trace".
- **Computed, never stored.** The grant is derived from `users.role` at request
  time. No migration, no rows written to `case_assignments`, and demoting a site
  admin removes their access immediately.
- **Lists stay assignment-scoped.** The Cases list, Case Chat, unread badges and
  notification recipients still key off `case_assignments` rows. A site admin is
  not subscribed to every case in the system; Admin → Cases remains how they
  reach a case they are not assigned to.
- **The lifecycle gate is untouched.** `require_cap` checks the capability and
  `CaseStatus::accepts_changes()` separately, so a declined or withdrawn case
  still refuses every write, including a site admin's.

## Implementation

### The grant — `src/server_fns/users.rs`

`AccountRole::has_full_case_access()`, plus `User::capabilities_for()` returning
`CaseCapability::ALL` for such a role. Because `permissions::capabilities_on`
delegates to `capabilities_for`, the entire server-side gate — `require_cap`,
`require_channel`, and every `#[server]` function behind them — follows from
that one method.

`User::is_assigned_to` deliberately still answers "is there a stored row": that
is what list scoping and the new-assignment notification actually mean.

### The capabilities the browser is told about — `src/server/db/capabilities.rs`

`get_single_case` / `get_multi_case` now take `&User` and short-circuit to the
full set, skipping the query. They fill `Case.capabilities` and
`CaseSummary.capabilities`, which is what every UI gate reads, so the client
follows the server without a single new UI branch. Their callers in
`db/cases.rs` (`get`, `admin_summary`, `get_summaries_for_user`, `admin_page`)
took `user_id: &str` and now take `&User`.

### Deletions

- `permissions::require_case_read_cap_or_admin`,
  `require_case_view_or_admin_read`, `require_case_evidence_read_or_admin`.
- `cases::load_admin_case`, which became byte-for-byte identical to `load_case`.
- The `admin_read` prop on `CaseDetail` and the "Admin read-only view" banner.

The four callers of the deleted helpers became ordinary `require_cap` checks:
case contacts (`ViewCase`), the evidence download route (`ViewEvidence`), and the
case audit scope (`require_operations_admin` plus `ViewCase`).

### Two SQL clauses that hardcoded the old rule

- `db/case_contacts.rs::list_for_contact` had an
  `OR viewer.role IN ('operations_admin','site_admin')` escape hatch — the
  admin-read path expressed in SQL. Narrowed to `site_admin`, and `site_admin`
  added to the `can_edit` expression so a site admin can edit the links they see.
- `db/cases.rs::search_editable` (the case picker for linking a person) gained a
  full-access branch, otherwise a site admin could not find an unassigned case in
  a picker they are entitled to use.

### UI

- Admin → Cases: the empty-capability badge says **"No access"** instead of
  "Admin read-only". Opening such a case shows a short panel explaining it, with
  a link to the admin's own user page — without it the page would render nothing,
  because `CaseDetail` swallows the load error.
- Admin → Users: a site admin's card shows "Site admins have full access to every
  case" in place of a capability editor that no longer means anything. This also
  stops anyone filing a pointless `CaseCapabilities` approval request against a
  site admin.

## What was deliberately left alone

Clients remain excluded from volunteer-only channels and properties by account
role, independently of capabilities. Ownership-gated actions (withdraw, owner
reassignment, accept/decline) keep their own gates — they were never
capability-based. Draft-note inspection keeps its audited `AdminInspectDraft`
path. Seed data is unchanged: the demo site admin keeps her four assignments so
her own Cases list is not empty, they are simply no longer what grants access.
