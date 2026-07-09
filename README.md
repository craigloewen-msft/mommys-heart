# Mommy's Heart CRM

A [Leptos](https://leptos.dev) (Rust) + [Axum](https://github.com/tokio-rs/axum)
fullstack app that provides both a **CRM website** and a **dedicated JSON API**
on a single Rust toolchain and a single deployment. The API also backs the
embeddable chat widget used on the Squarespace site.

## Architecture

Everything is one Rust crate compiled two ways (via `cargo-leptos`): a native
**server** binary (`ssr` feature) and a **WebAssembly** client bundle (`hydrate`
feature).

- **Dedicated API** — plain Axum routes under `server/api/` serve JSON at
  `/api/*`, split into one file per feature. This is the single API consumed by
  both the CRM website and the
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
    api/             #   dedicated /api/* Axum routes, split by feature
      mod.rs         #     merges the sub-routers
      health.rs      #     GET /api/health
      version.rs     #     GET /api/version
      chat.rs        #     POST /api/chat (RAG) + POST /api/reingest
      contacts.rs    #     GET /api/contacts(/:id)
      cors.rs        #     CORS layer for the widget
    service.rs       #   business logic (single source of truth)
    config.rs        #   Azure OpenAI / RAG settings from the environment
    data.rs          #   in-memory mock CRM data (to be replaced by a real DB)
    captcha.rs       #   Cloudflare Turnstile verification
    rag/             #   Retrieval-Augmented Generation chatbot pipeline
      mod.rs         #     ingest + query orchestration, in-memory vector store
      documents.rs   #     .docx extraction + chunking (port of app/ingest.py)
      azure.rs       #     Azure OpenAI embeddings + chat completions (reqwest)
      store.rs       #     in-memory cosine-similarity vector store
docs/                # .docx knowledge base ingested by the RAG pipeline
style/tailwind.css   # Tailwind v4 input (brand "primary" palette)
```

## Prerequisites

```bash
rustup target add wasm32-unknown-unknown
cargo install cargo-leptos
```

> **Tip:** a [dev container](.devcontainer/devcontainer.json) is provided. Open
> the repo in VS Code (or a Codespace) and "Reopen in Container" to get the full
> toolchain — Rust stable, the `wasm32-unknown-unknown` target, `cargo-leptos`,
> and ports `3000`/`3001` forwarded — with no local setup. Build artifacts and
> the crate cache live on named volumes, so builds stay fast on Windows/macOS.

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
Azure OpenAI keys (for the RAG chat), `ALLOWED_ORIGINS` (CORS), and
`TURNSTILE_SECRET_KEY` (optional CAPTCHA). Without Azure credentials the server
still runs and `/api/chat` returns a graceful "not configured" response.

## API endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET`  | `/api/health` | Liveness probe |
| `GET`  | `/api/version` | Deployed version + model config |
| `POST` | `/api/chat` | RAG chat over the `docs/` knowledge base |
| `POST` | `/api/reingest` | Rebuild the vector store from `docs/` |
| `GET`  | `/api/docs/:filename` | Render a `.docx` as a styled HTML page (source viewer) |
| `GET`  | `/api/docs/:filename/download` | Download the original `.docx` |
| `GET`  | `/api/contacts` | List CRM contacts (mock data) |
| `GET`  | `/api/contacts/:id` | Fetch a single contact (mock data) |

## RAG chatbot

`POST /api/chat` runs the retrieval-augmented pipeline ported from the original
Python app: the `.docx` files in `docs/` are parsed, chunked (~500 tokens, 100
overlap) and embedded with Azure OpenAI into an in-memory vector store at
startup (in the background, so the server boots immediately). A question is
embedded, the top matches retrieved by cosine similarity, and the chat model
answers — preferring the documents but falling back to general knowledge, and
reporting `source_type` (`documents` / `general_knowledge` / `mixed`) plus the
cited sources. Use `POST /api/reingest` to rebuild the store after changing the
documents.

## Status / roadmap

Working now: project structure, dedicated API split by feature, the real RAG
`/api/chat` over the `docs/` corpus, CORS for the widget, and a basic CRM
template (Dashboard / Contacts / Chat) served with SSR + hydration.

**Case management (in-memory demo):** a client-centric model where each
**Client** owns multiple **Cases** — one per need area (Housing, Family Court,
Immigration, Public Benefits, Mental Health, …). Cases capture volunteer
assignments, **case notes**, documents, cross-links to a client's other
(interconnected) cases, and a **timeline** of actions taken. The `/clients`
directory and `/clients/:id` pathway view show a client's connected needs
together rather than as isolated interactions. Data lives in
`src/mockdata.rs` + reactive `src/state.rs` signals (no database yet).

**Case taxonomy, client mapping & service pathways:** cases are tagged against a
built-in **service taxonomy** (`src/taxonomy.rs`) of five categories — Family
Law, Housing, Immigration, Social Services, and Mental Health & Support — each
with granular service types (e.g. Custody & visitation, Shelter placement, VAWA,
SNAP, Safety planning). Admins tag services when opening a case and edit them
inline on the case card. The **Insights** page (`/insights`) surfaces org-wide
service pathways: demand by category, top service needs, referral pathways
(categories that co-occur for the same client), and service gaps (open needs
that are unassigned or on hold).

Next phases:

- **Services provided, referrals, and outcomes** as structured records on a
  case (design hooks already noted alongside the case model).
- **Real persistence** for the CRM (e.g. SQLite/SQLx) + authentication.
- **Deployment** — containerize the Leptos server and add CI.
