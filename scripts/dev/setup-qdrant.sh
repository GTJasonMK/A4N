#!/usr/bin/env bash
# 下载官方 Qdrant release 到 tools/qdrant/bin（默认 v1.18.3 linux x86_64 gnu）
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION="${A4N_QDRANT_VERSION:-v1.18.3}"
DEST="$ROOT/tools/qdrant"
ARCH="$(uname -m)"
OS="$(uname -s)"

case "${OS}-${ARCH}" in
  Linux-x86_64|Linux-amd64)
    ASSET="qdrant-x86_64-unknown-linux-gnu.tar.gz"
    ;;
  Linux-aarch64|Linux-arm64)
    ASSET="qdrant-aarch64-unknown-linux-musl.tar.gz"
    ;;
  Darwin-x86_64)
    ASSET="qdrant-x86_64-apple-darwin.tar.gz"
    ;;
  Darwin-arm64)
    ASSET="qdrant-aarch64-apple-darwin.tar.gz"
    ;;
  *)
    echo "不支持的平台: ${OS}-${ARCH}" >&2
    exit 1
    ;;
esac

URL="https://github.com/qdrant/qdrant/releases/download/${VERSION}/${ASSET}"
mkdir -p "$DEST/bin" "$DEST/storage" "$DEST/config" "$DEST/snapshots"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
echo "→ 下载 $URL"
curl -fL --retry 3 --retry-delay 2 -o "$TMP/qdrant.tgz" "$URL"
tar -xzf "$TMP/qdrant.tgz" -C "$TMP"
BIN="$(find "$TMP" -type f -name qdrant | head -1)"
if [[ -z "$BIN" ]]; then
  echo "压缩包中未找到 qdrant 可执行文件" >&2
  exit 1
fi
install -m 755 "$BIN" "$DEST/bin/qdrant"
echo "$VERSION" > "$DEST/VERSION"
echo "$ASSET" > "$DEST/TARGET"
if [[ ! -f "$DEST/config/config.yaml" ]]; then
  cat > "$DEST/config/config.yaml" << 'YAML'
log_level: INFO
storage:
  storage_path: ./storage
  snapshots_path: ./snapshots
  on_disk_payload: true
service:
  host: 127.0.0.1
  http_port: 6333
  grpc_port: 6334
  enable_cors: true
cluster:
  enabled: false
telemetry_disabled: true
YAML
fi
echo "✓ 已安装: $DEST/bin/qdrant ($VERSION)"
"$DEST/bin/qdrant" --help >/dev/null 2>&1 || true
ls -lh "$DEST/bin/qdrant"
