import os
from dotenv import load_dotenv

load_dotenv()

# Application version. Bump this whenever the app's behavior changes so the
# deployed build can be checked via the GET /version endpoint.
APP_VERSION = "1.3.1"


class Settings:
    AZURE_OPENAI_ENDPOINT: str = os.environ.get("AZURE_OPENAI_ENDPOINT", "")
    AZURE_OPENAI_API_KEY: str = os.environ.get("AZURE_OPENAI_API_KEY", "")
    AZURE_OPENAI_CHAT_DEPLOYMENT: str = os.environ.get("AZURE_OPENAI_CHAT_DEPLOYMENT", "gpt-4o")
    AZURE_OPENAI_EMBEDDING_DEPLOYMENT: str = os.environ.get("AZURE_OPENAI_EMBEDDING_DEPLOYMENT", "text-embedding-ada-002")
    AZURE_OPENAI_API_VERSION: str = os.environ.get("AZURE_OPENAI_API_VERSION", "2024-12-01-preview")
    DOCS_DIR: str = os.environ.get("DOCS_DIR", "docs")
    CHROMA_DIR: str = os.environ.get("CHROMA_DIR", "chroma_data")
    # Comma-separated list of origins allowed to call the API (e.g. your
    # Squarespace site). Use "*" to allow any origin. Defaults to "*" so the
    # embeddable chat widget works out of the box; tighten this in production.
    ALLOWED_ORIGINS: str = os.environ.get("ALLOWED_ORIGINS", "*")

    # Cloudflare Turnstile CAPTCHA secret key. When set, the /api/chat endpoint
    # requires a valid Turnstile token and verifies it server-side. Leave empty
    # to disable CAPTCHA verification (e.g. local development).
    TURNSTILE_SECRET_KEY: str = os.environ.get("TURNSTILE_SECRET_KEY", "")

    @property
    def allowed_origins_list(self) -> list[str]:
        return [o.strip() for o in self.ALLOWED_ORIGINS.split(",") if o.strip()]


settings = Settings()
