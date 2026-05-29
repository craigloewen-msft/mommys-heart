import logging
from contextlib import asynccontextmanager

from fastapi import FastAPI, HTTPException
from fastapi.staticfiles import StaticFiles
from fastapi.responses import FileResponse, HTMLResponse
from pydantic import BaseModel

from app.config import settings
from app.ingest import ingest_documents
from app.query import query_documents
from app.viewer import render_docx_to_html

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


def _validate_settings():
    required = {
        "AZURE_OPENAI_ENDPOINT": settings.AZURE_OPENAI_ENDPOINT,
        "AZURE_OPENAI_API_KEY": settings.AZURE_OPENAI_API_KEY,
        "AZURE_OPENAI_CHAT_DEPLOYMENT": settings.AZURE_OPENAI_CHAT_DEPLOYMENT,
        "AZURE_OPENAI_EMBEDDING_DEPLOYMENT": settings.AZURE_OPENAI_EMBEDDING_DEPLOYMENT,
    }
    missing = [k for k, v in required.items() if not v]
    if missing:
        raise RuntimeError(
            f"Missing required environment variables: {', '.join(missing)}. "
            "Please set them via -e flags or --env-file when creating the container."
        )


@asynccontextmanager
async def lifespan(app: FastAPI):
    _validate_settings()
    logger.info("Starting document ingestion...")
    try:
        result = ingest_documents()
        logger.info(f"Ingestion result: {result}")
    except Exception as e:
        logger.error(f"Ingestion failed: {e}")
        logger.error("The app will start but queries will fail until documents are ingested.")
    yield


app = FastAPI(title="Document Search Chatbot", lifespan=lifespan)

app.mount("/static", StaticFiles(directory="app/static"), name="static")


class ChatRequest(BaseModel):
    message: str


class SourceInfo(BaseModel):
    filename: str
    heading: str
    snippet: str
    relevance: float
    anchor: str = ""


class ChatResponse(BaseModel):
    answer: str
    sources: list[SourceInfo]


@app.get("/")
async def root():
    return FileResponse("app/static/index.html")


@app.post("/api/chat", response_model=ChatResponse)
async def chat(request: ChatRequest):
    if not request.message.strip():
        raise HTTPException(status_code=400, detail="Message cannot be empty")

    try:
        result = query_documents(request.message)
        return ChatResponse(**result)
    except Exception as e:
        logger.error(f"Query failed: {e}")
        raise HTTPException(status_code=500, detail=f"Query failed: {str(e)}")


@app.get("/docs/{filename}")
async def view_document(filename: str):
    """Render a .docx file as a styled HTML page with heading anchors."""
    html = render_docx_to_html(filename)
    if html is None:
        raise HTTPException(status_code=404, detail=f"Document '{filename}' not found")
    return HTMLResponse(content=html)


@app.post("/api/reingest")
async def reingest():
    """Force re-ingestion of documents."""
    try:
        result = ingest_documents(force=True)
        return result
    except Exception as e:
        logger.error(f"Re-ingestion failed: {e}")
        raise HTTPException(status_code=500, detail=f"Re-ingestion failed: {str(e)}")
