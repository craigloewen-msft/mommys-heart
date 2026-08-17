# Fix volunteer editing on Contacts, Funding, and Organizations

## Goal

A volunteer with **Can manage contacts, organizations, and funding information** shall be able to open an existing record, click **Edit**, change its editable fields, save, and see the persisted values after reload.

This covers:

- the structured contact form on `/contacts/:id`;
- contact custom properties;
- the grant form reached from `/funding/:id` (the Funding page's editable record);
- the structured organization form on `/organizations/:id`; and
- organization custom properties.

## Findings

The routes and backing contact, grant, organization, and property mutation server functions already use the shared information-management permission. A permitted volunteer is intentionally authorized to perform these writes. The defect is therefore expected in the client-side edit/form lifecycle rather than a reason to restore an operations-admin restriction.

The three detail views use closely related reactive `Edit`/`Show` patterns and construct drafts from asynchronously loaded records. The fix should keep each opened editor stable, initialize it from the selected record once, and avoid reactive reconstruction/resetting while the volunteer types.

## Requirements

1. A permitted volunteer can enter edit mode on an existing contact, grant, and organization.
2. Every field that is editable for an administrator is also editable for that volunteer under the shared information-management grant.
3. Typing into one field does not reset that field or any other draft field.
4. Saving calls the existing mutation path, exits edit mode only after success, refreshes the displayed record, and survives a full page reload.
5. Validation and server errors remain visible without discarding the user's draft.
6. Cancel exits edit mode without persisting changes; reopening starts from the currently saved record.
7. Contact and organization custom-property rows can be changed, added, removed, cancelled, and saved by a permitted volunteer.
8. Linked-account identity fields on contacts remain read-only as designed. This fix must not expose private account projections or broaden contact-to-account linking.
9. A volunteer without the information-management grant and every client remain denied by both route guards and server functions.
10. Existing administrator editing, archive/restore behavior, audit records, and picker behavior must not regress.

## Implementation plan

1. Reproduce the issue with a seeded permitted volunteer on all three canonical detail routes. Inspect browser console and mutation requests to distinguish an edit-state, controlled-input, remount/reset, or submit failure.
2. Refactor the affected detail editor mounting/draft initialization to use stable component ownership and explicit begin-edit initialization rather than rebuilding editor state from reactive parent rendering.
3. Apply the same lifecycle correction to contact and organization property editors if reproduction confirms they share the failure.
4. Keep the existing centralized information-management authorization on all reads and writes; do not add role-only exceptions.
5. Ensure failures leave the editor open with its draft intact and successful saves reload authoritative server data.

## Verification

- Run formatting checks and `etc/dev.sh -- cargo check --no-default-features --features ssr`.
- Run `etc/dev.sh build`, then `etc/dev.sh run`, and wait for `MH_READY`.
- Sign in as seeded volunteer `dana@mommysheart.org` / `volunteer123` and verify edit, cancel, validation failure, successful save, and full-page persistence for a contact, grant, and organization.
- Verify contact and organization custom-property edit/add/remove/save flows as the volunteer.
- Verify a linked contact's account-owned fields remain locked while its CRM-owned fields remain editable.
- Smoke-test the same edits as an administrator.
- Revoke the volunteer's information-management grant and verify the canonical routes and representative direct mutation calls are denied; verify a client is also denied.
- Do not add permanent Cargo tests; this repository does not use them.

## Acceptance criteria

- A permitted volunteer can edit and persist existing contact, grant, and organization data from the canonical pages.
- Editable inputs retain typed values until save or cancel.
- Contact and organization custom properties are editable and persistent for that volunteer.
- Permission boundaries and linked-account field ownership remain unchanged.
- The app compiles and the browser verification above passes without console errors.
