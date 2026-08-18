# What has been delivered

This file describes what the application **does today**, by observable behavior.
For what is *not* built, see [`roadmap.md`](roadmap.md).

Each section lists the invariants that are actually enforced and, deliberately,
the limits that genuinely apply. Where an invariant is enforced by the database
rather than only by application code, that is stated — those survive a bug in the
server layer.

---

## Phase 1: secure case messaging

*Requirements: `REQ-MSG-001` – `REQ-MSG-009`.*

Authenticated in-app messaging attached to a case.

- Each case has shared channels and one permanent volunteer-only channel.
- Sending requires the `SendMessages` capability; reading requires `ViewCase`.
  Both are resolved from the stored channel, never from a case id supplied by
  the browser.
- A client account cannot discover the volunteer-only channel through the UI, a
  direct server-function call, unread counts, audit queries, or exports. It
  receives the same "not found" answer as for a channel that does not exist.
- Messages are immutable once sent. Channels are **archived**, not deleted;
  archived channels reject new messages and keep their full history. At least one
  active shared channel must remain on a non-declined case.
- A message, its recipient rows, and its audit metadata commit in one
  transaction. Email delivery is best-effort and happens only after commit.
- First-read timestamps are write-once: a later read never overwrites the
  original.
- Email and browser notifications are content-free. They say a secure message is
  waiting and nothing about the case, the client, the channel, or the message.
- An operations or site admin holding `ViewCase` can export a channel transcript
  as UTF-8 CSV. Every export is itself audited.

**Limits, stated on purpose:** no end-to-end encryption, no attachments, no SMS,
no email reply ingestion, no reactions or editing, no PDF transcripts, no
guarantee of emergency delivery, and no compliance certification.

---

## Phase 1: structured Case Notes

*Requirements: `REQ-CN-001` – `REQ-CN-011`.*

Formal internal service records, distinct from chat and from the Change Log.

- Notes move through `draft` → `finalized`, or `draft` → `discarded`. Migrated
  historical notes hold the `legacy` state.
- Only the author may edit or finalize their draft. An operations or site admin
  with `ViewCase` may *inspect* a draft through an audited path, but cannot
  rewrite it. Other team members do not see it until it is finalized.
- Finalization is terminal and requires passing server validation plus an
  in-app attestation: the author confirms accuracy and types their current full
  name, which the server checks against the authenticated account. This is an
  authenticated attestation, **not** a qualified digital signature.
- Finalized, legacy, and discarded notes are immutable. A database trigger
  rejects any update to a row in a terminal state, and a second trigger rejects
  `DELETE` on the table entirely.
- Corrections are made by appending an immutable signed addendum, which inherits
  the parent's audience and never replaces it. A trigger enforces that an
  addendum's parent is finalized or legacy and that audiences match.
- Discarding preserves an auditable tombstone rather than removing the row.
- Legacy notes keep their historical shared audience, so migration neither
  revoked nor widened access.
- Clients receive no structured-note content, metadata, or counts.
- Notes are listed per case, newest activity first, with server-side pagination
  and filters for date range, author, interaction type, state, urgency, and
  keyword.
- Note actions write content-free entries to a restricted note audit table. Note
  bodies and client-reported facts never enter the generic Change Log.
- The form states plainly that recording an urgent concern does not contact
  emergency services or replace the organization's escalation procedure.

**Limits, stated on purpose:** no full 19-section dynamic form, no supervisor
approval or returned revisions, no automated safety escalation, no attachments or
PDF output, no field-level sensitivity controls, no cross-case note search.

---

## Phase 1: audit and retention

*Requirements: `REQ-AUD-001` – `REQ-AUD-004`.*

- Change Log entries carry `shared` or `volunteer_only` visibility, filtered
  server-side by account role and case access on every query.
- Sensitive audit entries name the action and the record, never the content.
- The database prevents a message from naming a case different from its
  channel's case, and prevents channel deletion from cascading away history.
- Messages, finalized notes, addenda, archived channels, and note tombstones have
  no application delete path.
- Audit entries are pruned on a **10-year** retention window by a background
  task. That is the real, observable limit. Legal holds and an
  organization-approved retention policy do not exist yet.

---

## Phase 2: the CRM

*Requirements: `REQ-CRM-001` – `REQ-CRM-043`. See
[ADR-0005](adr/ADR-0005-contacts-are-not-users.md).*

### Contacts — people, with or without a login

The change that makes this a CRM: a `users` row is an **account**; a `contacts`
row is a **person**. Most contacts have no account at all, which is what lets the
organization record donors, funder program officers, partner caseworkers,
attorneys, court clerks, board members, and emergency contacts.

- A contact captures names, a preferred name, email, phone, mobile, address, job
  title, an optional organization, one or more contact types, a source, notes, a
  do-not-contact flag, and an active/archived state.
- A contact may link to at most one user, enforced by a partial unique index.
  Account-owned fields and link controls are visible only to operations/site
  admins and remain owned by `users`. Unlinking leaves the account able to sign
  in exactly as before; deleting an account nulls the link instead of erasing the
  person.
- The directory searches name, email, phone, and organization, and filters by
  type, organization, and archived state — all evaluated in SQL with server-side
  pagination.
- A contact referenced by a case, grant, or funding record cannot be deleted
  (`ON DELETE RESTRICT`). Archiving removes it from pickers while every existing
  reference keeps rendering.
- A database check requires a last name or an organization, so no nameless row
  can exist.
- Migration 0019 backfilled one contact per existing account, typed by role, so
  the directory was populated on first run. The seed does the same, because on a
  fresh database the migration runs before any user exists.
- The link is visible and navigable from both ends: a person's page shows their
  sign-in account with a link to the profile, and an admin viewing a profile sees
  a "Person record" panel linking back. Linking offers only accounts that have no
  person record yet, so it cannot create a duplicate or steal another contact's
  account.

### Contact properties — custom fields on a person

- Ordered key/value rows grouped under a free-text section heading, rewritten in
  place — the same shape `case_properties` already uses for cases.
- A blank value is meaningful: it is a field that has been named but not yet
  filled in.
- Property edits are audited against the contact.
- Every person begins with six low-sensitivity defaults for communication and
  relationship follow-up. Administrators can idempotently add missing defaults
  without replacing existing values or order.
- **No visibility axis, deliberately.** On a case, `shared` vs `volunteer_only`
  answers "may the client see this?"; clients cannot see contacts at all, so
  reusing that vocabulary would give it a second, weaker meaning.

### Organizations

- Funders, partner agencies, service providers, government bodies, courts, and
  employers, each with type, contact details, notes, and an archived state.
- Active names are unique case-insensitively; archived rows are exempt, so a name
  can be reused after an organization is retired.
- An organization's page lists the people filed under it, related grants,
  sectioned custom properties, and its Change Log.
- Administrators can create a person with the organization preselected, file an
  existing person there, explicitly move them from another organization, or
  remove the link when the person still has a displayable surname.
- `contacts.organization_id` remains the single source of truth for the person's
  current filing organization; no parallel relationship table is maintained.
- Every organization begins with four low-sensitivity relationship defaults.
  Only administrators may create, edit, link, archive, or change CRM properties.

### Case contacts — who is involved in a case

- Any number of contacts per case, each with a role (client, household member,
  emergency contact, attorney, opposing party, caseworker, provider contact,
  court professional, other) and an optional note.
- At most one primary contact per case, enforced by a partial unique index;
  promoting a new primary demotes the old one in the same transaction.
- The same person cannot hold the same role on a case twice.
- Reading requires `ViewCase`; editing requires `EditCase`. Removing a link
  resolves its case from the stored row, so the capability is always checked
  against the case the link actually belongs to.
- Clients never see the panel or the data. Audit entries are `volunteer_only` and
  record the role only — never the note text.
- The same links are shown on person detail with stable admin case links.
  Add/edit/remove remains authorized by the target case's stored `EditCase`
  capability, even when the workflow starts from a person or an admin read view.
- New verified client accounts receive a linked person record and defaults; a
  case signup also links that person to the new case as its primary Client.
- Active person and editable-case pickers use bounded, debounced server-side
  searches rather than downloading an arbitrary first page.

### Grants

The two-column `grants` stub from `0001_init.sql` is now a real award record,
extended in place so existing rows survived.

- Funder organization, program-officer contact, status, amounts requested and
  awarded, application and decision dates, period, purpose, reporting cadence,
  and notes.
- Statuses: `Prospect`, `Applied`, `Awarded`, `Active`, `Reporting`, `Closed`,
  `Declined`.
- **Database-enforced:** a grant that is awarded, active, reporting, or closed
  must have an awarded amount and a start and end date; a closed or declined
  grant must have a decision date; a period cannot end before it starts.
- Money is stored as integer minor units (cents) throughout — never floating
  point, which cannot represent a currency amount exactly.

### Funding

- Money received, as a grant payment, donation, in-kind gift, or other, with an
  amount, received date, optional grant, optional source organization or person,
  a reference, and notes.
- **Database-enforced:** a grant payment must name its grant; a donation must
  name a source organization or contact; an amount must be positive.
- A mistaken record is **voided with a reason**, never deleted. A trigger rejects
  `DELETE` outright, and a check requires a reason and timestamp whenever
  `voided` is set. Voided rows stay visible on the ledger and drop out of every
  rollup.
- Per-grant rollups (received, remaining, over-funded) and portfolio totals are
  computed in SQL, so they cannot drift from the ledger.
- Grants and funding require the per-user information-management grant. Permitted volunteers and administrators can view and change them; clients are refused.

### Access summary

| Object | Client | Volunteer | Operations / site admin |
| --- | --- | --- | --- |
| Contacts, organizations | none | full management when granted; sign-in account linking excluded | full management when granted |
| Contact and organization properties | none | full management when granted | full management when granted |
| Case contacts | none | information grant plus `ViewCase` to read and `EditCase` to edit | same, plus admin case inspection |
| Grants, funding | none | full management when granted | full management when granted |

Every information server function rejects a client account.

Contacts, Organizations, and Funding are top-level navbar areas governed by one
site-admin-assigned information-management grant. Admin remains focused on Cases,
Users, and operational tools. Linking a contact to a sign-in account stays
operations-admin-only because it exposes private account records.

**Limits, stated on purpose:** no field-level sensitivity or break-glass access,
no email campaigns or donor receipts, no accounting or payment-processor
integration, no pledges or recurring gifts, no funder report generation, no
duplicate detection or contact merging, no multi-organization affiliation
history, no household/relationship graph, no bulk import or export, and no
client-facing visibility of any CRM record. The
`volunteers` emergency-contact columns still exist and were deliberately not
migrated into contact records.

### Production-readiness safeguards

*Requirements: `REQ-CRM-044`–`REQ-CRM-048`, `REQ-SEC-001`–`REQ-SEC-005`.*

- Standalone and grant-linked funding mutations have their own Change Log scope;
  full funding details include notes and void attribution.
- CRM collections expose totals/load-more or bounded server-side relationship
  search rather than presenting a fixed first page as complete.
- Linked account identity fields are projected only for operations/site admins
  and remain read-only on the CRM form; preserved disagreements are recorded for
  review.
- New CRM audit and funding writes record both stable actor account ID and the
  historical display-name snapshot.
- Registration and password reset share one password policy. Password reset
  consumes the token, changes the hash, and revokes sessions and trusted devices
  in one transaction.
- Production startup fails closed when MFA email or the public origin is
  unconfigured. Secure cookies, browser security headers, and same-origin
  mutation checks follow the same production-mode decision.
- Evidence is temporarily unavailable at every UI and server boundary. Its
  preserved upload path requires content validation and fail-closed malware
  scanning before storage; rejected/error outcomes are audited without storing
  those bytes.

**Operational limit:** these engineering safeguards do not approve retention,
legal-hold, backup ownership, incident policy, accessibility targets, or an
independent review. Production approval remains blocked on the release evidence
listed in [`../operations/production-readiness.md`](../operations/production-readiness.md).

---

## How the invariants are enforced

Authorization is checked in `src/server/permissions.rs` at the server-function
boundary, and the repository queries in `src/server/db/` filter again. The rules
that must not depend on application code at all live in the schema:

| Invariant | Enforced by |
| --- | --- |
| Finalized/legacy/discarded notes never change | `case_notes_prevent_terminal_update` trigger |
| Case notes are never deleted | `case_notes_prevent_delete` trigger |
| An addendum's parent is finalized or legacy, audiences match | `case_note_addenda_validate_parent` trigger |
| Addenda never change | `case_note_addenda_prevent_update` / `_delete` triggers |
| Channels are never deleted | `prevent_case_channel_delete` trigger |
| A message's case matches its channel's case | `messages_channel_case_fkey` |
| Funding is never deleted | `funding_prevent_delete` trigger |
| A void carries a reason and a time | `funding_void_reason_check` |
| An awarded grant has an amount and a period | `grants_awarded_requires_terms_check` |
| A decided grant has a decision date | `grants_decided_requires_date_check` |
| Every contact has at least one type | `contacts_types_nonempty_check` |
| One primary contact per case | `case_contacts_one_primary_idx` |
| One contact per account | `contacts_user_id_key` |
| A contact has a surname or an organization | `contacts_named_check` |
| Referenced contacts/organizations/grants cannot be deleted | `ON DELETE RESTRICT` foreign keys |
