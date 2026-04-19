#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Define the location of the .env file (change if needed)
ENV_FILE="$SCRIPT_DIR/auth-service/.env"

# Check if the .env file exists
if ! [[ -f "$ENV_FILE" ]]; then
  echo "Error: .env file not found at $ENV_FILE"
  exit 1
fi

# Read each line in the .env file (ignoring comments)
while IFS= read -r line; do
  # Skip blank lines and lines starting with #
  if [[ -n "$line" ]] && [[ "$line" != \#* ]]; then
    # Split the line into key and value
    key=$(echo "$line" | cut -d '=' -f1)
    value=$(echo "$line" | cut -d '=' -f2-)
    # Export the variable
    export "$key=$value"
  fi
done < <(grep -v '^#' "$ENV_FILE")

# Ensure the standalone Postgres and Redis containers are running and fresh.
# reset_all.sh uses `docker`, which typically needs root — inherit the current
# sudo invocation so the user does not need to re-authenticate mid-script.
echo "==> Resetting backing services (Postgres, Redis)..."
"$SCRIPT_DIR/auth-service/scripts/reset_all.sh"

# Run docker-compose commands with exported variables
echo "==> Building and starting auth-service + app-service..."
docker compose build
docker compose up