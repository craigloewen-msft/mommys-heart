# Product documentation

This directory is the maintained product and engineering documentation for the
case-management and CRM system.

## The two questions

- **[`delivered.md`](delivered.md)** — what the system does **today**, described
  by observable behavior, with the invariants that are actually enforced and the
  limits that genuinely apply.
- **[`roadmap.md`](roadmap.md)** — what is **not** built, in ordered phases, with
  policy-blocked work separated from straightforward engineering.

If those two disagree with the application, the application is right and these
files are a bug.

## Also here

- **[`glossary.md`](glossary.md)** — the shared vocabulary both documents use.
  Worth reading first; several words ("contact", "user", "visibility", "legacy")
  have precise meanings here.
- **[`adr/`](adr/)** — architecture decision records. Each is a dated account of
  a material choice, its alternatives, and its consequences. ADRs are history:
  they are not updated when the system changes, they are superseded.

## Why this shape

This folder previously held per-feature `requirements.md`, `.allium` lifecycle
specs, and `executive.md` status files. That was the right structure while the
MVP was being specified and nothing existed yet.

Once the work shipped, those files described the same shipped behavior three
times over, which is how documentation starts to drift and then to lie. The
reasoning behind consolidating them is recorded in
[ADR-0006](adr/ADR-0006-documentation-lifecycle.md).

Requirements for *future* work still get written — in the task file for that
work, under `tasks/`, where they are actually used.

## Requirement identifiers

Stable IDs are still referenced from code comments and task files:

- `REQ-MSG-001`–`009` — secure messaging
- `REQ-CN-001`–`011` — structured Case Notes
- `REQ-AUD-001`–`004` — audit visibility and retention
- `REQ-CRM-001`–`043` — connected people, organizations, cases, grants, and funding
- `REQ-DOC-001`–`004` — documentation (see ADR-0006 for what changed)

## Documentation and RAG isolation

This folder is **not** part of the client-facing document corpus.

- The resource library is the `.docx` files stored directly under `docs/`.
- The RAG/document-ingest path parses those `.docx` files from the configured
  docs directory. It does not ingest this Markdown.
- Editing anything here changes internal documentation only. It does not change
  any `.docx`, and it does not change the assistant's grounded sources.

That boundary is deliberate (`REQ-DOC-004`) and still holds.
