# Remove time reporting and time estimates from the send email page

Scope: `src/pages/contact_mail.rs` (the Contacts → send mail page). No server or
scheduling behaviour changes — the batch scheduler keeps working exactly as it
does now, we only stop showing duration estimates in the UI.

## Changes

1. **Launch confirmation** (~line 199): drop the `hours` computation and the
   "The fixed batch schedule will take at least N hours." sentence. Confirmation
   becomes just "Send this message to N contacts?".
2. **Header blurb** (~line 370): reword to drop "per hour", e.g. "Choose eligible
   Contacts, write one message, and send it in hidden-recipient batches."
3. **"3. Review and start" summary** (~line 530-536): remove the
   "over at least N hours" clause and its `saturating_sub(1) / 60` math; keep
   "This will send to N contacts in hidden-recipient batches."
4. **Task progress panel** (~line 572, 629): remove the `hours_left` computation
   and the "Minimum time left" stat tile; make the `dl` a 3-column grid
   (`sm:grid-cols-3`).
5. **"Next planned batch"** line (~line 650): remove it, since it's a projected
   send time.

Keep factual audit timestamps ("Created by X on <date>", "Finished: <date>",
"Cancellation requested … on <date>") — these are records of what happened, not
time reporting/estimates. Say so if you'd rather those go too.

## Verification

- `etc/dev.sh -- cargo check --no-default-features --features ssr`
- Build + run, visit the send email page, confirm no duration text appears in
  the blurb, review section, confirm dialog, or active-task panel.
