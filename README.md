# Mommy's Heart CRM

A [Nuxt 3](https://nuxt.com) (Vue 3 + TypeScript) fullstack app that provides
both a **CRM website** and a **dedicated JSON API** on a single toolchain and a
single deployment. The API also backs the embeddable chat widget used on the
Squarespace site.

> **Migrating from Python.** This project began as a Python/FastAPI RAG chatbot.
> That original app is archived under [`legacy/`](legacy/) (kept runnable) while
> we rebuild on Nuxt. See [`legacy/README-legacy.md`](legacy/README-legacy.md).

## Architecture

- **Dedicated API** — Nitro server routes under `server/api/*` return JSON. This
  is the single API consumed by both the CRM website and the Squarespace widget.
- **Website** — Vue pages/components under `pages/`, `components/`, `layouts/`
  form the CRM UI and call the same API via the typed `useApi()` composable.
- **One build / one deploy** — `nuxt build` produces a single Node server
  (`.output/server/index.mjs`) serving both the API and the site.

```
server/          # dedicated API (Nitro)
  api/           #   /api/health, /api/version, /api/chat, /api/contacts
  middleware/    #   CORS for cross-origin Squarespace widget calls
  utils/         #   e.g. Cloudflare Turnstile CAPTCHA verification
  data/          #   in-memory mock CRM data (to be replaced by a real DB)
shared/types/    # API contract types shared by server + client
pages/           # CRM website routes (Dashboard, Contacts, Chat)
layouts/         # app shell (sidebar + top bar)
composables/     # useApi() typed API client
legacy/          # archived Python/FastAPI RAG app (reference for the port)
```

## Getting started

```bash
npm install
cp .env.example .env    # fill in values when needed
npm run dev             # http://localhost:3000
```

Other scripts: `npm run build`, `npm run start` (run the built server),
`npm run lint`, `npm run format`.

## Configuration

Runtime config is driven by `NUXT_*` environment variables (see `.env.example`),
mapped in `nuxt.config.ts`. The Azure OpenAI + Turnstile keys are carried over
from the legacy app and used once the RAG chat is ported.

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
basic CRM template (Dashboard / Contacts / Chat).

Next phases:

- **Port the RAG chatbot** from `legacy/` to the JS SDK; wire the real
  `/api/chat` + a vector store (e.g. SQLite + `sqlite-vec`).
- **Real persistence** for the CRM (SQLite via Drizzle) + authentication.
- **Deployment** — update the Azure App Service startup command to the Node
  server and refresh the GitHub Actions workflow (currently still the Python
  pipeline).
