# What has been delivered

This file describes what the application **does today**, by observable behavior.
For what is *not* built, see [`roadmap.md`](roadmap.md).

Each section lists the invariants that are actually enforced and, deliberately,
the limits that genuinely apply. Where an invariant is enforced by the database
rather than only by application code, that is stated — those survive a bug in the
server layer.

---

## The case lifecycle

A case moves through `Pending review` → `Open` / `Monitor` / `Closed`, or is
`Declined` at review. A case can also be **withdrawn** by the person who filed
it.

- **Withdrawn, never deleted.** A case row cannot be deleted at all: it cascades
  to `case_channels` and `case_notes`, both of which reject `DELETE` outright.
  Withdrawal is therefore a status, following the same archive-not-delete
  reasoning as ADR-0002.
- The **owner** may withdraw a case from any state except `Declined` (already
  finished) and `Withdrawn` (already done). Operations admins may withdraw on an
  owner's behalf; the audit entry names whoever actually did it.
- Withdrawal is gated on **ownership**, not a case capability. The public signup
  flow grants the client owner every capability, so capabilities cannot
  distinguish "this is my case" from "I may edit this case".
- A withdrawn case is **frozen**: `accepts_changes()` is false, so the existing
  capability gate rejects every write — edits, notes, files, and chat — exactly
  as it does for a declined case.
- A withdrawn case **disappears from the Cases list and the Inbox** for everyone
  who is not an operations admin. It stays readable by direct link and stays in
  the admin case directory, which is how an admin finds one to restore. It is
  also excluded from the case picker used to link contacts.
- **Only an operations admin can restore it**, and a restore returns the case to
  exactly the status it held before, from `status_before_withdrawal`.
- Case assignments survive withdrawal, so a restore is lossless.
- **Database-enforced:** a row with status `withdrawn` must carry both a
  withdrawal timestamp and the status to restore it to
  (`cases_withdrawal_complete`), so the state can never become unexplainable or
  unrecoverable.

### Case access

- What a user may do on a case is the set of `CaseCapability`s stored for them
  in `case_assignments`. Owning a case grants nothing by itself.
- **Site admins are the one exception:** they hold every capability on every
  case, with no assignment and nothing to grant. There is no separate read-only
  inspection mode — they get the ordinary case view, fully editable.
- **Operations admins hold only their stored capabilities.** They can grant
  themselves access to any case from the user directory without approval, and
  that grant is audited; the case directory shows "No access" for cases they hold
  nothing on.
- A declined or withdrawn case still refuses **every** write, from everyone
  including a site admin. Capability and lifecycle are checked separately.
- Case lists stay assignment-scoped: the Cases list and Case Chat show the cases
  a user is actually assigned to, so full access does not mean every case in the
  system appears in a site admin's own lists or unread counts.

---

## Account status

An account can be **deactivated** by a site admin — the way duplicate accounts get
cleaned up without destroying anything.

- **A role, not a deletion.** Deactivating sets the account role to
  `deactivated`. A `users` row cannot be deleted at all: `cases.owner_id` is
  `ON DELETE RESTRICT`, and audit, contact, and funding rows reference it. This
  follows the same archive-not-delete reasoning as case withdrawal and ADR-0002.
- **Site admins only.** Operations admins see the status read-only and cannot
  request the change.
- **It stops being a login.** Existing sessions and remembered devices are
  dropped in the same transaction, so a browser already signed in as that account
  is logged out on its next request. A sign-in attempt is refused *after* the
  password is checked, so the message is not an oracle for which addresses exist.
- **It disappears from the working lists**: the volunteer list, the Volunteers,
  Clients, and Other directory sections, the volunteer-application queue, the
  case-owner picker, the case-access picker, and the contact account-linking
  picker. It receives no notification emails and no unread badges.
- **It is still findable**, in its own "Deactivated accounts" section of Manage
  users, which is how an admin finds one to restore.
- **Nothing else changes.** The role held before, the case assignments, the
  volunteer agreement and its version, the information-management grant, the
  linked contact, and the audit history are all kept, so reactivating restores
  exactly the previous state.
- **The email address stays claimed**, so nobody re-registers over a retired
  duplicate by accident.
- **You cannot deactivate your own account**, which is also what makes it
  impossible to retire the last site admin: the actor must be a site admin and
  cannot be the target, so an active site admin always remains. Separately, a
  deactivated ex-admin does not count toward the "final site admin" check that
  guards ordinary demotions.
- Deactivation does **not** withdraw the account's cases. A case with staff work
  on it belongs to the organization, not to the account that filed it.
- Both transitions are audited, so the user's Change Log names who did it and when.

**Limits, stated on purpose:** there is no operations-admin request path for
deactivation, and no merging of two accounts' records — deactivating the
duplicate leaves the good account untouched rather than moving anything onto it.

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
- **Every finalized note is also filed as a Word document** in its case's
  `Case Notes` folder in the document library, so a note sits with the rest of
  that case's paperwork and can be opened, printed, or handed over without the
  app. The library holds that document; the database keeps only a pointer to it
  (`case_note_documents`) and no copy of its bytes. It is browsed, downloaded
  and opened through the ordinary case-documents surface, and the note page
  links to it under "Filed record".
- Filing happens **after** the note commits, so an unreachable library can never
  fail a finalization; a startup pass re-files whatever is missing or out of
  date. An addendum re-writes the same file, so the filed document is always the
  whole record including its corrections — and that file cannot be deleted
  through the app, matching the note's own absence of a delete path.
- The `Case Notes` folder is **volunteer-only**, because that is where those
  documents land and structured notes are staff-only records.

**Limits, stated on purpose:** no full 19-section dynamic form, no supervisor
approval or returned revisions, no automated safety escalation, no attachments,
no PDF output (the filed record is `.docx`), no field-level sensitivity controls,
no cross-case note search.

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

### Admin activity alerts

A notification category, **Admin activity alert**, that answers "what has been
happening on the site?" for administrators.

- It is a **view over the audit trail, not a second record of it**. Every event
  shown is a row the Change Log already wrote, plus finalized notes and addenda
  projected from the restricted note audit table. Nothing writes an event twice,
  so the feed cannot drift from the audit history.
- Covers case creation (including client signups), finalized case notes and
  addenda, case information and intake property edits, documents and folders,
  and contact, case-contact and organization changes. Account administration
  (role changes, capability grants, information-access decisions) and chat are
  deliberately excluded: they have their own notification categories.
- Offered and delivered to **site and operations administrators only**. The
  toggle does not appear on a volunteer's or client's Settings page, and the feed
  route is admin-gated on the server.
- Two surfaces: a paginated, category-filterable feed under **Admin → Activity**,
  and a **daily** roll-up email to administrators who have the category enabled.
- The digest tracks what it has already sent with a **watermark** (the last
  sequence mailed from each log), so it never marks or mutates an append-only
  audit table. When email is unconfigured the watermark holds still and nothing
  is lost.
- Events are **content-free**: who acted, what kind of thing they did, and which
  record. No note, message, or document content enters the feed or the email.
- Subject names are resolved when read, so a renamed case shows its current name
  and a deleted one falls back to its id.
- Retention is the audit log's own **10-year** window; the feed adds no separate
  retention because it stores no events.

Creating a case writes a `case / created` audit entry, so a case's Change Log
begins with its creation.

**Limits, stated on purpose:** no per-admin unread badge or read state, no
per-category subscription (the toggle is all-or-nothing), no digest for
non-administrators, a single request scans a bounded window of recent history,
and no first-class "Zoom interview link" record — a videoconference is covered as
a document filed in the case's Zoom Video folder or as a case note with that
interaction type.

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
- Every account has a person record, typed by role: registration creates one,
  and the seed does the same for the fixture accounts.
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

### Filtering by property — faceted search over custom fields

- The Contacts directory, the Organizations directory, and the send-mail
  recipient picker share one filter bar: pick a property name, tick the values
  actually recorded under it, and the choice becomes a removable chip.
- Values inside one property are OR; different properties are AND.
- Counts describe the records currently listed, with a facet's own chip excluded
  from its own counts, so a value that would return nothing is never offered.
- Keys and values are matched on their trimmed, lowercased form, so "Location"
  and "location" are one facet. Nothing anyone typed is rewritten; the most
  common spelling is shown.
- A blank value is a selectable facet ("not filled in") and is distinct from not
  having the property at all.
- The plain keyword box also matches property *values*, so a search for a place
  name finds it without building a filter first. Property *names* are not
  keyword-matched, since that would return every record carrying the field.
- On Contacts and Organizations the active filters live in the query string, so a
  filtered view can be linked to and reopened. The mail picker deliberately does
  not, since a half-composed campaign is not a shareable view — but its filters
  are part of the snapshot "send to all matching" re-resolves at send time.
- Facet reads carry the same guard as the property panels: information-management
  access plus a staff account, since a list of values is contact data.

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
  capability (which a site admin always holds), even when the workflow starts
  from a person rather than the case.
- New verified client accounts receive a linked person record and defaults; a
  case signup also links that person to the new case as its primary Client.
- Active person and editable-case pickers use bounded, debounced server-side
  searches rather than downloading an arbitrary first page.

### Grants

A grant is an award record:

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
| Case contacts | none | information grant plus `ViewCase` to read and `EditCase` to edit | same; a site admin holds both on every case |
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
- Case documents live in a SharePoint document library, reached app-only through
  Microsoft Graph. Each case gets a folder there; sharing invitations are issued
  per top-level folder so a client is never invited to volunteer-only paperwork,
  and one reconcile pass keeps those invitations matching `case_assignments`.
  Uploads are content-sniffed against a type allowlist and capped at 25 MB, and
  every folder request is addressed by a validated case-relative path rather than
  a library id. Finalized case notes are filed into each case's volunteer-only
  `Case Notes` folder and are the one thing in the library the app refuses to
  delete. Malware scanning is the tenant's (Microsoft 365 handles it), not
  this application's.

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
| A withdrawn case can always be explained and restored | `cases_withdrawal_complete` check |
| A stored deactivation names who did it and the role to restore | `account_deactivations_complete` check |
