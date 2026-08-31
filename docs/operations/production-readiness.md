# Production readiness gate

This is an operational gate, not a claim of certification. A release is not
production-approved until every required row has named evidence and an owner.

## Configuration gate

`APP_ENV=production` makes startup fail unless both conditions are met:

- ACS email is fully configured, so MFA cannot fall back to password-only login.
- `APP_URL` names the one accepted browser origin.

The server automatically marks authentication cookies `Secure` and applies
Content Security Policy, frame denial, no-sniff, no-referrer, permissions policy,
HSTS, and Origin validation for cookie-authenticated mutations. Deployment review
must confirm proxy headers and the public URL match this policy.

## Required release evidence

| Gate | Required evidence | Owner | Status |
| --- | --- | --- | --- |
| Database backup | Encrypted backup location, schedule, access list | Organization decision | Blocked |
| Document backup | SharePoint retention/versioning policy and library access list | Organization decision | Blocked |
| Recovery objectives | Approved RPO and RTO | Organization decision | Blocked |
| Restore drill | Dated database + document-library restore, integrity queries, elapsed time | Operations | Not run |
| Retention schedule | Approved record classes and retention periods | Legal/operations | Blocked |
| Legal holds | Approved hold authority, scope, release, and audit workflow | Legal/operations | Blocked |
| Incident response | Named incident lead, contact tree, containment and notification runbook | Organization decision | Blocked |
| Accessibility | Approved WCAG target; automated, keyboard, and screen-reader results | Product owner | Blocked |
| Security review | Dependency/secret review, authorization matrix, independent findings disposition | Security owner | Blocked |
| Browser matrix | Current version/date, desktop/mobile screenshots, console/network results | Engineering | Not run |

“Blocked” is deliberate. Engineering must not invent legal, retention,
accessibility, incident, or review policy on the organization's behalf.

## Restore drill procedure

1. Record application version, migration version, backup identifiers, and start
   time. Use an isolated recovery environment with no outbound email.
2. Restore PostgreSQL, and the case document library, to the same recovery point.
3. Start with production-like configuration, but a non-production origin and
   email sink.
4. Verify user/contact links, case ownership/capabilities, case-contact primary
   uniqueness, grant/funding rollups, void exclusions, audit actor attribution,
   that each case's `drive_item_id` still resolves to its folder, and document
   download authorization.
5. Record missing/corrupt rows, achieved recovery point, achieved recovery time,
   corrective actions, reviewer, and completion date.
6. Destroy the recovery environment and its copied sensitive data according to
   the approved procedure.

A backup that has not completed this drill is not verified.

## Security incident minimum runbook

Until the organization names owners and notification policy, production remains
blocked. The approved runbook must at minimum cover credential/session revocation,
trusted-device revocation, secret rotation, database/blob access containment,
audit preservation, legal-hold interaction, impact assessment, notification
approval, recovery validation, and post-incident review.

## Case document safety

Case files are stored in a SharePoint document library, so malware scanning,
versioning and retention are the tenant's responsibility rather than this
application's. Confirm before production approval that the library has
anti-malware enabled and a retention policy that matches the approved schedule
above.

The application still validates what it accepts: uploads are capped at 25 MB and
their true type is content-sniffed with `infer` against an allowlist (PDF, common
images, Office documents), so a file is judged by its bytes rather than its
extension or declared `Content-Type`.

Access to the library is app-only (Entra client credentials, `Files.ReadWrite.All`
+ `Sites.ReadWrite.All`). Two consequences to record at approval time: the
application can read every case folder regardless of who is signed in, so its
own capability checks are the effective control on reads; and inviting clients,
who are outside the tenant, requires external sharing on that site.
