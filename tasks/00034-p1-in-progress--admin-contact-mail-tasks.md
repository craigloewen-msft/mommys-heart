# Add cancellable, rate-scheduled contact mail tasks

## Goal

Add a dedicated **Send mail** page under Contacts where authorized operations and site administrators can select contact groups, compose one message, start a durable asynchronous send task, monitor its progress, and cancel its remaining work.

The campaign shall send one private email per recipient on a fixed one-minute schedule: at most 60 logical recipient emails per hour, with no catch-up bursts.

## Confirmed interpretation

1. The final role requirement governs the feature: both operations admins and site admins may view the page and use its actions. They must also retain the existing information-management grant because the page exposes contact records; no other role may reach the page or its server functions.
2. Only one contact-mail task may be active site-wide. This keeps the 60/hour campaign schedule deterministic and blocks every administrator from starting an overlapping campaign until the active task completes or reaches a terminal cancelled state.
3. “Specify a list” means selecting a one-time recipient group from existing Contacts, not creating a separate saved mailing-list entity.
4. The first recipient may be attempted immediately. Each later recipient receives its own `To` message at the next one-minute schedule boundary. Delays may make the task slower, but missed boundaries are skipped and never replayed as a burst.
5. Cancellation is cooperative and honest: it prevents the next recipient from starting. An email already handed to the provider cannot be recalled, so the task remains “Cancelling” until that in-flight attempt settles and then becomes “Cancelled.”
6. Progress counts a recipient as processed after its attempt finishes. The UI distinguishes provider-accepted sends from failed attempts; provider acceptance does not claim inbox delivery.

## Requirements

### REQ-MAIL-001 — Admin-only Contacts page

- Add a canonical `/contacts/mail` route using the normal application layout.
- Add a clear **Send mail** entry within the Contacts experience for users who have operations-admin permissions and information-management access.
- Guard the route reactively in the browser and independently enforce both authorization checks in every backing server function.
- Volunteers, clients, signed-out users, and administrators without the information-management grant shall not see contact-mail controls and shall receive no task or recipient data from direct API calls.
- Ensure the static mail route is not interpreted as `/contacts/:id`.

### REQ-MAIL-002 — Easy, bounded recipient selection

Provide an accessible recipient picker based on the canonical `contacts` records:

- search by contact name, organization, or email;
- filter by contact type, organization, and existing category/tag groups;
- select or clear individual contacts;
- select/clear the visible page and select all eligible matches without requiring the administrator to manually load every result;
- preserve selection while paging/filtering and show the selected total and a concise selection summary;
- provide an explicit review/confirmation step showing recipient count and estimated minimum duration before launch.

Only currently eligible contacts may enter the task snapshot:

- active, non-archived contacts;
- `do_not_contact = false`;
- a non-empty, minimally valid effective email address, using the linked account’s authoritative email where applicable;
- one recipient per case-insensitive email address, deterministically deduplicated.

The server shall resolve and revalidate the submitted selection when creating the task. It must reject an empty or stale/invalid selection with readable feedback rather than trusting browser-provided names or email addresses.

### REQ-MAIL-003 — Safe message composition

- Require a non-empty, length-bounded subject and message body.
- Show live validation and disable launch while inputs or recipient selection are invalid.
- Render the administrator’s message through a dedicated branded email template with matching HTML and plain-text forms.
- Treat composed text as text, not trusted HTML; escape it before producing HTML while preserving readable paragraphs/line breaks.
- Send a separate direct email to each snapshotted recipient so addresses are never disclosed to other recipients.

### REQ-MAIL-004 — Durable task and recipient snapshot

Add a migration for a durable contact-mail task and its ordered recipient snapshot. Persist enough data to display and safely resume the operation, including:

- task id, status, subject/body, creator id and display-name snapshot;
- original recipient total, accepted/failed counts, timestamps, next scheduled send time, cancellation metadata, and a task-level error when applicable;
- ordered recipient rows with the source contact id when available, name/email snapshots, per-recipient state, attempt timestamps, and final error detail;
- database constraints for valid statuses/counts and a partial unique index that permits only one queued/running/cancelling task site-wide.

Task creation and recipient snapshotting shall be one transaction. Contact edits made afterward shall not silently retarget or rewrite an active campaign.

### REQ-MAIL-005 — Maintainable Tokio task runner

Implement the operation as a cohesive Rust task type/service (a readable Rust `struct`, the language’s equivalent of the requested class), separate from Leptos handlers and database query details. Its responsibilities shall be explicit: claim/resume a task, wait for its next slot, observe cancellation, send one recipient, record the outcome, advance the schedule, and finalize.

- Start the runner with `tokio::spawn` after database initialization.
- Wake it when work is created and recover any non-terminal task after process restart.
- Coordinate through PostgreSQL so only one application worker can own the campaign sender if more than one process is running.
- Before provider I/O, durably mark the recipient attempt as started.
- If the process restarts with an indeterminate in-flight recipient, do not resend it and risk a duplicate; record it as an interrupted/unknown failure, then continue the remaining schedule.
- Keep database locks out of network waits and one-minute sleeps.
- Log task lifecycle and send failures without logging message bodies or unnecessary recipient data.

### REQ-MAIL-006 — Fixed 60/hour campaign schedule

- Anchor the schedule to task start and allow at most one new logical recipient attempt per 60 seconds.
- Send sequentially; never batch recipients into BCC and never process recipients concurrently.
- Skip elapsed schedule boundaries after a slow provider call, retry, server pause, or restart. Never catch up with back-to-back sends.
- Continue using the shared ACS transport and its existing retry/error classification.
- Preserve the lower-level process-wide 90-standard-emails/hour safety guard and OTP priority. That guard or provider latency may slow this campaign but may never make the campaign exceed its own one-per-minute limit.
- Count transport retries as part of the same logical recipient attempt, not as additional campaign recipients.
- In configured email dry-run mode, exercise the task/progress flow without contacting real mailboxes. If neither real email nor dry-run delivery is configured, reject task creation instead of creating stuck work.

### REQ-MAIL-007 — Live progress and send blocking

The page shall poll/reload the active or most recent task while it is non-terminal and display:

- queued, running, cancelling, completed, cancelled, or failed state;
- creator, subject, start/completion times, and original recipient total;
- a labelled accessible progress bar based on processed recipients out of the original total;
- accepted, failed, remaining, and cancelled/not-attempted counts as appropriate;
- the next planned send time and a useful estimate of remaining minimum duration while running;
- readable failed-recipient details without exposing them outside the authorized page.

While a queued, running, or cancelling task exists, disable the recipient/composer launch flow and enforce the same block transactionally on the server. A stale browser or concurrent administrator must not bypass it. Once the task reaches completed, cancelled, or failed, a new campaign may be created.

### REQ-MAIL-008 — Cancellation and failures

- Show a confirmation before requesting cancellation.
- Permit any authorized operations/site admin to cancel the active task and record who requested it and when.
- Make repeated or concurrent cancellation requests idempotent.
- Wake a sleeping worker promptly; stop before claiming the next pending recipient.
- Do not represent already accepted or failed recipient attempts as cancelled.
- Continue after an individual permanent or exhausted-retry failure, preserve its error on the recipient row, increment failed progress, and use the existing durable `email_failures` log with contact-mail task context.
- Reserve task-level `failed` for an unrecoverable runner/persistence problem. A task that processes every recipient with some individual failures is terminal “Completed” with a nonzero failed count.

## Likely implementation areas

- `migrations/0026_contact_mail_tasks.sql`
- `src/app.rs`, `src/lib.rs`
- `src/pages/contacts.rs` and a focused new contact-mail page/component
- `src/components/guard.rs`
- `src/server_fns/mod.rs` and a new contact-mail server-function module
- `src/server/db/mod.rs` and a new contact-mail repository
- `src/server/mod.rs` and a new task-runner/service module
- `src/server/email/templates.rs`
- `src/main.rs`

Keep the task runner, persistence, server-function authorization, and page state in separate modules. Do not put a long-running loop inside a request handler or make the browser responsible for timing sends.

## Acceptance scenarios

1. A site admin and an operations admin with information access can open `/contacts/mail`; a volunteer, client, signed-out user, or information-denied admin cannot see the entry or call its APIs.
2. An admin can combine search/type/organization/category filters, select individual or all matching eligible contacts across pages, clear selections, and see an accurate selected total.
3. Archived, do-not-contact, empty/invalid-address, duplicate-address, and newly ineligible contacts cannot enter the persisted recipient snapshot even if a stale client submits them.
4. Starting a campaign returns promptly, creates one durable task, snapshots direct recipients, sends the first recipient, and advances accepted/failed progress in the UI.
5. With a local fake mail service, recipient attempt start times are at least 60 seconds apart. Slow sends and a simulated pause skip missed slots and never produce a catch-up burst.
6. A second launch from the same or another admin is rejected while the first task is queued/running/cancelling, including simultaneous submissions; it becomes possible after a terminal state.
7. Cancellation during the minute wait wakes promptly and sends nobody else. Cancellation during provider I/O lets only that in-flight attempt settle, then marks the rest unattempted and the task cancelled.
8. An individual provider failure is visible, recorded in `email_failures`, and does not prevent later recipients from following their scheduled slots.
9. Restarting the app resumes remaining work. A recipient left in an indeterminate sending state is reported as interrupted rather than sent twice.
10. Every recipient receives a separate `To` email with the exact composed subject and equivalent escaped branded HTML/plain text; no recipient can see another recipient’s address.
11. Direct API attempts, stale tabs, malformed inputs, unknown contacts, and repeated cancellation requests fail safely with readable results and no authorization or concurrency bypass.
12. Desktop and mobile layouts remain usable by keyboard, progress is announced accessibly, controls explain why they are disabled, and browser console logs remain clean.

## Verification

- `cargo fmt --all -- --check`
- `etc/dev.sh -- cargo check --no-default-features --features ssr`
- `etc/dev.sh build`, then `etc/dev.sh run`, waiting for `MH_READY`
- Browser-check selection, composition, launch blocking, progress polling, completion, failure, and cancellation at desktop and mobile widths
- Exercise authorization with seeded site-admin, operations-admin, volunteer, and client sessions, including direct server-function calls
- Use a local fake ACS endpoint and temporary accelerated scheduler checks to verify ordering, no overlap, failure continuation, cancellation, restart recovery, and no duplicate recovery; remove temporary tests before review
- Inspect persisted task/recipient rows and the existing email-failure log for representative success, failure, cancellation, and restart cases
- Do not add permanent Cargo tests; this repository does not use them

## GPT-OSS evaluation note

The requested `ollama/gpt-oss:120b` model was tried for three bounded repository-exploration tasks (contacts/auth, mail infrastructure, and task persistence). The platform rejected each attempt before inference because the parent session’s `xhigh` reasoning effort is unsupported by that model. It therefore supplied no findings and was not useful for this task; the repository analysis and this plan were completed directly.
