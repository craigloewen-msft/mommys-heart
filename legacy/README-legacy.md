# Legacy: Python/FastAPI RAG chatbot

This folder is the **original** application — a Python/FastAPI RAG chatbot that
answered questions about the `.docx` files in `docs/`, served a chat UI, exposed
a JSON API (`/api/chat`, `/healthz`, `/version`, doc viewers), and shipped an
embeddable Squarespace widget.

It has been **archived here** while the project is rewritten as a Nuxt 3 (Vue +
TypeScript) fullstack CRM app at the repo root. Nothing here is wired into the
new app — it is kept intact as a **reference for porting the RAG logic** and the
Squarespace widget, and so we don't lose the working code or deploy notes.

## What's in here

- `app/` — FastAPI app (ingestion, query/RAG, docx viewer, chat + widget API).
- `widget/squarespace-chat-widget.html` — the embeddable Squarespace chat widget.
- `docs/` — the `.docx` corpus used by the RAG chatbot.
- `requirements.txt`, `startup.sh`, `Containerfile`, `.containerignore` — the
  Python runtime / Azure App Service deploy setup.
- `.env.example` — the Azure OpenAI + CORS + Turnstile environment variables.
- `README.md` — the original project README (full run/deploy/API docs).

## Running the legacy app (from this folder)

The original commands assume repo-root paths (e.g. `app.main:app`, `docs/`).
Run them from **inside `legacy/`**:

```bash
cd legacy
python -m venv .venv && source .venv/bin/activate
pip install -r requirements.txt
cp .env.example .env   # then fill in your Azure OpenAI keys
uvicorn app.main:app --host 0.0.0.0 --port 8000
```

See `README.md` in this folder for the full container and Azure App Service
deployment instructions.

## Porting status

The new Nuxt app currently ships a **stub** `/api/chat` with the same response
shape (`answer`, `sources`, `source_type`). Porting the real RAG pipeline
(ingestion, embeddings, vector search, chat) to the JS/TypeScript SDK is a later
phase — this folder is the source of truth for that work.
