# ADR index

| ADR | Status | Title | Summary |
| --- | --- | --- | --- |
| [ADR-0000](template.md) | template | ADR template | Reusable structure for future ADRs. |
| [ADR-0001](ADR-0001-docs-boundary-and-traceability.md) | partly superseded by [ADR-0006](ADR-0006-documentation-lifecycle.md) | Docs boundary and traceability | Keep Markdown docs separate from the `.docx`/RAG corpus, and align stable requirement IDs with the approved task. The `.docx`/RAG boundary still holds; the requirements-first file layout does not. |
| [ADR-0002](ADR-0002-messaging-audience-and-immutability.md) | accepted | Messaging audience and immutability | Preserve audience isolation with immutable messages, archival instead of delete, and restricted audit records. |
| [ADR-0003](ADR-0003-case-notes-lifecycle-and-legacy-migration.md) | accepted | Case Notes lifecycle and legacy migration | Use draft/finalized/legacy/discarded states, server-authoritative identity, immutable addenda, and preserved historical audience. |
| [ADR-0004](ADR-0004-audit-and-retention-honesty.md) | accepted | Audit and retention honesty | Make sensitive audit metadata content-free, audience-aware, and honest about the current 10-year retention window and missing legal-hold controls. |
| [ADR-0005](ADR-0005-contacts-are-not-users.md) | accepted | Contacts are not users | A `users` row is an account; a `contacts` row is a person. At most one optional link between them, with `users` remaining the source of truth for identity and authentication. |
| [ADR-0006](ADR-0006-documentation-lifecycle.md) | accepted | Documentation lifecycle | Replace per-feature requirements/Allium/executive files with a delivered-versus-roadmap split now that the work has shipped. |
