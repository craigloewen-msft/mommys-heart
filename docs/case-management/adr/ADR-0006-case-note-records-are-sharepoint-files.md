# ADR-0006: A case note's document lives in SharePoint

- Status: accepted
- Date: 2026-08-31

## Context

A finalized case note existed only as rows in `case_notes`, readable only
through this application's own note page. Every other artefact a case
accumulates — intake paperwork, court orders, service agreements — is a file in
the case's SharePoint folder, and each case was already provisioned with an
empty `Case Notes` folder that nothing ever wrote to.

That left the organization's most formal internal record as the one thing staff
could not open, print, attach to a court bundle, or hand to anyone without the
app in front of them, and the only case artefact filed by a mechanism unique to
this system.

The tension is that the row is not merely a convenience. ADR-0003 makes a
finalized note immutable by database trigger, undeletable, audited in a
restricted log, and correctable only by appending a signed addendum. None of
that survives a move to a file store.

## Decision

Split the two roles the record was playing, and give each one owner.

- The **record** stays the `case_notes` row, unchanged: same lifecycle, same
  triggers, same audit log, same addenda. It is what the app filters, searches
  and authorizes against.
- The **document** — what a person reads — is a `.docx` written into the case's
  `Case Notes` folder when the note is finalized. The library owns it. The
  application keeps no copy of its bytes, no mirror of the library's listing,
  and serves it through no path of its own: it is browsed, downloaded and opened
  through exactly the same case-documents surface as every other case file.

Postgres keeps one pointer row per note (`case_note_documents`: which file, its
link, and how many addenda it already contains) for the same reason a case keeps
its folder id — so the app can find the file again without searching the library
by name. It is a separate table because a finalized note row cannot be updated
at all.

Three further choices follow from making the library authoritative:

- **Filing runs after the commit, never inside it.** A note is immutable the
  moment its transaction lands, and a SharePoint outage must not be able to fail
  that write. Missed filings are repaired by a startup pass that recomputes what
  is missing or stale, not by a queue — the same reconcile-don't-remember shape
  as `sync_case_access`.
- **An addendum re-writes the same file.** The filed document is always the
  whole record including its corrections, rather than a first document plus a
  scattering of separate amendment files.
- **The app refuses to delete a filed record** from the library, mirroring the
  note's own absence of a delete path.

`.docx` was chosen as the format because the crates to write it (`zip`,
`quick-xml`) are already SSR dependencies, SharePoint previews and indexes it
natively, and the organization's existing case paperwork is Word.

## Consequences

- A case's notes are readable by anyone with access to its SharePoint folder,
  including offline and after this application is gone. That is the point.
- The `Case Notes` folder had to become **volunteer-only**. It was `Shared`,
  which invited the client to it; structured notes are staff-only records, and
  audience is decided by the top-level folder, so leaving it shared would have
  handed clients the team's working record. On the next reconcile, existing
  client invitations to that folder are withdrawn.
- Two representations of one note now exist, so they can disagree. Staleness is
  made *computable* rather than assumed: the pointer records how many addenda
  the document contains, the note page says plainly when a re-file is pending,
  and the startup pass repairs it. The row is authoritative whenever they differ.
- Filing is best-effort and asynchronous, so a document can lag its note by
  seconds — or by an outage. This is the deliberate cost of never letting the
  library fail a legal record.
- The document is generated from the record, so it is a faithful copy and never
  a second place to edit. There is no path by which changing the file changes
  the note.

## Alternatives considered

- **Move note content out of Postgres entirely, making the file the only
  record.** Rejected: it would delete the trigger-enforced immutability, the
  restricted audit trail, the addendum model and all note search and filtering.
  "SharePoint is the source of truth" is right about the *document* and wrong
  about the *record*.
- **PDF instead of `.docx`.** Rejected: a new dependency and font handling for
  one feature, worse SharePoint integration, and no gain for a text document.
- **Filing inside the finalization transaction.** Rejected: it would let a
  SharePoint outage fail the creation of an immutable legal record.
- **A second, volunteer-only folder for note records, leaving `Case Notes`
  shared.** Rejected: a case would then have two folders whose names both
  promise case notes, and the existing empty folder would stay empty.

## Requirement links

- `REQ-CN-001` – `REQ-CN-011` (structured Case Notes)
- `REQ-AUD-001` – `REQ-AUD-004` (audit and retention)
