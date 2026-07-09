# Privacy, Security & Data Governance

**Audience:** Mommy's Heart leadership and future engineers.
**Status:** This document accompanies a **proof-of-concept** implementation. The
governance features described below are demonstrated in the app with **in-memory
mock data** — they illustrate the concepts and the intended user experience, but
they are **not** production security controls. This document explains what the
POC shows today and the concrete work required to make each capability real.

Because Mommy's Heart serves survivors of domestic violence and vulnerable
families, the stakes for confidentiality are high: a data leak can create a
physical-safety risk, not just a privacy one. Treat every item marked
"Production next steps" as required before handling real client data.

---

## How the demo is built (context)

- Single Rust crate (Leptos + Axum), everything compiled into one deploy.
- **No database yet:** users, cases, volunteers, documents, the audit log, and
  the knowledge base all live in reactive in-memory signals
  (`src/state.rs`, seeded from `src/mockdata.rs`).
- **Auth is a demo:** sign-in filters an in-memory list; passwords are plaintext
  sample data; there are no server-side sessions or tokens.
- The AI chat assistant (RAG over the `docs/` corpus) is real and already acts
  as a form of institutional-knowledge retention.

Everything below that says "the POC demonstrates…" is enforced only in the
browser/UI layer over mock data. Real enforcement must move server-side.

---

## 1. End-to-end encryption

**Decision for this round: guidance only — intentionally not built.**

True end-to-end encryption (E2EE) means only the client devices hold the keys
and the server can never read the plaintext. That directly conflicts with core
features this product needs:

- The **AI chat assistant** must read documents to answer questions.
- **Search, reporting, and case collaboration** require the server to read data.
- **Authorized staff sharing** a case implies the server brokers access.

If the server cannot read the data, none of the above can work. E2EE is the
right tool for a private messenger, not for a collaborative CRM whose whole
purpose is shared, searchable case context.

**Recommended instead (standard for this kind of system):**

- **Encryption in transit:** HTTPS/TLS everywhere (app, API, and the Squarespace
  widget origin). Enforce HSTS.
- **Encryption at rest:** enable transparent encryption on the database and file
  storage (e.g. cloud-managed disk/blob encryption), plus a managed key service
  (KMS) with rotation.
- **Application-layer field encryption** for the most sensitive fields (e.g.
  safety plans, addresses): encrypt specific columns with keys held in the KMS
  so a raw database dump is not enough to read them. The server still decrypts
  on demand for authorized use.
- **Optional targeted E2EE** only for a narrow feature that does not need
  server-side processing (e.g. private direct messages between two staff). Scope
  it deliberately; do not apply it to case records.

The document "lock" icon and "encrypted at rest" labels in the demo are
**indicators of intent**, not real cryptography.

---

## 2. Secure document storage

**POC demonstrates:**

- Every document carries a **classification** — Public / Internal / Confidential
  / Restricted (`DocumentClassification` in `src/types.rs`).
- Classifications are shown as badges with a lock icon ("encrypted at rest"
  indicator).
- **Access is gated:** opening a Confidential/Restricted document requires the
  `ViewConfidentialDocs` permission. Volunteers without it see the file name
  masked and a "Locked" button; the attempt is recorded in the audit log.

**Production next steps:**

- Store bytes in a real object store (e.g. S3/Azure Blob) with server-side
  encryption and **short-lived signed URLs**, never public links.
- Enforce classification checks **server-side** on every download, not in the UI.
- Add virus/malware scanning on upload and validate file types/size.
- Log every access with the actual authenticated identity (see Audit trails).
- Consider watermarking or view-only rendering for the most sensitive files.

---

## 3. Role-based access control (RBAC)

**POC demonstrates:**

- Authorization levels: **Admin, Staff, Volunteer, Read-only** (`Role`).
- A **permission matrix** (`Permission` + `Role::permissions()`); features check
  `can(permission)` rather than comparing roles directly, so new levels slot in
  cleanly.
- **Route guards** (`src/components/guard.rs`) redirect users away from screens
  their level doesn't grant; the navigation only shows permitted links.
- Volunteers are scoped to **their own assigned cases**; Admin/Staff see all.

**Production next steps:**

- Enforce every permission **on the server/API**, not just in the UI (the UI can
  always be bypassed). The permission matrix should live behind the API.
- Bind permissions to a real authenticated session (see below).
- Add per-record access rules ("row-level security"): a volunteer can read a
  case only if assigned. Ideally enforce this in the database.
- Support least-privilege defaults and periodic **access reviews**.

---

## 4. Audit trails

**POC demonstrates:**

- An append-only, newest-first **audit log** (`AuditEvent`) recording sign-ins,
  case views/edits, document access **and denials**, assignments, role changes,
  legal holds, disposals, and knowledge edits.
- An **Admin/Staff-only Audit Log page** (`/audit`) with a "denials only" filter.

**Production next steps:**

- Persist to **append-only, tamper-evident** storage (write-once, hash-chained
  or WORM); the demo keeps it in memory and would reset on restart.
- Record real timestamps, source IP/device, and the authenticated user id.
- Make logs immutable to admins; ship them to a separate system for retention.
- Define alerting on suspicious patterns (e.g. repeated access denials, bulk
  exports) and a review cadence.

---

## 5. User permissions & authorization levels

**POC demonstrates:**

- The four levels above, each with a described scope and a permission set.
- Admins can change a user's role (`set_user_role`), which is audited.
- Role badges surface each user's level throughout the UI.

**Production next steps:**

- Manage users/roles behind authenticated admin APIs with server-side checks.
- Add invitation/approval flows for new volunteers and interns (self-service
  sign-up currently auto-creates a pending volunteer).
- Consider time-boxed access (auto-expire an intern's account after their term).
- Enforce strong auth: hashed+salted passwords (argon2/bcrypt), MFA for
  Admin/Staff, session expiry, and lockout on failed attempts.

---

## 6. Records-retention policies

**POC demonstrates:**

- A **retention class** per case — Standard (7 yrs) / Extended (10 yrs) /
  Permanent (`RetentionClass`).
- **Legal hold** toggle that blocks disposal regardless of policy.
- A **Governance page** (`/governance`) showing each record's retention status
  and disposal eligibility. Only **closed, non-permanent, non-held** records can
  be disposed, and disposal is audited.

**Production next steps:**

- Drive retention from **real dates** (opened/closed timestamps) and automate
  eligibility calculation and scheduled disposal jobs.
- Confirm retention periods with legal counsel for your jurisdiction and record
  type (court records vs. intake vs. communications differ).
- Make disposal a reviewed, two-person action with a certificate of destruction;
  ensure backups also honor disposal (or document the exception).
- Extend retention/legal-hold to documents and communications, not just cases.

---

## 7. Data ownership

**POC demonstrates:**

- An explicit model: **all records are owned by the organization**, never by an
  individual. Cases carry `created_by` (provenance) and `steward`
  (current accountable party); the Governance page states this plainly.
- When a volunteer is offboarded, their stewardship reverts to the organization.

**Production next steps:**

- Put data ownership in writing: volunteer/intern/contractor agreements should
  assign all work product and case data to Mommy's Heart.
- Ensure any third-party processors (hosting, AI provider, email) are covered by
  data-processing agreements and cannot claim or reuse the data.
- Maintain the ownership/stewardship metadata in the database with history.

---

## 8. Institutional knowledge retention

**POC demonstrates:**

- A **Knowledge Base** (`/knowledge`): templates, resources, best practices, and
  retained case-context notes, **owned by the organization** and independent of
  any individual (`KnowledgeItem`).
- **Volunteer offboarding** (`offboard_volunteer`) marks the person inactive and
  reassigns their case stewardship to the organization, so context does not
  leave with them.
- The existing **AI chat assistant** answers questions over the organization's
  documents — a living knowledge resource.

**Production next steps:**

- Persist the knowledge base and add versioning, categories/tags, search, and
  review/expiry so guidance stays current.
- Build a structured **offboarding checklist** (reassign cases, revoke access,
  capture handover notes, disable the account) — partially demonstrated today.
- Feed approved knowledge-base content and handover notes into the RAG corpus so
  the assistant reflects the latest institutional practice.
- Capture communications/case history so context survives staff turnover.

---

## Recommended sequence to go from POC to production

1. **Foundation:** introduce a real database (the roadmap suggests SQLite/SQLx)
   and real authentication with hashed passwords + server-side sessions.
2. **Move all RBAC and access checks server-side**, including per-record
   (row-level) rules and API-enforced document access.
3. **Encryption:** TLS everywhere, at-rest encryption + KMS, and field-level
   encryption for the most sensitive data.
4. **Persistent, tamper-evident audit logging** with real identities/timestamps.
5. **Automate retention & legal holds** with real dates and reviewed disposal.
6. **Harden auth:** MFA for staff, lockout, session expiry, time-boxed
   volunteer/intern accounts.
7. **Operational governance:** backups + tested disaster recovery, data-
   processing agreements with vendors (including the AI provider), incident-
   response plan, and periodic access reviews.
8. **Compliance review** with counsel for the applicable obligations (e.g. state
   privacy laws, VAWA confidentiality expectations, and any grant requirements).

> None of the current demo controls should be relied upon for real survivor
> data until the "Production next steps" above are implemented and independently
> reviewed.
