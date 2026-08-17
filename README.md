# Mommy's Heart management website

A Rust + Leptos (Axum SSR + WASM hydrate) application for managing users,
cases, evidence, messages, and audit logs. Data is persisted in **PostgreSQL**
via SQLx, with server-side authentication (argon2 password hashing + session
cookies).

## Run

```bash
etc/dev.sh build      # containers + compile the app (slow the first time)
etc/dev.sh run        # run that build in the foreground on this checkout's port
```

That is the whole workflow. Both commands start this checkout's own database
and storage containers, so several checkouts can run at once without
interfering. `run` prints the URL and then `MH_READY ...` once the port is
actually accepting connections. It never builds or watches source: stop it
before editing, rebuild explicitly, and start a fresh run for the next test pass.
Stopping `run` also stops this checkout's containers while preserving their data.

Also available: `etc/dev.sh reset` (wipe the database back to fresh seed data),
`etc/dev.sh clean` (remove all Mommy's Heart dev instances and volumes), and
`etc/dev.sh -- <cmd>` (run one command with this instance's env).

## View email templates

```
cargo run --features ssr -- preview-emails            # writes target/email-preview/
cargo run --features ssr -- preview-emails <out-dir>  # custom output directory
```

