# Case Notes executive status

This file is a point-in-time status summary for Case Notes scope. It is not the
normative source of behavior.

- **Status:** core drafting, atomic finalization, discard tombstones, addenda, legacy preservation, and case-level filters are implemented; final regression verification is tracked with this task.
- **Primary requirements:** `REQ-CN-001` through `REQ-CN-011`.
- **Cross-cutting links:** `REQ-AUD-001` through `REQ-AUD-004`, `REQ-DOC-001`,
  `REQ-DOC-002`, `REQ-DOC-003`, and `REQ-DOC-004`.

## MVP promise

Deliver a bounded internal Case Note record with draft save, discard tombstone,
server-side validation, authenticated in-app attestation, immutable finalization,
legacy-note preservation, immutable addenda, and per-case filtering/pagination.

## Current limits called out on purpose

- No full 19-section dynamic note form.
- No supervisor approval, returned revisions, or review queue.
- No automated urgent-safety escalation.
- No attachments, PDF generation, or stronger cryptographic signature claims.
- No granular field-level sensitivity controls.
- No global cross-client search.

## Dependencies

- Per-case capability enforcement
- Dedicated note storage and lifecycle transactions
- Legacy note migration path
- Restricted audit metadata and retention honesty
- Note list/filter UI and long-form create/detail route

## Open policy questions

1. What exact MVP picklists from the client packet are required at launch?
2. Should any administrative role be able to export or print notes later?
3. What audit burden is required for draft inspection by administrators?
4. What non-app escalation process should the UI warn users to follow?

## Acceptance outcomes to verify later

- Draft save/resume/discard behavior works as documented.
- Server validation rejects incomplete or invalid finalization attempts.
- Finalized and legacy notes stay immutable.
- Signed addenda attach beneath the parent note without replacing it.
- Clients do not observe Case Note existence through UI, counts, search, or
  Change Log metadata.
