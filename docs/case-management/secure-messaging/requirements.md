# Secure messaging requirements

This file is normative. It defines timeless messaging and messaging-adjacent
requirements for the case-management MVP.

## Scope

The MVP adds authenticated in-app case messaging with stronger audience
protection, immutable records, content-free notifications, and authorized
transcript export.

## Non-goals

The MVP does not provide end-to-end encryption, SMS delivery, attachments, email
reply ingestion, message editing, reactions, guaranteed emergency delivery, or a
claim of legal/compliance certification.

## Requirements

- **REQ-MSG-001 — Audience authorization.** When a user lists a channel, reads a
  transcript, sends a message, marks messages read, or exports a transcript, the
  system shall resolve the channel's stored case and audience and enforce access
  on the server. A client shall never receive volunteer-only channel metadata,
  message content, unread counts, audit entries, or exports.
- **REQ-MSG-002 — Immutable messages.** Once sent, message author, audience,
  timestamp, and body shall not be editable or permanently deletable through the
  application.
- **REQ-MSG-003 — Channel archival.** When an authorized manager removes a
  standard channel from active use, the system shall archive it instead of
  deleting it or its messages. Archived channels remain readable to authorized
  users, reject new messages, and remain available to authorized exports. The
  volunteer-only channel remains permanent. At least one active shared channel
  must remain on every non-declined case.
- **REQ-MSG-004 — Atomic send record.** When a message is accepted, the message,
  its eligible-recipient records, and content-free audit metadata shall commit
  together. A failed audit/recipient write shall fail the send rather than leave
  an unaudited message. Best-effort email delivery happens only after commit.
- **REQ-MSG-005 — Durable first-read evidence.** When an authorized user first
  receives message content from the server, the system shall preserve that user's
  first-read timestamp without overwriting it on later reads. Existing unread
  rows shall migrate without inventing historical read dates that are not known.
  Routine chat UI need not expose who read a message; authorized transcript
  exports may include this evidence.
- **REQ-MSG-006 — Safe notifications.** Email and browser notification summaries
  for new messages shall contain no message body, attachment, case name, client
  name, channel name, or other case detail. They may say only that a new secure
  message is waiting and direct the recipient to sign in. A user's existing
  notification opt-out remains honored.
- **REQ-MSG-007 — Validated text-only MVP.** The system shall trim messages,
  reject empty content, enforce a documented maximum length on both client and
  server, prevent duplicate submission while a send is in flight, and apply a
  reasonable server-side send throttle. Message attachments, editing, reactions,
  delivery by SMS, and email replies are deferred.
- **REQ-MSG-008 — Authorized export.** An operations or site administrator who
  also has `ViewCase` shall be able to download a UTF-8 CSV transcript of a
  channel they are allowed to see. The export shall include case/channel
  identifiers, archived state, message identifiers, author identifiers and name
  snapshots, server timestamps, bodies, and available first-read evidence. Every
  export shall itself create restricted audit metadata. PDF transcript output is
  deferred.
- **REQ-MSG-009 — No metadata side channel.** Counts, search results, unread
  state, audit queries, and inactive-case calculations shall apply the same
  audience rules as message reads.

## Dependencies

- Existing account authentication, MFA, and session enforcement.
- Existing per-case capability model, including `ViewCase` and `SendMessages`.
- Existing shared and volunteer-only channel concepts.
- Audience-aware audit visibility and transactional persistence.
- Notification settings that already allow user opt-out.

## Open policy questions

1. What message length limit is appropriate for MVP operational use?
2. Which admin roles may archive a standard shared channel?
3. What transcript-export handling policy applies after download outside the app?
4. Are any additional warning banners required before users export transcripts?

## Acceptance outcomes

- Authorized shared messaging works for assigned clients and team members.
- Clients cannot discover volunteer-only channel existence through direct or
  indirect metadata.
- Archived channels preserve history and reject new sends.
- External notifications remain content-free.
- Authorized exports succeed only within audience boundaries and create audit
  metadata.
