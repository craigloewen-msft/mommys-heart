#!/usr/bin/env bash
# Run the app against THIS checkout's own containers and ports.
#
# `cargo leptos` reads its site address and live-reload port from the process
# environment before the app itself starts, so those cannot come from .env.local
# the way DATABASE_URL does. Exporting them is the reason this wrapper exists.
#
# Usage:
#   etc/dev-run.sh              # cargo leptos watch (default)
#   etc/dev-run.sh serve        # cargo leptos serve
#   etc/dev-run.sh -- <cmd...>  # any command with this instance's env set,
#                               #   e.g. etc/dev-run.sh -- cargo test
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
[ -f "$REPO_ROOT/.env.local" ] || "$REPO_ROOT/etc/dev-db.sh" up

set -a; source "$REPO_ROOT/.env.local"; set +a
cd "$REPO_ROOT"

if [ "${1:-}" = "--" ]; then shift; exec "$@"; fi

echo "Starting '$MH_INSTANCE' on http://$LEPTOS_SITE_ADDR"
exec cargo leptos "${1:-watch}" "${@:2}"
