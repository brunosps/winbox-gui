#!/usr/bin/env bash
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
LOG_DIR="${WINBOX_E2E_SHIM_LOG_DIR:-$HERE/xdg/logs}"
mkdir -p "$LOG_DIR"
printf 'docker %s\n' "$*" >> "$LOG_DIR/shims.log"

if [ "${WINBOX_E2E_DOCKER_FAIL:-0}" = "1" ]; then
  echo "mock docker forced failure" >&2
  exit 42
fi

cmd="${1:-}"
case "$cmd" in
  info)
    echo "Client: Docker Engine - Community"
    echo "Server: Docker Engine - Community"
    exit 0
    ;;
  compose)
    sub="${2:-}"
    if [ "$sub" = "version" ]; then
      echo "Docker Compose version v2.29.0"
      exit 0
    fi
    echo "mock docker compose ${*:2}"
    exit 0
    ;;
  ps)
    echo "${WINBOX_E2E_DOCKER_STATE:-exited}"
    exit 0
    ;;
  network)
    if [ "${2:-}" = "inspect" ]; then
      cat <<'JSON'
[
  {
    "Name": "winbox-office_default",
    "IPAM": {
      "Config": [
        { "Subnet": "172.31.0.0/16", "Gateway": "172.31.0.1" }
      ]
    }
  }
]
JSON
      exit 0
    fi
    ;;
  logs)
    echo "windows started successfully"
    exit 0
    ;;
  pull)
    image="${2:-mock-image}"
    echo "latest: Pulling from $image"
    echo "e2e123: Pulling fs layer"
    echo "e2e123: Pull complete"
    echo "Digest: sha256:e2e"
    exit 0
    ;;
  pause|unpause|stop|kill|rm)
    exit 0
    ;;
esac

echo "mock docker: unsupported args: $*" >&2
exit 0
