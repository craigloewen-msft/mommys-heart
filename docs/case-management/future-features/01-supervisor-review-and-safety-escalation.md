# Future brief 01: supervisor review and safety escalation

This brief satisfies item 1 of `REQ-DOC-003`.

## User need

Staff need a reviewed official-record workflow for situations where a note must
be approved, returned for revision, or used to trigger an acknowledged urgent
safety escalation.

## Scope boundary

Add supervisor review, returned revisions, approval, urgent safety alerts, and
escalation acknowledgement on top of MVP Case Note finalization.

This brief does **not** define a full operational escalation policy, external
integrations, or medical/legal advice. It also does not imply that the MVP
already performs any alerting.

## Dependencies

- Stable Case Note draft/finalized lifecycle
- Explicit supervisor role and permission model
- Escalation routing ownership and after-hours policy
- Audit visibility for review and acknowledgement actions

## Open policy questions

1. Which roles may approve, return, or acknowledge an escalation?
2. What urgency threshold triggers an alert rather than only documentation?
3. What delivery channels are allowed for alerts, and what counts as
   acknowledgement?

## Acceptance outcomes

- A supervisor can review and approve or return a submitted record with reason.
- The full review trail is immutable and auditable.
- Urgent escalations require explicit acknowledgement and do not silently fail.
