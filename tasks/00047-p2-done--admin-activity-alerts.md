# Admin activity alerts

Give administrators a way to see the work happening across the site: new cases,
case notes, information edits, documents, and contact changes.

## Problem

The client asked for an alert covering "whenever someone creates a new case,
case note, intake summary, uploads or links a zoom interview, adds or edits
contacts, or uploads any new information or documents to a client profile".

Every existing `NotificationKind` sends **one email per event**, and its
recipients are the people *on the case*. Neither fits: that list is the busiest
set of actions in the app (dozens of emails a day per admin, so the first thing
an admin would do is turn it off), and half of it — contacts, organizations — is
not case-scoped at all.

## Approach

A seventh notification category, `AdminActivity`, delivered as **a feed plus a
daily digest** to **site and operations admins**.

The first implementation of this task wrote every event to its own
`admin_activity_events` table. That was wrong, and review caught it: ~80% of
those rows duplicated a fact `audit_log` already recorded, which meant two write
paths for the same event and two things to keep in step.

The justification given at the time — "a digest needs a per-row *already mailed*
marker, which must not go into an append-only audit table" — does not hold. A
single **watermark** (the last `seq` mailed from each log) does the same job
without ever writing to the audit trail.

So the feed is now a **read** over `audit_log`, unioned with a content-free
projection of `case_note_audit_log` (notes stay in their restricted table per
ADR-0003; the feed just reads both). The only state owned here is one watermark
row.

## What shipped

- `migrations/0030_admin_activity_alerts.sql` — the `notification_admin_activity`
  column, the one-row `admin_activity_digest_state` watermark (seeded to the
  current end of each log so the first digest is not a replay of all history),
  and `seq DESC` indexes on both logs, which neither had.
- `server_fns/admin_activity.rs` — `AdminActivityCategory::classify`, the single
  mapping from `(entity_type, field)` onto a category or `None`. This is the
  entire difference between the Change Log and this feed.
- `server/db/audit.rs` — the union query, the subject-name join, `summarize()`
  (field-level audit row → prose), and the watermark accessors. These live in
  the audit module rather than their own file because they are reads over the
  audit log; a separate `db/admin_activity.rs` implied a store that does not
  exist.
- `server/db/cases.rs` — **a `case / created` audit entry**, which did not exist.
- `server/notifications.rs` — the daily digest task, reading `since(watermark)`.
- `server/email/templates.rs`, `components/admin_activity.rs`,
  `pages/admin.rs` — the digest email and the Activity tab.

## Decisions worth remembering

- **Noise is filtered, not shown.** `user`-entity rows (role changes, capability
  grants, information-access decisions) and chat rows are excluded: they are
  account administration and messaging, each with its own notification category.
  Verified against seed data: 10 audit rows, 6 classified, 4 correctly dropped.
- **The watermark is clamped to each log's end on read.** `etc/dev.sh reset`
  truncates with `RESTART IDENTITY`, which would otherwise leave the watermark
  past every row and silently mail nothing forever.
- **Names are joined at read time**, so a renamed case shows its current name.
  The previous snapshot-based version would have shown the old name forever.
- **Zoom interviews have no first-class record.** Covered through the Document
  and Case note categories; a dedicated "paste a recording link" field would be
  separate work.
- **Document events are dormant** while `evidence::ensure_evidence_available()`
  refuses every evidence surface. They classify correctly the moment it returns.

## Not done

No per-admin unread badge (needs read state, a second table like
`channel_notifications`), no per-category subscription, no digest for
non-administrators. A feed request scans a bounded window of recent history
rather than paginating the full audit log.
