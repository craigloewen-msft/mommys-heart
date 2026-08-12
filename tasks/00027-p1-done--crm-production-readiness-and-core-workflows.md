# Complete CRM production readiness and core workflow coverage

## Why this work exists

The phase-2 CRM has a strong object model: people are separate from sign-in
accounts, organizations and cases are connected, grant/funding lifecycles have
database constraints, and sensitive case work has capability-based access.
The repository also follows its expected `src/server/db/<object>.rs` plus
`src/server_fns/<object>.rs` layout consistently.

A readiness audit found that it is not yet responsible to call the product a
complete, production-ready CRM. Some issues are defects in already-promised
behavior, some are operational safeguards, and some are missing day-to-day CRM
workflows.

### Confirmed strengths

- CRM modules follow the repository's object/repository/server-function layout.
- CRM server functions authenticate and authorize at the server boundary; UI
  guards are not treated as authorization.
- Most CRM writes and their audit entries are transactional.
- Money is integer minor units and funding corrections are voids, not deletes.
- Database constraints protect grant terms, positive funding, one primary case
  contact, one contact per account, and archived/reference integrity.
- Relationship pickers introduced in phase 2 use bounded server-side searches.
- The maintained documentation states important non-goals honestly.

### Confirmed readiness gaps

1. **Standalone funding is not fully audited.** Funding create/void writes an
   `audit_log` entry only when the row names a grant. A standalone donation,
   in-kind gift, or other receipt can therefore be created and voided with no
   Change Log entry (`src/server/db/funding.rs:108-184`). This conflicts with
   `REQ-CRM-018`, `REQ-CRM-022`, and the delivered claim that funding is audited.
2. **Funding notes are accepted but cannot be entered or viewed.** The form
   constructs and submits `notes`, but renders no Notes field; the ledger also
   omits stored notes (`src/pages/funding.rs:810-963`).
3. **Several CRM lists silently truncate.** Organization detail caps people and
   grants at 100, the organization directory caps at 200, the funding grant list
   at 100, and the funding ledger at 50 without paging controls. Funding pickers
   download the first 200 people/organizations, and the grant-payment picker is
   populated from the currently filtered first 100 grants
   (`src/pages/organizations.rs:61-75,220-249`; `src/pages/funding.rs:71-115`).
   This breaks the paginated grant-list promise in `REQ-CRM-015` and becomes
   incorrect, not merely slow, as data grows.
4. **Account and person records can drift.** Registration/backfill copies account
   name, email, phone, and address into a contact, while profile editing later
   updates only `users`. Contact editing can independently change the copied
   values (`src/server_fns/auth.rs:550-587`, `src/server/db/users.rs:598-646`,
   `src/server/db/contacts.rs:402-503`). This is inconsistent with the stated
   rule that the account owns identity/email for a linked person.
5. **MFA fails open when production email is misconfigured.** MFA is skipped in
   every debug build *or whenever email is not configured*, allowing password-only
   production login after a configuration omission (`src/server/auth.rs:158-162`,
   `src/server_fns/auth.rs:117-127`).
6. **Password reset bypasses the registration password policy.** Registration
   enforces 8–200 characters, but reset accepts any non-empty password and hashes
   it directly (`src/server_fns/auth.rs:239-269,743-769`).
7. **Password-change session revocation is best effort.** A failed session delete
   still reports a successful credential change, leaving existing sessions live
   (`src/server_fns/auth.rs:761-769`).
8. **Actor identity is only a mutable display snapshot in several records.** The
   generic audit table and funding rows store actor names but not stable actor IDs,
   despite `REQ-CRM-021` requiring both (`migrations/0001_init.sql:114-126`,
   `migrations/0019_crm_contacts_organizations_and_funding.sql:191-224`).
9. **Security/governance readiness is not implemented.** There is no demonstrated
   backup/restore drill, legal hold, incident runbook, independent security review,
   minimum accessibility gate, malware quarantine for existing evidence uploads,
   or explicit browser security-header policy. The roadmap acknowledges much of
   this at `docs/case-management/roadmap.md:148-168`.
10. **Requirement traceability has eroded.** The docs say stable IDs remain in
    code comments, but CRM implementation code contains no `REQ-CRM-*` references.
    Existing behavior is described well, but a grep cannot trace CRM requirements
    to their enforcement points.

## Browser verification status

A live browser pass was attempted, but no app instance was accepting connections
on the default or deterministic development port range. The checkout's command
runner also failed to spawn a shell process, so `etc/dev.sh build/run` could not
be started in this audit. This is a tooling/environment block, not a passed test.
The prior completed task records a browser pass at the time of implementation,
but that is not evidence that the present checkout still passes.

This task therefore requires a fresh browser matrix and must not close based only
on code inspection or prior task outcomes.

## Requirements

### Correct already-promised behavior

- **REQ-CRM-044 — Every funding mutation is auditable.** Whenever funding is
  recorded or voided, the system shall append an immutable audit entry in the
  same transaction, whether or not the funding names a grant. The entry shall
  identify the funding record and action without copying private notes. Grant,
  source-contact, and source-organization views shall link or mirror the event
  where applicable without creating divergent logs.

- **REQ-CRM-045 — Funding records are completely usable.** The funding form shall
  expose Notes with the same server-enforced length limit as the domain input.
  Authorized users shall be able to inspect the kind, amount, received date,
  source, grant, reference, notes, recorder, void state, void actor, reason, and
  timestamps. Voiding shall require an explicit confirmation and reason.

- **REQ-CRM-046 — CRM collections never silently truncate.** People,
  organizations, grants, funding, organization relationships, and funding-source
  pickers shall either paginate with an accurate server total or use a bounded,
  debounced server-side typeahead. No control shall imply that its first 50, 100,
  or 200 rows are the complete set. Changing a grant-list filter shall not remove
  otherwise valid grants from the record-funding picker.

- **REQ-CRM-047 — Linked-account field ownership is explicit and consistent.** A
  linked account shall remain authoritative for sign-in email and the account
  identity fields chosen by policy. The person UI shall mark projected account
  fields read-only. Saving either profile shall not leave two contradictory values
  presented as the same fact. Any independent CRM contact channel shall be named
  distinctly (for example, “CRM contact email”) and shall not be described as the
  account-owned sign-in email. Migration shall preserve data and flag conflicts
  rather than silently choosing a value.

- **REQ-CRM-048 — Stable actor attribution.** Every new CRM audit and funding
  mutation shall store the authenticated actor's stable user ID and a display-name
  snapshot supplied by the server. Renaming an account shall not change historical
  attribution, and duplicate names shall not make actors ambiguous. Legacy rows
  may retain a null actor ID with their original snapshot.

### Authentication and request hardening

- **REQ-SEC-001 — MFA fails closed outside development.** In a production-mode
  process, missing or unusable MFA email configuration shall prevent authenticated
  password-only login and shall produce an actionable startup/health error. Only
  an explicit development mode may bypass MFA, and every bypass shall be clearly
  logged without exposing credentials.

- **REQ-SEC-002 — One password policy.** Registration and password reset shall use
  the same server-side password validator, including minimum and maximum length.
  Browser validation may assist but shall not be the authority.

- **REQ-SEC-003 — Credential changes revoke sessions atomically.** A password
  reset shall not report success unless the password update, reset-token
  consumption, session revocation, and trusted-device revocation complete as one
  durable operation. The reset token shall remain single-use under concurrency.

- **REQ-SEC-004 — Browser request defenses are explicit.** Production responses
  shall define reviewed Content Security Policy, frame-ancestor, content-type,
  referrer, permissions, and transport-security headers. Authenticated mutation
  endpoints shall enforce a documented same-origin/CSRF strategy in addition to
  cookie `SameSite` behavior. The deployment shall reject insecure cookie settings
  in production.

- **REQ-SEC-005 — Uploaded evidence is quarantined until safe.** A newly uploaded
  evidence file shall not be downloadable until content-type validation and the
  configured malware scan succeed. Scan failure or timeout shall fail closed,
  preserve an auditable status, and avoid exposing bytes. Development may use an
  explicit fake scanner; production may not silently disable scanning.

### Operational production gate

- **REQ-OPS-001 — Recoverability is demonstrated.** Before production approval,
  operators shall document encrypted backup ownership, recovery objectives, and a
  restore procedure, then complete and record a restore drill against representative
  database and evidence data. A backup that has not been restored is not accepted
  as verified.

- **REQ-OPS-002 — Retention and legal hold are policy-backed.** Automatic deletion
  shall follow an approved retention schedule. A legal hold shall suspend relevant
  deletion jobs without making held data broadly visible. Until policy is approved,
  production readiness shall remain explicitly blocked rather than claiming
  compliance.

- **REQ-OPS-003 — Security and accessibility have release evidence.** Production
  approval shall include an incident-response owner/runbook, dependency and secret
  review, authorization test matrix, independent security review disposition, and
  an agreed accessibility target with automated and keyboard/screen-reader checks.

- **REQ-OPS-004 — Requirements remain traceable.** New and corrected enforcement
  points shall cite their requirement IDs in concise comments where the invariant
  is implemented. Delivered documentation shall not claim a requirement is met
  until its acceptance scenario has current evidence.

### Core CRM workflow increments

These are product increments, not blockers for correcting the defects above. They
remain separate deliverables so production hardening is not delayed by broad scope.

- **REQ-CRM-049 — Duplicate prevention and merge.** When an authorized user creates
  or imports a person, the system shall show likely matches based on normalized
  name, email, and phone before commit. An authorized merge shall transactionally
  repoint case links, funding, grant contacts, properties, and account links;
  preserve both source IDs in an immutable merge record; reject incompatible
  account links; and support audit reconstruction without deleting history.

- **REQ-CRM-050 — Referrals and service delivery.** Authorized case staff shall
  record a referral to a provider/person with service area, referred date, owner,
  status, follow-up date, outcome, and closure reason. Status transitions and
  terminal-state immutability shall be enforced server-side. Client visibility is
  disabled until the documented safety-policy decision is approved.

- **REQ-CRM-051 — Goals and progress.** Authorized case staff shall create a goal
  with owner, target/review date, status, and measurable outcome. Progress entries
  shall be append-only, attributed, and time-stamped by the server. Closing or
  abandoning a goal shall require an outcome/reason and preserve its history.

- **REQ-CRM-052 — Tasks, follow-up, and deadlines.** Authorized users shall create
  assigned, dated tasks linked to a person, organization, case, grant, referral,
  or goal. Completion/reopening shall be audited; overdue status shall be derived
  from server time. The UI and notifications shall not imply guaranteed legal or
  emergency deadline monitoring. Reminder escalation and client visibility remain
  disabled until ownership policy is approved.

- **REQ-CRM-053 — Data portability.** Authorized admins shall export contacts,
  organizations, relationships, grants, funding, and audit-safe identifiers in a
  documented, machine-readable format. Import shall provide dry-run validation,
  duplicate review, row-level errors, idempotency, and an auditable commit. Export
  authorization shall be at least as strict as on-screen access.

## Required browser scenario matrix

Use `etc/dev.sh build`, then `etc/dev.sh run`, and wait for `MH_READY`. Execute on
fresh seed data and repeat destructive scenarios after `etc/dev.sh reset`.

### Site/operations administrator

1. Create an organization, create a person under it, edit default properties,
   reload, search both directories, archive/restore each, and inspect both logs.
2. Link/unlink an account and verify the account still signs in; edit the account
   profile and confirm the person view follows the field-ownership policy.
3. Move a person between organizations with confirmation, remove the affiliation,
   and verify both organization logs plus the person log.
4. Add, edit, promote, and remove case contacts from both case and person views;
   verify atomic primary demotion, duplicate rejection, archived-target rejection,
   and read-only declined cases.
5. Create each grant status; verify invalid awarded/closed transitions are refused;
   filter and page beyond the first result window.
6. Record grant funding, standalone donation, and in-kind funding with notes. Void
   each, reconcile rollups, inspect full details, and verify every audit entry.
7. Search/select a person, organization, and grant beyond 200 seeded synthetic
   rows without downloading the full records.

### Volunteer

1. Read contacts/organizations only through permitted case-work surfaces.
2. Add/edit/remove a case contact only with `EditCase`; verify a viewer cannot.
3. Directly call create/archive/property/grant/funding admin server functions and
   verify refusal with no data mutation.
4. Confirm no dead admin-only person links render in volunteer case views.

### Client

1. Confirm no Admin/People/Organization/Funding navigation or case-people panel.
2. Directly call every CRM read and write function; verify a non-disclosing denial
   and no mutation.
3. Verify shared case messaging/evidence still works and volunteer-only existence,
   counts, and content remain undisclosed.

### Authentication, accessibility, and responsive behavior

1. Verify duplicate-submit prevention, password policy parity, session/trusted-device
   revocation, MFA fail-closed configuration, logout, and expired sessions.
2. Complete the CRM flows using keyboard controls at desktop and mobile widths;
   verify focus, labels, error announcements, combobox behavior, and no horizontal
   data loss.
3. Check browser console and network failures after each scenario. Capture current
   screenshots and retain a pass/fail record with the app version and date.

## Implementation sequence

1. Correct funding audit/detail defects and all silent truncation.
2. Decide and implement linked account/person field ownership with a safe migration.
3. Harden password reset, MFA configuration, session revocation, security headers,
   and evidence quarantine.
4. Add stable actor IDs and backfill nullable legacy attribution.
5. Run the full fresh-data browser/authorization/accessibility matrix.
6. Update `delivered.md`, `roadmap.md`, glossary, ADRs for material decisions, and
   concise requirement references only after verification.
7. Plan `REQ-CRM-049`–`053` as separate implementation tasks in dependency order:
   duplicate prevention before bulk import; referrals before goal/note reporting;
   policy decisions before reminder escalation or client visibility.

## Verification

- `cargo fmt --check`
- `etc/dev.sh -- cargo check --no-default-features --features ssr`
- `etc/dev.sh build`
- Fresh `etc/dev.sh reset` plus direct database invariant/audit queries
- Authorized and unauthorized direct server-function calls
- Desktop/mobile browser matrix above with a clean focused console
- Security-header inspection and production-mode negative configuration checks
- Backup/restore drill evidence and policy-blocked status where decisions remain

## Definition of done

The current CRM no longer loses audit coverage or hides data behind arbitrary
first-page limits; linked account/person facts have one explicit owner; funding is
fully usable; authentication and evidence handling fail closed in production; and
a current browser/authorization/accessibility matrix demonstrates the shipped
claims. Remaining workflow increments have stable requirements and dependency
order without being represented as already delivered.
