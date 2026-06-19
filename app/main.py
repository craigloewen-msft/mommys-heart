import logging
import os
from contextlib import asynccontextmanager
from pathlib import Path

import httpx
from fastapi import FastAPI, HTTPException, Request
from fastapi.staticfiles import StaticFiles
from fastapi.responses import FileResponse, HTMLResponse
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel

from app.config import settings, APP_VERSION
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

# Allow the embeddable chat widget (hosted on a different origin, e.g. a
# Squarespace site) to call the API from the browser.
app.add_middleware(
    CORSMiddleware,
    allow_origins=settings.allowed_origins_list,
    allow_credentials=False,
    allow_methods=["*"],
    allow_headers=["*"],
)

app.mount("/static", StaticFiles(directory="app/static"), name="static")


class ChatRequest(BaseModel):
    message: str
    captcha_token: str | None = None


class SourceInfo(BaseModel):
    filename: str
    heading: str
    snippet: str
    relevance: float
    anchor: str = ""


class ChatResponse(BaseModel):
    answer: str
    sources: list[SourceInfo]
    source_type: str = "documents"


@app.get("/")
async def root():
    return FileResponse("app/static/index.html")


@app.get("/healthz")
async def healthz():
    """Lightweight liveness probe for Azure App Service health checks."""
    return {"status": "ok"}


@app.get("/version")
async def version():
    """Report the deployed application version (use to confirm a deploy)."""
    return {
        "version": APP_VERSION,
        "chat_model": settings.AZURE_OPENAI_CHAT_DEPLOYMENT,
        "embedding_model": settings.AZURE_OPENAI_EMBEDDING_DEPLOYMENT,
        # Whether CAPTCHA enforcement is active (true when TURNSTILE_SECRET_KEY
        # is configured). Never exposes the secret itself.
        "captcha_enabled": bool(settings.TURNSTILE_SECRET_KEY),
    }


TURNSTILE_VERIFY_URL = "https://challenge.cloudflare.com/turnstile/v0/siteverify"


def _verify_captcha(token: str | None, remote_ip: str | None) -> bool:
    """Verify a Cloudflare Turnstile token server-side.

    Returns True if CAPTCHA is disabled (no secret configured) or the token is
    valid; False otherwise.
    """
    secret = settings.TURNSTILE_SECRET_KEY
    if not secret:
        return True  # CAPTCHA disabled
    if not token:
        return False
    data = {"secret": secret, "response": token}
    if remote_ip:
        data["remoteip"] = remote_ip
    try:
        resp = httpx.post(TURNSTILE_VERIFY_URL, data=data, timeout=10)
        return bool(resp.json().get("success"))
    except Exception as e:
        logger.error(f"CAPTCHA verification request failed: {e}")
        return False


@app.post("/api/chat", response_model=ChatResponse)
async def chat(request: ChatRequest, http_request: Request):
    if not request.message.strip():
        raise HTTPException(status_code=400, detail="Message cannot be empty")

    if settings.TURNSTILE_SECRET_KEY:
        remote_ip = http_request.client.host if http_request.client else None
        if not _verify_captcha(request.captcha_token, remote_ip):
            raise HTTPException(status_code=403, detail="CAPTCHA verification failed")

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


@app.get("/docs/{filename}/download")
async def download_document(filename: str):
    """Download the original .docx file."""
    # Guard against path traversal: only allow a bare filename inside DOCS_DIR.
    safe_name = os.path.basename(filename)
    filepath = Path(settings.DOCS_DIR) / safe_name
    if safe_name != filename or filepath.suffix != ".docx" or not filepath.is_file():
        raise HTTPException(status_code=404, detail=f"Document '{filename}' not found")
    return FileResponse(
        path=str(filepath),
        filename=safe_name,
        media_type="application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    )


@app.post("/api/reingest")
async def reingest():
    """Force re-ingestion of documents."""
    try:
        result = ingest_documents(force=True)
        return result
    except Exception as e:
        logger.error(f"Re-ingestion failed: {e}")
        raise HTTPException(status_code=500, detail=f"Re-ingestion failed: {str(e)}")
