# Document Search Chatbot

A proof-of-concept RAG (Retrieval-Augmented Generation) chatbot that answers questions about Word documents and cites its sources.

## How It Works

1. **Ingestion** — `.docx` files in `docs/` are parsed (paragraphs + tables), chunked, and embedded using Azure OpenAI. Embeddings are stored in ChromaDB.
2. **Query** — Your question is embedded, the most relevant chunks are retrieved, and Azure OpenAI generates an answer citing the source documents.
3. **UI** — A simple web chat interface shows the answer with expandable source cards.

## Prerequisites

- **wslc.exe** (Windows Subsystem for Linux Containers)
- **Azure OpenAI** resource with two model deployments:
  - A **chat model** (e.g., `gpt-4o`)
  - An **embedding model** (e.g., `text-embedding-ada-002`)

## Setup

### 1. Configure Environment Variables

Copy the example env file and fill in your Azure OpenAI credentials:

```powershell
Copy-Item .env.example .env
```

Edit `.env` with your values:

```env
AZURE_OPENAI_ENDPOINT=https://your-resource-name.openai.azure.com/
AZURE_OPENAI_API_KEY=your-api-key-here
AZURE_OPENAI_CHAT_DEPLOYMENT=gpt-4o
AZURE_OPENAI_EMBEDDING_DEPLOYMENT=text-embedding-ada-002
AZURE_OPENAI_API_VERSION=2024-12-01-preview
```

### 2. Build the Container Image

```powershell
wslc.exe build -t doc-search .
```

### 3. Run the Container

```powershell
wslc.exe create --name doc-search -p 8000:8000 --env-file .env doc-search
```

### 4. Open the App

Navigate to [http://localhost:8000](http://localhost:8000) in your browser.

## Adding New Documents

1. Place new `.docx` files in the `docs/` folder.
2. Rebuild the container image:
   ```powershell
   wslc.exe build -t doc-search .
   ```
3. Stop and recreate the container:
   ```powershell
   wslc.exe container rm doc-search
   wslc.exe create --name doc-search -p 8000:8000 --env-file .env doc-search
   ```

Alternatively, mount the docs folder as a volume so you don't need to rebuild:

```powershell
wslc.exe create --name doc-search -p 8000:8000 --env-file .env -v "E:\eDev\copilot\doc-search-test\docs:/app/docs:ro" doc-search
```

Then call the re-ingest endpoint after adding new files:

```powershell
curl -X POST http://localhost:8000/api/reingest
```

## Persisting Embeddings Across Container Restarts

To avoid re-computing embeddings each time the container is recreated, mount a volume for ChromaDB data:

```powershell
wslc.exe create --name doc-search -p 8000:8000 --env-file .env -v doc-search-chroma:/app/chroma_data doc-search
```

## Project Structure

```
├── docs/                  # Your Word documents
├── app/
│   ├── main.py            # FastAPI app with chat endpoint
│   ├── ingest.py          # Document parsing, chunking, embedding
│   ├── query.py           # RAG retrieval + LLM answer generation
│   ├── config.py          # Environment variable configuration
│   └── static/
│       └── index.html     # Chat web UI
├── .env.example           # Environment variable template
├── requirements.txt       # Python dependencies
├── Dockerfile             # Container build definition
└── README.md              # This file
```

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/` | Chat web UI |
| `POST` | `/api/chat` | Send a message, get answer + sources |
| `POST` | `/api/reingest` | Force re-ingestion of documents |
