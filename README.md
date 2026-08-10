# Mommy's Heart management website

A Rust + Leptos (Axum SSR + WASM hydrate) CRM. The CRM domain (users, cases,
evidence, messages, audit logs) is persisted in **PostgreSQL** via SQLx,
with server-side authentication (argon2 password hashing + session cookies).

## Run

```bash
etc/dev-db.sh up      # start this checkout's containers (already seeded)
etc/dev-db.sh build   # compile the app (slow the first time; do this once)
etc/dev-run.sh        # cargo leptos watch, on this checkout's own port
```

`etc/dev-db.sh up` prints the URL to open. `dev-run.sh` does not build — it
exits with a hint if you skip the build step — and prints `MH_READY ...` once
the port is actually accepting connections.

## View email templates

```
cargo run --features ssr -- preview-emails            # writes target/email-preview/
cargo run --features ssr -- preview-emails <out-dir>  # custom output directory
```

