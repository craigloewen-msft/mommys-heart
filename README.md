# Document Search Chatbot

A RAG chatbot that answers questions about the `.docx` files in `docs/` and cites its sources. Documents are chunked and embedded with Azure OpenAI (stored in ChromaDB); questions are embedded, matched, and answered by the chat model. When the documents don't cover a question, the model falls back to its own general knowledge — each answer is tagged with a `source_type` (`documents` / `general_knowledge` / `mixed`) so the UI can clearly distinguish the two.

## Configuration

Set these as environment variables (locally via `.env`, copied from `.env.example`):

```env
AZURE_OPENAI_ENDPOINT=https://your-resource-name.openai.azure.com/
AZURE_OPENAI_API_KEY=your-api-key-here
AZURE_OPENAI_CHAT_DEPLOYMENT=gpt-4o
AZURE_OPENAI_EMBEDDING_DEPLOYMENT=text-embedding-ada-002
AZURE_OPENAI_API_VERSION=2024-12-01-preview
ALLOWED_ORIGINS=*   # set to the embedding site's URL in production
```

> `AZURE_OPENAI_ENDPOINT` must be the bare resource URL (no `/openai/` suffix) or requests return `404`.

## Run Locally (container)

```powershell
wslc.exe build -t doc-search .
wslc.exe create --name doc-search -p 8000:8000 --env-file .env doc-search
wslc.exe start doc-search
```

Open http://localhost:8000. To update documents, add `.docx` files to `docs/`, rebuild, and recreate the container (or `POST /api/reingest`).

## Deploy to Azure App Service (Python 3.12, Linux)

Code deploy — the included `startup.sh` runs the app under Gunicorn.

```powershell
# Startup command
az webapp config set -g <rg> -n <app> --startup-file "startup.sh"

# App settings (same keys as .env), plus build flag
az webapp config appsettings set -g <rg> -n <app> --settings `
  AZURE_OPENAI_ENDPOINT="https://your-resource-name.openai.azure.com/" `
  AZURE_OPENAI_API_KEY="..." `
  AZURE_OPENAI_CHAT_DEPLOYMENT="gpt-4o" `
  AZURE_OPENAI_EMBEDDING_DEPLOYMENT="text-embedding-ada-002" `
  AZURE_OPENAI_API_VERSION="2024-12-01-preview" `
  ALLOWED_ORIGINS="https://www.mommysheartfoundation.com,https://mommysheartfoundation.com" `
  SCM_DO_BUILD_DURING_DEPLOYMENT="true"

# Deploy and verify
az webapp up -g <rg> -n <app> --runtime "PYTHON:3.12"
curl https://<app>.azurewebsites.net/healthz
```

Runs as a **single Gunicorn worker** (ingestion writes to a local ChromaDB store); scale *out* with more instances rather than adding workers.

## Squarespace Chat Widget

Paste the contents of [`widget/squarespace-chat-widget.html`](widget/squarespace-chat-widget.html) into a **Code block** on the page where you want the assistant (Edit page → Add Block → Code). It renders an inline, full AI chat pane wired to the deployed API. If the API URL changes, update `window.MH_CHAT_API_BASE` in the snippet and the API's `ALLOWED_ORIGINS`.

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET`  | `/` | Chat web UI |
| `GET`  | `/healthz` | Health check |
| `GET`  | `/version` | Report the deployed app version |
| `POST` | `/api/chat` | Send a message, get answer + sources + `source_type` |
| `GET`  | `/docs/{filename}` | View a source document as HTML (with heading anchors) |
| `GET`  | `/docs/{filename}/download` | Download the original `.docx` |
| `POST` | `/api/reingest` | Force re-ingestion of documents |
