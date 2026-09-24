# Mommy's Heart management website

A Rust + Leptos (Axum SSR + WASM hydrate) application for managing users,
cases, evidence, messages, and audit logs. Data is persisted in **PostgreSQL**
via SQLx, with server-side authentication (argon2 password hashing + session
cookies).

## Run

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

## View email templates

```
cargo run --features ssr -- preview-emails            # writes target/email-preview/
cargo run --features ssr -- preview-emails <out-dir>  # custom output directory
```

## License

MIT — see [LICENSE](LICENSE). The guides under `docs/` are Foundation content,
not software, and are not covered by it.
