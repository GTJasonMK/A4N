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

path_is_relevant() {
  case "$1" in
    *.rs|*/Cargo.toml|*/build.rs|*/Cargo.lock) return 0 ;;
    *) return 1 ;;
  esac
}

build_children() {
  local build_control="$1"
  local build_agent_worker="$2"
  local build_edge="$3"
  local build_novel_planning_worker="$4"
  local build_artifact_media_worker="$5"
  local build_outline_worker="$6"
  local build_blueprint_worker="$7"
  local build_asset_graph_worker="$8"
  local -a cargo_command=("$PROJECT_ROOT/scripts/cargo.sh" build)
  $RELEASE && cargo_command+=(--release)
  [[ "$build_control" == true ]] && cargo_command+=(-p a4n-control)
  [[ "$build_agent_worker" == true ]] && cargo_command+=(-p a4n-agent-worker)
  [[ "$build_edge" == true ]] && cargo_command+=(-p a4n-edge)
  [[ "$build_novel_planning_worker" == true ]] && cargo_command+=(-p a4n-novel-planning-worker)
  [[ "$build_artifact_media_worker" == true ]] && cargo_command+=(-p a4n-artifact-media-worker)
  [[ "$build_outline_worker" == true ]] && cargo_command+=(-p a4n-outline-worker)
  [[ "$build_blueprint_worker" == true ]] && cargo_command+=(-p a4n-blueprint-worker)
  [[ "$build_asset_graph_worker" == true ]] && cargo_command+=(-p a4n-asset-graph-worker)
  $FULL_HOST && {
    [[ "$build_control" == true || "$build_agent_worker" == true ]] \
      && cargo_command+=(--features embedded-capabilities)
  }
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

mark_changed_targets() {
  local changed_path="$1"
  path_is_relevant "$changed_path" || return 0
  case "$changed_path" in
    "$PROJECT_ROOT/crates/a4n-control/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-runtime/"*|"$PROJECT_ROOT/crates/a4n-orchestrator/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-agent-worker/"*)
      build_agent_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-agent-runtime/"*)
      build_agent_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-edge/"*)
      build_edge=true
      ;;
    "$PROJECT_ROOT/crates/a4n-novel-planning-worker-contract/"*)
      build_control=true
      build_novel_planning_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-novel-planning-worker-client/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-novel-planning-worker/"*)
      build_novel_planning_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-artifact-media-contract/"*)
      build_control=true
      build_artifact_media_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-artifact-media-client/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-artifact-media-worker/"*)
      build_artifact_media_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-outline-worker-contract/"*)
      build_control=true
      build_outline_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-outline-worker-client/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-outline-worker/"*)
      build_outline_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-blueprint-worker-contract/"*)
      build_control=true
      build_blueprint_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-blueprint-worker-client/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-blueprint-worker/"*)
      build_blueprint_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-asset-graph-worker-contract/"*)
      build_control=true
      build_asset_graph_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-asset-graph-worker-client/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-asset-graph-worker/"*)
      build_asset_graph_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-server/"*|"$PROJECT_ROOT/crates/a4n-tauri/"*)
      ;;
    *.rs|*/Cargo.toml|*/build.rs|*/Cargo.lock)
      build_control=true
      build_agent_worker=true
      build_edge=true
      build_novel_planning_worker=true
      build_artifact_media_worker=true
      build_outline_worker=true
      build_blueprint_worker=true
      build_asset_graph_worker=true
      ;;
  esac
}

watch_paths=("$PROJECT_ROOT/crates" "$PROJECT_ROOT/Cargo.toml" "$PROJECT_ROOT/Cargo.lock")
log_info "监听 Rust/Cargo 变更，按需重编译受影响的 Runtime worker"

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
    build_control=false
    build_agent_worker=false
    build_edge=false
    build_novel_planning_worker=false
    build_artifact_media_worker=false
    build_outline_worker=false
    build_blueprint_worker=false
    build_asset_graph_worker=false
    mark_changed_targets "$changed_path"

    # 合并同一轮编辑器保存产生的 close_write/create/moved_to 事件。
    changed_paths=("$changed_path")
    while IFS= read -r -t 0.3 pending_path; do
        changed_paths+=("$pending_path")
        mark_changed_targets "$pending_path"
    done || true

    if [[ "$build_control" == false && "$build_agent_worker" == false && "$build_edge" == false && "$build_novel_planning_worker" == false && "$build_artifact_media_worker" == false && "$build_outline_worker" == false && "$build_blueprint_worker" == false && "$build_asset_graph_worker" == false ]]; then
        continue
    fi
    log_info "检测到源码变更: ${changed_paths[*]}；构建目标:$(
        [[ "$build_control" == true ]] && printf ' a4n-control'
        [[ "$build_agent_worker" == true ]] && printf ' a4n-agent-worker'
        [[ "$build_edge" == true ]] && printf ' a4n-edge'
        [[ "$build_novel_planning_worker" == true ]] && printf ' a4n-novel-planning-worker'
        [[ "$build_artifact_media_worker" == true ]] && printf ' a4n-artifact-media-worker'
        [[ "$build_outline_worker" == true ]] && printf ' a4n-outline-worker'
        [[ "$build_blueprint_worker" == true ]] && printf ' a4n-blueprint-worker'
        [[ "$build_asset_graph_worker" == true ]] && printf ' a4n-asset-graph-worker'
    )"
    if ! build_children "$build_control" "$build_agent_worker" "$build_edge" "$build_novel_planning_worker" "$build_artifact_media_worker" "$build_outline_worker" "$build_blueprint_worker" "$build_asset_graph_worker"; then
        log_warn "Control/Agent worker 编译失败；保留当前运行产物，等待下一次源码变更"
    else
        publish_reload_marker
        log_ok "受影响的 Runtime worker 已重新编译"
    fi
done <"$watch_fifo"
