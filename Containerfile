FROM python:3.12-slim

WORKDIR /app

# Install system deps for chromadb (sqlite3, build tools)
RUN apt-get update && \
    apt-get install -y --no-install-recommends build-essential && \
    rm -rf /var/lib/apt/lists/*

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY app/ ./app/
COPY docs/ ./docs/

EXPOSE 8000

ENV DOCS_DIR=/app/docs
ENV CHROMA_DIR=/app/chroma_data

CMD ["uvicorn", "app.main:app", "--host", "0.0.0.0", "--port", "8000"]
