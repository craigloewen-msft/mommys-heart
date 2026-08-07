# Admin case review: accept / decline a case, and show the client who's on it

## Problem

A case can be created by the public signup flow with no staff involvement, and
nothing in the system records whether the organization has *decided to take it*.

- `cases.status` (`Open | Monitor | Closed`) is guarded by the `EditCase`
  capability, and the signup flow grants the client owner a full capability
  assignment — so the client can change it themselves. It cannot be used as a
  staff decision signal.
- `case_assignments` answers "who may touch this case", not "did we accept it".
  An unassigned case is ambiguous: still triaging, accepted-but-unstaffed, or
  quietly declined.
- `admin_approval_requests` (0008) approves *user capability requests*, never a
  case.
- The client is told nothing at all after signup.

## Approach

Add a **review state** that is deliberately separate from the lifecycle status,
following the `visibility` vs `section` split established in migration 0004: one
column per question, no fused flags.

`review_state` answers "has the org accepted this case?" (admin-owned).
`status` keeps answering "where is this case in its life?" (staff-owned).
"Is someone working it?" is **derived**, never stored, so it cannot drift.

## Plan

### 1. Schema — `migrations/0013_case_review_state.sql`

```sql
ALTER TABLE cases
    ADD COLUMN review_state    TEXT NOT NULL DEFAULT 'accepted',
    ADD COLUMN review_reason   TEXT NOT NULL DEFAULT '',
    ADD COLUMN reviewed_by     TEXT NOT NULL DEFAULT '',
    ADD COLUMN reviewed_at     TEXT NOT NULL DEFAULT '';
CREATE INDEX cases_review_state_idx ON cases(review_state);
```

Values (text slugs, matching the `from_slug()` round-trip convention):
`pending_review`, `accepted`, `declined`. Existing rows default to `accepted`
so nothing already in flight is retroactively put in limbo.

### 2. Domain — `src/server_fns/cases.rs`

- `CaseReviewState` enum with `ALL` / `label()` / `slug()` / `from_slug()` /
  `badge_classes()`, mirroring `CaseStatus`.
- Add `review_state`, `review_reason`, `assigned_volunteers: Vec<String>` (display
  names) to both `Case` and `CaseSummary`.
- `CaseSummary::work_state()` — the single derived helper every surface renders:
  `Submitted | UnderReview | AwaitingVolunteer | InProgress { volunteer } | Declined | Closed`.
  Both the client banner and the staff chip read this one function.

### 3. Server fn — `set_case_review_state(case_id, state, reason)`

- Gated on **account role** (`OperationsAdmin` / `SiteAdmin`) via
  `src/server/permissions.rs`, *not* a `CaseCapability`. Deciding whether the org
  takes a case is an org-level act; it must not be reachable by a case-scoped
  grant. Follows the precedent set for `volunteer_only` visibility in 0004.
- Requires a non-empty `reason` when declining.
- Writes an audit/change-log entry (same path `set_case_status` uses).
- Fires `notify_case(..., Audience::Everyone, "accepted this case" /
  "declined this case")` so the client is emailed on decision.

### 4. Creation defaults

- Case-signup materialization (`src/server/db/pending_registrations.rs` →
  `cases::create`) inserts `pending_review`.
- Staff-created cases (`create_case` by a volunteer/admin) insert `accepted` —
  a staff member creating a case *is* the acceptance.

### 5. Close the status loophole

`set_case_status` additionally requires the caller to be staff (volunteer or
admin account role) on top of `EditCase`, so a client owner can no longer flip
their own case to `Closed`. Small, contained, and removes the reason status was
unusable as a signal.

### 6. Admin UX — `src/pages/admin.rs`

Third tab beside "Case access" / "Requests", named **"Cases"**. This is the
admin's overall view of *every* case in the system, not just a review queue —
review is one action available from it, not the tab's purpose.

Built to the same pattern as the existing tabs (debounced search input, windowed
`Load more` paging, `reload` tick after a mutation), so it reads as a sibling of
"Case access" rather than a new kind of screen.

**Data.** Promote the existing lookup into a proper paged listing:
`admin_list_cases_page(offset, limit, query, filter)` in
`src/server_fns/cases.rs`, backed by a `cases::list_page` in
`src/server/db/cases.rs` generalized from the current `search_lite` (same
`summary_select` projection, plus `review_state` and assigned-volunteer names).
Admin-role gated, `"true"` message-count scope like the other admin lookups.
`admin_search_cases` becomes a thin wrapper or is folded in, so there is one
query path.

**Layout.**

- Search box over case name / id / client name.
- Filter chips: `All` (default) · `Pending review` · `Accepted` · `Declined` ·
  `Unstaffed` · `Inactive`. `Pending review` carries the count badge, reusing
  the `admin_request_pending` mechanism in `AppState` for a
  `cases_pending_review` count so the tab itself shows attention-needed at a
  glance without being reduced to a queue.
- Rows show: case name (links to the case), client/owner, `status` badge,
  work-state chip from `work_state()`, assigned volunteers (or "Unstaffed"),
  last activity / inactive badge, submitted-at.

**Actions, inline per row.**

- **Accept** / **Decline** for anything `pending_review` (decline opens a
  required reason field); re-review remains available on already-decided cases
  via a small "Change decision" control, so a mistake is fixable without SQL.
- Link through to the existing capability assignment UI for staffing, so
  accepting → assigning is one continuous path and there is no second flow to
  learn.

A case row is the same `CaseSummary` the client-facing list already renders, so
the chip logic is shared rather than duplicated.

### 7. Client & staff UX — `src/pages/cases.rs`

- **Case list card**: one work-state chip next to the existing status badge,
  rendered from `work_state()`. Reuses `badge()` and the Tailwind badge classes
  already in the file.
- **Case detail header**: for a client viewer, a one-sentence plain-language
  banner instead of jargon:
  - pending: "Submitted — a coordinator is reviewing your case."
  - accepted, unstaffed: "Accepted — we're arranging support for you."
  - accepted, staffed: "Being worked on by Jane D."
  - declined: "Not accepted" plus `review_reason`.
- Staff keep seeing the raw `status` dropdown unchanged; the review state is
  read-only for them unless they are an admin.

## Explicitly not doing

- No fourth `CaseStatus` variant. Overloading status is what caused the
  ambiguity in the first place.
- No separate "is being worked on" boolean. It is derived from
  `accepted + assignment`, so assigning a volunteer updates the client's view
  with no extra step anyone can forget.
- No reuse of `admin_approval_requests`; that table is shaped around
  user/capability requests and bending it to cases would fuse two workflows.

## Verification

- `cargo check --no-default-features --features ssr`.
- Manual pass with `etc/dev-run.sh`: sign up a case as a client → it appears in
  the admin "Cases" tab (and under the `Pending review` filter, with the count
  badge) and the client sees "Submitted"; accept it → client sees "Accepted";
  assign a volunteer → client sees "Being worked on by …" and the row leaves the
  `Unstaffed` filter; decline another → client sees the reason. Confirm search
  and `Load more` work across more cases than one page, and that a client
  account can no longer change `status`.
