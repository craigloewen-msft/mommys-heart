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

---

# RESULTS (executed 2026-08-08)

Executed in full against a live instance on `http://127.0.0.1:3180` with
`EMAIL_DRY_RUN=true`. Two real client case signups were driven end to end
(`testclient+001` accepted, `testclient+002` declined). All email assertions were
verified against the dry-run log; bodies were confirmed with a temporary
`plain_text` debug log that has since been **removed** (working tree is clean).

## Verdict

The happy path is solid. Signup, verification, admin accept/deny, status
transitions, permission gating and email recipients all behave correctly. The
problems found are one real bug (unthrottled resend), one design question
(declined cases stay fully writable), and several UX/polish issues.

## 1. Bugs

### B1 — Verification "Resend" is completely unthrottled (highest severity)
Six rapid clicks of **Resend** produced six verification emails in ~2 seconds.
`register` / `register_case_signup` are throttled via `throttle::Action::Register`
(5 / 15 min), but `resend_registration_code` (`auth.rs:537-575`) has **no throttle
at all**. Since the recipient address is attacker-chosen at signup time, this is
a usable email-bombing amplifier and directly defeats the stated purpose of the
register throttle ("so this endpoint cannot be used to email-bomb a victim with
verification codes", `auth.rs:268-269`). Recommend applying the same throttle
scope, or a short per-challenge cooldown.

### B2 — Failed duplicate-email attempts burn the throttle budget
`record_failure` runs *before* the `email_exists` check (`auth.rs:281-290`).
Observed: 4 duplicate-email attempts, then the 5th returned
`"Too many sign-up attempts. Please try again in about 15 minutes."` A user who
simply forgot they had an account gets locked out of the signup form for 15
minutes. Recommend recording the failure only after the duplicate check.

### B3 — No password strength rule anywhere
A **one-character password (`a`)** was accepted without complaint on `/register`
and the account proceeded to verification. There is no length or composition
check in either registration path. Flagged in the plan as "confirm whether
intended" — confirmed present, and worth a deliberate decision.

### B4 — No length validation on any field
A 5,000-character first name was accepted and stored without error. No 500, but
no bound either; worth a sane cap before it reaches email subjects and the UI.

## 2. UX / sense-making concerns

### U1 — A declined case remains fully writable by the client (the key question)
After decline, the client retains **all 8 case capabilities** granted at signup
(verified in `case_assignments`). On the declined case I was still able to:

- upload a file ("Doc on declined case") into Case Notes — succeeded;
- post a chat message ("Hello, is anyone reading this declined case?") — succeeded;
- open the case-properties **Edit** form.

Critically, `case_assignments` for that case contains **exactly one row set — the
client themself**. No volunteer or admin is assigned, so those uploads and
messages are guaranteed to reach nobody. This is the "plausible bug" the plan
anticipated, and it is real: the product is silently accepting work product into
a dead case. Recommend either revoking write capabilities on decline, or making
the declined case visibly read-only.

### U2 — Decision emails use a generic subject that hides the decision
Both accept and decline send the *same* subject:
`"[Mommy's Heart] <case>: Case data changed"`. The decision is only visible in
the body. Compare the signup notification, which is properly specific:
`"[Mommy's Heart] New case signup: <case>"`. A client being told their case was
declined deserves a subject line that says so.

### U3 — Awkward grammar in the decision email body
Verified body text:

> `Maria Nguyen declined this case: Outside our service area; referred to Lakeside Legal Aid. on the case "Denny Denied case".`

The reason sentence is interpolated mid-sentence, so the trailing `on the case
"..."` lands after a full stop. The accept variant reads only slightly better:
`Maria Nguyen accepted this case on the case "Testy McTest case".` — "this case
... on the case" is redundant.

### U4 — Declined case offers no path forward
The decline reason is shown clearly (`This case was not accepted: <reason>`), but
nothing suggests what to do next. The client can use **+ New case** to file
again, but nothing points there from the declined case.

### U5 — Client "+ New case" shows an irrelevant Status dropdown
The client-facing new-case form offers **Open / Monitor / Closed**. The server
correctly ignores this and forces `PendingReview` (`cases.rs:281-284`), so this is
cosmetic, not a security hole — but it promises the client a choice they don't
have. Verified: a client-created case landed as "Pending review".

### U6 — Stale error text persists after a blocked submit
On `/register`, an old error ("Passwords do not match.") remains on screen when a
subsequent submit is blocked by native email validation, so the visible message
contradicts the actual problem.

### U7 — `/register` marks no field as required
No field carries `required`, so empty submits round-trip to the server. The
server message is clean ("Please fill in first name, email, and password."), but
the case-signup form does this better with native `required` on every field.
Note `/register` also never requires last name, while case signup does.

### U8 — The "tick the box" error on `/case-signup` is unreachable
Both the checkbox and the submit button are disabled until the terms are scrolled
to the end, so `"Please tick the box to confirm you accept the terms."` cannot
fire. Harmless, but it is dead code and the disabled-button state gives the user
no explanation for why they are stuck.

## 3. Email findings

All emails verified via the `EMAIL_DRY_RUN=true` log.

| Action | Subject | Recipients | Correct? |
|---|---|---|---|
| Case signup submitted | `New case signup: <case>` | `operations@`, `admin@` | Yes |
| Verification code | `[auth email dev] <email>: email verification code NNNNNN` | client | Yes |
| Case **accepted** | `<case>: Case data changed` | `testclient+001@example.com` | Recipient right, subject wrong (U2) |
| Case **declined** | `<case>: Case data changed` | `testclient+002@example.com` | Recipient right, subject wrong (U2) |

The key recipient check from the plan passed: **the new client does receive the
decision email**, exactly one recipient, with `CaseData` enabled by default.
Decline body correctly contains the admin-entered reason verbatim.

### Is the dry-run log sufficient to audit emails?
**No.** `dispatch` logs only subject + recipients (`notifications.rs:190-204`);
the body is never logged. Since both decisions share one generic subject, the log
alone **cannot distinguish an approval from a denial** — I had to patch in a
temporary body log to verify. `cargo run -- preview-emails` renders bodies but
only with placeholder data, so it cannot confirm real interpolation.
Recommend a permanent dev-only body log behind the existing dry-run branch.

### Config note (not a bug)
Email links point at `http://127.0.0.1:3000` while the app serves on `3180`. This
is just the `APP_URL` default (`config.rs:101`), configurable per environment.

## 4. Verified working

- Terms scroll-gate correctly enables the checkbox only at the end.
- All three required intake fields enforced natively.
- Password mismatch caught on both forms with the correct copy.
- Duplicate email rejected on `/register`.
- Wrong verification code → `"That code is incorrect. Please try again."`
- Successful verification creates user (`role=client`) + case (`pending_review`)
  transactionally; unverified registrations create **no** user row.
- Accept: `pending_review → open`, `reviewed_by = Maria Nguyen`, `reviewed_at` set.
- Decline: `→ declined` with reason persisted.
- Empty decline reason rejected **both** client-side and server-side (confirmed by
  calling the server fn directly with a whitespace-only reason).
- Double-decision guard: `"This case is already declined and is not awaiting review."`
- Permissions: volunteer redirected away from `/admin` and rejected by the server
  (`"Operations-admin permissions required."`); **client cannot approve their own
  case** (same rejection).
- Accepted case: status reads `"Accepted — your case team is working on it."`,
  file upload into Case Notes works, default folders created correctly.
- Pending case: reads `"Submitted — a coordinator is reviewing your case."`

## 5. Suggested priority

1. **B1** unthrottled resend — security, fix first.
2. **U1** declined cases stay writable — decide the intended behavior.
3. **U2/U3** decision email subject + grammar — client-facing clarity.
4. **B2** throttle burn on duplicate email.
5. **B3/B4** password strength and length caps.
6. **U4–U8** polish.
