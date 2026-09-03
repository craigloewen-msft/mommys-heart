# Mommy's Heart management website

A Rust + Leptos (Axum SSR + WASM hydrate) application for managing users,
cases, evidence, messages, and audit logs. Data is persisted in **PostgreSQL**
via SQLx, with server-side authentication (argon2 password hashing + session
cookies).

## Run

The database is declared in `.kingdom/services.toml` and raised by Kingdom IDE
while anyone is working on this project — one shared instance, not one per
checkout. With it up there is nothing to configure:

```bash
cargo leptos build     # compile (slow the first time)
cargo leptos watch     # run it, rebuilding as you edit
```

`cargo leptos serve` runs without watching. Leave `DATABASE_URL` unset and the
app uses `postgres://postgres:postgres@localhost:5432/postgres`; a fresh
database seeds itself from the mock fixtures on first boot, so the demo logins
work straight away. To reset it to those fixtures, run
`cargo run --no-default-features --features ssr -- seed` — which wipes data
anyone else is using.

Under Kingdom IDE the build may name its assets after Kingdom's own Leptos app;
this is harmless, as asset URLs are derived from the build's output name at
runtime. Case documents live in a SharePoint document library; with no tenant
configured the app stores them under `target/sharepoint/` instead and logs that
it has done so, so the feature works locally with nothing to set up. See
`.env.example` for the tenant settings.

Without Kingdom, any PostgreSQL 16 will do: point `DATABASE_URL` at it and the
app migrates and seeds on startup.

## AI reports

`/reports` (operations administrators and above) turns a plain-language request — "total
funding received per month, as a line chart" — into a chart, a table and a CSV
export. On the server the request goes to an agent that is given the database's
structure and a read-only query tool: it explores the real data first, checking
how a status is actually spelled or what a date column really contains, and only
then commits to the query behind the report. The exploratory queries and the
final SQL are kept on the page so a number can always be traced back.

**Reports cannot change anything.** Every query the agent runs passes two
independent gates: `server::reports::sql_guard` rejects anything that is not a
single `SELECT`/`WITH` — no second statement, no comments, no writing keyword,
no credential table or column — and `server::reports::execute` then runs it
inside a PostgreSQL `READ ONLY` transaction, with a statement timeout, that is
rolled back afterwards. The second gate is the one that cannot be talked around:
PostgreSQL refuses the write itself.

Needs `AZURE_OPENAI_ENDPOINT`, `AZURE_OPENAI_API_KEY` and
`AZURE_OPENAI_REPORT_DEPLOYMENT` (see `.env.example`). Without them the page
says so rather than failing.

## View email templates

```
cargo run --features ssr -- preview-emails            # writes target/email-preview/
cargo run --features ssr -- preview-emails <out-dir>  # custom output directory
```

