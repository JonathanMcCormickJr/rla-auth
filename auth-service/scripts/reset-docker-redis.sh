#!/usr/bin/env bash
set -euo pipefail

CONTAINER_NAME="redis-db"
REDIS_PORT=6379
APP_PORT=3000
IMAGE="redis:7.0-alpine"

# Kill any old auth-service instance on the app port
if ss -tlnp 2>/dev/null | grep -q ":${APP_PORT} "; then
    echo "Killing old auth-service on port $APP_PORT..."
    for pid in $(ss -tlnp 2>/dev/null | grep ":${APP_PORT} " | grep -oP 'pid=\K[0-9]+' | sort -u); do
        echo "  Killing PID $pid"
        kill "$pid" 2>/dev/null || true
    done
    sleep 1
fi

# Stop and remove ALL containers using the target port, not just our named one
echo "Stopping container '$CONTAINER_NAME'..."
docker stop "$CONTAINER_NAME" 2>/dev/null && echo "  Stopped." || echo "  Not running."

echo "Removing container '$CONTAINER_NAME'..."
docker rm "$CONTAINER_NAME" 2>/dev/null && echo "  Removed." || echo "  Not found."

# Find and kill any other containers bound to our port
echo "Checking for other containers on port $REDIS_PORT..."
for cid in $(docker ps -q --filter "publish=$REDIS_PORT" 2>/dev/null); do
    echo "  Stopping container $cid..."
    docker stop "$cid" 2>/dev/null || true
    docker rm "$cid" 2>/dev/null || true
done

# Kill any docker-proxy or other processes still holding the port
if ss -tlnp 2>/dev/null | grep -q ":${REDIS_PORT} "; then
    echo "Port $REDIS_PORT is still in use. Killing occupying processes..."
    for pid in $(ss -tlnp 2>/dev/null | grep ":${REDIS_PORT} " | grep -oP 'pid=\K[0-9]+' | sort -u); do
        echo "  Killing PID $pid"
        kill "$pid" 2>/dev/null || true
    done
    sleep 2

    # Force-kill if still around
    if ss -tlnp 2>/dev/null | grep -q ":${REDIS_PORT} "; then
        echo "  Processes didn't exit cleanly, force-killing..."
        for pid in $(ss -tlnp 2>/dev/null | grep ":${REDIS_PORT} " | grep -oP 'pid=\K[0-9]+' | sort -u); do
            kill -9 "$pid" 2>/dev/null || true
        done
        sleep 1
    fi

    if ss -tlnp 2>/dev/null | grep -q ":${REDIS_PORT} "; then
        echo "Error: port $REDIS_PORT is still occupied after force-kill."
        echo "  Restarting Docker daemon as last resort..."
        systemctl restart docker 2>/dev/null || service docker restart 2>/dev/null || {
            echo "Error: could not restart Docker. Please run: sudo systemctl restart docker"
            exit 1
        }
        sleep 2
    fi
fi

echo "Starting fresh '$CONTAINER_NAME' container..."
docker run --name "$CONTAINER_NAME" \
    -p "$REDIS_PORT:6379" \
    -d "$IMAGE"

echo "Waiting for Redis to be ready..."
for i in $(seq 1 30); do
    if docker exec "$CONTAINER_NAME" redis-cli ping 2>/dev/null | grep -q "PONG"; then
        echo "Redis is ready."
        exit 0
    fi
    sleep 1
done

echo "Error: Redis did not become ready in time."
exit 1
