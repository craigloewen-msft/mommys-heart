# Phase 2: a real CRM — contacts, organizations, funding, and grants

## Why this is phase 2

Phase 1 made the app good at *case work*: secure messaging and structured Case
Notes. It is still not a CRM, because it has no concept of **a person who is not
a login account**.

Today `users` is the only representation of a human being, and it demands a
unique email and a password hash. That means the organization cannot record a
donor, a funder's program officer, a partner-agency caseworker, an opposing
attorney, a board member, or a volunteer's emergency contact — none of them
should ever be given a login. A volunteer's emergency contact is currently four
flat columns on the `volunteers` table (`emergency_first_name`,
`emergency_last_name`, `emergency_relationship`, `emergency_phone`), which is
exactly the shape a CRM exists to replace.

The money side is a stub. `migrations/0001_init.sql` creates:

```sql
CREATE TABLE grants (
    id   TEXT PRIMARY KEY,
    name TEXT NOT NULL
);
```

There is no `src/server/db/grants.rs`, no server functions, and no UI. The table
is referenced only by the `TRUNCATE` in `reseed()` and by `advance_id_sequence`.
Funding is not modelled at all.

So phase 2 builds the CRM layer the rest of the system has been missing:
contacts, the organizations they belong to, custom properties on people, the
links between people and cases, and real grant and funding records.

## Goal

Add the standard CRM object model, each following the existing
`src/server/db/<object>.rs` + `src/server_fns/<object>.rs` convention:

1. **Organizations** — funders, partner agencies, service providers, courts, and
   employers.
2. **Contacts** — people the organization knows, with or without a login.
3. **Contact properties** — custom key/value fields on a person, grouped into
   sections, mirroring how `case_properties` already works for cases.
4. **Case contacts** — who is involved in a case, and in what role.
5. **Grants** — the existing stub built out into a real award record.
6. **Funding** — money received, against a grant or as a standalone gift, with
   rollups against what was awarded.
7. **Volunteer management** — the existing volunteer application, agreement, and
   logged hours surfaced on the person record instead of in isolation.

## The central design decision: contacts are not users

A **user** is an account that can sign in. A **contact** is a person the
organization has a relationship with. Most contacts will never have an account.

- A contact may link to at most one user, and a user to at most one contact.
- When a contact is linked, `users` stays the source of truth for identity,
  email, and authentication. The contact record never duplicates the password
  hash or the role, and never becomes a second login path.
- Deleting or archiving a contact must never affect the linked account.

This split is what makes a CRM possible, and it is material enough to record as
an ADR (see "Documentation").

## Access model

CRM records are organization-wide, not per-case, so the per-case
`CaseCapability` model does not apply to them. Authorization is by account role:

| Object | Client | Volunteer | Operations / site admin |
| --- | --- | --- | --- |
| Contacts, contact properties | no access | read, create, edit | all, plus archive |
| Organizations | no access | read | all |
| Case contacts | no access | read and edit, gated by the case's existing `ViewCase` / `EditCase` | same |
| Grants, funding | no access | no access | all |

- **Clients get nothing.** Every CRM server function shall reject a client
  account, and no CRM navigation shall render for one. This is the same audience
  rule structured Case Notes already enforce.
- Money is back-office: grants and funding are operations-admin and above.
- Front-line relationship management is open to volunteers, because the people
  who meet contacts are the ones who must be able to record them.

**Field-level sensitivity is deliberately out of scope.** All contact data in
this phase is staff-visible. Contact properties therefore get no `visibility`
column: on a case, `shared` versus `volunteer_only` answers "can the client see
this?", and since clients cannot see contacts at all, reusing that vocabulary
here would give it a second, weaker meaning. `migrations/0004` is explicit that
the app should have one word per question. Sensitive-field controls remain
roadmap work that is blocked on policy.

## Requirements

### Contacts

- **REQ-CRM-001 — Contact record.** A contact shall capture first name, last
  name, preferred name, email, phone, mobile, postal address, job title, an
  optional organization, one or more contact types, a free-text source, an
  internal description, a do-not-contact flag, and an active/archived state. A
  last name or an organization shall be required so no nameless row can exist.
- **REQ-CRM-002 — Optional account link.** A contact may reference at most one
  `users` row, enforced by a unique constraint on the nullable column. Linking
  and unlinking shall be explicit, audited actions available to operations
  admins. The linked account's role and email remain owned by `users`; the
  contact shall render them read-only rather than storing a second copy.
- **REQ-CRM-003 — Contact types.** A contact shall carry one or more types from
  a fixed list: client, volunteer, staff, donor, funder contact, partner,
  service provider, attorney, court professional, government agency, emergency
  contact, board member, other. Types classify a person; they never grant access.
- **REQ-CRM-004 — Directory.** Authorized staff shall see a paginated contact
  directory with server-side keyword search across name, email, phone, and
  organization, and filters for type, organization, and archived state. Search
  and pagination shall be evaluated in SQL, never by loading the directory into
  the browser.
- **REQ-CRM-005 — Archive, never delete.** A contact that is referenced by a
  case link, a grant, or a funding record shall not be deletable. Archiving
  removes a contact from pickers while leaving every existing reference intact
  and readable, following the channel-archival precedent in ADR-0002.

### Contact properties

- **REQ-CRM-006 — Custom properties on a person.** A contact shall support
  ordered key/value properties grouped under a free-text section heading,
  mirroring `case_properties`: primary key `(contact_id, ord)`, a replace-in-
  place write, and the section vocabulary already in `src/helpers/sections.rs`.
  A blank value is meaningful — it is a labelled field waiting to be filled in.
- **REQ-CRM-007 — Property auditing.** Adding, changing, and removing a property
  shall write Change Log entries naming the property key and its old and new
  values, matching how case property edits are already audited.

### Organizations

- **REQ-CRM-008 — Organization record.** An organization shall capture name,
  type (funder, partner, service provider, government, court, employer, other),
  website, phone, email, address, an internal description, and an
  active/archived state. Name shall be unique case-insensitively among active
  organizations.
- **REQ-CRM-009 — Organization detail.** An organization's page shall list its
  contacts and, for administrators, its grants, so a funder relationship can be
  seen in one place.
- **REQ-CRM-010 — Administrative management.** Only operations and site admins
  may create, edit, or archive organizations. Volunteers may read them in order
  to file a contact under one.

### Case contacts

- **REQ-CRM-011 — People on a case.** A case shall link to any number of
  contacts, each with a role on that case (client, household member, emergency
  contact, attorney, opposing party, caseworker, provider contact, court
  professional, other), an optional note, and a primary flag limited to one
  contact per case. Reading requires `ViewCase`; editing requires `EditCase`.
- **REQ-CRM-012 — Staff-only, restricted audit.** Case-contact links shall be
  invisible to client accounts in content, counts, and existence. Their writes
  shall record content-free `volunteer_only` Change Log entries — identifiers
  and roles only, never note text — consistent with REQ-AUD-002 and ADR-0004.

### Grants

- **REQ-CRM-013 — Build out the grant stub.** A forward-only migration shall
  extend the existing `grants` table in place, preserving its `id` and `name`
  columns and any rows, adding: funder organization, optional program-officer
  contact, status, amount requested, amount awarded, application date, decision
  date, period start and end, purpose, reporting cadence, and internal notes.
- **REQ-CRM-014 — Grant lifecycle.** Statuses shall be `Prospect`, `Applied`,
  `Awarded`, `Active`, `Reporting`, `Closed`, and `Declined`. Entering `Awarded`
  or later shall require an awarded amount and a period; entering `Declined` or
  `Closed` shall require a decision date. Amounts shall be stored as integer
  minor units, never floating point.
- **REQ-CRM-015 — Grant list and detail.** Administrators shall see a paginated,
  filterable grant list with totals by status, and a detail page showing the
  funder, the program officer, the award, the period, and the funding received
  against it.

### Funding

- **REQ-CRM-016 — Funding record.** A funding record shall capture an amount in
  minor units, a kind (grant payment, donation, in-kind, other), a received
  date, an optional grant, an optional source organization, an optional source
  contact, a reference such as a cheque or transfer number, and notes. A grant
  payment shall require a grant; a donation shall require a source organization
  or contact.
- **REQ-CRM-017 — Rollups.** A grant shall report total received, the remaining
  balance against its awarded amount, and whether receipts exceed the award.
  Rollups shall be computed in SQL rather than by summing in the browser.
- **REQ-CRM-018 — Correction, not deletion.** A funding record may be voided
  with a reason, which excludes it from rollups while leaving it visible and
  audited. There shall be no hard-delete path, consistent with REQ-AUD-004.

### Volunteer management

- **REQ-CRM-019 — The volunteer as a person.** A contact linked to a volunteer
  account shall show, on one page, the volunteer's application status, agreement
  version and date, decision and decider, logged hours with an all-time total,
  their case assignments, and their contact properties. Existing volunteer data
  shall be read through the existing modules, not copied into the contact.
- **REQ-CRM-020 — Preserve the SSN boundary.** The volunteer SSN remains omitted
  from ordinary reads and available only through the existing audited
  `reveal_ssn` path. The CRM shall not add a second route to it.

### Cross-cutting

- **REQ-CRM-021 — Authoritative identity and time.** The server shall populate
  every actor id, display-name snapshot, and timestamp. The browser shall not be
  trusted for authorship or time.
- **REQ-CRM-022 — Transactional writes.** Each create, edit, archive, link, and
  void shall commit its record and its audit metadata in one transaction; a
  failed audit write shall fail the operation.
- **REQ-CRM-023 — Auditing.** Extend `audit::Entity` with `Contact`,
  `Organization`, and `Grant` so CRM changes use the existing Change Log rather
  than a second log, and surface that log on each detail page for admins.
- **REQ-CRM-024 — Honest UI.** Nothing in the CRM shall imply email campaigns,
  automated donor receipts, accounting-system integration, reminders, or funder
  report submission. None of those are delivered here.

## Implementation shape

1. **Migration `0019_crm_contacts_organizations_and_funding.sql`**, forward-only:
   - `organizations`, `contacts`, `contact_properties`, `case_contacts`, and
     `funding` tables, with `CHECK`s on every enumerated column so slugs stay in
     sync with the Rust `from_slug` helpers, as `0018` does.
   - `ALTER TABLE grants` for the new columns, keeping existing data.
   - A partial unique index giving one primary contact per case, and a unique
     index on `contacts.user_id` where it is not null.
   - Backfill one contact row per existing user, linked by `user_id`, so the
     directory is populated from day one and staff are immediately manageable.
   - Indexes for every filter and search the UI offers.
   - The `volunteers` emergency-contact columns are **left untouched**; promoting
     them into real contact records is a follow-on, so this migration never
     moves data out of a table the app is actively using.
2. **Domain and server layer:** new `src/server/db/{organizations, contacts,
   contact_properties, case_contacts, grants, funding}.rs` and matching
   `src/server_fns/` modules, registered in the existing `mod.rs` files. Every
   server function resolves the caller with `require_user`, rejects clients
   through a shared `require_staff` helper, and gates writes with
   `require_operations_admin` or the case capability where one applies.
3. **Repository-layer defense:** role filtering is applied in SQL as a second
   defense behind the server function, the way `case_notes` does it.
4. **New routes and UI:**
   - `/people` and `/people/:id` — directory and person detail.
   - `/organizations` and `/organizations/:id`.
   - `/admin/grants`, `/admin/grants/:id`, and `/admin/funding`.
   - A "People" entry in the main nav for volunteers and above, and a third
     admin workspace for money alongside Manage cases and Manage users in
     `src/pages/admin.rs`.
   - A case-contacts panel in the `CaseDetail` view tree in `src/pages/cases.rs`,
     rendered only for non-client accounts, as `CaseNotesPanel` is.
5. **Reuse, do not fork:** the `Page<T>` envelope, `ChangeLog`, `Loading`,
   `ProfileLink`, `helpers::sections`, and the existing Tailwind panel and badge
   idioms.
6. **Seed data:** extend `src/mockdata.rs` and `src/server/db/seed.rs` with
   organizations, contacts that are not users, contacts linked to seeded users,
   contact properties, case-contact links, grants across several statuses, and
   funding records including a voided one. Add every new table to the `TRUNCATE`
   in `reseed()` and to `advance_id_sequence`.
7. Comments stay to one or two sentences, per repository convention.

## Suggested build order

Each stage compiles, runs, and is demoable on its own:

1. Migration, organizations, contacts, contact properties, and the people
   directory and detail pages.
2. Case contacts and the case panel.
3. Grants and funding, with the admin money workspace and rollups.
4. The volunteer view on the person page.
5. Seed data, full verification pass, then the documentation restructure.

## Documentation

After the code is verified, replace the phase-1 specification scaffolding under
`docs/case-management/` with a delivered-versus-remaining split. The folder is
renamed `docs/product/`, since it now covers more than case management.

- **`ADR-0005-contacts-are-not-users.md`** — records the contact/user split, the
  optional one-to-one link, why contacts carry no credentials or roles, and why
  contact properties have no visibility axis while case properties do.
- **`ADR-0006-documentation-lifecycle.md`** — removing the spec files
  contradicts the accepted ADR-0001 and REQ-DOC-001, so it is recorded as a
  superseding decision rather than made silently: requirements-first scaffolding
  was right while the MVP was being specified, and a delivered/roadmap split is
  right now that it has shipped. Mark ADR-0001 partially superseded in the index.
- **`delivered.md`** — what HAS BEEN DONE, by observable behavior: phase 1
  (secure messaging, structured Case Notes) and phase 2 (the CRM), each with its
  enforced invariants and its explicit limits. Absorbs both `executive.md` files
  and the durable content of both `requirements.md` files.
- **`roadmap.md`** — what is TO BE DONE, in ordered phases, with policy-blocked
  items separated from straightforward engineering. The seven `future-features/`
  briefs fold in as one row each, keeping their dependencies, open policy
  questions, and acceptance outcomes.
- **Keep** `adr/` and `glossary.md`, extending the glossary with the CRM terms.
- **Delete** `secure-messaging/`, `case-notes/`, and `future-features/`,
  including the `.allium` files, once their durable content has landed.
- **`README.md`** describes the new structure and keeps the `.docx`/RAG
  isolation note from REQ-DOC-004, which remains true.

## Explicitly deferred

- Field-level sensitivity, restricted contact data, and break-glass access.
- Promoting the `volunteers` emergency-contact columns into linked contacts.
- Email campaigns, mail merge, donor receipts, and any outbound contact
  messaging.
- Accounting or payment-processor integration, pledges, recurring gifts, and
  soft credits.
- Funder report generation and submission.
- Duplicate detection and contact merging.
- Household or relationship graphs between contacts.
- Client-facing visibility of any CRM record.
- Bulk import and export.

## Acceptance scenarios

- An operations admin creates an organization and a contact filed under it who
  has no user account, then adds custom properties in two sections; both survive
  a reload and appear in the Change Log.
- A volunteer searches the directory by partial surname, phone, and organization
  name, and the result count comes from the server rather than the page.
- Linking a contact to an existing user shows the account's role and email
  read-only; unlinking leaves the account untouched and is audited.
- Archiving a contact removes it from pickers while existing case links and
  funding records still render it; deleting one that is referenced is refused.
- A volunteer adds an attorney and an emergency contact to a case with roles,
  marks one contact primary, and a second primary is refused.
- A client signed in to that same case sees no people panel, no People nav, and
  direct server-function calls for every CRM endpoint fail without disclosing
  that the records exist — verified by calling the endpoints directly, not just
  by checking the UI.
- A volunteer receives an authorization error from every grant and funding
  endpoint.
- An admin moves a grant to `Awarded` without an amount and is refused; with an
  amount and period it succeeds.
- Two funding records are logged against that grant; the detail page shows total
  received and remaining balance. Voiding one with a reason removes it from the
  rollup while leaving it visible and audited.
- A volunteer's person page shows their agreement version, application decision,
  logged-hours total, and case assignments, and offers no route to their SSN.
- The seeded database demonstrates every status of grant, funding, and contact
  type without hand-entry.

## Verification

Follow the repository workflow; no permanent `cargo test` suites.

1. `cargo fmt --check` and
   `etc/dev.sh -- cargo check --no-default-features --features ssr`.
2. Full `etc/dev.sh build` for SSR plus hydration.
3. `etc/dev.sh reset` to confirm the migration and seed data apply to a fresh
   database, and confirm the contact backfill and the preserved `grants` rows
   against a database created before the migration.
4. `etc/dev.sh run`, wait for `MH_READY`, then browser passes at desktop and
   mobile widths as client, volunteer, operations-admin, and site-admin.
5. For every new server function, exercise an authorized call plus direct
   unauthorized and client-account calls.
6. Inspect the database after lifecycle operations to confirm the archive-not-
   delete restrictions, the one-primary-contact-per-case index, the unique
   `contacts.user_id`, void-excluded rollups, and audit rows.
7. Check the browser console and network panel for errors, and confirm loading
   states, duplicate-submit prevention, and accessible labels and errors.
8. Capture screenshots of the people directory, a person detail page with custom
   properties, an organization page, the case people panel, the grant detail
   with funding rollups, and a client's view showing the CRM absent, for the
   final summary.

## Definition of done

The organization can record and manage people who are not users, file them under
organizations, attach custom properties to them, attach them to cases, and track
grants and the funding received against them — all invisible to clients, all
audited, none of it hard-deletable. The seed data demonstrates every state, and
the documentation states plainly what has been delivered and what remains, with
the contact/user split and the documentation change each recorded as an ADR.

## Outcome

Delivered the CRM layer and restructured the documentation.

**Schema** (`migrations/0019_crm_contacts_organizations_and_funding.sql`):
`organizations`, `contacts`, `contact_properties`, `case_contacts`, and
`funding` tables; the two-column `grants` stub extended in place; a backfill
creating one contact per existing account. Invariants that must not depend on
application code were verified directly against Postgres: awarded grants require
an amount and period, decided grants require a decision date, grant payments
require a grant, donations require a source, funding cannot be deleted (only
voided, with a reason), one primary contact per case, one contact per account,
and referenced contacts/organizations/grants cannot be deleted.

**Code:** six new `server/db` repositories and six `server_fns` modules, a
shared `server_fns/crm.rs` (integer-cents money, a `chrono`-free date check that
also compiles to WASM), `audit::Entity` extended with Contact/Organization/Grant,
and new pages for `/people`, `/organizations`, `/admin/funding`, plus a case
people panel and a People nav entry.

**Two bugs found and fixed during verification:**

1. On a *fresh* database the migration's contact backfill ran before any user
   existed, so no account-linked contacts were created (and `reseed` truncates
   them). The seed now performs the same backfill; verified 9/9 accounts linked.
2. The portfolio "outstanding" figure subtracted unrelated donations from grant
   awards. `grants::totals` now counts only funding recorded against a grant.

**Verified in the browser** as site-admin, volunteer, and client: server-side
search, rollups reconciled against SQL ($185,000 awarded − $107,500 received =
$77,500, voided record excluded), desktop and mobile layouts, and a clean
console. Authorization was tested by calling the endpoints directly, not just
through the UI: a client is refused every CRM endpoint including writes (and no
row was created), and a volunteer reads contacts but is refused grants, funding,
and archiving.

**Note for reviewers:** the `wslc` container runtime on this machine could not
bind ports for new containers, so `etc/dev.sh reset/run` could not complete.
Verification used a Docker Postgres and `cargo leptos watch` against it. No
repository script was changed; `etc/dev.sh` should work normally where `wslc`
is healthy.

**Documentation** now answers two questions: `delivered.md` (what has been
built) and `roadmap.md` (what remains, with policy-blocked work separated). The
per-feature `requirements.md`, `.allium`, `executive.md`, and future-brief files
were removed; ADR-0005 records the contact/user split and ADR-0006 records the
documentation change itself, since it supersedes part of ADR-0001.
