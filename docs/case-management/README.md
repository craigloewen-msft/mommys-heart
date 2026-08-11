# Case management documentation

This directory is the maintained product and engineering specification home for the
case-management MVP described in `tasks/00024-p1-in-progress--case-management-mvp.md`.
It defines what the MVP is, what is deferred, and what later work still needs
policy or operational decisions.

## Purpose

- keep timeless requirements separate from point-in-time delivery status;
- give implementation and review work stable requirement IDs;
- capture material architecture decisions in one ADR chain;
- map the client's broader request into an honest roadmap; and
- isolate maintained Markdown specs from the existing `.docx` resource library.

## Scope

The MVP documentation in this folder covers two delivered foundations:

1. secure in-app case messaging; and
2. structured internal Case Notes.

It also captures deferred work as roadmap items and future briefs so the project
can grow without pretending that unsupported behavior already exists.

## Folder map

- `glossary.md` — shared terms used across the MVP docs.
- `roadmap.md` — roadmap coverage for the 19 client-request sections plus
  submission/search outcomes.
- `adr/` — architecture decision record index, template, and accepted decisions.
- `secure-messaging/` — normative requirements, lifecycle spec, and executive status.
- `case-notes/` — normative requirements, lifecycle spec, and executive status.
- `future-features/` — seven concise briefs required by REQ-DOC-003.

## Timeless requirements vs. status

- `requirements.md` and `.allium` files are normative. They describe intended
  behavior and invariants without claiming that implementation is complete.
- `executive.md` files are status snapshots. They summarize current scope,
  dependencies, open questions, and what still needs verification.
- ADRs record decisions and trade-offs at the time they were made. They are not
  release notes.

## Requirement traceability

The stable IDs in this documentation intentionally match the approved task:

- `REQ-MSG-001` through `REQ-MSG-009`
- `REQ-CN-001` through `REQ-CN-011`
- `REQ-AUD-001` through `REQ-AUD-004`
- `REQ-DOC-001` through `REQ-DOC-004`

Implementation comments and verification notes should reference these IDs rather
than inventing new identifiers for the same requirement.

## Documentation and RAG isolation

This folder is not part of the client-facing `.docx` document corpus.

- The existing resource library remains the `.docx` files stored directly under
  `docs/`.
- The current RAG/document-ingest path parses `.docx` files from the configured
  docs directory and does not ingest this Markdown folder.
- Updating Markdown here changes maintained internal product documentation only.
  It does not update any `.docx` file, and it does not automatically change the
  RAG assistant's grounded source library.

That boundary is deliberate and satisfies `REQ-DOC-004`.

## Reading order

1. Read `glossary.md`.
2. Read `roadmap.md` for MVP vs. deferred scope.
3. Read `secure-messaging/requirements.md` and `case-notes/requirements.md`.
4. Use the `.allium` files for lifecycle, invariants, and verification planning.
5. Read the relevant `executive.md` file for present status and open work.
6. Read the future briefs before starting deferred feature design.
