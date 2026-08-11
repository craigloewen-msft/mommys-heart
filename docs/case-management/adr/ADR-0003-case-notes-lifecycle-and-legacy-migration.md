# ADR-0003: Case Notes lifecycle and legacy migration

- Status: accepted
- Date: 2026-08-10

## Context

The MVP must add structured internal Case Notes while preserving historical
free-text notes and their audience. It also must distinguish informal chat,
generic audit records, and formal service documentation. The main design tension
is between useful iterative drafting and preserving an authoritative immutable
record once a note is finalized.

## Decision

- Use explicit note states for `draft`, `finalized`, `discarded`, and `legacy`.
- Let only the author edit or finalize a draft; administrators may inspect draft
  content through a limited path but may not rewrite it.
- Treat finalization as terminal and require authenticated in-app attestation.
- Preserve migrated historical notes as immutable legacy records with their
  historical audience unchanged.
- Allow later corrections only through immutable signed addenda attached to the
  parent note.

## Consequences

- Staff can save incomplete work without weakening the integrity of finalized
  records.
- Historical notes survive migration without silent audience revocation or
  widening.
- The MVP remains intentionally smaller than the full 19-section dynamic form and
  does not yet include supervisor approval or returned revisions.
- Search, filters, and exports must treat drafts and discarded tombstones
  differently from finalized and legacy records.

## Alternatives considered

- Reuse the existing free-text note model and add more fields incrementally.
- Allow supervisors or administrators to directly edit another author's draft.
- Permit finalized-note edits with visible version history.

## Requirement links

- `REQ-CN-001`
- `REQ-CN-002`
- `REQ-CN-003`
- `REQ-CN-004`
- `REQ-CN-005`
- `REQ-CN-006`
- `REQ-CN-007`
- `REQ-CN-008`
- `REQ-CN-009`
- `REQ-CN-010`
- `REQ-DOC-003`
- `REQ-CN-011`
