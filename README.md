# Mommy's Heart CRM

A [Leptos](https://leptos.dev) (Rust) + [Axum](https://github.com/tokio-rs/axum)
fullstack app: a CRM website plus a small JSON API that also backs the
embeddable Squarespace chat widget. One Rust crate builds two ways (via
`cargo-leptos`): a native **server** (`ssr`) and a **WebAssembly** client
(`hydrate`).

> V1 is mocked in-memory (see `src/mockdata.rs`); a real database comes later.

## Screens

- **Login** / **Register**
- **Cases** (`/cases`) — view and manage the cases you own or are assigned to;
  `/cases/new` opens a full intake form
- **Inbox** (`/inbox`) — a chat thread per case for everyone assigned to it
- **Grants** (`/grants`) — view and manage grants (admin only)
- **Admin** (`/admin`) — view all users and manage their permissions

## Data model

- **User** — name, contact details, a global `AccountRole` (Client / Volunteer /
  Admin), per-case capabilities, and a change log.
- **Case** — name, status (Open / Monitor / Closed), owner, notes, evidence,
  properties, and a change log.
- **Evidence** — belongs to a case (name, uploader, timestamp, notes).
- **Grant** — name.
- **Message** — a message posted to a case's chat thread.

Permissions are two layers. The global account role gates app-level access
(only Admins reach the Admin dashboard and Grants). Per-case access is a
fine-grained **set of capabilities** (view case, edit, add notes, view/upload/
delete evidence, send messages), so the same person can hold different rights on
different cases. Admins and case owners implicitly hold every capability;
`CasePreset`s (Viewer / Contributor / Manager) seed a common set quickly.

## API endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET`  | `/api/health` | Liveness probe |
| `GET`  | `/api/version` | Version + model config |
| `POST` | `/api/chat` | RAG chat over the `docs/` knowledge base |
| `POST` | `/api/reingest` | Rebuild the vector store from `docs/` |
| `GET`  | `/api/docs/:filename` | Render a `.docx` as HTML |
| `GET`  | `/api/docs/:filename/download` | Download the original `.docx` |

## Getting started

```bash
rustup target add wasm32-unknown-unknown
cargo install cargo-leptos

cp .env.example .env       # optional: Azure OpenAI keys, CORS, CAPTCHA
cargo leptos watch         # dev server at http://127.0.0.1:3000
```

Other commands:

```bash
cargo leptos build         # build server + wasm + CSS into target/site
cargo fmt && cargo clippy --no-default-features --features ssr
```

A [dev container](.devcontainer/devcontainer.json) with the full toolchain is
provided.

## Configuration

Environment variables (see `.env.example`), loaded from `.env` in development:
Azure OpenAI keys (RAG chat), `ALLOWED_ORIGINS` (CORS), and `TURNSTILE_SECRET_KEY`
(optional CAPTCHA). Without Azure credentials the server still runs and
`/api/chat` returns a graceful "not configured" response.
