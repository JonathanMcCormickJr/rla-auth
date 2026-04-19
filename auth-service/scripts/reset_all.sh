#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "==> Refreshing Postgres container..."
"$SCRIPT_DIR/reset-docker-db.sh"

echo "==> Refreshing Redis container..."
"$SCRIPT_DIR/reset-docker-redis.sh"

echo "==> All containers refreshed."
