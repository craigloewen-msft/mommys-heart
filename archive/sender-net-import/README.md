# sender.net subscriber import (one-time, archived)

Migrated the sender.net export `Subscribers-export-x0ZWAi.csv` — 1255
subscribers, 35 columns — into `contacts`, `organizations` and
`contact_properties`.

A **self-contained crate**: it builds and runs from this directory with no
edits anywhere else. It depends on the app as a path dependency and writes
through the same repository functions the UI uses, so every validation rule and
database CHECK still applies. Nothing here is compiled into or shipped with the
application — the parent `Cargo.toml` does not reference it.

## Running it

```bash
cd archive/sender-net-import

# Preview. Reads the file and the database, writes nothing.
cargo run --release -- ../../Subscribers-export-x0ZWAi.csv \
  --actor you@mommysheart.org --dry-run

# Import.
cargo run --release -- ../../Subscribers-export-x0ZWAi.csv \
  --actor you@mommysheart.org
```

`--actor` must be an existing account's email; the import is recorded as that
person, so the Change Log attributes it to someone real. An unknown address
exits before anything is written.

The first build takes a few minutes (it compiles the app as a library). No
`RUST_LOG` needed — the report prints on its own, and an inherited `RUST_LOG`
cannot suppress it.

### Which database it writes to

`DATABASE_URL` decides, and it is echoed on startup with the password redacted.
Check that line before answering the LIVE warning.

* **Local dev** — set nothing. It falls back to `../../.env.local`, this
  checkout's container.
* **Production** — pass it explicitly. When `DATABASE_URL` is already set, the
  local `.env.local` is ignored entirely, so it cannot silently redirect a
  production run:

```bash
DATABASE_URL="postgres://user:pass@host:5432/db?sslmode=require" \
  cargo run --release -- /path/to/export.csv --actor you@mommysheart.org --dry-run
```

> **Always pass `DATABASE_URL` explicitly**, as above. This tool writes to
> whichever database it is given, so an ambiguous environment is the one thing
> that could point a production import at a local container.

**Idempotent by email.** A contact whose address already exists is skipped, so an
interrupted run is resumed by re-running it. Verified twice: a second full run
reported `0 created, 1255 skipped`, and a run interrupted mid-flight (351
organizations written, 0 contacts) recovered on the next attempt to exactly
1255 contacts / 357 organizations with no duplicates.

## Mapping

Contact fields:

| CSV | Field | Rule |
| --- | --- | --- |
| `Email` | `email` | |
| `First name` / `Last name` | `first_name` / `last_name` | repaired where needed (below) |
| `Primary Phone Number` else `Phone` | `phone` | |
| `Phone` | `mobile` | only when it lost to a primary number |
| `Mailing Address` else `Address` | `address` | the loser becomes a property |
| `Title` | `job_title` | |
| `Website` | `website` | written by a follow-up `UPDATE`; see below |
| `Notes` | `description` | |
| `Company` | `organization_id` | resolve-or-create, `kind = other` |
| `Groups`/`Type`/`Tag` | `types[]` | mapped labels only |
| `Email status` | `do_not_contact` | `unsubscribed` / `spam_reported` |
| — | `source` | `sender.net` |

The other 23 columns become properties in a `Sender.net import` section,
appended after the code-owned defaults so those keep their ordinals.

Type mapping: donor→Donor, volunteer→Volunteer, board member→BoardMember,
attorney/lawyer→Attorney, client/parent client→Client, vendor→ServiceProvider.
Unmapped labels (`Invite`, `VIP`, `Hamptons`, `New York`, …) are outreach
segments, preserved as properties; a contact with no mapped label gets `Other`.

## Things worth knowing before the next run

- **`contacts::create` does not write `website`.** Its `INSERT` omits the column;
  only the directory updates it. The importer issues the same `UPDATE` afterwards.
  Anyone reusing `contacts::create` elsewhere needs that too.
- **`contacts_named_check`** needs a surname or an organization. Eleven rows had
  neither and are repaired: a two-word first name is split; a lone word becomes
  the surname; a nameless row derives one from the email local part. Every repair
  is listed in the run report.
- **Invisible characters.** Phone columns carry `U+202D`/`U+202C` bidi overrides
  and the file is UTF-8 **with a BOM**; both are stripped.
- **Embedded newlines.** 62 cells (mostly `Address`) span lines. Short fields are
  collapsed to one line; `Notes` keeps its internal breaks.
- **Bounced ≠ unsubscribed.** Only explicit opt-outs set `do_not_contact` (93).
  The 116 bounced are left contactable; their status is still recorded.
- **Upstream data entry.** One contact has `Company: …` / `Title: …` / `Type: …`
  prefixes inside the values. These are preserved verbatim; the `Type:` prefix is
  handled for type mapping but the stray text in `Company`/`Title` is the
  source's, not the importer's. Four `Company` values are street addresses and
  became organizations named accordingly.

## Verified on a local database

1255 created / 0 failed, 351 organizations, 93 do-not-contact, 9127 properties.
A cell-by-cell audit of all 16150 non-empty CSV values found every one present.
Contacts, organizations, the property section and the "Do not contact" badge all
render correctly in the UI.


## The export itself

`Subscribers-export-x0ZWAi.csv` is deliberately **not** committed — it holds
names, emails, phone numbers and home addresses for 1255 real people. It is
covered by a `*.csv` rule in `.gitignore`. Keep it out of the repository.
