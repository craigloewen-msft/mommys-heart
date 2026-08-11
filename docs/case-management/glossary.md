# Glossary

## Case management MVP

The bounded product increment defined by the approved task. It adds secure
in-app case messaging and structured internal Case Notes without attempting to
ship the client's entire 19-section request at once.

## Secure messaging

Authenticated in-app case communication with server-enforced audience rules,
immutable messages, audit metadata, content-free notifications, and authorized
export. In this MVP it does **not** mean end-to-end encryption, SMS delivery,
email reply ingestion, or emergency dispatch.

## Case Chat

The person-to-person message experience attached to a case. It includes shared
channels and one permanent volunteer-only channel.

## Shared channel

A case chat channel visible to clients and authorized team members under the
same server-side case-access rules.

## Volunteer-only channel

A case chat channel that clients must never discover, query, or export, even if
another defect accidentally grants broad case capabilities.

## Case Note

A formal internal service record authored by a non-client user. It is distinct
from chat, the generic Change Log, and historical free-text notes.

## Legacy note

A migrated historical free-text note preserved in an immutable legacy state. It
keeps its historical audience instead of silently changing access during MVP
migration.

## Draft note

An incomplete structured Case Note visible to its author and limited
administrative inspection paths until finalization.

## Finalized note

A signed, immutable structured Case Note that has passed server-side validation
and is visible to authorized non-client case viewers.

## Addendum

An immutable, signed supplement attached to a finalized or legacy note. It adds
information without replacing the parent note.

## Restricted audit metadata

Content-free audit records that identify the action and record involved without
copying message bodies, note narratives, client names, or other substantive
content into the generic Change Log.

## First-read evidence

The write-once timestamp showing when a specific authorized user first received
message content from the server.

## Executive status file

A point-in-time summary of scope, delivery state, dependencies, open policy
questions, and acceptance outcomes. It is intentionally separate from timeless
requirements.

## ADR

Architecture Decision Record. A concise record of a material design choice,
including context, decision, and consequences.

## Client specification section

One major area from the client's broader 19-section request. The roadmap tracks
all 19 sections plus submission/search outcomes, even where the exact original
section title still needs confirmation from the client source packet.

## Submission/search outcomes

The client-request expectations around how staff submit official records and how
authorized users search or retrieve them. The roadmap tracks these separately so
MVP limitations are explicit.
