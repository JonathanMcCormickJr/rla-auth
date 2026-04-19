#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$PROJECT_ROOT"

ENV_FILE="$PROJECT_ROOT/auth-service/.env"
if [ ! -f "$ENV_FILE" ]; then
    echo "Error: .env file not found at $ENV_FILE"
    exit 1
fi

# Export variables from .env so docker compose can interpolate ${POSTGRES_PASSWORD}, etc.
set -a
# shellcheck disable=SC1090
. "$ENV_FILE"
set +a

echo "Stopping compose 'db' service..."
docker compose stop db >/dev/null 2>&1 || true

echo "Removing compose 'db' service (and its volume)..."
docker compose rm -f -v -s db >/dev/null 2>&1 || true
docker volume rm -f "$(basename "$PROJECT_ROOT")_db" >/dev/null 2>&1 || true

echo "Starting fresh 'db' service..."
docker compose up -d db

echo "Waiting for PostgreSQL to be ready..."
for _ in $(seq 1 30); do
    if docker compose exec -T db pg_isready -U postgres >/dev/null 2>&1; then
        echo "PostgreSQL is ready."
        exit 0
    fi
    sleep 1
done

echo "Error: PostgreSQL did not become ready in time."
exit 1
