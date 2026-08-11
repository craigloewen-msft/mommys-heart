# ADR-0002: Messaging audience and immutability

- Status: accepted
- Date: 2026-08-10

## Context

The existing application already has case chat, per-case capabilities, and one
volunteer-only channel per case. The MVP must harden that foundation rather than
replace it. The major risks are metadata leakage to clients, partial writes that
leave official communication unaudited, and any edit/delete behavior that weakens
record integrity.

## Decision

- Preserve audience as a first-class server-enforced property of every channel
  and message query path.
- Keep messages immutable after send.
- Archive standard channels instead of deleting them or their history.
- Require the message row, recipient/read state, and restricted audit metadata
  to commit atomically.
- Keep external notifications content-free and post-commit only.

## Consequences

- The application keeps a credible official-message record without pretending to
  offer end-to-end encryption or certified retention.
- Clients are protected from volunteer-only content and metadata side channels.
- Some convenience features such as message edit/delete, attachments, and email
  reply ingestion remain deferred.
- Export, unread-count, and search paths must share the same audience filtering
  rules as transcript reads.

## Alternatives considered

- Allow message editing with version history.
- Permit hard deletion or channel deletion for cleanup.
- Generate notifications before durable commit.

## Requirement links

- `REQ-MSG-001`
- `REQ-MSG-002`
- `REQ-MSG-003`
- `REQ-MSG-004`
- `REQ-MSG-005`
- `REQ-MSG-006`
- `REQ-MSG-007`
- `REQ-MSG-008`
- `REQ-MSG-009`
- `REQ-AUD-001`
- `REQ-AUD-002`
- `REQ-AUD-003`
