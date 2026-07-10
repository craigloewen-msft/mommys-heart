#!/usr/bin/env bash
# Stand up (or tear down) a local PostgreSQL container for development using
# wslc.exe — the WSL container CLI (a Docker-equivalent). The app's SSR backend
# connects to it via the DATABASE_URL in .env.
#
# Usage:
#   etc/dev-db.sh up      # create + start the container (idempotent)
#   etc/dev-db.sh down    # stop + remove the container (keeps the data volume)
#   etc/dev-db.sh reset   # remove the container AND its data volume
#   etc/dev-db.sh seed    # (re)populate the database with demo/test data
#   etc/dev-db.sh logs    # tail container logs
#   etc/dev-db.sh psql    # open a psql shell inside the container
set -euo pipefail

WSLC="${WSLC:-wslc.exe}"
NAME="mommys-heart-db"
VOLUME="mommys-heart-pgdata"
IMAGE="postgres:16"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

POSTGRES_USER="${POSTGRES_USER:-mommysheart}"
POSTGRES_PASSWORD="${POSTGRES_PASSWORD:-localdev}"
POSTGRES_DB="${POSTGRES_DB:-mommysheart}"
HOST_PORT="${HOST_PORT:-5432}"

up() {
  if "$WSLC" list --all 2>/dev/null | grep -q "$NAME"; then
    echo "Container '$NAME' already exists; starting it."
    "$WSLC" start "$NAME"
  else
    echo "Creating and starting '$NAME' ($IMAGE)."
    "$WSLC" run -d \
      --name "$NAME" \
      -e "POSTGRES_USER=$POSTGRES_USER" \
      -e "POSTGRES_PASSWORD=$POSTGRES_PASSWORD" \
      -e "POSTGRES_DB=$POSTGRES_DB" \
      -p "$HOST_PORT:5432" \
      -v "$VOLUME:/var/lib/postgresql/data" \
      "$IMAGE"
  fi
  echo "Postgres is listening on 127.0.0.1:$HOST_PORT"
  echo "DATABASE_URL=postgres://$POSTGRES_USER:$POSTGRES_PASSWORD@127.0.0.1:$HOST_PORT/$POSTGRES_DB"
}

down() {
  "$WSLC" stop "$NAME" 2>/dev/null || true
  "$WSLC" remove "$NAME" 2>/dev/null || true
  echo "Removed container '$NAME' (data volume '$VOLUME' kept)."
}

reset() {
  down
  "$WSLC" volume remove "$VOLUME" 2>/dev/null || true
  echo "Removed data volume '$VOLUME'."
}

# Load demo/test data into the database. Reuses the app's own seeder (so the demo
# passwords are hashed correctly and the fixtures never drift), wiping any
# existing CRM data first. Reads DATABASE_URL from the project's .env.
seed() {
  echo "Populating the database with demo/test data..."
  ( cd "$REPO_ROOT" && cargo run --quiet --no-default-features --features ssr -- seed )
  echo "Done. Demo logins are listed in the README."
}

case "${1:-up}" in
  up) up ;;
  down) down ;;
  reset) reset ;;
  seed) seed ;;
  logs) "$WSLC" logs -f "$NAME" ;;
  psql) "$WSLC" exec -it "$NAME" psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" ;;
  *) echo "Usage: $0 {up|down|reset|seed|logs|psql}" >&2; exit 1 ;;
esac
