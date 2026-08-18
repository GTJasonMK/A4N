#!/usr/bin/env bash
# 下载项目固定版本的官方 Qdrant release 到 tools/qdrant/bin
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
DEST="$PROJECT_ROOT/tools/qdrant"
CONFIG="$DEST/config/config.yaml"
ARCH="$(uname -m)"
OS="$(uname -s)"

[[ -r "$CONFIG" ]] || die "缺少 Qdrant 配置: $CONFIG"

case "${OS}-${ARCH}" in
  Linux-x86_64|Linux-amd64)
    PLATFORM="linux-x86_64"
    ;;
  Linux-aarch64|Linux-arm64)
    PLATFORM="linux-aarch64"
    ;;
  Darwin-x86_64)
    PLATFORM="macos-x86_64"
    ;;
  Darwin-arm64)
    PLATFORM="macos-aarch64"
    ;;
  *)
    echo "不支持的平台: ${OS}-${ARCH}" >&2
    exit 1
    ;;
esac

IFS=$'\t' read -r VERSION ASSET ARCHIVE_SHA256 <<<"$(qdrant_release_metadata "$PLATFORM")"
URL="https://github.com/qdrant/qdrant/releases/download/${VERSION}/${ASSET}"
mkdir -p "$DEST/bin" "$DEST/storage" "$DEST/snapshots"
TMP="$(make_target_temp_dir qdrant-setup)"
trap 'rm -rf "$TMP"' EXIT
ARCHIVE="$TMP/$ASSET"
echo "→ 下载 $URL"
curl -fL --retry 3 --retry-delay 2 -o "$ARCHIVE" "$URL"
actual_sha256="$(sha256_file "$ARCHIVE")"
[[ "$actual_sha256" == "$ARCHIVE_SHA256" ]] \
  || die "Qdrant 安装包校验失败：得到 $actual_sha256，预期 $ARCHIVE_SHA256"
tar -xzf "$ARCHIVE" -C "$TMP"
BIN="$(find "$TMP" -type f -name qdrant | head -1)"
if [[ -z "$BIN" ]]; then
  echo "压缩包中未找到 qdrant 可执行文件" >&2
  exit 1
fi
install -m 755 "$BIN" "$DEST/bin/qdrant"
echo "✓ 已安装: $DEST/bin/qdrant ($VERSION)"
