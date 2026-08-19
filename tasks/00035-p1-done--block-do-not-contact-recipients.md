# Fail-safe: block email to contacts marked "do not contact"

## Problem

`do_not_contact` is only honoured once, at selection time: the eligibility CTE in
`src/server/db/contact_mail.rs` (`candidate_cte`, `WHERE NOT c.archived AND NOT
c.do_not_contact`) filters candidates when a task is started. After that the
recipient list is frozen in `contact_mail_recipients` and the service
(`src/server/contact_mail.rs::run_task`) sends batches one per minute, so a large
task keeps sending for a long time. If someone marks a contact "do not contact"
(or archives them) while a task is in flight, that contact still gets the mail.

There is also no guard at the transport layer: `send_contact_batch` /
`send_standard_batch` will deliver to any address handed to them, so a bug or a
future caller can bypass the preference entirely.

## Plan

Add two independent layers, both server-side.

### 1. Re-verify each batch immediately before it is dispatched

- New repository fn in `src/server/db/contact_mail.rs`, e.g.
  `suppressed_contact_ids(contact_ids: &[String]) -> Result<HashSet<String>, sqlx::Error>`,
  selecting ids from `contacts` where `do_not_contact OR archived`.
- `contact_mail_recipients` stores `contact_id`, but `MailRecipientRecord` does
  not carry it; add `contact_id` to that struct (populated in `start_task`) so
  the service can re-check by id.
- In `run_task`, right after the batch is claimed and before
  `send_contact_batch`, drop suppressed recipients from the batch and mark them
  `failed` with the error `"Blocked: contact is marked do not contact."` (counts
  into `failed_count`, appears in the task's failure list). If the whole batch is
  suppressed, skip the send.
- If the suppression query itself errors, fail closed: skip the batch and record
  the recipients as failed with the query error, rather than sending.

### 2. Last-mile address guard in the batch transport

- In `src/server/email/communicationservice.rs::send_contact_batch`, before
  rendering/sending, look up the batch's addresses against contacts with
  `do_not_contact = true` (case-insensitive on trimmed effective email, matching
  the existing `EFFECTIVE_EMAIL` expression so linked-user addresses are covered).
- Any matching recipient is removed and returned as an `EmailBatchOutcome` with
  `Err("Blocked: recipient is marked do not contact.")`, and logged with
  `tracing::error!` (this layer firing means layer 1 was bypassed).
- Scope: this guard applies to the contact-mail batch path only. Transactional
  auth mail (OTP, verification, password reset in `auth_notifications.rs`) is
  deliberately untouched — it is account security mail, not outreach.

### Notes / non-goals

- No migration: reuse the existing `failed` recipient status plus an explicit
  error string, so no `contact_mail_recipients_status_check` change is needed.
- The UI copy on `src/pages/contact_mail.rs` already tells admins that
  do-not-contact entries are excluded; suppressed-mid-task recipients will now
  surface in the task failure list, which is the desired visible signal.

## Verification

1. `etc/dev.sh -- cargo check --no-default-features --features ssr`.
2. `etc/dev.sh build` then `etc/dev.sh run`; start a contact-mail task with
   enough recipients to span multiple batches, flag one of the later recipients
   "do not contact" while it runs, and confirm they are reported as blocked and
   never sent.
3. Confirm a normal task with no flagged recipients still completes unchanged,
   and that login OTP email still sends.
