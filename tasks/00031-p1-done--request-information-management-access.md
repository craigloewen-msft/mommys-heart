# Let operations admins request information-management access

## Goal

Allow an operations admin to submit the existing site-admin approval workflow when an eligible user needs the single **Information access** grant that unlocks Contacts, Organizations, and Funding.

This changes the earlier decision in task 00029 that this setting had no request path. The permission itself, its protected views, and its authorization semantics remain unchanged.

## Scope and behavior

- Add **Information access** as a third administrative request kind alongside role changes and case permissions.
- An operations admin may request the grant for an eligible non-client user, including themselves. They cannot approve their own request or directly mutate the grant.
- This request path grants denied access only. Revocation remains a direct site-admin action rather than an operations-admin request.
- A site admin continues to grant or revoke the setting directly from a user card.
- Approval atomically grants the existing `information_management_access` value and records the existing user audit entry with the deciding site admin as actor.
- Denial resolves the request without changing the user.
- Reject no-op, duplicate pending, invalid-target, and stale requests. If the target's setting changes after filing, approval must fail safely and require a fresh request.
- Preserve the rule that clients cannot gain effective Contacts, Organizations, or Funding access; do not create ineffective grant requests for client accounts.

## Implementation plan

### 1. Extend request persistence

Add a migration after `0023_information_management_access.sql` that:

- admits an `information_access` request kind;
- stores the current and requested information-access booleans;
- updates the per-kind row-shape constraint; and
- adds a partial unique index preventing more than one pending information-access request per target user.

Existing role and case-permission requests and indexes must remain valid.

### 2. Extend the request domain and server API

Update `AdminRequestKind`, `AdminRequest`, row conversion, labels, summaries, filtering, and request counts for the new kind.

Add a server function for an operations admin to request information access. It shall validate the target, note length, role eligibility, current denial, and duplicate-pending state before creating the request and sending the standard filed-request notification.

Extend the atomic decision path so approval:

1. locks the pending request and target user;
2. confirms the stored access value still matches the request snapshot;
3. grants access through a transaction-aware form of the existing user mutation/audit logic; and
4. resolves the request in the same transaction.

The ordinary site-admin-only `set_information_management_access` endpoint remains authoritative for direct changes.

### 3. Add the operations-admin user-card control

In the existing **Information access** section of Admin → Users:

- retain the current direct toggle for site admins;
- for an operations admin viewing a denied eligible user, show an optional request note and a clear **Request access** action;
- show submission progress and success/error feedback;
- refresh request badges after submission; and
- keep granted users and ineligible clients read-only, with accurate explanatory text.

Do not add another permission or separate Contacts, Organizations, and Funding toggles.

### 4. Integrate review queues, history, and badges

Use the shared `AdminRequestCenter` for an Information access queue/history in Admin → Users and the combined request-center component if it remains in use.

Extend:

- request-center headings, loading/empty copy, cards, and change summaries;
- app badge loading/state/reset logic;
- Admin navbar and Users-workspace attention totals; and
- standard filed/decided email rendering through the generic request kind and summary APIs.

Site admins see all pending information-access requests; operations admins see only requests they filed, matching the existing visibility rules.

### 5. Keep authorization boundaries unchanged

This task does not alter route guards or information-domain server permissions. Access begins only after approval changes the existing stored grant. Operations admins must not be able to bypass approval by calling either mutation endpoint directly.

## Acceptance criteria

1. An operations admin can submit a grant request for a denied, non-client target user, including themselves, with an optional note.
2. Direct operations-admin calls to the site-admin grant/revoke endpoint remain forbidden.
3. A site admin sees the request in Admin → Users, receives the existing request notification (subject to notification settings), and can approve or deny it with a decision note.
4. Approval grants the existing single Information access setting and creates exactly one target-user audit entry naming the deciding site admin; denial creates no grant audit entry.
5. The requester sees the result in request history and receives the standard decision notification.
6. Pending badges and Users-workspace attention counts include information-access requests and update after filing or deciding.
7. Duplicate pending, already-granted, client-target, missing-target, already-resolved, and stale requests fail clearly without partial changes.
8. Existing role and case-permission request behavior is unchanged.
9. Contacts, Organizations, and Funding remain inaccessible until approval and become available through the existing permission gates after approval/session refresh.
10. Site admins retain direct grant and revoke controls.

## Follow-up: account permission emails

- Generalize the existing **Assigned to a case** email preference to **Account permissions changed**, preserving each user's stored choice.
- Use that preference for case-assignment, account-role, volunteer-approval, and information-access change emails.
- Send the affected user an **Account permissions changed** email when information access is granted or revoked, including changes approved through an administrative request.
- Do not add another notification category.

## Verification

- Run formatting checks and `etc/dev.sh -- cargo check --no-default-features --features ssr`.
- Run `etc/dev.sh build`, then `etc/dev.sh run`, and wait for `MH_READY`.
- Reset/seed the database and verify the new migration applies cleanly.
- Browser-check operations-admin submission for another user and self, request notes, duplicate prevention, badges, queue visibility, history, approval, denial, and site-admin direct revoke.
- Verify a client target is rejected and a direct operations-admin mutation call is forbidden.
- Verify stale approval fails after a site admin directly changes the target setting.
- Confirm approved access unlocks the existing Contacts, Organizations, and Funding navigation/routes without changing account role or case permissions.
- Do not add permanent Cargo tests; this repository does not use them.
