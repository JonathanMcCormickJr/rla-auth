#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$PROJECT_ROOT"

ENV_FILE="$PROJECT_ROOT/auth-service/.env"
if [ -f "$ENV_FILE" ]; then
    # Export variables from .env so docker compose can interpolate them.
    set -a
    # shellcheck disable=SC1090
    . "$ENV_FILE"
    set +a
fi

echo "Stopping compose 'redis' service..."
docker compose stop redis >/dev/null 2>&1 || true

echo "Removing compose 'redis' service..."
docker compose rm -f -v -s redis >/dev/null 2>&1 || true

echo "Starting fresh 'redis' service..."
docker compose up -d redis

echo "Waiting for Redis to be ready..."
for _ in $(seq 1 30); do
    if docker compose exec -T redis redis-cli ping 2>/dev/null | grep -q "PONG"; then
        echo "Redis is ready."
        exit 0
    fi
    sleep 1
done

echo "Error: Redis did not become ready in time."
exit 1
