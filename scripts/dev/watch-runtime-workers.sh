#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"

RELEASE=false
FULL_HOST=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --release) RELEASE=true ;;
        --full) FULL_HOST=true ;;
        *) die "未知 worker watcher 参数: $1" ;;
    esac
    shift
done

require_command inotifywait
require_command cargo

export A4N_DEV_ENTRY_BINARY=a4n-server
export A4N_DEV_BUILD_INCLUDE_CLI=1
export A4N_DEV_BUILD_FULL_HOST=0
$FULL_HOST && export A4N_DEV_BUILD_FULL_HOST=1
export A4N_DEV_BUILD_MANAGED_SIDECARS="${A4N_DEV_BUILD_MANAGED_SIDECARS:-1}"
if ! $RELEASE; then
    export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-1}"
fi

path_is_relevant() {
    case "$1" in
        *.rs|*/Cargo.toml|*/build.rs|*/Cargo.lock) return 0 ;;
        *) return 1 ;;
    esac
}

path_requires_runtime_build() {
    local changed_path="$1"
    path_is_relevant "$changed_path" || return 1

    case "$changed_path" in
        "$PROJECT_ROOT/crates/host/a4n-server/"*|"$PROJECT_ROOT/crates/host/a4n-cli/"*|"$PROJECT_ROOT/crates/host/a4n-tauri/"*)
            return 1
            ;;
        "$PROJECT_ROOT/Cargo.toml"|"$PROJECT_ROOT/Cargo.lock"|"$PROJECT_ROOT/crates/"*)
            return 0
            ;;
    esac
    return 1
}

build_targets() {
    # Keep startup's feature graph; Cargo decides which targets actually need recompilation.
    local -a cargo_command=("$SCRIPT_DIR/run-cargo-targets.sh")
    $RELEASE && cargo_command+=(--release)
    A4N_DEV_BUILD_ONLY=1 "${cargo_command[@]}"
}

publish_reload_marker() {
    local marker="${A4N_RUNTIME_WORKER_RELOAD_MARKER:-}"
    [[ -n "$marker" ]] || return 0

    mkdir -p "$(dirname "$marker")"
    local temporary_marker
    temporary_marker="$(mktemp "${marker}.tmp.XXXXXX")"
    printf '%s\n' "$(date +%s%N)" >"$temporary_marker"
    mv -f "$temporary_marker" "$marker"
}

watch_paths=("$PROJECT_ROOT/crates" "$PROJECT_ROOT/Cargo.toml" "$PROJECT_ROOT/Cargo.lock")
log_info "监听 Rust/Cargo 变更，复用启动构建图检查受影响产物"

watch_tmp_dir="$(make_target_temp_dir runtime-worker-watcher)"
watch_fifo="$watch_tmp_dir/events"
mkfifo "$watch_fifo"
inotify_pid=""

cleanup_watcher() {
    local status=$?
    trap - EXIT INT TERM
    if [[ -n "$inotify_pid" ]]; then
        kill "$inotify_pid" 2>/dev/null || true
        wait "$inotify_pid" 2>/dev/null || true
    fi
    rm -f "$watch_fifo"
    rmdir "$watch_tmp_dir" 2>/dev/null || true
    exit "$status"
}

trap cleanup_watcher EXIT INT TERM
inotifywait --monitor --recursive --quiet \
    --event close_write,moved_to,create,delete \
    --format '%w%f' "${watch_paths[@]}" >"$watch_fifo" &
inotify_pid=$!

while IFS= read -r changed_path; do
    build_runtime=false
    path_requires_runtime_build "$changed_path" && build_runtime=true

    changed_paths=("$changed_path")
    while IFS= read -r -t 0.3 pending_path; do
        changed_paths+=("$pending_path")
        path_requires_runtime_build "$pending_path" && build_runtime=true
    done || true

    $build_runtime || continue
    log_info "检测到源码变更: ${changed_paths[*]}；检查统一开发构建图"
    if build_targets; then
        publish_reload_marker
        log_ok "受影响的 Runtime 进程已重新编译"
    else
        log_warn "Runtime 进程编译失败；保留当前运行产物，等待下一次源码变更"
    fi
done <"$watch_fifo"
