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

SIDECAR_PACKAGES=(
    "a4n-prompt-service"
    "a4n-llm-gateway"
    "a4n-embedding-service"
    "a4n-rag-service"
    "a4n-image-service"
    "a4n-video-service"
    "a4n-import-service"
    "a4n-export-service"
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

if ! $SKIP_BUILD && ! $DRY_RUN; then
    require_onnxruntime
    guard_build_cache
fi

for package in "${SIDECAR_PACKAGES[@]}"; do
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

    source_binary="$PROFILE_DIR/${package}${BINARY_EXT}"
    target_binary="$EXTERNAL_BIN_DIR/${package}-${TARGET_TRIPLE}${BINARY_EXT}"
    if $DRY_RUN; then
        log_info "copy $package:"
        print_command cp "$source_binary" "$target_binary"
        log_ok "$(basename "$target_binary")"
    else
        [[ -f "$source_binary" ]] || die "缺少 sidecar 二进制: $source_binary"
        cp "$source_binary" "$target_binary"
        chmod +x "$target_binary" 2>/dev/null || true
        log_ok "$(basename "$target_binary")"
    fi
done
