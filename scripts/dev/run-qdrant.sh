#!/usr/bin/env bash
# 启动项目内 tools/qdrant 的官方 release 二进制
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEST="$ROOT/tools/qdrant"
BIN="$DEST/bin/qdrant"
if [[ ! -x "$BIN" ]]; then
  echo "未找到 $BIN，先执行: ./scripts/dev/setup-qdrant.sh" >&2
  exit 1
fi
cd "$DEST"
mkdir -p storage snapshots
export QDRANT__STORAGE__STORAGE_PATH="${QDRANT__STORAGE__STORAGE_PATH:-$DEST/storage}"
export QDRANT__STORAGE__SNAPSHOTS_PATH="${QDRANT__STORAGE__SNAPSHOTS_PATH:-$DEST/snapshots}"
# 默认本机
export QDRANT__SERVICE__HOST="${QDRANT__SERVICE__HOST:-127.0.0.1}"
export QDRANT__SERVICE__HTTP_PORT="${QDRANT__SERVICE__HTTP_PORT:-6333}"
export QDRANT__SERVICE__GRPC_PORT="${QDRANT__SERVICE__GRPC_PORT:-6334}"
echo "→ Qdrant HTTP :${QDRANT__SERVICE__HTTP_PORT}  gRPC :${QDRANT__SERVICE__GRPC_PORT}"
echo "  storage: $QDRANT__STORAGE__STORAGE_PATH"
echo "  A4N_QDRANT_URL 建议: http://127.0.0.1:${QDRANT__SERVICE__GRPC_PORT}"
if [[ -f "$DEST/config/config.yaml" ]]; then
  exec "$BIN" --config-path "$DEST/config/config.yaml"
else
  exec "$BIN"
fi
