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

build_children() {
  local -a cargo_command=("$PROJECT_ROOT/scripts/cargo.sh" build)
  $RELEASE && cargo_command+=(--release)
  cargo_command+=(-p a4n-control -p a4n-agent-worker)
  $FULL_HOST && cargo_command+=(--features embedded-capabilities)
  CARGO_ARTIFACTS_DISABLED=1 CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-1}" \
    "${cargo_command[@]}"
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

changed_path_is_relevant() {
  case "$1" in
    "$PROJECT_ROOT/crates/a4n-server/"*|"$PROJECT_ROOT/crates/a4n-tauri/"*) return 1 ;;
    *.rs|*/Cargo.toml|*/build.rs|*/Cargo.lock) return 0 ;;
    *) return 1 ;;
  esac
}

watch_paths=("$PROJECT_ROOT/crates" "$PROJECT_ROOT/Cargo.toml" "$PROJECT_ROOT/Cargo.lock")
log_info "监听 Rust/Cargo 变更，按需重编译 Control 与 Agent worker"

watch_tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/a4n-runtime-workers.XXXXXX")"
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
    changed_path_is_relevant "$changed_path" || continue

    # 合并同一轮编辑器保存产生的 close_write/create/moved_to 事件。
    while IFS= read -r -t 0.3 pending_path; do
        changed_path="$pending_path"
    done || true

    log_info "检测到源码变更: ${changed_path#"$PROJECT_ROOT/"}"
    if ! build_children; then
        log_warn "Control/Agent worker 编译失败；保留当前运行产物，等待下一次源码变更"
    else
        publish_reload_marker
        log_ok "Control/Agent worker 已重新编译"
    fi
done <"$watch_fifo"
