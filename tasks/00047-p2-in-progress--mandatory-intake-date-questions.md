# Two mandatory date-or-never questions on the case intake

The intake questionnaire now asks, and requires an answer to, two questions the
team needs before a case can be triaged:

1. **Date of the most recent domestic-abuse incident**
2. **Date of the client's most recent therapy appointment**

Both apply to the client case signup (`/case-signup/details`) and the staff
"New case" form, because both render the same questionnaire.

## Decisions

- **A date or an explicit "Never", never blank.** For either question the event
  may genuinely never have happened, and that is a real answer rather than a
  skipped one. A new `IntakeInput::DateOrNever` carries that meaning, and
  `CaseIntake::validate()` rejects a blank answer to either question.
- **"Never" is stored as the literal word**, not as an empty value. An empty
  property would be indistinguishable from a question nobody has filled in yet,
  which is exactly the distinction these questions exist to make.
- **Dates are stored as `MM-DD-YYYY`**, the format the Foundation's paper forms
  use, matching how a volunteer's date of birth is displayed. The case property
  list renders values verbatim, so the stored value is the displayed one.
- **No future dates.** Both ask for a *most recent* past event, so a later date
  is always a mistake. Today itself is accepted.
- **Validated in the shared helper**, so the browser and the server enforce the
  same rule: `create_case` and `start_case_signup` both already call
  `CaseIntake::validate()` before storing anything.
- **The date box is only `required` while "Never" is unticked.** The signup page
  submits through a real `<form>`, so a statically-required date input would have
  made a legitimate "Never" answer unsubmittable.
- **No migration.** Intake answers become ordinary `case_properties` rows created
  per case; there is no column or enum to change. Existing cases keep the
  properties they were created with and are not backfilled.

## Shared date helpers

`helpers/dates.rs` is new, holding `parse_iso`, `to_us`, and `today` — lifted
unchanged from the private helpers in `helpers/volunteer_details.rs`, which now
calls them. The intake module needed exactly these and should not import from the
volunteer module to get them.
