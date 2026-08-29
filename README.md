# Mommy's Heart management website

A Rust + Leptos (Axum SSR + WASM hydrate) application for managing users,
cases, evidence, messages, and audit logs. Data is persisted in **PostgreSQL**
via SQLx, with server-side authentication (argon2 password hashing + session
cookies).

## Run

The database and blob storage are declared in `.kingdom/services.toml` and
raised by Kingdom IDE while anyone is working on this project — one shared set,
not one per checkout. With them up there is nothing to configure:

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
runtime. Evidence uploads are disabled unless blob storage is configured, and
the server logs a warning and carries on.

Without Kingdom, any PostgreSQL 16 will do: point `DATABASE_URL` at it and the
app migrates and seeds on startup.

### Upgrading from the old per-checkout containers

`etc/dev.sh` is archived at `etc/archived/dev.sh` and no longer used. It left
per-checkout containers and volumes behind, which you can clear once you no
longer want their data:

```bash
docker rm -f $(docker ps -aq --filter name='^mh-(db|storage)-') 2>/dev/null
docker volume rm $(docker volume ls -q --filter name='^mh-(pgdata|blobdata)-') 2>/dev/null
```

## View email templates

```
cargo run --features ssr -- preview-emails            # writes target/email-preview/
cargo run --features ssr -- preview-emails <out-dir>  # custom output directory
```

