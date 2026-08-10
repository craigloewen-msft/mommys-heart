#!/usr/bin/env bash
# Run the app against THIS checkout's own containers and ports.
#
# `cargo leptos` reads its site address and live-reload port from the process
# environment before the app itself starts, so those cannot come from .env.local
# the way DATABASE_URL does. Exporting them is the reason this wrapper exists.
#
# This wrapper does NOT build. Run `etc/dev-db.sh build` first; starting a cold
# compile here would sit inside a caller's readiness timeout with no output.
#
# Usage:
#   etc/dev-run.sh              # cargo leptos watch (default)
#   etc/dev-run.sh serve        # cargo leptos serve
#   etc/dev-run.sh -- <cmd...>  # any command with this instance's env set,
#                               #   e.g. etc/dev-run.sh -- cargo test
#
# Readiness contract for automated harnesses: once the site port actually
# accepts connections this prints a single line beginning `MH_READY`. If the
# server dies first it prints `MH_FAILED` and exits non-zero. Wait for one of
# those two tokens — never for an app log line, which may never arrive.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# Unconditional: `up` is idempotent, and a stale .env.local pointing at stopped
# containers otherwise makes the app panic at startup and hang the caller.
"$REPO_ROOT/etc/dev-db.sh" up >/dev/null

set -a; source "$REPO_ROOT/.env.local"; set +a
cd "$REPO_ROOT"

# One-off commands get their own build directory. Sharing `target/` with a live
# `cargo leptos watch` makes the two invalidate each other's fingerprints (they
# build different feature sets), turning every watch rebuild into a slow one.
if [ "${1:-}" = "--" ]; then
  shift
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target/oneshot}"
  exec "$@"
fi

# Refuse to run unbuilt. A cold `cargo leptos` build takes many minutes and
# would be invisible to a caller waiting on readiness.
if ! compgen -G "$REPO_ROOT/target/site/pkg/*" >/dev/null 2>&1; then
  echo "dev-run: not built yet — run 'etc/dev-db.sh build' first" >&2
  exit 1
fi

SITE_HOST="${LEPTOS_SITE_ADDR%:*}"
SITE_PORT="${LEPTOS_SITE_ADDR##*:}"

echo "Starting '$MH_INSTANCE' on http://$LEPTOS_SITE_ADDR"

# Run as a child, not exec, so we can watch for readiness alongside it.
cargo leptos "${1:-watch}" "${@:2}" &
CHILD=$!

# Forward termination to the child so Ctrl-C leaves no orphaned cargo/leptos.
trap 'kill -TERM "$CHILD" 2>/dev/null || true' INT TERM

# Poll the real socket rather than scraping logs: a port that accepts a
# connection cannot be a false positive.
port_open() { (exec 3<>"/dev/tcp/$SITE_HOST/$SITE_PORT") 2>/dev/null; }

while ! port_open; do
  if ! kill -0 "$CHILD" 2>/dev/null; then
    status=0; wait "$CHILD" 2>/dev/null || status=$?
    [ "$status" -ne 0 ] || status=1
    echo "MH_FAILED dev-run: server exited before becoming ready (status $status)"
    exit "$status"
  fi
  sleep 0.5
done

echo "MH_READY listening on http://$LEPTOS_SITE_ADDR"

wait "$CHILD"
