# Secure messaging executive status

This file is a point-in-time status summary for messaging scope. It is not the
normative source of behavior.

- **Status:** core messaging, unread/first-read handling, archival, content-free notifications, and audited CSV export are implemented; final regression verification is tracked with this task.
- **Primary requirements:** `REQ-MSG-001` through `REQ-MSG-009`.
- **Cross-cutting links:** `REQ-AUD-001` through `REQ-AUD-004`, `REQ-DOC-001`,
  `REQ-DOC-002`, `REQ-DOC-003`, and `REQ-DOC-004`.

## MVP promise

Deliver authenticated in-app case messaging with server-enforced audience
isolation, immutable messages, channel archival, content-free notifications,
authorized CSV export, and durable first-read evidence.

## Current limits called out on purpose

- No end-to-end encryption claim.
- No attachments, SMS, email reply ingestion, reactions, or message editing.
- No promise of emergency delivery or escalation.
- No PDF transcript output.
- No certification or independent compliance attestation implied by these docs.

## Dependencies

- Per-case capability enforcement
- Existing shared + volunteer-only channel model
- Transactional DB writes
- Restricted audit metadata and retention honesty
- Content-free notification templates

## Open policy questions

1. Who can archive shared channels?
2. Should transcript export be further limited by case status?
3. Are there organization rules for retaining exported CSV files outside the app?

## Acceptance outcomes to verify later

- Shared-channel messaging works end to end for assigned participants.
- Volunteer-only metadata never reaches clients.
- Send, recipient state, and audit metadata commit atomically.
- First-read evidence is write-once.
- Notification copy is content-free.
- Export authorization and export auditing both succeed.
