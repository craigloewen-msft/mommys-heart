# Mommy's Heart management website

A Rust + Leptos (Axum SSR + WASM hydrate) CRM. The CRM domain (users, cases,
evidence, grants, messages, audit logs) is persisted in **PostgreSQL** via SQLx,
with server-side authentication (argon2 password hashing + session cookies).

## Run

`cargo leptos watch`

## View email templates

```
cargo run --features ssr -- preview-emails            # writes target/email-preview/
cargo run --features ssr -- preview-emails <out-dir>  # custom output directory
```

