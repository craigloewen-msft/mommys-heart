# sender.net subscriber import (one-time, archived)

Migrated the sender.net export `Subscribers-export-x0ZWAi.csv` — 1255
subscribers, 35 columns — into `contacts`, `organizations` and
`contact_properties`.

Archived rather than deleted so the next export can reuse it. It is **not**
compiled into the application: `import_senderdotnet.rs` lives here, outside
`src/`, and the app builds without it.

## Reinstating it

1. `cp import_senderdotnet.rs ../../src/server/`
2. In `src/server/mod.rs`, add `pub mod import_senderdotnet;`
3. In `Cargo.toml`, add `csv = { version = "1", optional = true }` to
   `[dependencies]` and `"dep:csv"` to the `ssr` feature list.
4. In `src/main.rs`, add the CLI branch beside the existing `seed` branch:

```rust
if std::env::args().nth(1).as_deref() == Some("import-contacts") {
    let args: Vec<String> = std::env::args().skip(2).collect();
    let dry_run = args.iter().any(|a| a == "--dry-run");
    let actor_at = args.iter().position(|a| a == "--actor");
    let actor = actor_at
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| "admin@example.com".to_string());
    let Some(path) = args
        .iter()
        .enumerate()
        .find(|(i, a)| !a.starts_with("--") && Some(*i) != actor_at.map(|at| at + 1))
        .map(|(_, a)| a)
    else {
        panic!("usage: import-contacts <file.csv> [--dry-run] [--actor <email>]");
    };
    if let Err(e) = mommys_heart_app::server::db::init().await {
        panic!("failed to initialize database: {e}");
    }
    if let Err(e) =
        mommys_heart_app::server::import_senderdotnet::run(path, dry_run, &actor).await
    {
        panic!("import failed: {e}");
    }
    return;
}
```

## Running it

`--actor` must be an existing account's email; the import writes as that person,
so the Change Log attributes it to someone real.

```bash
# Preview. Reads the file and the database, writes nothing.
RUST_LOG=info etc/dev.sh -- cargo run --no-default-features --features ssr -- \
  import-contacts Subscribers-export.csv --dry-run --actor admin@mommysheart.org

# Import.
RUST_LOG=info etc/dev.sh -- cargo run --no-default-features --features ssr -- \
  import-contacts Subscribers-export.csv --actor admin@mommysheart.org
```

`RUST_LOG=info` is required — the whole report goes through `tracing`.

**Idempotent by email.** A contact whose address already exists is skipped, so an
interrupted run can simply be repeated. Verified: a second run over the same file
reported `0 created, 1255 skipped, 0 failed` and left every count unchanged.

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
render correctly.

## The export itself

`Subscribers-export-x0ZWAi.csv` is deliberately **not** committed — it holds
names, emails, phone numbers and home addresses for 1255 real people. It is
covered by a `*.csv` rule in `.gitignore`. Keep it out of the repository.
