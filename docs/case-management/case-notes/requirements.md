# Case Notes requirements

This file is normative. It defines timeless Case Note and Case Note-adjacent
requirements for the case-management MVP.

## Scope

The MVP adds structured internal Case Notes with drafts, validation,
finalization, authenticated in-app attestation, immutable addenda, legacy note
preservation, filtering, pagination, and restricted audit metadata.

## Non-goals

The MVP does not provide the full 19-section dynamic note form, supervisor
review/approval, returned revisions, automated urgent-safety escalation,
attachments, PDF generation, field-level sensitivity controls, or global
cross-client note search.

## Requirements

- **REQ-CN-001 — Internal notes and preserved legacy audience.** New structured
  Case Notes shall be visible only to non-client users with `ViewCase`. Existing
  free-text notes shall migrate to an immutable `Legacy` state while preserving
  their existing shared audience so the migration does not silently revoke or
  widen historical access. Clients shall receive neither structured-note content
  nor metadata/counts indicating that internal notes exist.
- **REQ-CN-002 — Authoritative identity.** The server shall populate the author
  user ID, display-name snapshot, account-role snapshot, case ID, creation time,
  update time, and finalization time. The browser shall not be trusted to supply
  author, role, case status, or timestamps.
- **REQ-CN-003 — Draft lifecycle.** A non-client user with `AddNotes` may create
  and explicitly save an incomplete draft and return later. Only the author may
  edit or finalize that draft. Other assigned team members shall not see it until
  finalization; an operations/site administrator with `ViewCase` may inspect it
  but may not rewrite it. Discarding a draft shall preserve a restricted,
  auditable tombstone rather than hard-delete the row.
- **REQ-CN-004 — Bounded structured form.** The MVP form shall capture:
  - activity date, start time, end time, calculated total minutes, optional
    location, and delayed-entry reason when the activity date is not today;
  - one primary interaction/activity type from the client's supplied list;
  - contact category, direction, completion outcome, and participant summary;
  - one or more related service areas;
  - purpose/objective;
  - client-reported information and separately verified/observed information,
    with information-source selections;
  - actions taken;
  - client response/outcome and progress/barriers;
  - safety/urgency level and details when above routine;
  - next steps; and
  - a concise factual narrative.

  Fields may offer `Unknown`, `Client declined`, and `Not applicable` where
  relevant. Repeatable participant records, multiple primary activities, and all
  19 original sections are deferred.
- **REQ-CN-005 — Finalization validation.** Drafts may be incomplete. Before
  finalization the server shall require valid non-future activity timing, an
  interaction type, contact category, at least one service area, purpose,
  actions, outcome, urgency, next steps or an explicit not-applicable choice,
  and narrative. End time must follow start time for the MVP's same-day activity
  model. Elevated urgency requires details. Delayed entry requires an
  explanation.
- **REQ-CN-006 — Electronic attestation.** To finalize, the author shall confirm
  review/accuracy and type their current full name. The server shall verify that
  name against the authenticated user and store the name and server signing time.
  This is an authenticated in-app attestation, not a qualified digital
  signature.
- **REQ-CN-007 — Immutable final record.** Finalization is terminal. A finalized
  or legacy note shall not be edited, reverted to draft, discarded, or
  permanently deleted through the application.
- **REQ-CN-008 — Signed addenda.** A non-client user with `AddNotes` may append a
  signed addendum to a finalized or legacy note. An addendum shall record its
  own author identity/role snapshots, reason, supplemental or corrected
  information, affected categories, follow-up, signature, and server timestamp.
  It inherits the original note's audience and is immutable. The original
  remains visible.
- **REQ-CN-009 — Case-level list and filters.** Authorized users shall see notes
  for one selected case, newest activity first, with server-side pagination and
  filters for date range, author, primary interaction, note state, urgency, and
  keyword. Draft and discarded records follow REQ-CN-003 visibility. Global
  cross-client search is deferred.
- **REQ-CN-010 — Restricted auditing.** Creating, saving, discarding, finalizing,
  viewing through an administrative draft-inspection path, exporting, and adding
  an addendum shall create content-free restricted audit metadata. Note bodies
  and client-reported facts shall not be copied into the generic Change Log.
- **REQ-CN-011 — Safety warning.** The form shall clearly state that recording an
  urgent or emergency concern does not contact emergency services or replace the
  organization's escalation procedure. Automated safety escalation is deferred
  and must not be implied by the UI.

## Dependencies

- Existing account authentication and per-case capability enforcement.
- New structured note lifecycle storage with server-authoritative timestamps.
- Restricted audit metadata and audience-aware Change Log filtering.
- Migration support for legacy free-text notes.
- Dedicated note create/detail routes and per-case list/filter UI.

## Open policy questions

1. Which interaction/activity list from the client packet is authoritative for
   the MVP form choices?
2. Which users, if any, may export Case Notes in a later phase?
3. How should administrative draft inspection be surfaced without normalizing it
   into day-to-day workflow?
4. Which urgent-safety categories would later justify workflow escalation rather
   than only documentation?

## Acceptance outcomes

- Authorized staff can save an incomplete draft and continue it later.
- Only the author may edit or finalize a draft; admins may inspect but not edit.
- Finalized and legacy notes remain immutable and accept only immutable addenda.
- Clients cannot discover Case Note existence through content or metadata.
- Case-level filters and pagination work within the authorized case boundary.
