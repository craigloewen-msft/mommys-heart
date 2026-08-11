# ADR chain

This directory holds the shared architecture decision record chain for the
case-management MVP.

## Purpose

Use ADRs for material decisions that affect schema shape, lifecycle behavior,
authorization, migration, auditability, or documentation boundaries.

## Conventions

- File names use `ADR-XXXX-short-title.md`.
- ADR numbers are stable and never reused.
- `proposed`, `accepted`, `superseded`, and `rejected` are the allowed statuses.
- Update `index.md` whenever an ADR is added or superseded.
- ADRs capture context and consequences; they do not restate the full
  requirements set.

## Minimum sections

Use `template.md` when adding a new ADR:

1. Status
2. Date
3. Context
4. Decision
5. Consequences
6. Alternatives considered
7. Requirement links
