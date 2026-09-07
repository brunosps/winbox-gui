#!/usr/bin/env bash
# Seeds an isolated XDG home with one busybox-backed "test-profile" so the
# E2E suite can exercise launch flows against a lightweight container
# instead of dockurr/windows. Run before `npm run test:e2e`.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$HERE/xdg"
PROFILE="test-profile"

rm -rf "$ROOT"
chmod +x "$HERE/mock-docker.sh" "$HERE"/shims/* 2>/dev/null || true

CFG="$ROOT/config/winbox/profiles/$PROFILE"
DATA="$ROOT/data/winbox/profiles/$PROFILE"
mkdir -p "$CFG" "$DATA/storage" "$ROOT/cache" "$ROOT/logs"

cat > "$CFG/config.env" <<EOF
IMAGE_FAMILY=linux_distro
VERSION=
BOOT=alpine
ISO_PATH=
RAM_SIZE=1G
CPU_CORES=1
DISK_SIZE=2G
USERNAME=test
PASSWORD=test
LANGUAGE=
REGION=
KEYBOARD=
TZ=UTC
WEB_PORT=18006
RDP_PORT=13389
SSH_PORT=12222
CONTAINER_NAME=winbox-$PROFILE
STORAGE_DIR=$DATA/storage
SHARED_DIR=$DATA/storage
OEM_DIR=$CFG/oem
MEM_LIMIT=2G
CPU_LIMIT=1
BUNDLES=
GPU_BDF=
EXTRA_PORTS=
EOF
chmod 600 "$CFG/config.env"

# Compose template that boots busybox so we don't need a real qemu image.
# The E2E suite only validates the lifecycle wrappers, not the guest OS.
cat > "$CFG/compose.yml" <<'EOF'
services:
  winbox:
    image: busybox:latest
    container_name: ${CONTAINER_NAME}
    command: ["sh", "-c", "echo 'windows started successfully' && sleep infinity"]
    ports:
      - "127.0.0.1:${WEB_PORT}:8006"
      - "127.0.0.1:${RDP_PORT}:3389/tcp"
    restart: unless-stopped
EOF

echo "seeded $PROFILE under $ROOT"
