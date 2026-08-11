#!/usr/bin/env bash
#=============================================================================
# Prepare Tauri externalBin sidecar binaries.
#
# Usage:
#   ./scripts/dev/prepare-tauri-external-bin.sh
#   ./scripts/dev/prepare-tauri-external-bin.sh --target x86_64-unknown-linux-gnu
#   ./scripts/dev/prepare-tauri-external-bin.sh --debug
#   ./scripts/dev/prepare-tauri-external-bin.sh --no-build
#   ./scripts/dev/prepare-tauri-external-bin.sh --dry-run
#
# Tauri appends "-<target-triple>{.exe}" to every bundle.externalBin entry.
# This script builds each sidecar and copies it to target/tauri-sidecars with
# the exact suffixed filename Tauri expects.
#=============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"

SERVICE_CATALOG="$PROJECT_ROOT/crates/a4n-service-protocol/service-catalog.json"
require_command ruby
sidecar_package_lines="$(ruby -rjson -e 'JSON.parse(File.read(ARGV.fetch(0))).each { |entry| puts entry.fetch("binary") }' "$SERVICE_CATALOG")"
SIDECAR_PACKAGES=()
while IFS= read -r package; do
    [[ -z "$package" ]] || SIDECAR_PACKAGES+=("$package")
done <<<"$sidecar_package_lines"
[[ "${#SIDECAR_PACKAGES[@]}" -gt 0 ]] || die "sidecar service catalog 为空"
TOOL_SPECS=(
    "a4n-cli:a4n"
    "a4n-server:a4n-server"
    "a4n-migration:a4n-migration"
)

BUILD_MODE="release"
TARGET_TRIPLE=""
SKIP_BUILD=false
DRY_RUN=false

print_command() {
    local arg
    printf '  '
    for arg in "$@"; do
        printf '%q ' "$arg"
    done
    printf '\n'
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --target)
            if [[ $# -lt 2 || -z "${2:-}" ]]; then
                die "--target 需要提供 Rust target triple"
            fi
            TARGET_TRIPLE="$2"
            shift 2
            ;;
        --debug)
            BUILD_MODE="debug"
            shift
            ;;
        --release)
            BUILD_MODE="release"
            shift
            ;;
        --no-build)
            SKIP_BUILD=true
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --help|-h)
            print_script_usage "$0" 14
            exit 0
            ;;
        *)
            die "未知参数: $1；使用 --help 查看用法"
            ;;
    esac
done

host_target_triple() {
    rustc -vV | sed -n 's/^host: //p'
}

target_root() {
    local root="${CARGO_TARGET_DIR:-$PROJECT_ROOT/target}"
    case "$root" in
        /*) printf '%s\n' "$root" ;;
        *) printf '%s/%s\n' "$PROJECT_ROOT" "$root" ;;
    esac
}

cargo_profile_dir() {
    local root="$1"
    if [[ -n "$TARGET_TRIPLE" ]]; then
        printf '%s/%s/%s\n' "$root" "$TARGET_TRIPLE" "$BUILD_MODE"
    else
        printf '%s/%s\n' "$root" "$BUILD_MODE"
    fi
}

binary_extension() {
    if [[ "$TARGET_TRIPLE" == *windows* ]]; then
        printf '%s\n' ".exe"
    fi
}

if [[ -z "$TARGET_TRIPLE" ]]; then
    TARGET_TRIPLE="$(host_target_triple)"
fi
[[ -n "$TARGET_TRIPLE" ]] || die "无法确定 Rust target triple"

TARGET_ROOT="$(target_root)"
PROFILE_DIR="$(cargo_profile_dir "$TARGET_ROOT")"
EXTERNAL_BIN_DIR="$TARGET_ROOT/tauri-sidecars"
BINARY_EXT="$(binary_extension)"

case "$TARGET_TRIPLE" in
    *-apple-darwin)
        RUNTIME_NAME="libonnxruntime.dylib"
        ;;
    *-windows-*)
        RUNTIME_NAME="onnxruntime.dll"
        ;;
    *-linux-*)
        RUNTIME_NAME="libonnxruntime.so.1"
        ;;
    *)
        die "不支持的 Tauri bundle target: $TARGET_TRIPLE"
        ;;
esac

if ! $DRY_RUN; then
    mkdir -p "$EXTERNAL_BIN_DIR"
fi

if $DRY_RUN; then
    log_section "准备 Tauri externalBin sidecar dry-run"
else
    log_section "准备 Tauri externalBin sidecar"
fi
log_info "target: $TARGET_TRIPLE"
log_info "profile: $BUILD_MODE"
log_info "output: $EXTERNAL_BIN_DIR"

if ! $DRY_RUN; then
    require_onnxruntime
fi
if ! $SKIP_BUILD && ! $DRY_RUN; then
    guard_build_cache
fi

prepare_external_binary() {
    local package="$1"
    local binary_name="$2"
    local cargo_args source_binary target_binary runtime_dependency

    cargo_args=(cargo build -p "$package")
    if [[ "$BUILD_MODE" == "release" ]]; then
        cargo_args+=(--release)
    fi
    if [[ -n "$TARGET_TRIPLE" ]]; then
        cargo_args+=(--target "$TARGET_TRIPLE")
    fi

    if ! $SKIP_BUILD; then
        if $DRY_RUN; then
            log_info "build $package:"
            print_command "${cargo_args[@]}"
        else
            (
                cd "$PROJECT_ROOT"
                "${cargo_args[@]}"
            )
        fi
    elif $DRY_RUN; then
        log_info "build $package: skipped (--no-build)"
    fi

    source_binary="$PROFILE_DIR/${binary_name}${BINARY_EXT}"
    target_binary="$EXTERNAL_BIN_DIR/${binary_name}-${TARGET_TRIPLE}${BINARY_EXT}"
    if $DRY_RUN; then
        log_info "copy $package:"
        print_command cp "$source_binary" "$target_binary"
        log_ok "$(basename "$target_binary")"
    else
        [[ -f "$source_binary" ]] || die "缺少 externalBin 二进制: $source_binary"
        cp "$source_binary" "$target_binary"
        if [[ "$TARGET_TRIPLE" != *-windows-* ]]; then
            chmod +x "$target_binary"
        fi
        if [[ "$TARGET_TRIPLE" == *-apple-darwin && "$package" == "a4n-embedding-service" ]]; then
            runtime_dependency="$(otool -L "$target_binary" | awk '/libonnxruntime.*dylib/ { print $1; exit }')"
            [[ -n "$runtime_dependency" ]] || die "embedding sidecar 未链接 ONNX Runtime"
            install_name_tool -change "$runtime_dependency" '@rpath/libonnxruntime.dylib' "$target_binary"
            codesign --force --sign - "$target_binary"
        fi
        log_ok "$(basename "$target_binary")"
    fi
}

for package in "${SIDECAR_PACKAGES[@]}"; do
    prepare_external_binary "$package" "$package"
done
for spec in "${TOOL_SPECS[@]}"; do
    prepare_external_binary "${spec%%:*}" "${spec#*:}"
done

runtime_source="$(onnxruntime_root)/lib/$RUNTIME_NAME"
runtime_target="$EXTERNAL_BIN_DIR/$RUNTIME_NAME"
if $DRY_RUN; then
    log_info "copy ONNX Runtime:"
    print_command cp "$runtime_source" "$runtime_target"
else
    [[ -f "$runtime_source" ]] || die "缺少 ONNX Runtime 动态库: $runtime_source"
    rm -f \
        "$EXTERNAL_BIN_DIR/libonnxruntime.dylib" \
        "$EXTERNAL_BIN_DIR/libonnxruntime.so.1" \
        "$EXTERNAL_BIN_DIR/onnxruntime.dll"
    cp "$runtime_source" "$runtime_target"
    log_ok "$RUNTIME_NAME"
fi
