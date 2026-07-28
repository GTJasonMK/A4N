#!/usr/bin/env bash
#=============================================================================
# 安装 A4N 固定版本的 ONNX Runtime 动态库。
#
# 用法:
#   ./scripts/dev/setup-onnxruntime.sh
#   ./scripts/dev/setup-onnxruntime.sh --force
#
# 默认安装到 ~/.local/share/a4n/onnxruntime/1.24.2，不依赖 Cargo target。
# 可通过 A4N_ONNXRUNTIME_ROOT 覆盖安装目录。
#=============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"

VERSION="1.24.2"
FORCE=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --force)
            FORCE=true
            shift
            ;;
        --help|-h)
            print_script_usage "$0" 10
            exit 0
            ;;
        *)
            die "未知参数: $1；使用 --help 查看用法"
            ;;
    esac
done

case "$(uname -s)-$(uname -m)" in
    Linux-x86_64)
        ARCHIVE_NAME="onnxruntime-linux-x64-${VERSION}.tgz"
        ARCHIVE_SHA256="43725474ba5663642e17684717946693850e2005efbd724ac72da278fead25e6"
        ARCHIVE_FORMAT="tgz"
        SOURCE_DIR_NAME="onnxruntime-linux-x64-${VERSION}"
        RUNTIME_LIBRARY="libonnxruntime.so"
        ;;
    Darwin-arm64)
        ARCHIVE_NAME="onnxruntime-osx-arm64-${VERSION}.tgz"
        ARCHIVE_SHA256="0af4fa503e8ea285245b47ee42d0a7461b8156a81270857da0c1d4ecf858abde"
        ARCHIVE_FORMAT="tgz"
        SOURCE_DIR_NAME="onnxruntime-osx-arm64-${VERSION}"
        RUNTIME_LIBRARY="libonnxruntime.dylib"
        ;;
    MINGW*-x86_64|MSYS*-x86_64|CYGWIN*-x86_64)
        ARCHIVE_NAME="onnxruntime-win-x64-${VERSION}.zip"
        ARCHIVE_SHA256="8e3e9c826375352e29cb2614fe44f3d7a4b0ff7b8028ad7a456af9d949a7e8b0"
        ARCHIVE_FORMAT="zip"
        SOURCE_DIR_NAME="onnxruntime-win-x64-${VERSION}"
        RUNTIME_LIBRARY="onnxruntime.dll"
        ;;
    *)
        die "自动安装目前仅支持 Linux/Windows x86_64 和 macOS arm64；请手动安装 ONNX Runtime ${VERSION} 并设置 A4N_ONNXRUNTIME_ROOT"
        ;;
esac

INSTALL_DIR="$(onnxruntime_root)"
DOWNLOAD_URL="https://github.com/microsoft/onnxruntime/releases/download/v${VERSION}/${ARCHIVE_NAME}"

if configure_onnxruntime && ! $FORCE; then
    log_ok "ONNX Runtime ${VERSION} 已安装: $A4N_ONNXRUNTIME_ROOT"
    exit 0
fi

if [[ -e "$INSTALL_DIR" ]] && ! $FORCE; then
    die "安装目录已存在但内容不完整或版本不匹配: $INSTALL_DIR；确认无需保留后使用 --force 重装"
fi

if [[ "$ARCHIVE_FORMAT" == "zip" ]]; then
    require_command unzip
else
    require_command tar
fi

parent_dir="$(dirname "$INSTALL_DIR")"
mkdir -p "$parent_dir"
work_dir="$(mktemp -d "$parent_dir/.onnxruntime-${VERSION}.XXXXXX")"
archive="$work_dir/$ARCHIVE_NAME"
cleanup() {
    rm -rf "$work_dir"
}
trap cleanup EXIT

log_section "下载 ONNX Runtime ${VERSION}"
log_info "$DOWNLOAD_URL"
if command -v wget >/dev/null 2>&1; then
    wget --progress=bar:force:noscroll -O "$archive" "$DOWNLOAD_URL"
elif command -v curl >/dev/null 2>&1; then
    curl -fL --retry 3 --connect-timeout 15 --max-time 300 -o "$archive" "$DOWNLOAD_URL"
else
    die "未找到下载工具: wget 或 curl"
fi

if command -v sha256sum >/dev/null 2>&1; then
    actual_sha256="$(sha256sum "$archive" | awk '{print $1}')"
elif command -v shasum >/dev/null 2>&1; then
    actual_sha256="$(shasum -a 256 "$archive" | awk '{print $1}')"
else
    die "未找到 SHA-256 工具: sha256sum 或 shasum"
fi
if [[ "$actual_sha256" != "$ARCHIVE_SHA256" ]]; then
    die "ONNX Runtime 安装包校验失败：得到 $actual_sha256，预期 $ARCHIVE_SHA256"
fi
log_ok "SHA-256 校验通过"

extract_dir="$work_dir/extracted"
mkdir -p "$extract_dir"
if [[ "$ARCHIVE_FORMAT" == "zip" ]]; then
    unzip -q "$archive" -d "$extract_dir"
else
    tar -xzf "$archive" -C "$extract_dir"
fi
source_dir="$extract_dir/$SOURCE_DIR_NAME"
[[ -r "$source_dir/VERSION_NUMBER" ]] || die "安装包缺少 VERSION_NUMBER"
[[ -r "$source_dir/lib/$RUNTIME_LIBRARY" ]] || die "安装包缺少 $RUNTIME_LIBRARY"

if [[ -e "$INSTALL_DIR" ]]; then
    rm -rf "$INSTALL_DIR"
fi
mv "$source_dir" "$INSTALL_DIR"

configure_onnxruntime || die "ONNX Runtime 安装完成后校验失败: $INSTALL_DIR"
log_ok "ONNX Runtime ${VERSION} 已安装: $A4N_ONNXRUNTIME_ROOT"
log_info "Cargo 将使用 ORT_LIB_PATH=$ORT_LIB_PATH 动态链接，不再在构建阶段下载运行库"
