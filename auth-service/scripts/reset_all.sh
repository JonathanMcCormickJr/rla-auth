#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

ENV_FILE="$PROJECT_ROOT/auth-service/.env"
if [ -f "$ENV_FILE" ]; then
    set -a
    # shellcheck disable=SC1090
    . "$ENV_FILE"
    set +a
fi

echo "==> Stopping app containers (freeing host ports 3000/8000)..."
(cd "$PROJECT_ROOT" && docker compose stop auth-service app-service >/dev/null 2>&1 || true)

echo "==> Refreshing Postgres container..."
"$SCRIPT_DIR/reset-docker-db.sh"

echo "==> Refreshing Redis container..."
"$SCRIPT_DIR/reset-docker-redis.sh"

echo "==> All containers refreshed."
