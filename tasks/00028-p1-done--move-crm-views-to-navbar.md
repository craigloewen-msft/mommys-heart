# Move Contacts, Organizations, and Funding from Admin to the main navbar

## Goal

Make the application’s top-level information architecture match these records’ long-term role: Contacts, Organizations, and Funding are first-class application areas, not Admin sub-workspaces.

The canonical destinations shall be:

- `/contacts` and `/contacts/:id`
- `/organizations` and `/organizations/:id`
- `/funding` and `/funding/:id`

Admin shall return to its operational purpose: Cases, Users, and Admin tools.

## Confirmed product decisions

1. **Contacts becomes one merged experience.** The existing navbar Contact Directory and Admin People workspace use the same `contacts` records. They shall be consolidated under Contacts rather than leaving two competing directories.
2. **Information management uses one explicit grant.** Permitted non-client accounts can fully manage Contacts, Organizations, grants, and Funding. Clients remain excluded; Admin dashboard access and case capabilities remain separate.
3. **Old Admin URLs are removed.** Existing `/admin/people`, `/admin/organizations`, and `/admin/funding` URLs may return the normal 404; clean canonical routing is preferred over compatibility redirects.

## Current-state findings

- `src/components/layout.rs` already puts `/contacts` in both desktop and mobile navigation for volunteer-capable roles.
- `src/pages/contacts.rs` is an outreach-oriented directory with category/tag filters, contact editing, and a communication log.
- `src/pages/people.rs` is a second view of the same records with structured CRM fields, type/organization/archive filters, properties, case relationships, account linking, and audit history. It is currently mounted through the Admin shell.
- Organizations and Funding are also mounted under `/admin`, and `AdminWorkspaceNav` presents all three as peers of Cases and Users.
- The unified information-management permission governs both reads and writes across Contacts, Organizations, grants, and Funding.
- Several case, profile, contact, organization, and funding links hard-code the old Admin paths.
- Adding two more direct links to the existing desktop navbar at its current breakpoint can cause crowding, so the expanded navigation needs an explicit responsive pass.

## Requirements

### REQ-NAV-001 — Canonical top-level routes

The router shall expose list and detail pages at the six canonical URLs above. Each page shall use the normal authenticated `Layout` and a title appropriate to its area, not an Admin heading or Admin workspace tab strip.

The application shall update every internal link to use canonical URLs, including links from:

- contact and organization directories;
- contact and organization detail panels;
- case-contact panels;
- user profiles;
- organization grant lists; and
- grant/funding list and detail Back actions.

### REQ-NAV-002 — Navbar visibility and responsive behavior

Both desktop and collapsed/mobile navigation shall show:

- **Contacts**, **Organizations**, and **Funding** to eligible non-client accounts with information-management access; and
- none of those entries to clients or denied accounts.

Active-link styling shall cover list and detail routes. The desktop navigation shall not wrap, overlap account controls, or clip at intermediate widths; use an appropriate collapse breakpoint or similarly clear responsive treatment. The collapsed menu shall close after navigation and remain keyboard usable.

### REQ-NAV-003 — Admin is only Cases and Users

Remove People, Organizations, and Funding from `AdminWorkspace`, `AdminWorkspaceNav`, and the Admin page composition. Keep:

- Cases and its pending-work badge;
- Users and its pending-work badge; and
- the existing Admin tools section.

Update the two-item Admin workspace layout and descriptive copy so it no longer claims to manage people or money there. `/admin` shall continue to default to `/admin/cases`, and the top-level Admin navbar badge shall continue to aggregate pending operational work.

### REQ-NAV-004 — One Contacts directory and detail experience

There shall be one user-facing Contacts area backed by the existing canonical `contacts` records. Do not retain separate Contact Directory and People destinations or duplicate records.

The merged experience shall preserve the useful behavior of both current views:

- keyword search;
- contact type, organization, archive-state, and category/tag filtering where authorized;
- accurate result counts and bounded/paginated loading rather than an unbounded browser load;
- category/tag display and management;
- structured contact identity and CRM fields;
- organization links, contact types, source, notes, and do-not-contact state;
- outreach/communication history;
- contact properties, related cases, linked account state, archive controls, and change history for roles already authorized to use them; and
- stable `/contacts/:id` detail links that can be refreshed and bookmarked.

Present one coherent directory/detail flow rather than stacking both existing pages unchanged. Extract or reorganize shared components and server projections where needed so there is one source of UI behavior per concern.

Permission-sensitive controls shall be honest: permitted volunteers receive full information-management controls. Case-linked actions still follow case capabilities, and login-account linking remains restricted to administrators because it exposes private account records.

The consolidation must not create a second contact persistence model. Categories and communications remain layers over the same `contacts` rows.

### REQ-NAV-005 — Organizations as a staff-readable top-level area

Create top-level Organization list and detail page wrappers guarded by information-management access. Permitted non-client accounts may create, edit, archive, restore, manage properties, connect people, inspect audit history, and work with related grants.

### REQ-NAV-006 — Funding as a top-level information area

Move the Funding/grant list and detail pages to `/funding` and `/funding/:id`, link them directly from the main navbar for permitted non-client accounts, and remove the Admin workspace tab strip. Preserve totals, forms, ledger records, voiding, audit history, and detail Back navigation.

### REQ-NAV-007 — Remove old Admin routes

Remove the old `/admin/people*`, `/admin/organizations*`, and `/admin/funding*` routes. Internal links shall use only the canonical navbar URLs; no redirect compatibility layer is required.

### REQ-NAV-008 — Preserve authorization and data invariants

Do not broaden server authorization as part of moving the UI. In particular:

- clients remain unable to read CRM records;
- organization-wide contact and organization reads remain staff-only;
- permitted non-client accounts may manage contact, organization, grant, and funding data;
- case relationship mutations continue to require their existing stored case capability; and
- login-account linking remains operations-admin-only.

No schema migration or data copy is expected. Existing audit, archive-not-delete, linked-account ownership, and void-not-delete behavior must remain unchanged.

### REQ-NAV-009 — Documentation and terminology

Update current product documentation and concise module comments that describe these views as living under Admin. Historical completed-task records do not need to be rewritten unless a current statement would otherwise misrepresent delivered behavior.

Use **Contacts** consistently in navigation and page headings; “person/people” may remain natural descriptive language inside contact and organization details.

## Likely implementation areas

- `src/app.rs`: canonical routes and removal of old Admin routes.
- `src/components/layout.rs`: role-aware desktop/mobile links and responsive breakpoint/layout.
- `src/pages/admin.rs`: remove the three non-operational workspaces and simplify Admin navigation.
- `src/pages/contacts.rs` and `src/pages/people.rs`: consolidate directory/detail behavior and route wrappers.
- `src/pages/organizations.rs`: top-level wrappers, canonical links, and explicit role-sensitive panels.
- `src/pages/funding.rs`: top-level wrappers, canonical links, and removal of Admin workspace chrome.
- `src/components/case_contacts.rs`, `src/pages/profile.rs`, and other cross-linking components: canonical Contact links.
- Contact-directory/server projection code only where needed to support one bounded, filterable merged directory; authorization checks must remain at least as strict as they are now.
- Current product documentation/comments describing Admin-only placement.

## Acceptance criteria

1. An admin sees direct Contacts, Organizations, Funding, and Admin entries in desktop and collapsed navigation; Admin itself contains only Cases and Users.
2. A permitted volunteer sees and can fully manage Contacts, Organizations, and Funding, but does not gain Admin dashboard access.
3. A client sees none of the three links and cannot open their canonical routes or call their backing APIs successfully.
4. `/contacts` is the only contact directory; `/contacts/:id` combines the current outreach/category/communication capabilities with authorized structured CRM detail, without a competing People workspace.
5. `/organizations` and `/organizations/:id` provide full management to permitted non-client accounts.
6. `/funding` and `/funding/:id` provide the complete grant/funding workflow to permitted non-client accounts without Admin tabs or Admin page titles.
7. The six old Admin list/detail routes are removed and may return the normal 404.
8. Internal links no longer send users through old Admin CRM URLs.
9. Direct authorization checks confirm that route movement did not grant any role new server-side data access.
10. At mobile, intermediate, and wide desktop widths, the navbar and all moved list/detail pages have no horizontal clipping or unreachable controls; active states and Back links are correct.

## Verification

- Run formatting checks and `etc/dev.sh -- cargo check --no-default-features --features ssr`.
- Run `etc/dev.sh build`, then `etc/dev.sh run`, and wait for `MH_READY`.
- Browser-check all canonical list/detail routes as a seeded site admin, operations admin, volunteer, and client.
- Exercise contact search/filter/pagination, category management, communication logging, structured admin detail, organization read/admin management, and grant/funding list/detail mutations.
- Call representative Contacts, Organizations, Grants, and Funding server functions directly for denied roles to verify backend boundaries, not just hidden navigation.
- Check desktop, intermediate, and mobile widths, keyboard navigation, active-link styling, Back behavior, and browser console errors.
- Do not add permanent Cargo tests; this repository does not use them.
