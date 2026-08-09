#!/usr/bin/env bash
# Assert the role/subtype invariants hold in this checkout's dev database.
#
#   users.role = 'volunteer'  <=>  volunteers row with status = 'approved'
#   users.role = 'client'      =>  clients row
#
# These are maintained by `users::apply_role_in` (the only code allowed to change
# a role) and by the seed. Nothing in Postgres enforces them -- a cross-table
# invariant can't be a CHECK constraint -- so this script is how we notice if a
# new write path ever bypasses that function. Prints offending rows and exits 1.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
[ -f "$REPO_ROOT/.env.local" ] || "$REPO_ROOT/etc/dev-db.sh" up >/dev/null
set -a; source "$REPO_ROOT/.env.local"; set +a

WSLC="${WSLC:-wslc.exe}"

problems=$("$WSLC" exec "mh-db-$MH_INSTANCE" psql -U "${POSTGRES_USER:-mommysheart}" \
  -d "${POSTGRES_DB:-mommysheart}" -tA -F' | ' -c "
SELECT 'volunteer role without an approved record', u.id, u.email
  FROM users u LEFT JOIN volunteers v ON v.user_id = u.id
 WHERE u.role = 'volunteer' AND (v.user_id IS NULL OR v.status <> 'approved')
UNION ALL
SELECT 'approved record without the volunteer role', v.user_id, u.email
  FROM volunteers v JOIN users u ON u.id = v.user_id
 WHERE v.status = 'approved' AND u.role <> 'volunteer'
UNION ALL
SELECT 'client role without a client record', u.id, u.email
  FROM users u LEFT JOIN clients c ON c.user_id = u.id
 WHERE u.role = 'client' AND c.user_id IS NULL;" | tr -d '\r' | sed '/^$/d')

if [ -n "$problems" ]; then
  echo "Role/subtype integrity violations:"
  echo "$problems"
  exit 1
fi
echo "Role/subtype integrity OK."
