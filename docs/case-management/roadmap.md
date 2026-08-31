# What is still to be done

This file is the forward-looking half. For what the system **does today**, see
[`delivered.md`](delivered.md).

Work is grouped into ordered phases. The last section lists work that is blocked
on an organizational decision rather than on engineering — those items are not
scheduled, because scheduling them would imply the decision has been made.

## Status key

- **Next** — the natural following increment; dependencies are in place.
- **Later** — wanted, but depends on something in an earlier phase.
- **Blocked on policy** — cannot be specified responsibly until the organization
  answers a question that is not an engineering question.

---

## Phase 3 (next)

### 3.0 Production approval gates

Engineering safeguards for MFA, credential reset, browser requests, case-document
validation, stable actor attribution, funding audit, and collection scaling are
delivered. Production approval remains blocked until the organization supplies
the owners and evidence in
[`../operations/production-readiness.md`](../operations/production-readiness.md):
retention/legal hold, backup and restore objectives/drill, incident response,
accessibility target, and independent security review.


### 3.1 Referrals and service records (`REQ-CRM-050`)

Record that a client was referred to a provider, with status and outcome, rather
than describing it in a note narrative.

- **Depends on:** organizations and contacts (delivered in phase 2).
- **Shape:** per-case, staff-only, with a status lifecycle whose terminal states
  are trigger-enforced, reusing the `ServiceArea` taxonomy that already exists in
  `src/server_fns/case_notes.rs` rather than inventing a parallel one.
- **Open question:** may a client ever see that they were referred somewhere?

### 3.2 Client goals and service-plan objectives (`REQ-CRM-051`)

Track what a case is working toward, with an append-only progress history.

- **Depends on:** nothing outstanding.
- **Shape:** editable while active, with every status change and progress note
  appended immutably — the `case_note_addenda` pattern.
- **Open question:** how formal must plan approval and review be?

### 3.3 Linking Case Notes to goals and referrals

So a note can record which goal it advanced.

- **Depends on:** 3.1 and 3.2.
- **Care needed:** finalized notes are immutable and their finalization runs in
  an intricate transaction. The link belongs in a separate table written in the
  same transaction, not as a new column on `case_notes`.

### 3.4 Duplicate detection and contact merging (`REQ-CRM-049`)

Phase 2 makes it possible to create a contact for someone who already has one —
for instance when an account is added later and never linked.

- **Depends on:** contacts (delivered).
- **Shape:** fuzzy match on name, email, and phone at creation time, plus a merge
  that repoints case links, grants, and funding rows and keeps an audit trail.

---

## Phase 4 (later)

### 4.1 Multiple and historical organization affiliations

Represent a person who belongs to several organizations, with organization-
specific titles and relationship history. Today's `contacts.organization_id`
remains one current filing organization; additional affiliations must become a
first-class relationship model rather than custom properties or a second source
of truth.

- **Depends on:** a concrete reporting or workflow need for multiple concurrent
  affiliations and a decision about historical snapshots.

### 4.2 Tasks, reminders, court deadlines, and a case timeline (`REQ-CRM-052`)

Dated obligations that currently live in note narratives or chat.

- **Depends on:** an event model, and reminder ownership being decided.
- **Must not imply:** guaranteed deadline monitoring or legal-notice handling.
- **Open questions:** which deadlines are informational versus must-alert? Who
  owns an overdue task on a shared case? What may a client see?

### 4.3 Funder reporting

Turn the grant and funding records into the reports funders actually ask for.

- **Depends on:** 3.1 (service records) and a funding-code taxonomy; reporting
  against grants alone is not enough, because funders ask about services
  delivered.
- **Open questions:** which reports are operational convenience versus official
  submissions? What funding-code hierarchy is organization-approved?

### 4.4 Detailed service units and funding codes

Structured time and service units allocated against funding sources.

- **Depends on:** 3.1, and finance/operations ownership.

### 4.5 Attachments, document links, and PDF output

Attach or link supporting material to notes and messages; render an immutable
record as a portable document.

- **Depends on:** the case document library and a stable PDF template.
- **Open questions:** which file types are acceptable? Must a generated PDF
  include every addendum and audit marker?

### 4.6 Global authorized search

Search across cases, notes, and contacts from one place.

- **Depends on:** a search index strategy with strict audience filtering; today's
  retrieval is deliberately per-case.
- **Constraint:** results must contain only records the caller is allowed to know
  exist — a search index is the easiest way to leak existence.

### 4.7 Bulk import and export (`REQ-CRM-053`)

Onboarding an existing contact list, and getting data out.

- **Depends on:** duplicate detection (3.4), or an import will create thousands
  of near-duplicates.

---

## Blocked on policy

These are not scheduled. Each needs an organizational answer first.

### Supervisor review, approval, and returned revisions

Add review states above Case Note finalization.

- **Blocked on:** which roles may supervise, approve, or return an official
  record. The app has operations-admin and site-admin roles, but neither was
  designed as a clinical supervisor, and picking one arbitrarily would bake a
  guess into the record.

### Urgent safety alerts and acknowledgement

Escalate an urgent safety concern beyond documentation.

- **Blocked on:** who must be alerted, how fast, through which channel, what
  counts as acknowledgement, and who is on call after hours. Until that is
  settled the UI must keep saying plainly that recording a concern does not
  contact emergency services.

### Sensitive-field controls and break-glass access

Scope some records or fields more narrowly than "all staff", with an audited
emergency override.

- **Blocked on:** what qualifies as sensitive, who may override, and what
  approval or notification follows. This is the reason contact properties
  deliberately have no visibility column today — see
  [ADR-0005](adr/ADR-0005-contacts-are-not-users.md).

### Retention, legal holds, and governance

Backup and restore drills, incident response, accessibility standards, and
independent security review.

- **Blocked on:** an approved retention schedule and legal-hold workflow, a
  restore objective and drill cadence, a minimum accessibility bar, and a review
  budget.
- **Today's honest position:** audit entries are pruned on a 10-year window and
  nothing else is claimed. See
  [ADR-0004](adr/ADR-0004-audit-and-retention-honesty.md).

### Client-facing CRM visibility

Whether a client should ever see referral, goal, or case-contact records about
themselves.

- **Blocked on:** a safety review. Some of these records name people a client
  must not be shown as being in contact with.

---

## Deliberate non-goals

Not planned, and listed so nobody assumes they are merely late:

- End-to-end encryption, and any HIPAA/VAWA/SOC 2 certification claim.
- Email campaigns, mail merge, and automated donor receipts.
- Accounting or payment-processor integration; the funding ledger records money
  received, it is not a general ledger.
- Provider portals or logins for external organizations.
- SMS, email reply ingestion, and real-time sockets.
