# ADR-0001: Docs boundary and traceability

- Status: accepted
- Date: 2026-08-10

## Context

The approved task requires a maintained specification home under
`docs/case-management/` while the existing application already treats `.docx`
files under `docs/` as a separate resource library and current RAG input path.
If that boundary stays implicit, future contributors may assume that editing
Markdown here updates the client resource corpus or the assistant's grounded
sources.

The task also requires stable requirement IDs that can trace from requirements
through implementation comments and verification notes.

## Decision

- Keep all maintained case-management specs in Markdown under
  `docs/case-management/`.
- Treat those Markdown files as internal product/engineering documentation only.
- Preserve the `.docx` files under `docs/` as the separate client resource
  library and current RAG ingest source.
- Reuse the task's stable requirement IDs unchanged rather than renumbering the
  MVP requirements inside documentation.

## Consequences

- Product documentation can evolve without altering the client document library.
- RAG behavior does not silently change when Markdown specs change.
- Reviewers can trace requirements consistently across docs, code comments, and
  verification notes.
- A future decision is still needed if the organization wants Markdown specs to
  participate in RAG ingestion or public document publication.

## Alternatives considered

- Mix Markdown specs into the same directory and workflow as `.docx` files.
- Create a second set of documentation-specific requirement IDs.

## Requirement links

- `REQ-DOC-001`
- `REQ-DOC-004`
