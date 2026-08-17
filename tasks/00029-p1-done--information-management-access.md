# Add one permission for Contacts, Organizations, and Funding

## Goal

Give site administrators one per-user switch labeled:

> **Can manage contacts, organizations, and funding information**

The switch is an additional access gate over the unified navbar information areas. It does not replace account roles, grant a higher role, or create separate permissions for each feature.

## Dependency

Implement this after task `00028-p1-in-progress--move-crm-views-to-navbar.md`, which is already consolidating the duplicate Contact Directory and Admin People experiences and establishing these canonical navbar destinations:

- `/contacts` and `/contacts/:id`
- `/organizations` and `/organizations/:id`
- `/funding` and `/funding/:id`

This task shall preserve that consolidation. `/contacts` remains the only contact workspace and provides the merged, best available contact-management experience. Do not recreate an Admin People workspace or a second contacts UI.

If task 00028 has not landed when implementation begins, rebase onto it or wait for it rather than implementing both tasks independently.

## Confirmed product decisions

1. There is exactly **one** permission toggle, not separate Contacts, Organizations, Funding, view, or edit permissions.
2. Only **site administrators** may grant or revoke it.
3. The permission is additive to existing role checks and never elevates an account role.
4. Contacts has one canonical workspace in the navbar; the former Contact Directory and Admin People experiences stay consolidated.
5. Because all three related areas share one permission, their contact, organization, and grant pickers may work normally whenever the user is otherwise authorized for the workflow.

## Access semantics

Let the stored permission be called `information management access` in requirements; implementation may use a concise internal name as long as the product label above is unchanged.

| Account role | Permission denied | Permission granted |
|---|---|---|
| Client | No Contacts, Organizations, or Funding access | Still no access; the permission does not elevate the client role |
| Volunteer | No access to these three information areas | Unified Contacts access and the existing volunteer-level Organization access; no admin-only management or Funding access |
| Operations admin | No access to these three information areas | Full role-authorized Contacts, Organizations, grants, and Funding workflows |
| Site admin | No feature access, but can still use Admin → Users to restore access | Full role-authorized workflows and ability to grant/revoke the permission |

Existing role and case-capability restrictions remain authoritative inside the permitted area. For example:

- a permitted volunteer keeps the contact actions already intended for volunteers and read-only Organization behavior, but does not gain admin-only archive, account-linking, property, audit, grant, or funding powers;
- Funding and grants still require operations-admin permissions in addition to this switch;
- case mutations still require their existing stored case capability where applicable; and
- Admin → Cases and Admin → Users remain role-controlled and usable by an otherwise authorized admin even when this information permission is denied.

The permission is independent of role changes. Promoting, demoting, or re-promoting an account shall not silently overwrite an explicit grant or denial. A stored grant on an ineligible role has no effect until the role is also eligible.

## Requirements

### REQ-IMA-001 — Persist one audited per-user grant

Add one non-null boolean permission to user persistence.

Migration behavior shall be safe for existing installations:

- existing volunteers, operations admins, and site admins receive the grant so deployment preserves their current access;
- existing clients remain denied;
- newly registered accounts default to denied; and
- fresh seeded environments explicitly grant the permission to the seeded non-client accounts needed to exercise the information features.

Carry the value through every authenticated user load path, including login, MFA/session restoration, direct session extraction, admin user loading, seed/mock construction, and the lightweight client-side current-user summary. A full page refresh must produce the same navbar and route decision as a fresh login.

Grant/revoke changes shall be transactional, no-op when unchanged, and recorded in the target user's existing audit log with the authenticated site administrator as actor. No “last holder” rule is needed because the switch does not control access to Admin → Users and a site admin can restore it.

### REQ-IMA-002 — Site-admin-only management control

Add a clearly named access section to the existing Admin → Users user detail/card.

- Site admins can grant or revoke the switch and receive clear success/error feedback.
- Operations admins cannot mutate it and have no request/approval path for it; if status is shown to them, it is read-only.
- The server mutation independently requires `require_site_admin`; hiding or disabling a browser control is not authorization.
- The exact product-facing checkbox/switch label is **“Can manage contacts, organizations, and funding information.”**
- Helper text explains that account roles still limit what the user can do.
- The target user's site-admin-only change log shows grants and revocations.

A site admin may change their own switch. Denying it must not remove their access to Admin → Users, so they can grant it again.

### REQ-IMA-003 — Central authorization helpers

Add centralized server-side helpers for checking/requiring the permission and use them consistently with the existing role/case guards.

The required decision is always an intersection, never an either/or bypass:

```text
existing role or case authorization
AND
information management permission
```

Do not scatter raw boolean checks through server functions where a shared helper can express the rule. Error messages should state that access to contacts, organizations, and funding information has not been granted, without leaking record existence.

### REQ-IMA-004 — Protect all feature entry points on the server

Apply the additional permission gate to authenticated server functions that expose or mutate these information domains, while retaining every stricter existing role/capability check:

- the unified contact directory, contact detail, categories/tags, and communication history;
- structured contacts, contact properties, archive state, account links, organization links, and contact search/picker endpoints;
- organizations, organization properties, and organization search/picker endpoints;
- grants, grant search/picker endpoints, funding records, rollups, recording, and voiding;
- contact-, organization-, grant-, and funding-scoped audit history; and
- contact relationships shown or edited from case, profile, contact, or organization surfaces, including case-contact APIs that disclose contact information.

Internal system workflows that create or maintain linked contact rows as part of registration/case signup are not user-facing feature access and must continue to work.

When a server function is shared by two permitted workflows, keep it shared; the one large permission intentionally avoids separate cross-feature dependency rules.

### REQ-IMA-005 — Navbar, routes, and contextual UI

After task 00028's canonical navigation is in place:

- show Contacts and Organizations only when the signed-in account has both an eligible non-client role and this permission;
- show Funding only when the account has operations-admin permissions and this permission;
- apply equivalent reactive route guards to list and detail routes;
- denied authenticated users are redirected to `/cases` without briefly rendering protected content;
- old Admin information routes remain removed; and
- desktop and collapsed/mobile navigation make the same decision.

Hide or neutralize embedded entry points when denied, including contact links/panels on cases and profiles and related contact/organization/funding links. A hidden link is not a substitute for the server checks in REQ-IMA-004.

Revocation must take effect on the user's next server request and after session restoration. The current browser may retain an already-rendered screen until navigation/refresh, but no subsequent protected API call may succeed.

### REQ-IMA-006 — Preserve the single best Contacts experience

Verify the contact consolidation from task 00028 while adding the gate:

- `/contacts` is the only contact list workspace;
- `/contacts/:id` is the only contact detail destination;
- there is no separate “People”/Admin contact tab or duplicated persistence model;
- category/tag filtering and management, communication history, structured contact fields, contact types, organization relationships, source/notes/do-not-contact state, properties, related cases, linked account state, archive behavior, and audit history remain available according to the intersection of role and the new permission; and
- internal links use canonical `/contacts` URLs, with no legacy `/admin/people*` routes.

Do not regress bounded/paginated loading, audit behavior, archive-not-delete semantics, linked-account ownership, or responsive behavior delivered by task 00028.

### REQ-IMA-007 — Documentation and terminology

Update concise current documentation and module comments that describe access as role-only. Product copy should use the exact permission label and continue to call the unified area **Contacts**. Historical completed task records do not need rewriting.

## Likely implementation areas

- A new migration after `0022_contact_directory.sql` for the user permission and existing-user backfill.
- `src/server_fns/users.rs`, `src/server/db/users.rs`, `src/mockdata.rs`, and `src/server/db/seed.rs` for the persisted/authenticated user shape.
- `src/server_fns/auth.rs` and `src/state.rs` for session restoration and reactive client visibility.
- `src/server/permissions.rs` and `src/components/guard.rs` for centralized server/client gates.
- `src/components/admin_user_card.rs` and the user audit path for site-admin management.
- Contact, organization, grant, funding, property, audit, and case-contact server-function modules for authoritative enforcement.
- Canonical route/navbar/contact files delivered by task 00028, plus case/profile contextual links and panels.

## Acceptance criteria

1. A site admin can grant and revoke exactly one switch on any user, including themselves; an operations admin cannot mutate it by direct API call.
2. Grants/revocations are visible in the target user's audit log with the correct actor, and no-op saves do not create duplicate entries.
3. Existing eligible users retain access after migration, existing clients remain denied, new registrations are denied, and seeded staff accounts can exercise the features.
4. A denied operations admin still reaches Admin → Cases and Admin → Users but sees no Contacts, Organizations, or Funding navigation and cannot open their canonical routes or call representative backing APIs.
5. A permitted operations/site admin receives the complete role-authorized Contacts, Organizations, grants, and Funding experience.
6. A permitted volunteer can use the volunteer-authorized portions of unified Contacts and Organizations but cannot gain Funding or admin-only mutations; a denied volunteer receives none of the three information areas.
7. A client receives none of these features even if the boolean is stored as granted.
8. Direct calls to contact, organization, grant, funding, scoped-audit, property, picker, and case-contact endpoints enforce both their prior authorization and the new permission.
9. `/contacts` and `/contacts/:id` remain the only contact workspace/detail experience; no duplicate People destination is reintroduced.
10. Login, MFA completion, session restoration, refresh, desktop nav, and mobile nav all reflect the same permission value.
11. Revocation blocks the user's next protected server call without requiring session deletion or password reset.

## Verification

- Run formatting checks and `etc/dev.sh -- cargo check --no-default-features --features ssr`.
- Run `etc/dev.sh build`, then `etc/dev.sh run`, and wait for `MH_READY`.
- Reset/seed the database and verify migration and fresh-seed defaults.
- Browser-check grant, denial, and self-recovery flows as a site admin.
- Browser-check permitted and denied sessions for site admin, operations admin, volunteer, and client roles, including refresh and a mobile viewport.
- Exercise the unified Contacts list/detail, Organizations list/detail, Funding list/grant detail, related pickers, case-contact panels, and scoped change logs.
- Attempt representative protected server functions directly as denied users to verify backend enforcement rather than navigation hiding alone.
- Confirm old Admin information URLs are absent and canonical routes enforce their guards.
- Do not add permanent Cargo tests; this repository does not use them.
