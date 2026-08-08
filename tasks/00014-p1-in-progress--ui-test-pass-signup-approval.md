# UI test pass: client signup → admin accept/deny → client aftermath

Full manual UI test pass over the new signup / account-management / case-review
features, driven through a real browser against a live dev instance.

## Why this is a task and not an Explore answer

Explore mode's sandbox blocks `etc/dev-db.sh up` (`Permission denied` on
`~/.cache/mommys-heart-devdb/ports/.lock`) and blocks localhost network access,
so no app instance can be started or driven. Everything below was mapped
statically; it needs to be executed against a running server.

## Environment setup

```bash
etc/dev-db.sh up
EMAIL_DRY_RUN=true etc/dev-run.sh     # see "Email visibility" below
```

Seeded accounts (`src/mockdata.rs:51-119`):

| Role | Email | Password |
|---|---|---|
| Site admin | `admin@mommysheart.org` | `admin123` |
| Volunteer | `dana@mommysheart.org` | `volunteer123` |
| Client | `jamie@example.com` | `client123` |

## Email visibility — decide this first

The user asked to "read the output to make sure they get the right emails".
Today that is only partly possible:

- `configured_email()` (`src/server/notifications.rs:26-29`) returns `None`
  unless ACS is configured **or** `EMAIL_DRY_RUN` is truthy. **With no env vars
  set, every notification email is silently dropped and there is nothing to
  read.** So `EMAIL_DRY_RUN=true` is mandatory for this pass.
- Even with dry-run, `dispatch()` (`notifications.rs:190-204`) logs **only the
  subject and recipient addresses**:
  `[email dry-run] would send "<subject>" to <addrs> (<n> recipient(s); ...)`.
  The rendered body is never logged.
- Bodies can only be seen out-of-band via `cargo run -- preview-emails`
  (`src/main.rs:32-47`), which renders every template with **placeholder** data
  into `target/email-preview/index.html`. That verifies wording, not the real
  interpolated values for our test case.

**Proposed approach:** run the pass with `EMAIL_DRY_RUN=true`, verify
recipients + subjects from the log, and separately verify body wording from the
preview gallery. If gaps show up (e.g. wrong name/case interpolated), add a
temporary debug log of `email.text` in `dispatch` to confirm, and report whether
a permanent dev-only body log is worth adding.

## Part 1 — New user signup, including bad input

Two distinct entry points; test both.

### 1a. `/register` (standalone)

This form has **no `required` attributes on any field**
(`src/pages/register.rs:76-123`) and the only client check is the password
match. Probe:

- All fields empty → submit. Expect: server-side rejection. Note that `register`
  in `src/server_fns/auth.rs:260-267` checks first name / email / password
  emptiness — confirm the message surfaces in the red error box and is not a
  raw `ServerFnError` string.
- Password mismatch → `"Passwords do not match."` (`register.rs:31-34`).
- Malformed email (`notanemail`) → browser `type="email"` validation should
  block. Confirm the native tooltip appears and nothing is submitted.
- Duplicate email (`jamie@example.com`) →
  `"An account with that email already exists."` (`auth.rs:283-290`).
- **Very weak password** (e.g. `a`) → there is **no strength rule anywhere in
  this flow**. Confirm it is accepted and flag whether that's intended.
- **Oversized input** — paste ~5000 chars into first name. There is no length
  validation; check whether it 500s, silently truncates, or renders broken.
- **Throttle** — `Action::Register`, 5 attempts / 15 min lockout
  (`src/server/db/throttle.rs:17-21`). Note that `record_failure` fires
  **before** the duplicate-email check (`auth.rs:269-290`), so failed duplicate
  attempts burn the budget. Trigger it deliberately and confirm:
  `"Too many sign-up attempts. Please try again in about N minutes."`
  Then confirm this doesn't wedge the rest of the pass (use fresh emails).

### 1b. `/case-signup` (client case intake) — the main flow

- `/case-signup`: confirm the accept checkbox is disabled until the terms are
  scrolled to the end (`"Scroll to the end of the terms to continue."`,
  `case_signup.rs:157-160`), and that submitting unchecked gives
  `"Please tick the box to confirm you accept the terms."`.
- `/case-signup/details`: leave required intake fields blank →
  `"Please complete the required field \"...\"."` (`helpers/case_intake.rs:170-189`)
  for each of: *Is the case in litigation?*, *What is the action sought?*,
  *Occupation of client*.
- Password mismatch → `"The passwords do not match."` (`case_signup.rs:288-291`).
- Then complete a **valid** signup with a fresh email (e.g.
  `testclient+001@example.com` / `Testpass123`).

### 1c. Verification

- Land on `/case-signup/verify`. The 6-digit code is logged in dev as
  `[auth email dev] <email>: ...` (`auth_notifications.rs:76-80`) — grab it
  from the server log.
- Enter a **wrong** code → `"That code is incorrect. Please try again."`
- Confirm the attempt counter burns the challenge after 5 wrong codes
  (`pending_registrations.rs:238-252`) and that the resulting UX is sane (does
  the user get a clear "register again" message, or a dead end?).
- Test **Resend** (`verify_email.rs:127-136`) — note there is **no throttle on
  resend**; check whether that's a concern.
- Enter the correct code → expect user row (`role = Client`) + case row
  (`status = PendingReview`) created in one transaction, session cookie set,
  redirect to `/cases`.

## Part 2 — Admin accept / deny

Sign in as `admin@mommysheart.org`, go to **Admin → "Case Requests"** tab
(`src/pages/admin.rs:176-201`, rendered by `<CaseRequests>` in
`src/components/admin_cases.rs`).

### Accept path
- Click **"Accept"** (`admin_cases.rs:124-130`) → calls
  `set_case_review_decision(case_id, true, "")`.
- Expect status `PendingReview → Open`, `reviewed_by` = admin's full name.
- Expect a `notify_case(..., "accepted this case", Audience::Everyone)` email
  (`cases.rs:377-391`). **Verify from the dry-run log that the new client is
  actually a recipient** — this is the key check, since the client only receives
  it if `recipients_for_case` includes them and their notification settings
  have `CaseData` enabled by default.

### Deny path
- Run a second signup so there's a fresh pending case.
- Click **"Decline"** → confirm the reason textarea appears
  (`"Reason (shown to the client)"`, `admin_cases.rs:143-149`).
- Submit **empty** reason → `"Please give a reason so the client knows why."`
  (client, `admin_cases.rs:97-102`). Confirm the server also rejects it
  (`"A reason is required when declining a case."`, `cases.rs:340-346`) — worth
  hitting the server fn directly to prove the guard isn't client-only.
- Submit a real reason → status `Declined`, reason stored, and a
  `"declined this case: <reason>"` email. **Check the reason text is actually in
  the email the client receives**, not just in the UI.

### Double-decision guard
- Re-decide an already-decided case → expect
  `"This case is already <status> and is not awaiting review."`
  (`cases.rs:346-352`). Check the admin UI doesn't leave a stale button that
  produces this error as a confusing raw message.

### Permissions
- Confirm a **volunteer** (`dana@mommysheart.org`) cannot reach the Case
  Requests tab or call the decision fn (`require_operations_admin`,
  `cases.rs:334-339`).
- Confirm the **client owner cannot approve their own case** — the signup grants
  them every case capability, so this is gated on account role only. Worth
  explicitly probing.

## Part 3 — The client's view afterwards (the judgement call)

Log back in as the new test client for each case and assess **whether the
experience makes sense**, not just whether it errors:

### Accepted case
- Does `/cases` show it as Open? Is it obvious it was approved?
- Can they open channels, upload files, use the new file-management features?
- Do the default channels/folders created by `create_from_signup_in`
  (`src/server/db/cases.rs:436-441`) look right and non-empty-confusing?

### Declined case
- The status renders as `"This case was not accepted: <reason>"`
  (`cases.rs:91-95`). Check where this actually appears and whether it's
  prominent enough.
- **Key question:** the client retains *all* case capabilities from signup. So
  on a declined case, can they still open channels, post messages, upload files,
  edit case properties? Enumerate exactly what is still clickable and judge
  whether that is coherent. A declined case that still accepts file uploads and
  messages nobody will read is a plausible bug.
- Can they re-apply / open a new case? Is there any path forward offered, or is
  it a dead end?

### Pending case
- Before an admin decides, check what the client sees. Is "waiting on review"
  communicated, or does it look like a normal open case?

## Deliverable

A written report covering, per step: what was expected, what happened, and
screenshots for anything visually wrong. Explicitly separate:

1. **Bugs** — errors, 500s, wrong/missing emails, broken validation.
2. **UX / sense-making concerns** — especially the declined-case capability
   question and the dead-end paths.
3. **Email findings** — recipients and subjects per action, plus whether the
   dry-run log is sufficient to audit emails at all.

No source changes are expected beyond (optionally) a temporary debug log for
email bodies, which must be removed before review.
