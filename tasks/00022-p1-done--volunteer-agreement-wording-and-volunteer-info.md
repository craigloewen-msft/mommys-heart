# Volunteer agreement: Foundation-approved wording, plus volunteer info captured on the application and shown on the profile

Two connected changes:

1. Replace the **placeholder** volunteer agreement wording in
   `src/helpers/volunteer_terms.rs` with the Foundation's approved text (below,
   verbatim) and bump the version.
2. Collect the **volunteer info** the Foundation asks for as part of accepting
   the agreement, store it on the `volunteers` row, and surface it on the
   profile as an editable "Volunteer information" panel.

The existing shape is already right for this: `/volunteer-agreement` renders
`VOLUNTEER_AGREEMENT_SECTIONS` behind a scroll-to-end gate and posts
`apply_to_volunteer(version)`; `/profile` shows a "Volunteer agreement" panel to
the owner and to operations admins. Both get extended rather than rewritten.

## Decisions taken (asked and answered)

- **SSN is optional**, stored in full, and **never sent to the browser** — not
  even the last four digits. The profile DTO carries a single `has_ssn: bool`,
  so the panel can say "On file" or "Not provided" without any of the number
  leaving the server. A separate site-admin-only server fn returns the full
  number on explicit request and writes an audit entry naming who revealed it
  and when. Every disclosure is therefore a deliberate, logged act.
- **One page, two sections.** The info fields live on `/volunteer-agreement`
  below the agreement, disabled until the agreement is read to the end and the
  acceptance box is ticked. A single submit files the whole application.
- **Email and phone prefill from the account** and save back to it. Email is the
  sign-in identity and stays read-only on this form; phone is editable and
  writes through to `users.phone`.
- **The owner can edit the info in place** on their profile, like contact
  details. Edits are audited.

## 1. The agreement wording

`src/helpers/volunteer_terms.rs`:

- Delete the `# PLACEHOLDER WORDING` module note — it exists precisely to be
  removed when approved wording lands.
- Replace `VOLUNTEER_AGREEMENT_SECTIONS` with the text in **Appendix A**,
  verbatim.
- Replace `VOLUNTEER_ATTESTATION` with the closing acknowledgement paragraph
  (Appendix A, last block).
- Bump `VOLUNTEER_AGREEMENT_VERSION` to `"2026-08-10"`, dropping the
  `placeholder-` prefix.

Structure it with the existing `TermsSection { heading, paragraphs }`:

- The three `WHEREAS`/`NOW, therefore` clauses are the preamble: one section
  with an empty `heading`, matching how `helpers/terms.rs` opens.
- Each numbered paragraph is one section. Where the number carries a title, the
  heading is number-plus-title (`"1. The Volunteer Position, Duties, and
  Responsibilities"`) and the prose after it becomes the paragraphs. Where it
  does not (3, 4, 10, 13, 15), the heading is the bare number (`"3."`) so the
  numbering stays visible and the prose stays untouched.
- The two banner lines (`NONDISCLOSURE AGREEMENT, COMMITMENT, & LIABILITY` and
  `ACCEPTANCE OF RISK with RELEASE AND INDEMNITY`) are sections with that
  heading and an empty `paragraphs` slice. The page renderer already tolerates
  this: it `Show`s the heading only when non-empty and maps over the paragraphs.

Transcription rules, so the diff is defensible later:

- Copy the wording **exactly**, including its own quirks (`"a Administrative
  Agency"`, `"by his or her own effects and responsibility"`, the unnumbered
  `Compensation.` paragraph sitting under item 3, `"terminated at follows"`).
  These are the Foundation's text, not ours to correct.
- The single exception is `"shallcontinue"` in item 16, which is a copy/paste
  artifact rather than wording — write `"shall continue"`.
- Use curly apostrophes and quotes as they appear; Rust string literals need
  `\"` for the straight quotes around defined terms like `("Duties and
  Responsibilities")`.
- Do **not** add an opening recital naming the parties or an effective date. The
  supplied text begins at the `WHEREAS` clauses and the pane renders only what
  the Foundation approved. The page already frames it outside the agreement box
  ("Read and accept the volunteer agreement" plus the explanatory blurb).

### Re-signing: bumping the version has consequences

Today `has_agreement()` means "the version string is non-empty", and both the
profile card and `apply_to_volunteer` treat that as done-forever. After the bump,
anyone who accepted `placeholder-2026-01-01` would be left holding wording the
app refuses to render (`profile.rs` deliberately hides non-matching text) and
would have no route back. Fix it in the same change:

- Add `VolunteerApplication::is_current_agreement()` — non-empty **and** equal to
  `VOLUNTEER_AGREEMENT_VERSION`.
- `pages/profile.rs`: the "Volunteer agreement" prompt card keys off
  `is_current_agreement()` rather than `has_agreement()`, so a volunteer on old
  wording is asked to read and accept the current one. The card's existing
  "we don't have your signed volunteer agreement on file" blurb becomes "the
  volunteer agreement has been updated; please read and accept the current
  version" when they signed an older one.
- `server_fns::volunteers::apply_to_volunteer`: the "You have already accepted
  the volunteer agreement" refusal fires only when the stored version is already
  the current one.
- `db::users::volunteers_page` maps `AgreementStatus::Completed` to "stored
  version equals the current one", so the admin list shows outdated signers as
  `Outstanding`. Pass the current version in from the caller rather than
  importing `helpers` into a SQL string.

## 2. Volunteer info

### Fields

From the Foundation's form. Required marked `*`.

| Field | Required | Notes |
| --- | --- | --- |
| Volunteer skills and area of focus | * | Multi-line free text |
| Date of birth | * | Stored as `DATE`, displayed `MM-DD-YYYY` |
| Social Security Number | | Optional, see handling below |
| Email | * | Prefilled and read-only — it is the sign-in identity |
| Phone number | * | `(000) 000-0000`, prefilled from `users.phone` |
| Emergency contact first name | * | |
| Emergency contact last name | * | |
| Emergency contact relationship | | |
| Emergency contact phone | * | `(000) 000-0000` |

The form's bare **"Date"** field (between date of birth and SSN) is the signing
date. It is not collected: `volunteers.agreed_at` already records exactly when
the agreement was accepted, and a self-typed date next to a server timestamp is
a contradiction waiting to happen. The profile panel labels `agreed_at` as the
date of signature.

The consent sentence — *"by providing the information below, Volunteer consents
to the Foundation performing a background check"* — is rendered above the contact
fields on both the application form and the profile edit form, so it is never
separated from the fields it governs.

### Shared validation: `src/helpers/volunteer_details.rs`

A new pure helper module (`helpers/` compiles for both server and WASM, so one
implementation serves the form and the server fn — the `CaseIntake::validate`
pattern):

```rust
pub struct VolunteerDetails { /* the fields above, all String */ }

impl VolunteerDetails {
    pub fn normalized(&self) -> Self;        // trim; phones to (000) 000-0000; SSN to digits
    pub fn validate(&self) -> Result<(), String>;
}

pub fn format_phone(raw: &str) -> Option<String>;   // 10 digits -> "(555) 123-4567"
pub fn format_ssn(digits: &str) -> String;          // "123456789" -> "123-45-6789", for the reveal
pub fn format_dob(iso: &str) -> String;             // "1990-04-02" -> "04-02-1990"
```

Validation: required fields non-empty; phones exactly 10 digits after stripping
formatting; SSN either empty or exactly 9 digits; date of birth a real date, in
the past, and no earlier than 1900. Age is **not** gated — paragraph 13 of the
agreement contemplates minors working with guardian permission.

Date of birth uses `<input type="date">` (as `components/volunteer_hours.rs`
already does), so entry is unambiguous and the value arrives as `YYYY-MM-DD`;
only the *display* is `MM-DD-YYYY`, matching the Foundation's form.

### Schema: `migrations/0016_volunteer_details.sql`

Columns on the existing `volunteers` row — one row per person already, and the
info is part of the same signed application:

```sql
ALTER TABLE volunteers
    ADD COLUMN skills_focus        TEXT NOT NULL DEFAULT '',
    ADD COLUMN date_of_birth       DATE,
    -- Full number, never sent to a browser; reads go through db::volunteers::ssn(),
    -- which is called by exactly one site-admin-only, audited server fn.
    ADD COLUMN ssn                 TEXT NOT NULL DEFAULT '',
    ADD COLUMN emergency_first_name   TEXT NOT NULL DEFAULT '',
    ADD COLUMN emergency_last_name    TEXT NOT NULL DEFAULT '',
    ADD COLUMN emergency_relationship TEXT NOT NULL DEFAULT '',
    ADD COLUMN emergency_phone        TEXT NOT NULL DEFAULT '';
```

Defaults rather than `NOT NULL` without one: the migration- and seed-backfilled
volunteers predate the form entirely, and `''` is the same "predates it" signal
`agreement_version` already uses. `date_of_birth` is nullable for the same
reason — there is no sensible empty date.

### Server layer

`src/server_fns/volunteers.rs`:

- `VolunteerDetails` (from `helpers`) is what goes *up*. What comes *down* is a
  sibling read DTO carrying `has_ssn: bool` in place of `ssn`, so no part of the
  number has a field to travel in.
- `VolunteerApplication` gains `details: VolunteerDetailsView`. Same row, same
  visibility rule, so it rides along with the record already being loaded.
- `apply_to_volunteer(agreement_version, details)` — validates the version as it
  does now, then `details.normalized().validate()`, then writes the volunteer row
  and `users.phone` in one transaction.
- `save_my_volunteer_details(details)` — new. `require_user`, always writes the
  caller's own row, refuses when they have no volunteer record. Same validation.
- `reveal_volunteer_ssn(user_id) -> String` — new. `require_site_admin`. Returns
  the full number and writes an audit entry before returning it
  (`Entity::User`, field `"volunteer SSN"`, old `""`, new `"revealed"`). The
  number itself never enters the audit log.

`src/server/db/volunteers.rs`:

- `SELECT_COLUMNS` picks up the new columns, with `ssn <> '' AS has_ssn` and
  `to_char(date_of_birth, 'YYYY-MM-DD')`. The `ssn` column itself is **not** in
  this list in any form — the ordinary read path never has the digits in hand,
  so it cannot leak them by accident.
- `apply(...)` takes the details and writes them in the same upsert.
- `save_details(user_id, details, actor)` — updates the row and appends an audit
  entry per changed field, mirroring `users::update_profile`. The SSN entry
  records `"set"`/`"removed"`, never a value.
- `ssn(user_id) -> Result<String, _>` — the single raw read, reachable only from
  `reveal_volunteer_ssn`.
- `list_pending`'s `Volunteer` row gains `skills_focus` so an admin can judge an
  application without leaving the queue.

On submit, phone flows to `users.phone` as well as the volunteer row: one source
of truth for contact, with the volunteer copy being what the signed application
carried.

### UI

**`src/pages/volunteer_agreement.rs`** — a third `<section>` below the
attestation block, headed "Volunteer information", with the consent sentence
above the contact fields. Every input is `disabled` until
`read_to_end && accepted`, with a line explaining why. Submit stays one button;
it calls `validate()` first and shows the message in the existing error slot.
Prefill email and phone from `state.current_user_summary` (fetch the profile on
mount if the summary lacks phone).

**`src/pages/profile.rs`** — a "Volunteer information" panel above the existing
"Volunteer agreement" panel, rendered whenever `p.volunteer` is present and
carries details, under the same owner-or-operations-admin rule that already
governs that block. Rows via the existing `DetailRow`: skills and area of focus,
date of birth, SSN, phone, emergency contact name / relationship / phone.

- SSN renders as "On file" or "Not provided" — no digits, for the owner or
  anyone else. Site admins get a "Reveal" button next to it that calls
  `reveal_volunteer_ssn` and swaps in the full number for that page view only,
  with a note that the reveal was logged. Non-site-admins see no button.
- The owner gets an "Edit" button opening an inline form in the panel, in the
  style of the existing profile edit form. SSN there is a blank input with
  placeholder "Leave blank to keep the number on file", plus a "Remove the number
  on file" checkbox shown only when one exists — the form cannot prefill a value
  it is never sent.

The panel is a component, `src/components/volunteer_details.rs`, rather than more
inline closures in `profile.rs` — that file is already long, and this follows
`components/volunteer_hours.rs`.

**`src/components/admin_volunteers.rs`** — `ApplicationCard` shows the
applicant's skills and area of focus under their email.

### Seed

`src/server/db/seed.rs` fills the new columns for one or two seeded volunteers
(including one with an SSN) so the panel, the "On file" state and the reveal are
demoable; the rest stay backfill-shaped with empty details.

## Verification

- `etc/dev-run.sh -- cargo check --no-default-features --features ssr`,
  `cargo fmt`, no new clippy warnings.
- Migration applies to the seeded dev database; existing volunteers keep their
  rows with empty details.
- In the browser, as a client: `/profile` → "Become a volunteer" → the info
  fields are disabled until the agreement is scrolled to the end and the box is
  ticked → submitting with a bad phone or a 5-digit SSN is refused with the
  message from the shared validator → a good submit files the application and
  the card switches to "being reviewed".
- As a site admin: the pending card shows the skills text; approve; open that
  user's profile and confirm the info panel, that the SSN row reads "On file"
  with no digits, that "Reveal" returns the full number, and that the reveal
  appears in that user's change log naming the admin. Confirm an operations
  admin sees the panel but no Reveal button. Check the `load_profile` response
  in devtools and confirm it contains no fragment of the number.
- As the volunteer: edit the panel, confirm the phone change lands on the
  contact block too, that leaving SSN blank keeps the stored number, and that
  the remove checkbox clears it.
- Confirm a volunteer still on `placeholder-2026-01-01` is prompted to accept the
  current agreement and can do so, and that the admin list shows them as
  `Outstanding` until they do.
- Read the rendered agreement against Appendix A end to end, checking the
  numbering runs 1–20 and no paragraph was dropped or reflowed.

---

# Appendix A — the agreement wording, verbatim

WHEREAS, the Volunteer desires an opportunity to help those in need and gain valuable knowledge, experience, education, and training in the Foundation's causes and undertakings for legal and social reform;

WHEREAS, the Foundation offers the Volunteer this opportunity;

NOW, therefore, the Foundation and Volunteer, in consideration of the mutual promises, conditions and covenants contained herein, hereby agree as follows:

**1. The Volunteer Position, Duties, and Responsibilities.** The Volunteer shall work without financial compensation in one or more of the Foundation's (1) operations, (2) marketing and public relations, (3) fundraising, (4) legal, and/or (5) mental health services department.

The Volunteer's duties will be determined by Julianne Michelle Reeves Stroh, the Foundation's President, or an officially designated officer/employee nominated and confirmed as agent by the Foundation's President. The Foundation expects all volunteers to engage in and to perform, as they may be directed to do, in one or more of the following duties: (1) to assist in marketing and promotional efforts, (2) research, (3) outreach, (4) fundraising, (5) financial reporting and analysis, (6) product design and/or (7) service delivery, and/or finally (8) in developing strategies of advocacy, (by conversation and investigation, both on-line and "in library") and writing to advance the mission of the foundation.

The Volunteer may be responsible for the following duties and/or tasks, including but not limited to: reading, writing, sponsorship outreach, content creation, generating financial statements, research, search engine optimization, online ads and marketing, fundraising, strategy, press releases, intakes, case management and organization, responding to press inquiries, and event planning and promotion. ("Duties and Responsibilities").

**2. Work Schedule.** The Volunteer is not required to work particular days or hours; the Volunteer will determine the days and hours, during the Foundation's business hours, during which the Volunteer will be working. However, the Foundation expects the Volunteer to establish and maintain a relatively consistent work schedule, informing his or her supervisor(s) of the same for day-to-day project planning, such that the Foundation can expect the Volunteer to be available to work on approximately the same days and at approximately the same times each week. Volunteer will apprise the Foundation's president of his or her expected work schedule each week.

**3.** The Foundation takes no responsibility for, denies and will accept no liability for any assignments or work the volunteer accepts or documents the Volunteer creates without the Foundation's express knowledge or consent.

Compensation. The Parties agree this is an unpaid position in that the volunteer will neither be directly nor indirectly financially compensated for the duties performed at the Foundation.  The volunteer will at all times be responsible for maintaining his or her own insurance, as the Foundation does not and will not provide coverage of any kind.  The Volunteer agrees that the opportunity to help others and garner practical experience and insight under supervision by and with the assistance of attorneys, legal and business consultants, mental health practitioners, and other experts will constitute good and sufficient compensation for his/her services.

**4.** The Volunteer accepts the opportunity to work for the Foundation as reasonable compensation and consideration for his or her time, and agrees to the Volunteer helping others and gaining valuable knowledge, experience, education, and training in the Foundation's industry as consideration for the Duties and Responsibilities. The Volunteer's work for the Foundation may also entitle the Volunteer to credit from the college or university she or he is attending.

**5. Employment Status.** The Volunteer is not an employee of the Foundation. Since there is no financial compensation, the Foundation will not withhold any federal, state or city income or any other taxes on behalf of the Volunteer. The Volunteer will not be regarded as an employee or servant of the Foundation for the purpose of any federal or state employment, labor, unemployment, tax, or other law or regulation. The Volunteer again acknowledges: the Foundation will not obtain disability, worker's compensation, or any other type of insurance on the Volunteer's behalf or for the benefit of Volunteer. The Foundation will not provide the Volunteer with any of the benefits which may or may not be provided by the Foundation to its employees, such as group medical coverage and paid vacation time. No Volunteer will receive any perquisites of any kind from the Foundation except for acknowledgment and confirmation of his or her services.  Lastly, the Volunteer will not act as an agent or officer of the Foundation and does not have the authority to bind the Foundation in any manner whatsoever.

**6. Work for Hire.** All images, videos, text, writings, working papers, manuals, computer programs, source codes, graphics, plans, artwork, copyrights or trademarks, documents and other works ("Works") created wholly or in part by the Volunteer, as a result of the Volunteer's working for the Foundation, shall conclusively be deemed to be "works made for hire" within the meaning of 17 U.S.C. § 201(b), even though Volunteer is not an employee of the Foundation. All copyright and other intellectual property rights in and to such Works belong to the Foundation, not to the Volunteer. The Volunteer hereby assigns to the Foundation all intellectual property rights in all Works created wholly or in part by Volunteer pursuant to this Agreement.

**7. Term.** This Agreement shall commence upon the Effective Date of this Agreement and will continue for one year.

**8. Unbecoming Conduct.**  You agree to refrain from making any statements or engaging in any actions that might be offensive to the Foundation's employees or volunteers and/or that might be construed by another person as harassment based in part or in whole on such person's sex, race, national origin, age, sexual orientation, disability, religion, or political affiliation.  The Volunteer shall refrain from making statements on behalf of the Foundation, on social media or any other form of communication, unless expressly authorized to do so.  The Volunteer shall refrain from any use of alcohol and recreational drugs during the working day and at all times while at the office, in court, or anywhere else in public or private meetings with other Foundation personnel.  Additionally, the Volunteer agrees not to disparage the Foundation or its officers, directors, employees, grantors, donors, clients, affiliates or agents, in any manner likely to be harmful to them or their business, business reputation or personal reputation; provided, however, that Volunteer shall respond accurately and fully to any question, inquiry or request for information when required by legal process.

## NONDISCLOSURE AGREEMENT, COMMITMENT, & LIABILITY

**9. Maintaining Confidentiality of Information.** In working for the Foundation, Volunteer will have access to and will obtain non-public information concerning the Foundation's business, board members, actual and prospective clients, actual and prospective sponsors or donors, suppliers, finances, marketing and business strategies and plans, contacts, products or services, passwords or login credentials, marketing and promotion concepts and ideas (whether or not copyrightable), and other matters (collectively, "Information"). Volunteer agrees that all such Information is confidential and proprietary information which derives independent economic value, actually or potentially, from not being generally known to the public, to competitors, and to other persons that can or might obtain economic value from the disclosure or use of it.  The Volunteer at all times accepts full responsibility and liability for his or her breach of this agreement, and commits to exercising the maximum caution in the handling of proprietary information without exception or exclusion.

**10.** Volunteer will maintain all such Information on a confidential basis while working for the Foundation and at all times thereafter, without limitation, unless expressly ordered to disclose any such information by a Administrative Agency or Court of Competent Jurisdiction in proceedings of which the Volunteer guarantees that, by his or her own effects and responsibility, he or she shall provide the Foundation of any such court or administrative order at least 7 business days advance, legally proper, notification.

**11. Termination.** This Agreement may be terminated at follows:

a. At any time by either Party upon written notice to the other Party.

b. By the Foundation due to the Volunteer's breach of the Agreement.

Upon termination, the Volunteer shall return all the Foundation content, materials, and all work product to the Foundation on the same day of termination, unless barred by supervening order or "act of God."  Volunteer accepts full responsibility and liability for immediately returning all material in all media, including but not limited to all papers, electronically stored data or programs, and memoranda belonging to the Foundation which might have been used or created by the labor of the volunteer, but in no event beyond thirty (30) days after the date of termination. Failure to return documents and electronic programs or media within thirty (30) days may result in legal action to recover the same.

**12. Representations and Warranties.** The performance and obligations of either Party will not violate or infringe upon the rights of any third-party or violate any other agreement between the Parties, individually, and any other person, organization, or business or any law or governmental regulation.

**13.** The Volunteer further represents that the Volunteer is duly authorized to work wherever assigned in the United States and is of legal age to accept and perform work (including express permission of parents or guardian, if working as a minor).

## ACCEPTANCE OF RISK with RELEASE AND INDEMNITY

**14. ASSUMPTION OF RISK AND INDEMNIFICATION.**  THE VOLUNTEER RELEASES AND FOREVER DISCHARGES AND AGREES TO INDEMNIFY THE FOUNDATION, ITS OFFICERS, AGENTS, AND AFFILIATES FROM ANY AND ALL LIABILITY FOR ANY INJURY, DAMAGE, LOSS, COST, OR EXPENSE (INCLUDING, WITHOUT LIMITATION, ATTORNEY'S FEES) THAT THE VOLUNTEER MAY AT ANY TIME HAVE OR INCUR, ARISING OUT OF OR IN ANY MANNER RELATED TO ANY LOSS, DAMAGE, OR INJURY, INCLUDING BUT NOT LIMITED TO EMOTIONAL INJURY, SUFFERING, BODILY INJURY, LOSS OF PROPERTY, OR DEATH, THAT MAY BE SUSTAINED BY THE VOLUNTEER OR BY ANY PROPERTY BELONGING TO THE VOLUNTEER, WHILE WORKING WITH THE FOUNDATION.  THE VOLUNTEER ALSO AGREES TO INDEMNIFY THE FOUNDATION, ITS OFFICERS, AGENTS, AND AFFILIATES FOR ANY DAMAGES, CLAIMS, LOSSES, COSTS, OBLIGATIONS, LIABILITIES, AND EXPENSES, INCLUDING BUT NOT LIMITED TO ATTORNEY'S FEES, DIRECTLY OR INDIRECTLY SUFFERED OR INCURRED BY THE FOUNDATION AND/OR ANY OF ITS AFFILIATES AS A RESULT OF ANY DIRECT OR INDIRECT ACT OR OMISSION OF THE VOLUNTEER.

**15.** Sections 8, 9, 10, and 14 remain in full force and effect even after termination of the Agreement by its natural termination or early termination by either Party.

**16. Severability.** In the event any provision of this Agreement is deemed invalid or unenforceable, in whole or in part, that part shall be severed from the remainder of the Agreement and all other provisions shall continue in full force and effect as valid and enforceable.

**17. Waiver.** The failure by either Party to exercise any right, power, or privilege under the terms of this Agreement will not be construed as a waiver of any subsequent or further exercise of that right, power, or privilege or the exercise of any other right, power, or privilege.

**18. Dispute.** Any controversy or claim arising out of or relating to this contract, or the breach thereof, shall be settled by arbitration administered by the American Arbitration Association under its Commercial Arbitration Rules, and judgment on the award rendered by the arbitrator(s) may be entered in any court having jurisdiction thereof.  The Parties agree that this Agreement shall be governed by New York State law, with venue for any dispute to be mandatory in the borough of Manhattan in New York County, New York.

**19. Legal and Binding Agreement.** This Agreement is legal and binding between the Parties as stated above. This Agreement may be entered into and is legal and binding both in the United States and throughout Europe. The Parties each represent that they have the authority to enter into this Agreement.

**20. Entire Agreement.** This Agreement constitutes the entire agreement and understanding between the Volunteer and the Foundation. It supersedes any and all previous discussions, negotiations, understandings, and agreements between them.

The Agreement between the parties is entirely written, and does not include any oral promises, oral representations, or other oral statements. This Agreement may not be modified except in a document signed by both parties.

### Attestation (`VOLUNTEER_ATTESTATION`)

The Foundation's paper form ends with a signature clause. This is an online
acceptance that captures no signature from either party, so that clause is
replaced by what actually happens here. **As shipped:**

Volunteer acknowledges, affirms, and certifies that they have read and reviewed, and now and hereby agree to all above-stated 20 paragraphs.  By accepting below, Volunteer certifies that they understand the terms and conditions set forth above are a binding contract with the Foundation.

The original paper wording, for reference, read "By affixing their signature,
together with the Foundation President, Volunteer certifies that...".
