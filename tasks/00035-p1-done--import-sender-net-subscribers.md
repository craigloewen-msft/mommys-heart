# Import sender.net subscriber export into contacts

One-time migration of `Subscribers-export-x0ZWAi.csv` (1255 subscribers, 35
columns) from sender.net into the production `contacts` table, with every column
preserved.

The importer is a **throwaway**: a temporary `import-contacts` branch in
`src/main.rs`, run once against production, then deleted in the same task. It is
not committed as a feature. Nothing in `src/` survives this task except the
deleted-again diff, so there is no new surface to maintain.

## Decisions (confirmed)

| Question | Decision |
| --- | --- |
| Form | Throwaway script, removed after the run |
| `Company` | Create all 351 organizations and link them |
| `do_not_contact` | `unsubscribed` + `spam_reported` only → 93 flagged. Bounced (116) are **not** flagged |
| Groups/Type/Tag segments | Preserved verbatim as contact properties; no `contact_categories` created |

## Source data (verified, not assumed)

- 1255 data rows, 35 columns, UTF-8 **with BOM** — read with a BOM-stripping reader.
- **0 duplicate emails, 0 blank emails.** Email is a safe natural key.
- Every value fits existing limits (`MAX_NAME` 120 / `MAX_SHORT_TEXT` 200 /
  `MAX_LONG_TEXT` 2000). Longest `Notes` is 921 chars. **No truncation needed.**
- 351 distinct companies (case-insensitively — no collisions), 341 singletons.
- 62 fields contain embedded newlines (mostly `Address`).
- Phone fields contain invisible Unicode direction overrides `U+202D` / `U+202C`.
- `Office Number` is empty in all 1255 rows.

## Column mapping — all 35 accounted for

### To `contacts` columns

| CSV column | Target | Rule |
| --- | --- | --- |
| `Email` | `email` | verbatim, trimmed |
| `First name` | `first_name` | see name repair below |
| `Last name` | `last_name` | see name repair below |
| `Primary Phone Number`, `Phone` | `phone` | `Primary Phone Number` if present, else `Phone` |
| `Phone` | `mobile` | when `Phone` did not land in `phone` (it is the SMS-subscribed number) |
| `Mailing Address`, `Address` | `address` | `Mailing Address` if present, else `Address`; the loser becomes a property |
| `Title` | `job_title` | 41 rows |
| `Website` | `website` | 18 rows — **see gotcha below** |
| `Notes` | `description` | 765 rows |
| `Company` | `organization_id` | resolve-or-create, `kind = other` |
| `Groups`, `Type`, `Tag` | `types[]` | mapped subset only (below) |
| `Email status` | `do_not_contact` | `unsubscribed` or `spam_reported` → true |
| — | `source` | literal `"sender.net"` |

### To `contact_properties`, section `Sender.net import`

Every remaining column, written only when non-empty, in a stable order:

`Groups` · `Type` · `Tag` · `Invite` · `Email status` · `Sms status` ·
`Transactional Email Status` · `Transactional Sms Status` · `Location` ·
`Created` (as *Subscribed on*) · `Agreement Date` · `Secondary Email` ·
`Secondary Website` · `Company Website` · `Secondary Phone Number` · `Fax` ·
`Office Number` · `Home` · `Secondary address` · the unused one of
`Address`/`Mailing Address` · `Orders Count` · `Total spent` ·
`Last order number` · `Currency` · `Full Name` (only when it differs from
`First name` + `Last name`)

These are appended **after** the code-owned defaults that
`add_defaults_for_new_contact` inserts, so the standard Communication /
Relationship fields keep their ordinals.

### Type mapping (`Groups` ∪ `Type` ∪ `Tag`, split on `,` and `.`, case-folded)

`donor`→Donor · `volunteer`→Volunteer · `board member`→BoardMember ·
`attorney`/`lawyer`→Attorney · `client`/`parent client`→Client ·
`vendor`→ServiceProvider

Everything unmapped (`Invite`, `VIP`, `Hamptons`, `New York`, `Florida`,
`May10th Invite`, …) is a **segment**, preserved as a property only.

Resulting distribution: Other 556, Donor 319, ServiceProvider 246, Volunteer
127, Client 19, BoardMember 15, Attorney 8. Max 3 types on any one row, well
under `MAX_TYPES` (6). Rows with no mapped type get `[Other]` — `types` may not
be empty.

## Two things that will break a naive implementation

**1. `contacts::insert_in` does not write `website`.** Confirmed by reading
`src/server/db/contacts.rs` — the `INSERT` lists 15 columns and `website` is not
among them; only `contact_directory::save` updates it (`UPDATE contacts SET
website = $2`). The 18 website values are silently dropped unless the script
follows each create with that same `UPDATE`.

**2. Eleven rows violate `contacts_named_check`** (`last_name <> '' OR
organization_id IS NOT NULL`) and would abort the transaction. Repair, following
the existing `split_name` convention in `contact_directory.rs` where a single
word becomes the *last* name:

- `karen@brsmatlaw.com` — `First name` is `"Karen Rosenthal"` → split into Karen / Rosenthal.
- 7 rows with a one-word first name and no company (`Mala`, `Founder`, `Norry`,
  `Diane`, `West`, `Neustein`, `Flowers`) → move that word into `last_name`.
- 3 rows with no name at all (`donate@sbs.org.ge`, `rakeshp@gmail.com`,
  `ogunyemi.gabrielolusola@yahoo.com`) → derive `last_name` from the email
  local-part.

All 11 are listed in the run report for human review afterwards.

## Normalisation

- Strip the BOM; strip `U+202D`/`U+202C` from all phone/fax fields.
- Collapse embedded newlines and runs of whitespace in every short-text field;
  leave `Notes` (→ `description`) intact apart from trimming.
- Trim `Created` (values arrive with a leading space).

## Plan

1. Add `csv = "1"` to `[dependencies]` and the `ssr` feature list.
2. Add a temporary `import-contacts <path> [--dry-run] [--actor <email>]` branch
   in `src/main.rs`, beside the existing `seed` / `preview-emails` branches.
3. Resolve the actor: look up the admin by `--actor` email and use their real
   `users.id` for `audit::set_actor_in_transaction`, so `audit_log.actor_user_id`
   satisfies its FK to `users(id)` and the Change Log attributes the import to a
   real person.
4. Parse and normalise all 1255 rows; build the org name → id map first,
   creating the 351 organizations via `organizations::create`.
5. Create each contact through `contacts::create` (reusing `ContactInput::validate`
   so the DB `CHECK`s cannot be bypassed), then the `website` `UPDATE`, then
   append the import properties.
6. Skip any row whose email already exists, so a partial run can be resumed.
7. Print a report: created / skipped / failed, the 11 repaired names, the 4
   address-like company values that became organizations, and per-row errors.
8. Verify on a local DB with `--dry-run`, then for real; spot-check a contact
   with many populated columns.
9. Run against production.
10. **Delete** the `main.rs` branch, the `csv` dependency, and the CSV file;
    leave the tree exactly as found.

## Verification

- `--dry-run` reports 1255 would-create, 351 organizations, 93 `do_not_contact`,
  0 failures.
- After the real run: `contacts` +1255, `organizations` +351, and a spot-checked
  contact (e.g. `ysalaam@council.nyc.gov`, which populates Groups, Phone,
  Secondary Phone, Notes, Secondary address and Address) shows every CSV value
  either in a field or a property.
- Final `git status` shows no source changes.

## Out of scope

Contact categories, a reusable importer, an admin upload UI, and any dedupe
against existing contacts beyond the email-exists skip.
</text>