# Future brief 06: sensitive permissions and break-glass access

This brief satisfies item 6 of `REQ-DOC-003`.

## User need

Some cases require stricter handling for sensitive notes, safe-contact data, and
exceptional emergency access than the MVP's broad internal-note model provides.

## Scope boundary

Add granular sensitive-note permissions, break-glass access, access reviews,
safe-contact preferences, and field-level restrictions.

This brief does **not** define the final policy matrix, excuse ordinary users
from least-privilege review, or imply that MVP notes already have field-level
protection.

## Dependencies

- Baseline internal-note and audit model
- Permission matrix design by role and circumstance
- Review and revocation workflows
- Safe-contact data classification and UI treatment

## Open policy questions

1. What qualifies as sensitive enough for restricted visibility?
2. Who may invoke break-glass access, and what approval or notification follows?
3. Which fields, if any, may clients later manage for safe-contact preferences?

## Acceptance outcomes

- Sensitive records can be scoped more narrowly than general internal notes.
- Break-glass access is explicit, exceptional, and auditable.
- Access reviews reveal stale or overly broad permissions.
