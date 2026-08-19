# Add cancellable, rate-scheduled contact mail tasks

## Goal

Add a dedicated **Send mail** page under Contacts where authorized operations and site administrators can select contact groups, compose one message, start a process-local asynchronous send task, monitor its progress, and cancel its remaining work.

The campaign shall submit hidden-recipient batches on a fixed hourly schedule: at most 60 contacts per hour, with no catch-up bursts. The shared ACS mail service splits each hourly group at its 50-recipient provider limit.

## Confirmed interpretation

1. The final role requirement governs the feature: both operations admins and site admins may view the page and use its actions. They must also retain the existing information-management grant because the page exposes contact records; no other role may reach the page or its server functions.
2. Only one contact-mail task may be active in the running application process. The task service enforces this in memory and blocks every administrator from starting an overlapping campaign until the active task completes or reaches a terminal cancelled state.
3. “Specify a list” means selecting a one-time recipient group from existing Contacts, not creating a separate saved mailing-list entity.
4. The first group of up to 60 contacts may be attempted immediately. Each later group starts at the next hourly schedule boundary. Delays may make the task slower, but missed boundaries are skipped and never replayed as a burst.
5. Cancellation is cooperative and honest: it prevents the next hourly group from starting. A batch already handed to the lower mail service cannot be recalled, so the task remains “Cancelling” until that in-flight group settles and then becomes “Cancelled.”
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
- Use the shared lower-level batch path and BCC so recipient addresses are never disclosed to other recipients.

### REQ-MAIL-004 — Start and final history records

Add a migration for a contact-mail task and its ordered recipient snapshot.

- Commit the task and immutable recipient snapshot once when sending starts.
- Keep live progress, cancellation, and scheduling in the application process.
- Commit the terminal task and per-recipient outcomes once when sending completes, is cancelled, or fails.
- Store task id, status, subject/body, creator identity snapshot, recipient counts, timestamps, cancellation metadata, and final errors.
- Store ordered recipients with contact id, name/email snapshots, attempt timestamps, status, and final error detail.
- Do not use a database unique index, lease, polling loop, or per-recipient progress writes to coordinate active work.
- Contact edits made after start shall not silently retarget the snapshotted recipients.

### REQ-MAIL-005 — Maintainable in-memory Tokio task service

Implement the operation as a cohesive Rust task service, separate from Leptos handlers and database query details.

- Expose small start, current-task, and cancel operations around one process-local active task.
- Spawn the send operation with `tokio::spawn` when an administrator starts it; do not run an idle background polling loop.
- Keep live progress under Tokio synchronization and use cancellation notification to wake a scheduled wait promptly.
- Keep database work out of provider calls and hourly sleeps.
- On startup, close a start record left active by a prior process as interrupted/failed; do not resend it.
- Retry the one terminal history write until it commits.
- Log lifecycle and failures without logging message bodies or recipient addresses.

### REQ-MAIL-006 — Fixed 60/hour campaign schedule

- Anchor the schedule to task start and submit at most 60 contacts per hourly boundary.
- Submit one task group at a time. The lower mail service shall split it into BCC requests of at most the ACS provider limit and return one keyed outcome per contact.
- Skip elapsed schedule boundaries after a slow provider call or retry. Never catch up with back-to-back groups.
- Keep configuration, branded rendering, dry-run behavior, ACS chunking, transport retries, and durable failure logging in the shared lower email service.
- Preserve the lower-level process-wide standard-email safety guard and OTP priority.
- Count transport retries as part of the same provider batch, not as additional task contacts.
- In configured email dry-run mode, exercise the task/progress flow without contacting real mailboxes. When email is unconfigured, let the lower mail service return failed outcomes for the submitted contacts.

### REQ-MAIL-007 — Live progress and send blocking

The page shall poll/reload the active or most recent task while it is non-terminal and display:

- queued, running, cancelling, completed, cancelled, or failed state;
- creator, subject, start/completion times, and original recipient total;
- a labelled accessible progress bar based on processed recipients out of the original total;
- accepted, failed, remaining, and cancelled/not-attempted counts as appropriate;
- the next planned send time and a useful estimate of remaining minimum duration while running;
- readable failed-recipient details without exposing them outside the authorized page.

While a queued, running, or cancelling task exists in the process-local task service, disable the recipient/composer launch flow and enforce the same block atomically in that service. A stale browser or concurrent administrator request in the same application process must not bypass it. Once the task reaches completed, cancelled, or failed, a new campaign may be created.

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
4. Starting a campaign returns promptly, commits its start history and recipient snapshot, submits the first group, and advances accepted/failed progress in the UI from memory.
5. With a local fake mail service, a 60-contact task group is split into provider-safe batches of 50 and 10, and task groups beyond the first start on hourly boundaries. Slow sends skip missed slots and never produce a catch-up burst.
6. A second launch from the same or another admin through the same application process is rejected while the first task is queued/running/cancelling, including simultaneous submissions; it becomes possible after a terminal state.
7. Cancellation during the hourly wait wakes promptly and submits no later group. Cancellation during provider I/O lets the in-flight group settle, then marks the rest unattempted and the task cancelled.
8. An individual provider failure is visible, recorded in `email_failures`, and does not prevent later recipients from following their scheduled slots.
9. Restarting the app does not resend in-memory work. A start record left active by the prior process is closed as interrupted/failed so duplicate sends are not attempted.
10. Every recipient receives the exact composed subject and equivalent escaped branded HTML/plain text through hidden BCC batches; no recipient can see another recipient’s address.
11. Direct API attempts, stale tabs, malformed inputs, unknown contacts, and repeated cancellation requests fail safely with readable results and no authorization or concurrency bypass.
12. Desktop and mobile layouts remain usable by keyboard, progress is announced accessibly, controls explain why they are disabled, and browser console logs remain clean.

## Verification

- `cargo fmt --all -- --check`
- `etc/dev.sh -- cargo check --no-default-features --features ssr`
- `etc/dev.sh build`, then `etc/dev.sh run`, waiting for `MH_READY`
- Browser-check selection, composition, launch blocking, progress polling, completion, failure, and cancellation at desktop and mobile widths
- Exercise authorization with seeded site-admin, operations-admin, volunteer, and client sessions, including direct server-function calls
- Use dry-run/fake delivery and temporary checks to verify ordering, in-process overlap prevention, failure continuation, cancellation, start/final history writes, and interrupted-process cleanup; remove temporary tests before review
- Inspect persisted task/recipient rows and the existing email-failure log for representative success, failure, cancellation, and restart cases
- Do not add permanent Cargo tests; this repository does not use them

## GPT-OSS evaluation note

The requested `ollama/gpt-oss:120b` model was tried for three bounded repository-exploration tasks (contacts/auth, mail infrastructure, and task persistence). The platform rejected each attempt before inference because the parent session’s `xhigh` reasoning effort is unsupported by that model. It therefore supplied no findings and was not useful for this task; the repository analysis and this plan were completed directly.

## Completion

- Added `/contacts/mail` and an admin-only Contacts entry for operations/site admins with information-management access.
- Added searchable, filterable, paginated contact selection with individual, visible-page, and all-matching controls.
- Excluded archived, do-not-contact, invalid-email, and case-insensitive duplicate recipients in the server query; linked-account emails remain authoritative.
- Added validated text composition and a branded, escaped HTML/plain-text contact-mail template delivered through the shared lower-level BCC batching path.
- Added a readable process-local `ContactMailTaskService` with atomic one-task blocking, Tokio spawning, live in-memory progress, cancellation notification, and a fixed hourly schedule of up to 60 contacts that skips missed boundaries.
- Added start and final history transactions for tasks and recipient snapshots. Startup closes interrupted start-only records without resending them.
- Added progress, accepted/failed/not-attempted counts, next-send timing, failure details, cancellation confirmation, and polling cleanup to the UI.
- Removed recipient addresses from low-level email transport logs.
- Verified formatting, SSR compilation, and native hydrate feature compilation.
- Verified dry-run start/final persistence, overlap blocking, cancellation, and interruption cleanup with temporary smoke binaries; removed them afterward.
- Verified the lower mail service split a real 60-contact submission to a local fake ACS endpoint into hidden-recipient requests of 50 and 10, returning 60 keyed successful outcomes.
- Rendered the contact-mail email preview successfully.
- Full `etc/dev.sh build`/browser verification was unavailable because this host lacks `cargo-leptos` and the `wasm32-unknown-unknown` Rust target.
- GPT-OSS was attempted for three exploration tasks, but the platform rejected it before inference because the model does not support this session's `xhigh` effort setting; it produced no findings.
