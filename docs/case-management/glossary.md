# Glossary

## User (account)

A row in `users`: something that can **sign in**. It requires a unique email and
a password hash, and it carries the account role that governs app-level access.

An account whose role is **deactivated** is retired: it cannot sign in and grants
nothing, but it is never deleted. It keeps its email address, its case
assignments, and its history, and a site admin can restore it to the role it held
before (recorded in `account_deactivations`).

## Contact (person)

A row in `contacts`: a **person the organization has a relationship with**. Most
contacts have no account and never will — donors, funder program officers,
partner caseworkers, opposing attorneys, court clerks, board members, emergency
contacts.

A contact may link to at most one user. When it does, `users` stays the source of
truth for identity, email, and role. The distinction is deliberate and is
recorded in [ADR-0005](adr/ADR-0005-contacts-are-not-users.md).

## Organization

An outside body: a funder, partner agency, service provider, government agency,
court, or employer. Contacts are filed under one; grants are awarded by one.

## Contact property

An ordered custom field on a person, grouped under a free-text section heading.
New person records begin with a small set of blank communication and relationship
defaults; these do not duplicate first-class contact or organization fields.
There is **no visibility axis** because clients cannot see contacts at all.

## Organization property

An ordered custom field on an organization. It has the same section/key/value
shape as a contact property and is visible only in the administrative workspace.
New organizations begin with blank relationship defaults.

## Filing organization

The one current organization stored on `contacts.organization_id`. It is the
person's primary filing context, not an affiliation history or a claim that the
person has no other real-world relationships.

## Case contact

A link between a case and a contact, carrying that person's role on that case.
At most one contact per case may be marked primary.

## Grant

An award sought or received from a funder, with a status from `Prospect` through
`Closed` or `Declined`, an awarded amount, and a period.

## Funding

Money received: a grant payment, donation, in-kind gift, or other. Amounts are
stored as integer minor units (cents), never floating point.

## Voided

A funding record corrected by marking it void with a reason, rather than deleting
it. It stays visible on the ledger and drops out of every rollup. There is no
delete path.

## Archived

A contact or organization retired from use. It disappears from pickers, but every
existing reference to it keeps working. The counterpart of channel archival.

## Withdrawn

A case taken back by the person who filed it — the answer to "I opened this by
mistake". The case is frozen and drops out of every non-admin list, but nothing
is deleted, and an administrator can restore it to exactly the status it held
before. Distinct from *archived* (contacts, channels) and from *declined*, which
is the organization's decision rather than the client's.

## Case management MVP

The bounded product increment covering secure in-app case messaging and
structured internal Case Notes, rather than the client's entire 19-section
request at once.

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

A migrated free-text note preserved in an immutable legacy state. It keeps the
audience it was written under rather than silently gaining a new one.

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
