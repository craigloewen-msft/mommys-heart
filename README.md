# Mommy's Heart management website

A Rust + Leptos (Axum SSR + WASM hydrate) CRM. The CRM domain (users, cases,
evidence, messages, audit logs) is persisted in **PostgreSQL** via SQLx,
with server-side authentication (argon2 password hashing + session cookies).

## Run

```bash
etc/dev-db.sh up      # start this checkout's containers (already seeded)
etc/dev-run.sh        # cargo leptos watch, on this checkout's own port
```

`etc/dev-db.sh up` prints the URL to open.

## View email templates

```
cargo run --features ssr -- preview-emails            # writes target/email-preview/
cargo run --features ssr -- preview-emails <out-dir>  # custom output directory
```

