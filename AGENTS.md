# Working in this repo

## Running the app

The database is **already running**. It is declared in `.kingdom/services.toml`
and Kingdom IDE raises it while any agent is working on this project. There is
nothing to start and nothing to configure:

```bash
cargo leptos build     # compile (slow the first time)
cargo leptos watch     # run it, rebuilding as you edit
```

Use `cargo leptos serve` instead of `watch` if you want a build that does not
rebuild on change. Either prints the address it is serving on.

### Two things that look broken but are not

Kingdom IDE is itself a Leptos app and exports its own `LEPTOS_*` settings into
your shell. `cargo-leptos` reads those in preference to `[package.metadata.leptos]`,
so built assets may be named after *its* app (`target/site/pkg/kingdom-ide.*`)
rather than this one. That is harmless: the served HTML derives every asset URL
from the build's own output name, so the page is styled and hydrates either way.
Don't try to "fix" the filenames, and don't hardcode an asset path in the markup.

Case documents logging `SharePoint not configured; case documents use the
on-disk store` on startup is also expected locally. Case files live in a
SharePoint document library; with no tenant configured the app keeps them in
`target/sharepoint/` instead, which supports the whole feature — browsing,
upload, download, and grant/revoke — so there is nothing to set up to work on
it. Production fails fast rather than falling back.

**These are shared, not yours.** One database serves every agent on this
project at once. Rows you insert, edit or delete are seen by everybody, and
another agent may be reading the table you are writing. Nothing arbitrates that
— so prefer adding your own rows to mutating fixtures others may be asserting
on, and do not truncate or reseed while someone else is working.

### Reaching the database

Leave `DATABASE_URL` unset and the app connects to
`postgres://postgres:postgres@localhost:5432/postgres`, which is where an
isolated plan reaches the shared database.

If your plan is on the machine's own network rather than its own, `localhost`
is not it: your system prompt names the container's real address (something
like `172.31.4.10:5432`). Export it before running:

```bash
export DATABASE_URL=postgres://postgres:postgres@<address>/postgres
```

A fresh database seeds itself from the mock fixtures on first boot, so the demo
logins work immediately. To put it back to those fixtures deliberately — which
**wipes data other agents may be using** — run:

```bash
cargo run --no-default-features --features ssr --target-dir target/oneshot -- seed
```

### One-off commands and compile checks

Give one-off commands their own build directory so their SSR-only artifacts do
not invalidate the SSR+hydrate artifacts `cargo leptos build` creates:

```bash
cargo check --no-default-features --features ssr --target-dir target/oneshot
```

A bare `cargo check` in the repo root works, but it shares `target/` with
cargo-leptos. The two build different feature sets (SSR binary vs hydrate WASM
lib), so they invalidate each other's fingerprints and alternating between them
makes both noticeably slower.

## Cargo tests

This repo does not make use of any `cargo test` functionality so do not write any. If you need any to test your own code then feel free to write it, use it for temporary testing, but then remove it when it is time for the user to review and you are done your task.

## Coding convention

Make sure your comments and concise and short and usually max 1 or 2 sentences.

Top level database 'items' or 'objects' are represented in the coding structure. You can see them under 'src/componenents/server/db/' and they have similar structures under 'server_fns' to expose access to them.
