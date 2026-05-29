#!/usr/bin/env bash
# R-0001 AC7 — Docker image builds and serves /health → 200.
#
# Steps:
#   1. docker build -t fitai-api:qa-<rand> backend
#   2. docker run -d -p 0:8080 ... (ephemeral host port)
#   3. resolve mapped host port via `docker port`
#   4. poll http://127.0.0.1:$PORT/health (15 retries x 1s)
#   5. assert final status == 200
#   6. cleanup (container + image) via trap EXIT — runs on every exit path
#
# Pre-implementation red state: `docker build` fails — no backend/Dockerfile.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TAG="fitai-api:qa-$RANDOM-$$"
CONTAINER_ID=""

cleanup() {
    local exit_code=$?
    if [[ -n "$CONTAINER_ID" ]]; then
        echo "[r-0001-ac7] cleanup: stop+rm container $CONTAINER_ID"
        docker rm -f "$CONTAINER_ID" >/dev/null 2>&1 || true
    fi
    echo "[r-0001-ac7] cleanup: rmi $TAG"
    docker rmi -f "$TAG" >/dev/null 2>&1 || true
    exit "$exit_code"
}
trap cleanup EXIT INT TERM

echo "[r-0001-ac7] docker build -t $TAG $REPO_ROOT/backend"
docker build -t "$TAG" "$REPO_ROOT/backend"

echo "[r-0001-ac7] docker run -d -p 0:8080 $TAG"
CONTAINER_ID="$(docker run -d -p 0:8080 "$TAG")"
echo "[r-0001-ac7] container id: $CONTAINER_ID"

# Resolve the host-side port that Docker mapped to container port 8080.
# `docker port` output looks like "0.0.0.0:49162" or "[::]:49162"; take the
# last colon-delimited field.
HOST_PORT=""
for _ in 1 2 3 4 5; do
    HOST_PORT="$(docker port "$CONTAINER_ID" 8080/tcp 2>/dev/null | head -n1 | awk -F: '{print $NF}' || true)"
    if [[ -n "$HOST_PORT" ]]; then
        break
    fi
    sleep 1
done

if [[ -z "$HOST_PORT" ]]; then
    echo "[r-0001-ac7] FAIL: could not resolve mapped host port"
    exit 1
fi
echo "[r-0001-ac7] mapped host port: $HOST_PORT"

# Poll /health. 15 attempts with 1 s between (~15 s budget) absorbs cold-start
# jitter on CI runners without a fixed sleep.
STATUS=""
for attempt in $(seq 1 15); do
    STATUS="$(curl -s -o /dev/null -w "%{http_code}" "http://127.0.0.1:$HOST_PORT/health" || true)"
    if [[ "$STATUS" == "200" ]]; then
        echo "[r-0001-ac7] /health → 200 on attempt $attempt"
        echo "[r-0001-ac7] PASS"
        exit 0
    fi
    echo "[r-0001-ac7] attempt $attempt: status=$STATUS (retrying in 1s)"
    sleep 1
done

echo "[r-0001-ac7] FAIL: /health never returned 200 (last status: $STATUS)"
echo "[r-0001-ac7] last 50 lines of container logs:"
docker logs --tail 50 "$CONTAINER_ID" || true
exit 1
