# ADR index

| ADR | Status | Title | Summary |
| --- | --- | --- | --- |
| [ADR-0000](template.md) | template | ADR template | Reusable structure for future ADRs. |
| [ADR-0001](ADR-0001-docs-boundary-and-traceability.md) | accepted | Docs boundary and traceability | Keep Markdown docs separate from the `.docx`/RAG corpus, and align stable requirement IDs across docs and code. |
| [ADR-0002](ADR-0002-messaging-audience-and-immutability.md) | accepted | Messaging audience and immutability | Preserve audience isolation with immutable messages, archival instead of delete, and restricted audit records. |
| [ADR-0003](ADR-0003-case-notes-lifecycle-and-legacy-migration.md) | accepted | Case Notes lifecycle and legacy migration | Use draft/finalized/legacy/discarded states, server-authoritative identity, immutable addenda, and preserved historical audience. |
| [ADR-0004](ADR-0004-audit-and-retention-honesty.md) | accepted | Audit and retention honesty | Make sensitive audit metadata content-free, audience-aware, and honest about the current 10-year retention window and missing legal-hold controls. |
| [ADR-0005](ADR-0005-contacts-are-not-users.md) | accepted | Contacts are not users | A `users` row is an account; a `contacts` row is a person. At most one optional link between them, with `users` remaining the source of truth for identity and authentication. |
| [ADR-0006](ADR-0006-case-note-records-are-sharepoint-files.md) | accepted | A case note's document lives in SharePoint | The immutable row stays the record; the readable document is a `.docx` the library owns, filed after the commit and re-written when an addendum is added. |
