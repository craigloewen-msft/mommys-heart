# ADR-0006: Documentation lifecycle — from requirements-first to delivered/roadmap

- Status: accepted
- Date: 2026-08-11
- Supersedes: the documentation-structure half of ADR-0001

## Context

ADR-0001 and `REQ-DOC-001` established a requirements-first documentation home:
per-feature `requirements.md`, `.allium` lifecycle specs, and `executive.md`
status files, plus future-feature briefs. That structure was correct while the
case-management MVP was being specified — it gave the work stable requirement IDs
and kept timeless requirements separate from delivery status before any of it
existed.

The MVP has since shipped, and phase 2 has added the CRM. The requirements files
now describe behavior that is implemented and verifiable, so two documents
describe the same thing: `requirements.md` says what the system shall do, and
`executive.md` says how much of it is done. Readers have to reconcile them to
answer the only two questions that matter day to day: what does this system do
today, and what is left?

Keeping the `.allium` and `requirements.md` files as well would mean maintaining
three descriptions of shipped behavior, which drift apart the moment one is
edited without the others.

## Decision

- Replace the per-feature spec scaffolding with two documents:
  - `delivered.md` — what has been built, described by observable behavior, with
    the invariants that are actually enforced and the limits that genuinely
    apply.
  - `roadmap.md` — what is not built, in ordered phases, separating
    straightforward engineering from work blocked on organizational policy.
- Delete `secure-messaging/`, `case-notes/`, and `future-features/`, including
  the `.allium` files, once their durable content has landed in those two.
- Keep `adr/` and `glossary.md`. ADRs are dated records of decisions and their
  trade-offs, not descriptions of current behavior, so they do not go stale in
  the same way. The glossary is the shared vocabulary both documents rely on.
- Keep the requirement IDs (`REQ-MSG`, `REQ-CN`, `REQ-AUD`, `REQ-DOC`,
  `REQ-CRM`). They are referenced from code comments and from the task files,
  and `delivered.md` continues to cite them.
- Requirements for *future* work still get written — in the task file for that
  work, which is where they are actually used, rather than in a parallel tree.

## Consequences

- Two questions, two documents: nobody has to cross-reference a requirements file
  against a status file to learn what the system does.
- Requirement IDs survive, so existing code comments and traceability still
  resolve.
- The Allium lifecycle specs are lost as artifacts. Their content — the states,
  transitions, and invariants — is preserved in `delivered.md` prose and, more
  durably, in the database triggers and `CHECK` constraints that enforce it.
- Future feature work must state its own requirements in its task file. That is
  already the repository's habit.
- `REQ-DOC-001`'s specific file layout no longer holds. `REQ-DOC-004`
  (documentation isolation from the `.docx`/RAG corpus) is unaffected and
  remains true.

## Alternatives considered

- Keep everything and accept the duplication. Rejected: three descriptions of
  shipped behavior is how documentation starts lying.
- Delete the docs folder entirely and rely on code. Rejected: the roadmap and the
  policy-blocked list are genuinely useful and exist nowhere else.
- Silently delete the spec files without an ADR. Rejected: it contradicts an
  accepted decision, so it should be recorded as one.

## Requirement links

- `REQ-DOC-001` (superseded in part)
- `REQ-DOC-002`
- `REQ-DOC-003`
- `REQ-DOC-004`
