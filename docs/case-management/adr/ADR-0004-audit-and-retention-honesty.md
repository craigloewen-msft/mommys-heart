# ADR-0004: Audit and retention honesty

- Status: accepted
- Date: 2026-08-10

## Context

The MVP introduces more sensitive internal records and must not overstate what
the current application can retain or prove. The repository already prunes audit
entries using a 10-year retention window, but legal holds, organization-approved
retention policy, and stronger compliance assertions are future work.

## Decision

- Keep message and Case Note audit records content-free in the generic Change Log.
- Make audit visibility audience-aware and server enforced.
- State the currently observable audit retention limit as 10 years.
- Explicitly document that legal holds, backup/restore governance, and external
  compliance review are not MVP-delivered controls.

## Consequences

- The documentation remains honest about present system behavior.
- Reviewers can inspect concrete retention code paths without inferring stronger
  guarantees than the application currently offers.
- Future governance work must deliberately extend retention and legal-hold
  behavior instead of inheriting accidental claims from MVP docs.

## Alternatives considered

- Claim indefinite retention because records are append-only in normal app flows.
- Copy substantive message and note content into generic audit rows for easier
  debugging.

## Requirement links

- `REQ-AUD-001`
- `REQ-AUD-002`
- `REQ-AUD-003`
- `REQ-AUD-004`
- `REQ-DOC-002`
- `REQ-DOC-003`
