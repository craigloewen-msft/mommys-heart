#!/bin/bash
# Startup command for Azure App Service (Linux, Python).
#
# Set this as the "Startup Command" of your Web App:
#   az webapp config set -g <rg> -n <app> --startup-file "startup.sh"
#
# Notes:
# - A SINGLE worker is used on purpose: document ingestion runs on startup and
#   writes to a local ChromaDB store. Multiple workers would race to rebuild the
#   collection and can corrupt it. Scale out with more App Service instances
#   instead, or move ChromaDB to a shared/remote store first.
# - Azure App Service expects the app to listen on the port given by $PORT
#   (defaults to 8000 for Python).
# - The long timeout gives the startup ingestion (embedding the .docx files via
#   Azure OpenAI) time to finish before the worker is considered unhealthy.
gunicorn app.main:app \
  --worker-class uvicorn.workers.UvicornWorker \
  --workers 1 \
  --bind 0.0.0.0:${PORT:-8000} \
  --timeout 600 \
  --access-logfile - \
  --error-logfile -
