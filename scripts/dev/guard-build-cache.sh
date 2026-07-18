#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
TARGET_DIR="$PROJECT_ROOT/target"
LIMIT_GIB="${A4N_BUILD_CACHE_LIMIT_GIB:-30}"
LOCK_FILE="${XDG_RUNTIME_DIR:-/tmp}/a4n-cache-cleanup.lock"

# The project entry scripts are cross-platform, while safe process inspection
# and the shared cleanup lock currently use Linux facilities.
if [[ "$(uname -s)" != "Linux" ]]; then
    exit 0
fi

if [[ ! "$LIMIT_GIB" =~ ^[1-9][0-9]*$ ]]; then
    echo "无效的 A4N_BUILD_CACHE_LIMIT_GIB：$LIMIT_GIB（必须是正整数）" >&2
    exit 2
fi
LIMIT_BYTES=$((LIMIT_GIB * 1024 * 1024 * 1024))

[[ -d "$TARGET_DIR" ]] || exit 0

exec 9>"$LOCK_FILE"
if ! flock -n 9; then
    echo "A4N 构建缓存检查正在运行，本次跳过。"
    exit 0
fi

SIZE_BYTES="$(du -s -B1 "$TARGET_DIR" 2>/dev/null | awk '{print $1}')"
SIZE_BYTES="${SIZE_BYTES:-0}"
if (( SIZE_BYTES <= LIMIT_BYTES )); then
    exit 0
fi

a4n_build_is_running() {
    local proc cwd cmdline

    for proc in /proc/[0-9]*; do
        [[ -r "$proc/cmdline" ]] || continue
        cmdline="$(tr '\0' ' ' < "$proc/cmdline" 2>/dev/null || true)"
        case "$cmdline" in
            *cargo*|*rustc*) ;;
            *) continue ;;
        esac

        cwd="$(readlink "$proc/cwd" 2>/dev/null || true)"
        if [[ "$cwd" == "$PROJECT_ROOT" || "$cwd" == "$PROJECT_ROOT/"* ||
              "$cmdline" == *"$TARGET_DIR"* ]]; then
            return 0
        fi
    done

    return 1
}

SIZE_HUMAN="$(numfmt --to=iec-i --suffix=B "$SIZE_BYTES")"
if a4n_build_is_running; then
    echo "A4N 构建缓存已达 $SIZE_HUMAN，但存在其他编译进程，本次跳过清理。"
    exit 0
fi

echo "A4N 构建缓存已达 $SIZE_HUMAN，超过 ${LIMIT_GIB}GiB，构建前自动清理。"
cargo clean --manifest-path "$PROJECT_ROOT/Cargo.toml" --target-dir "$TARGET_DIR"
