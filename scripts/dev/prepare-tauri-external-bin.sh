#!/usr/bin/env bash
#=============================================================================
# Prepare Tauri externalBin binaries.
#
# Usage:
#   ./scripts/dev/prepare-tauri-external-bin.sh
#   ./scripts/dev/prepare-tauri-external-bin.sh --target x86_64-unknown-linux-gnu
#   ./scripts/dev/prepare-tauri-external-bin.sh --debug
#   ./scripts/dev/prepare-tauri-external-bin.sh --no-build
#   ./scripts/dev/prepare-tauri-external-bin.sh --dry-run
#
# Tauri appends "-<target-triple>{.exe}" to every bundle.externalBin entry.
# This script builds each bundled binary and copies it to target/tauri-sidecars with
# the exact suffixed filename Tauri expects.
#=============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"

tool_rows="$(release_tool_rows)"
TOOL_SPECS=()
while IFS=$'\t' read -r package binary; do
    TOOL_SPECS+=("$package:$binary")
done <<<"$tool_rows"

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

TARGET_ROOT="$CARGO_TARGET_DIR"
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
    log_section "准备 Tauri externalBin dry-run"
else
    log_section "准备 Tauri externalBin"
fi
log_info "target: $TARGET_TRIPLE"
log_info "profile: $BUILD_MODE"
log_info "output: $EXTERNAL_BIN_DIR"

if ! $DRY_RUN; then
    require_onnxruntime
fi

build_external_binaries() {
    local spec package component_package is_component base_rustflags="${RUSTFLAGS:-}"
    local -a cargo_base=(cargo build)
    [[ "$BUILD_MODE" == "release" ]] && cargo_base+=(--release)
    [[ -n "$TARGET_TRIPLE" ]] && cargo_base+=(--target "$TARGET_TRIPLE")
    local -a component_packages=() all_args=("${cargo_base[@]}") host_args=("${cargo_base[@]}") component_args=("${cargo_base[@]}")
    while IFS= read -r component_package; do
        [[ -n "$component_package" ]] && component_packages+=("$component_package")
    done < <(runtime_component_packages)
    for spec in "${TOOL_SPECS[@]}"; do
        package="${spec%%:*}"
        all_args+=(-p "$package")
        is_component=false
        for component_package in "${component_packages[@]}"; do
            [[ "$package" == "$component_package" ]] && is_component=true
        done
        if $is_component; then
            component_args+=(-p "$package")
        else
            host_args+=(-p "$package")
        fi
    done
    if [[ "$BUILD_MODE" == "release" ]]; then
        component_args+=(--config 'profile.release.lto="off"')
    fi

    if $SKIP_BUILD; then
        $DRY_RUN && log_info "build externalBin: skipped (--no-build)"
        return 0
    fi
    if $DRY_RUN; then
        log_info "build externalBin:"
        if [[ "$BUILD_MODE" == "release" ]]; then
            print_command "${host_args[@]}"
            log_info "Host retains fat LTO; component packages use -C prefer-dynamic with release LTO off"
            print_command "${component_args[@]}"
        else
            print_command "${all_args[@]}"
        fi
    else
        (
            cd "$PROJECT_ROOT"
            if [[ "$BUILD_MODE" != "release" ]]; then
                "${all_args[@]}"
            else
                if [[ -n "$base_rustflags" ]]; then
                    export RUSTFLAGS="$base_rustflags"
                else
                    unset RUSTFLAGS
                fi
                "${host_args[@]}"
                export RUSTFLAGS="$(runtime_component_rustflags "$base_rustflags" "$TARGET_TRIPLE" tauri)"
                "${component_args[@]}"
            fi
        )
    fi
}

prepare_external_binary() {
    local package="$1"
    local binary_name="$2"
    local source_binary target_binary

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
        log_ok "$(basename "$target_binary")"
    fi
}

build_external_binaries
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

copy_shared_rust_runtime() {
    $DRY_RUN && { log_info "copy shared Rust runtime: $EXTERNAL_BIN_DIR"; return 0; }
    [[ "$BUILD_MODE" == "release" ]] || return 0
    local source pattern rust_std_dir db_library_name
    rust_std_dir="$(rustc --target "$TARGET_TRIPLE" --print target-libdir)"
    local -a patterns=()
    case "$TARGET_TRIPLE" in
        *-apple-darwin) db_library_name="liba4n_db.dylib"; patterns=("libstd-"*.dylib) ;;
        *-windows-*) db_library_name="a4n_db.dll"; patterns=("std-"*.dll "libstd-"*.dll) ;;
        *) db_library_name="liba4n_db.so"; patterns=("libstd-"*.so) ;;
    esac
    source="$PROFILE_DIR/deps/$db_library_name"
    [[ -f "$source" ]] || die "缺少共享数据库动态库: $source"
    cp -L "$source" "$EXTERNAL_BIN_DIR/$db_library_name"
    for pattern in "${patterns[@]}"; do
        for source in "$PROFILE_DIR/$pattern"; do
            [[ -f "$source" ]] || continue
            cp -L "$source" "$EXTERNAL_BIN_DIR/$(basename "$source")"
        done
    done
    for pattern in "${patterns[@]}"; do
        for source in "$rust_std_dir/$pattern"; do
            [[ -f "$source" ]] || continue
            cp -L "$source" "$EXTERNAL_BIN_DIR/$(basename "$source")"
        done
    done
}

copy_shared_rust_runtime
