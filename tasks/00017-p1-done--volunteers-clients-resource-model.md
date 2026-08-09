# Make `volunteers`/`clients` a sound resource model

Follow-up to the volunteer-application work on this branch. The subtype tables
landed, but the invariant that ties them to `users.role` is asserted nowhere and
is violated in practice. This makes the model actually hold.

## The defects being fixed

1. **Subtype rows are missing on a fresh database.** `dev-db.sh reset` runs
   migrations against an empty DB (the backfill matches nothing), then `seed.rs`
   inserts users directly. Result on a clean seed: 3 volunteer-role users, 4
   client-role users, **0 rows in both subtype tables**.
2. **Three separate write paths mutate `users.role` without maintaining the
   subtype tables** — `users::set_role`, `admin_requests::decide` (pre-existing),
   and `volunteers::decide` (mine). Verified contradiction:
   `users.role=client but volunteers.status=approved`. Promoting a client via the
   role dropdown is a dead end: they get the role with no `volunteers` row, so
   the profile reads "Outstanding" forever and `apply_to_volunteer` refuses them
   with "Your account already has volunteer access" — unreachable state.
3. **Error handling diverges from `admin_requests`**, the closest sibling: it has
   an `Error` enum whose `Display` gives user-facing sentences; mine leaks
   `sqlx::Error::Protocol("already_decided")` to the UI on a genuine race.
4. **Dead code**: `clients::ensure`, `clients::exists`, the
   `pending_volunteer_application_count` server fn (zero callers), and
   `VolunteerStatus::slug` (written, then bypassed by hardcoded string literals).
5. **N+1 in `list_pending`**: 1+2N queries, materialising a full `User` with
   `assigned_cases` for a queue that renders name and email.

## The model, stated explicitly

`users.role` stays the **single source of truth for access**. The subtype tables
hold what is true *because* of that role — for volunteers, the agreement and
application lifecycle. Two invariants, written down and enforced in one place:

- **A:** `users.role = 'volunteer'` ⟺ a `volunteers` row exists with
  `status = 'approved'`
- **B:** `users.role = 'client'` ⟹ a `clients` row exists

B is one-way on purpose: `clients` carries no status, so a retained row for
someone who has moved on is history, not a contradiction.

A needs a way to express "was approved, isn't a volunteer now", so `status` gains
a **`revoked`** variant. Demotion revokes rather than deleting, because the
agreement they accepted is a real historical fact an admin may need to read. A
revoked user may accept the agreement again, exactly like a denied one.

## Changes

### One choke point for role changes

New `users::apply_role_in(tx, user_id, role, actor_name)` — the only code that
may change a role. In one transaction it locks the user row, no-ops when
unchanged, updates `users.role`, reconciles the subtype tables per A and B, and
writes the audit entry.

All three existing writers are routed through it:

| Caller | Today | After |
| --- | --- | --- |
| `users::set_role` | bare UPDATE + audit | wraps `apply_role_in` in a tx |
| `admin_requests::decide` (approve role request) | own UPDATE + own audit | calls `apply_role_in` |
| `volunteers::decide` (approve application) | own UPDATE + own audit | calls `apply_role_in` |

That collapses three copies of "change a role" into one and fixes the
pre-existing `admin_requests` path as a side effect.

### Seed satisfies the invariant

`seed.rs` creates the matching subtype row for each seeded user, so a fresh
database is consistent from the start and the migration backfill becomes a
repair for existing databases rather than the only mechanism.

### Migration

Edit `0015_volunteers_and_clients.sql` in place rather than adding a migration to
fix a migration in the same unmerged PR: add `revoked` to the CHECK constraint,
and make the backfill idempotent (`ON CONFLICT DO NOTHING`) so it repairs an
existing database. I will reset my dev DB so the checksum change is a non-issue.

### Error handling

`server/db/volunteers.rs` gets an `Error` enum modelled on `admin_requests`:
`Database`, `NotFound`, `AlreadyDecided`, with `Display` giving sentences a user
should see ("This application has already been decided.") and
`From<sqlx::Error>`.

### Cleanup

- Delete `clients::ensure`, `clients::exists`, `pending_volunteer_application_count`.
- Use `VolunteerStatus::slug()` in `decide()` instead of hardcoded literals.
- `list_pending` becomes a single `JOIN` returning a narrow row (id, name, email,
  agreed_at) rather than 1+2N queries and a full `User`.

## Outcome

Implemented as planned, plus one case the plan surfaced while building: someone
who holds the volunteer role but has never signed (promoted directly, or
predating the agreement) needs a way to sign. `apply_to_volunteer` therefore
treats acceptance from an existing volunteer as *recording the agreement*, not
as filing an application — otherwise signing would demote them into a review
queue. Their profile shows a distinct "we don't have your signed agreement"
prompt instead of "Become a volunteer".

`etc/check-role-integrity.sh` is the guard against regressions. It reported all
seven violations on the old code and is clean on every path now.

## Verification

- `cargo check`, `cargo fmt`, no new clippy warnings.
- **Integrity check query** asserting zero role/subtype mismatches, run against a
  freshly reset database and again after exercising every transition. This is the
  class of bug that silently reappears, so the query goes in the handoff notes:

  ```sql
  SELECT 'volunteer role without approved record' AS problem, u.id FROM users u
  LEFT JOIN volunteers v ON v.user_id = u.id
  WHERE u.role = 'volunteer' AND (v.user_id IS NULL OR v.status <> 'approved')
  UNION ALL
  SELECT 'approved record without volunteer role', v.user_id FROM volunteers v
  JOIN users u ON u.id = v.user_id
  WHERE v.status = 'approved' AND u.role <> 'volunteer'
  UNION ALL
  SELECT 'client role without client record', u.id FROM users u
  LEFT JOIN clients c ON c.user_id = u.id
  WHERE u.role = 'client' AND c.user_id IS NULL;
  ```

- In-browser, each transition, checking the invariant after each: promote a
  client to volunteer via the **role dropdown** (previously the dead end — must
  now produce an approved record, and the agreement must still be acceptable
  afterwards to replace the empty version); demote a volunteer to client (must
  revoke, not contradict); approve and deny an application; approve a role
  request through the admin-requests path.
