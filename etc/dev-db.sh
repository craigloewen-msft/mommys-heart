#!/usr/bin/env bash
# Stand up (or tear down) the local development backing services using wslc.exe —
# the WSL container CLI (a Docker-equivalent). This manages BOTH containers the
# SSR backend needs:
#   * PostgreSQL      — the CRM database (via DATABASE_URL in .env)
#   * Azurite         — the Azure Storage emulator for case evidence files
#                       (via AZURE_STORAGE_CONNECTION_STRING in .env)
#
# Usage:
#   etc/dev-db.sh up            # create + start both containers (idempotent)
#   etc/dev-db.sh down          # stop + remove both containers (keeps data volumes)
#   etc/dev-db.sh reset         # remove both containers AND their data volumes
#   etc/dev-db.sh seed          # (re)populate the database with demo/test data
#   etc/dev-db.sh logs [db|storage]   # tail a container's logs (default: db)
#   etc/dev-db.sh psql          # open a psql shell inside the database container
set -euo pipefail

WSLC="${WSLC:-wslc.exe}"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# ── PostgreSQL ──────────────────────────────────────────────────────────────
DB_NAME="mommys-heart-db"
DB_VOLUME="mommys-heart-pgdata"
DB_IMAGE="postgres:16"
POSTGRES_USER="${POSTGRES_USER:-mommysheart}"
POSTGRES_PASSWORD="${POSTGRES_PASSWORD:-localdev}"
POSTGRES_DB="${POSTGRES_DB:-mommysheart}"
HOST_PORT="${HOST_PORT:-5432}"

# ── Azurite (Azure Storage emulator) ────────────────────────────────────────
STORAGE_NAME="mommys-heart-storage"
STORAGE_VOLUME="mommys-heart-blobdata"
STORAGE_IMAGE="mcr.microsoft.com/azure-storage/azurite"
BLOB_PORT="${BLOB_PORT:-10000}"

db_up() {
  if "$WSLC" list --all 2>/dev/null | grep -q "$DB_NAME"; then
    echo "Container '$DB_NAME' already exists; starting it."
    "$WSLC" start "$DB_NAME"
  else
    echo "Creating and starting '$DB_NAME' ($DB_IMAGE)."
    "$WSLC" run -d \
      --name "$DB_NAME" \
      -e "POSTGRES_USER=$POSTGRES_USER" \
      -e "POSTGRES_PASSWORD=$POSTGRES_PASSWORD" \
      -e "POSTGRES_DB=$POSTGRES_DB" \
      -p "$HOST_PORT:5432" \
      -v "$DB_VOLUME:/var/lib/postgresql/data" \
      "$DB_IMAGE"
  fi
  echo "Postgres is listening on 127.0.0.1:$HOST_PORT"
  echo "DATABASE_URL=******127.0.0.1:$HOST_PORT/$POSTGRES_DB"
}

storage_up() {
  if "$WSLC" list --all 2>/dev/null | grep -q "$STORAGE_NAME"; then
    echo "Container '$STORAGE_NAME' already exists; starting it."
    "$WSLC" start "$STORAGE_NAME"
  else
    echo "Creating and starting '$STORAGE_NAME' ($STORAGE_IMAGE)."
    # --skipApiVersionCheck lets the newer Azure SDK's x-ms-version header work
    # against the emulator without pinning it to an exact Azurite release.
    "$WSLC" run -d \
      --name "$STORAGE_NAME" \
      -p "$BLOB_PORT:10000" \
      -v "$STORAGE_VOLUME:/data" \
      "$STORAGE_IMAGE" \
      azurite-blob --blobHost 0.0.0.0 --skipApiVersionCheck
  fi
  echo "Azurite blob service is listening on 127.0.0.1:$BLOB_PORT"
  echo "AZURE_STORAGE_CONNECTION_STRING (dev, well-known key) — see .env.example"
}

up() {
  db_up
  storage_up
}

down() {
  for name in "$DB_NAME" "$STORAGE_NAME"; do
    "$WSLC" stop "$name" 2>/dev/null || true
    "$WSLC" remove "$name" 2>/dev/null || true
  done
  echo "Removed containers '$DB_NAME' and '$STORAGE_NAME' (data volumes kept)."
}

reset() {
  down
  "$WSLC" volume remove "$DB_VOLUME" 2>/dev/null || true
  "$WSLC" volume remove "$STORAGE_VOLUME" 2>/dev/null || true
  echo "Removed data volumes '$DB_VOLUME' and '$STORAGE_VOLUME'."
}

# Load demo/test data into the database. Reuses the app's own seeder (so the demo
# passwords are hashed correctly and the fixtures never drift), wiping any
# existing CRM data first. Reads DATABASE_URL from the project's .env.
seed() {
  echo "Populating the database with demo/test data..."
  ( cd "$REPO_ROOT" && cargo run --quiet --no-default-features --features ssr -- seed )
  echo "Done. Demo logins are listed in the README."
}

logs() {
  case "${1:-db}" in
    db) "$WSLC" logs -f "$DB_NAME" ;;
    storage) "$WSLC" logs -f "$STORAGE_NAME" ;;
    *) echo "Usage: $0 logs [db|storage]" >&2; exit 1 ;;
  esac
}

case "${1:-up}" in
  up) up ;;
  down) down ;;
  reset) reset ;;
  seed) seed ;;
  logs) logs "${2:-db}" ;;
  psql) "$WSLC" exec -it "$DB_NAME" psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" ;;
  *) echo "Usage: $0 {up|down|reset|seed|logs|psql}" >&2; exit 1 ;;
esac
