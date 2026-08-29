# Let a site admin deactivate an account

Duplicate accounts accumulate: someone signs up twice, or an account is created
for a person who already had one. There was no way to retire one. This adds a
**deactivated** account role: the account stops being a login, drops out of the
volunteer list and every directory section and picker, and lands in one
"Deactivated accounts" list where it stays findable and reversible.

## Why a role and not a boolean flag

The first draft of this proposed `users.deactivated BOOLEAN`. Reviewing it
against the code changed the decision, because nearly every role-aware query in
`src/server/db/` matches roles **positively**:

| Query | With a `deactivated` role |
| --- | --- |
| `volunteers_page` — `WHERE u.role = 'volunteer'` | the volunteer list excludes them, no code change |
| `directory_role_filter` | the three live sections exclude them; a fourth arm shows them |
| `settings::recipients_for_site_admins` / `_for_admins` | no notification email, no code change |
| `case_contacts` viewer check — `role IN ('operations_admin','site_admin')` | correctly denies |

A boolean would need a hand-added `AND NOT u.deactivated` in every one of those,
and each is a place a future query can forget. The role makes the default
correct.

It also inherits `users::set_role_in` wholesale: the `lock_role_changes_in`
advisory lock, the "you cannot demote the final site admin" guard, the
volunteer/client subtype reconciliation, and the role audit entry. None of that
had to be reimplemented.

**The cost, and how it is paid.** A fifth role is *fail-open* for predicates
written as "not a client", which meant "is staff". Every one was found and flipped
to the positive list that already existed as `has_volunteer_privileges()` /
`AccountRole::STAFF_ROLES_SQL`: `permissions::has_volunteer_access`,
`cases.rs` summary select, `channel_notifications` (insert and unread),
`messages` read receipts, `settings::recipients_for_case`, `state.rs`, and
`pages/cases.rs`. "Is staff" is now expressed one way everywhere.

**Losing the previous role.** `set_role_in` overwrites `users.role`, so the role
to restore is stored — the part `cases.status_before_withdrawal` plays for a
withdrawn case.

**The volunteer agreement survives.** Leaving Volunteer sets `volunteers.status`
to `revoked`; returning upserts it back to `approved` *without touching
`agreement_version`*. A deactivate/reactivate round-trip is lossless, verified
against seed volunteer Dana Patel: agreement version and all 23 case assignments
came back unchanged.

## Why a subtype table and not columns on `users`

The metadata (who, when, why, previous role) lives in `account_deactivations`,
keyed by user, not as four columns on `users`. This is the rule
`0015_volunteers_and_clients.sql` already states: `users` is the base record, and
data that exists *because* of a role belongs in a table keyed by the user it
extends. `users` is also the hottest table in the app — read on every
authenticated request through `resolve_by_session_token` — and those columns
would be dead on every row for a state that applies to a handful of accounts.

The hot paths never touch the new table: they only test `users.role`. It is read
in exactly two places, the admin card (`users::get`) and the Deactivated
accounts section.

## Decisions

- **Site admin only**, matching `set_user_role` and
  `set_information_management_access`. No `admin_requests` kind was added.
- **Not in the role dropdown.** `AccountRole::ASSIGNABLE` backs the two selects;
  `set_user_role` and `admin_requests::create_role` reject `Deactivated`. This
  mirrors `CaseStatus::Withdrawn`'s absence from the staff status dropdown, so
  the transition always carries who, when, and why.
- **You cannot deactivate yourself.** This is also what makes retiring the last
  site admin impossible: the actor must be a site admin and cannot be the target,
  so an active one always remains. The count guard in `set_role_in` is therefore
  unreachable *through this path*; it still guards ordinary demotions, and a
  deactivated ex-admin no longer props up its count (verified).
- **Sessions and trusted devices are dropped** in the same transaction, as
  `reset_password` already does.
- **The email stays claimed** — `users.email` is `UNIQUE` and `email_exists` is
  unchanged, so nobody re-registers over a retired duplicate.
- **Cases are not touched.** Task 00046 concluded a case with staff work on it
  belongs to the organization, not the account that filed it.

## Implementation

- `migrations/0030_account_deactivation.sql` — `account_deactivations` with an
  `account_deactivations_complete` CHECK, and a partial index on the live rows.
  A reactivated account keeps its row with `reactivated_at` set, so "was once
  deactivated" survives the restore.
- `server/db/deactivations.rs` — the new subtype module, beside `volunteers.rs`
  and `clients.rs`.
- `users::set_deactivated` — one transaction: lock, guard, delegate to
  `set_role_in`, write the record, drop sessions/devices, audit
  `account status`.
- `set_account_deactivated` server fn, the Account status card section, and the
  Deactivated accounts directory section (own `dw` query window).
- Seed: a tenth user, a deactivated duplicate of Jamie Rivera, so the restore
  path is demoable straight after `etc/dev.sh reset`.

## Cross-table invariant

"role `deactivated` ⟺ a live row here" cannot be a CHECK constraint. It is held
by `set_deactivated` being the single writer, exactly as 0015 holds
`role = 'volunteer'` ⟺ an approved `volunteers` row. The schema still guarantees
any row present is *complete*, so a deactivation is never unexplainable or
unrestorable.

## Verification

Manual, in the browser, all passing:

1. Seeded duplicate is absent from Clients, present under Deactivated accounts
   with its reason; sign-in refused with the deactivation message.
2. Deactivating volunteer Dana Patel removes her from Volunteers, the pending
   queue, and the owner/case-access/account-linking pickers; assignments still
   listed on her card; sessions cleared.
3. Reactivating restores role, agreement version, information grant, and all 23
   assignments; she reappears under Volunteers.
4. The Change Log shows both `role` and `account status` transitions with the
   right actor.
5. Self-deactivation refused; operations admin gets a read-only section and is
   refused by the server directly ("Site admin access required.").
6. `set_user_role` with `deactivated` refused; demoting the last active site
   admin still refused with a deactivated ex-admin present.

## Note for the reviewer

`target/site/pkg` in a dev checkout can be clobbered by `LEPTOS_*` environment
variables exported by another Leptos project in the same shell, which makes
`etc/dev.sh build` emit that project's asset names and serve a 404 stylesheet.
Unsetting them before `build`/`run` fixes it. Unrelated to this change, but it
cost time to spot.
