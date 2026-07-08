# Mommy's Heart CRM

A [Leptos](https://leptos.dev) (Rust) + [Axum](https://github.com/tokio-rs/axum)
fullstack app that provides both a **CRM website** and a **dedicated JSON API**
on a single Rust toolchain and a single deployment. The API also backs the
embeddable chat widget used on the Squarespace site.

## Architecture

Everything is one Rust crate compiled two ways (via `cargo-leptos`): a native
**server** binary (`ssr` feature) and a **WebAssembly** client bundle (`hydrate`
feature).

- **Dedicated API** — plain Axum routes under `server/rest.rs` serve JSON at
  `/api/*`. This is the single API consumed by both the CRM website and the
  Squarespace widget (cross-origin, hence the CORS layer).
- **Website** — Leptos (Vue-like reactive) components render the CRM UI and load
  data through `api_client`, which calls the same `/api/*` endpoints.
- **One source of truth** — both the REST API and the UI data loaders delegate
  to `server/service.rs`.
- **One build / one deploy** — `cargo leptos build` produces the server binary
  plus the hashed wasm/JS/CSS in `target/site`, served by the same process.

```
src/
  main.rs            # Axum server entry (ssr): mounts Leptos routes + /api + CORS
  lib.rs             # module tree + wasm hydrate() entry
  app.rs             # <App/> router + SSR document shell
  types.rs           # API contract types shared by server + client (serde)
  api_client.rs      # UI data client: SSR calls the service; browser fetches /api
  components/layout.rs  # app shell (sidebar + top bar)
  pages/             # Dashboard, Contacts (list + detail), Chat
  server/            # ssr-only
    rest.rs          #   dedicated /api/* Axum routes + CORS
    service.rs       #   business logic (single source of truth)
    data.rs          #   in-memory mock CRM data (to be replaced by a real DB)
    captcha.rs       #   Cloudflare Turnstile verification
style/tailwind.css   # Tailwind v4 input (brand "primary" palette)
```

## Prerequisites

```bash
rustup target add wasm32-unknown-unknown
cargo install cargo-leptos
```

## Getting started

```bash
cp .env.example .env       # fill in values when needed
cargo leptos watch         # dev server with hot reload at http://127.0.0.1:3000
```

Other commands:

```bash
cargo leptos build         # build server + wasm + CSS into target/site
cargo leptos serve         # build once, then run the server
cargo fmt && cargo clippy --no-default-features --features ssr
```

## Configuration

Environment variables (see `.env.example`), loaded from `.env` in development:
Azure OpenAI keys (for the RAG chat once ported), `ALLOWED_ORIGINS` (CORS), and
`TURNSTILE_SECRET_KEY` (optional CAPTCHA).

## API endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET`  | `/api/health` | Liveness probe |
| `GET`  | `/api/version` | Deployed version + model config |
| `POST` | `/api/chat` | Chat (currently a stub; RAG port pending) |
| `GET`  | `/api/contacts` | List CRM contacts (mock data) |
| `GET`  | `/api/contacts/:id` | Fetch a single contact (mock data) |

## Status / roadmap

This is the **scaffold**. Working now: project structure, dedicated API with a
stubbed `/api/chat` (matching the legacy contract), CORS for the widget, and a
basic CRM template (Dashboard / Contacts / Chat) served with SSR + hydration.

Next phases:

- **Port the RAG chatbot** from the original Python app (see git history) to
  Rust; wire the real `/api/chat` + a vector store.
- **Real persistence** for the CRM (e.g. SQLite/SQLx) + authentication.
- **Deployment** — containerize the Leptos server and add CI.
