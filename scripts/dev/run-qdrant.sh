#!/usr/bin/env bash
# 启动项目内 tools/qdrant 的官方 release 二进制
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
DEST="$PROJECT_ROOT/tools/qdrant"
BIN="$DEST/bin/qdrant"
CONFIG="$DEST/config/config.yaml"
if [[ ! -x "$BIN" ]]; then
  echo "未找到 $BIN，先执行: ./scripts/dev/setup-qdrant.sh" >&2
  exit 1
fi
if [[ ! -r "$CONFIG" ]]; then
  echo "缺少 Qdrant 配置: $CONFIG" >&2
  exit 1
fi
cd "$DEST"
mkdir -p storage snapshots
echo "→ Qdrant config: $CONFIG"
exec "$BIN" --config-path "$CONFIG"
