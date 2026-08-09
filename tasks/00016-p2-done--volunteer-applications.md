# Volunteer applications: `volunteers`/`clients` subtype objects, an agreement to accept, and admin approval

Builds on the Volunteers tab just added. Four connected pieces:

1. Restructure `users` as a base object with `volunteers` and `clients` subtype
   objects layered on it.
2. A **volunteer agreement** a non-volunteer accepts from their profile, which
   files an application.
3. A **Pending volunteer requests** section on the admin Volunteers tab with
   approve/deny.
4. Admins can **read the accepted agreement** on that user's profile.

## 1. `users` base + `volunteers` / `clients` subtypes

### Schema (`migrations/0015_volunteers_and_clients.sql`)

`users` stays the base row (identity, contact, `role`). Two subtype tables keyed
by `user_id`, mirroring the base/derived split:

```sql
CREATE TABLE volunteers (
    user_id            TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    status             TEXT NOT NULL CHECK (status IN ('pending','approved','denied')),
    agreement_version  TEXT        NOT NULL,
    agreed_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    decided_by         TEXT        REFERENCES users(id) ON DELETE SET NULL,
    decided_by_name    TEXT        NOT NULL DEFAULT '',
    decision_note      TEXT        NOT NULL DEFAULT '',
    decided_at         TIMESTAMPTZ
);
CREATE INDEX volunteers_pending_idx ON volunteers (agreed_at) WHERE status = 'pending';

CREATE TABLE clients (
    user_id    TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

One row per volunteer, holding both the accepted agreement and the application
state — a denial is an update in place (status `denied`, decision recorded), and
re-applying overwrites the row with a fresh `pending` (so denial is not final,
per the decision below). `clients` is deliberately thin for now: it exists to
establish the symmetric structure and give client-only data a home.

Backfill in the same migration: insert a `volunteers` row (`status='approved'`,
`agreement_version=''` meaning "predates the agreement") for every existing user
whose role is `volunteer`, and a `clients` row for every `client`. Seeded demo
users therefore keep working.

### Rust layering

- `src/server/db/volunteers.rs` and `src/server/db/clients.rs` — new db modules
  alongside `users.rs`, following the existing `Row` + `into_domain` convention.
- `src/server_fns/volunteers.rs` — the shared DTOs and server functions.
- Composition over inheritance, matching how `UserProfile` already wraps a user:

```rust
/// A volunteer: the base user plus the volunteer-specific record.
pub struct Volunteer {
    pub user: User,
    pub application: VolunteerApplication,
}
```

`VolunteerApplication { status, agreement_version, agreed_at, decided_by_name,
decision_note, decided_at }` with `enum VolunteerStatus { Pending, Approved,
Denied }`.

The `AgreementStatus`/`VolunteerListItem` seam added in the previous change is
now backed by real data: `db::users::agreement_status()` is deleted and
`volunteers_page` joins `volunteers` to report `Completed` (an approved row with
a non-empty `agreement_version`) or `Outstanding` (approved but backfilled, i.e.
predates the agreement). The `NotTracked` variant goes away.

## 2. The volunteer agreement

`src/helpers/volunteer_terms.rs`, modelled exactly on `helpers/terms.rs`:
`VOLUNTEER_AGREEMENT_VERSION`, `VOLUNTEER_AGREEMENT_SECTIONS: &[TermsSection]`,
`VOLUNTEER_ATTESTATION`, and `is_current()`. Reuse the existing `TermsSection`
type rather than redefining it.

**The wording is PLACEHOLDER.** I will write plain, accurate volunteer-role text
(confidentiality, no guaranteed outcomes, not employment, at-will withdrawal),
but it is not reviewed legal prose. A prominent `//! PLACEHOLDER` note goes at
the top of the module, and the handoff calls it out for replacement before this
is used with real volunteers.

### Page: `/volunteer-agreement`

A new `src/pages/volunteer_agreement.rs` reusing the `case_signup.rs` step-1
pattern verbatim in style: scrollable `max-h-[28rem]` bordered pane, sections
rendered from the constant, scroll-to-end gate, then the attestation, the
"I have read and accept" checkbox, and Accept/Cancel. Accept calls
`apply_to_volunteer(VOLUNTEER_AGREEMENT_VERSION)`; Cancel returns to `/profile`.

### Profile entry point

On `/profile` (own profile only), when the viewer is **not** a volunteer and has
no pending application, show a "Become a volunteer" card with a short blurb and
a button to `/volunteer-agreement`. When an application is pending, the card
instead reads "Your volunteer application is being reviewed." Nothing is shown
for existing volunteers/admins. Requires `UserProfile` to carry the viewer's
volunteer state.

## 3. Admin approval

**Where it lives:** on the `volunteers` row, not as a new `admin_requests` kind.
The admin_requests machine is built around snapshot-and-compare of a field that
already exists on another table (previous role vs requested role, current caps vs
requested caps) with stale-detection between filing and decision. An application
has no such prior value — the record *is* the application — so a new kind would
mean mostly-NULL columns and a `CHECK` exception. Keeping it on `volunteers`
also keeps the accepted agreement and its approval in one place, which is what
piece 4 needs to display.

**Server fns** in `src/server_fns/volunteers.rs`:

- `apply_to_volunteer(agreement_version)` — `require_user`; rejects if the caller
  already has volunteer privileges or a pending row; rejects a stale version via
  `is_current`; upserts a `pending` row.
- `list_pending_volunteer_applications()` — `require_operations_admin`.
- `decide_volunteer_application(user_id, approve, note)` — `require_site_admin`,
  matching the existing rule that changing a user's role is site-admin-only. In one
  transaction: lock the row, reject if not still `pending`, and on approve set
  `users.role = 'volunteer'` **and** the volunteer row to `approved`, writing an
  `audit::record_in_transaction` role entry exactly as `admin_requests::decide`
  does; on deny set `denied` with the note. Then fire the email.

Operations admins see the pending list read-only (no Approve/Deny buttons), so
approving never becomes a second path around the site-admin role gate.

**Emails** — `templates::volunteer_application_decided(...)` in the existing
branded-template style, sent via a `notify_volunteer_decision(...)` helper in
`server/notifications.rs` using the established `tokio::spawn` fire-and-forget
pattern. Approval and denial both email the applicant; **denial has no page UI
beyond disappearing from the pending list**, as specified. Since a denied user
may re-apply, the profile button returns for them.

**UI** — a "Pending volunteer requests" section at the top of the Volunteers tab
(`src/pages/admin/volunteers.rs`), listing applicant name (profile link), email,
when they accepted, and Approve / Deny buttons with an optional note, following
`components/admin_requests.rs`'s `RequestCard`. Hidden when empty. A count badge
on the Volunteers tab: add `volunteer_requests_pending` to `AppState` and to
`load_app_badges` alongside the existing counts (populated for operations admins
too, since they can see the queue even though only site admins may decide).

## 4. Viewing the agreement on a profile

On `/profile/:id`, a viewer with operations-admin permissions (and the owner
themselves) sees a "Volunteer agreement" panel: status, version, accepted
timestamp, who decided it, and a "View agreement" toggle that expands the full
agreement text as accepted. Because the record stores the version string, the
page renders the current constant only when the versions match, and otherwise
states which version was accepted rather than showing wording the person never
saw — the same reasoning `helpers/terms.rs` documents.

`load_profile` gains a `volunteer: Option<VolunteerApplication>` field populated
under the same visibility rule as `volunteer_hours`.

## Verification

- `cargo check --no-default-features --features ssr`, `cargo fmt`, no new clippy
  warnings.
- Migration applies to the seeded dev database and backfills existing
  volunteers/clients.
- In-browser, end to end: as a client, `/profile` shows "Become a volunteer" →
  agreement gates on scroll + checkbox → accept files the application and the
  card switches to "being reviewed"; as an operations admin the Volunteers tab
  shows the pending request with a badge (read-only for an operations admin; a
  site admin gets Approve/Deny) → approve flips the user's role and they
  appear in the volunteer list as Completed; the admin then opens that user's
  profile and reads the accepted agreement. Repeat with deny (email only) and
  confirm the denied user can re-apply.
- Email is dry-run in dev; confirm the intended send is logged for both outcomes.
