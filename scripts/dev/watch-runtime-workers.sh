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
  local build_foreshadowing_worker="$9"
  local build_object_profile_worker="${10}"
  local build_scene_profile_worker="${11}"
  local build_portrait_worker="${12}"
  local build_source_snapshot_worker="${13}"
  local build_profile_material_worker="${14}"
  local build_project_status_worker="${15}"
  local build_theme_read_worker="${16}"
  local build_project_read_worker="${17}"
  local build_import_review_read_worker="${18}"
  local build_inspiration_history_worker="${19}"
  local build_project_access_worker="${20}"
  local build_project_collaboration_worker="${21}"
  local build_project_mutation_worker="${22}"
  local build_chapter_scene_worker="${23}"
  local build_chapter_content_worker="${24}"
  local build_workflow_read_worker="${25}"
  local build_config_read_worker="${26}"
  local build_character_mutation_worker="${27}"
  local build_video_record_worker="${28}"
  local build_manga_record_worker="${29}"
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
  [[ "$build_foreshadowing_worker" == true ]] && cargo_command+=(-p a4n-foreshadowing-worker)
  [[ "$build_object_profile_worker" == true ]] && cargo_command+=(-p a4n-object-profile-worker)
  [[ "$build_scene_profile_worker" == true ]] && cargo_command+=(-p a4n-scene-profile-worker)
  [[ "$build_portrait_worker" == true ]] && cargo_command+=(-p a4n-portrait-worker)
  [[ "$build_source_snapshot_worker" == true ]] && cargo_command+=(-p a4n-source-snapshot-worker)
  [[ "$build_profile_material_worker" == true ]] && cargo_command+=(-p a4n-profile-material-worker)
  [[ "$build_project_status_worker" == true ]] && cargo_command+=(-p a4n-project-status-worker)
  [[ "$build_theme_read_worker" == true ]] && cargo_command+=(-p a4n-theme-read-worker)
  [[ "$build_project_read_worker" == true ]] && cargo_command+=(-p a4n-project-read-worker)
  [[ "$build_import_review_read_worker" == true ]] && cargo_command+=(-p a4n-import-review-worker)
  [[ "$build_inspiration_history_worker" == true ]] && cargo_command+=(-p a4n-inspiration-worker)
  [[ "$build_project_access_worker" == true ]] && cargo_command+=(-p a4n-project-access-worker)
  [[ "$build_project_collaboration_worker" == true ]] && cargo_command+=(-p a4n-project-collaboration-worker)
  [[ "$build_project_mutation_worker" == true ]] && cargo_command+=(-p a4n-project-mutation-worker)
  [[ "$build_chapter_scene_worker" == true ]] && cargo_command+=(-p a4n-chapter-scene-worker)
  [[ "$build_chapter_content_worker" == true ]] && cargo_command+=(-p a4n-chapter-content-worker)
  [[ "$build_workflow_read_worker" == true ]] && cargo_command+=(-p a4n-workflow-read-worker)
  [[ "$build_config_read_worker" == true ]] && cargo_command+=(-p a4n-config-read-worker)
  [[ "$build_character_mutation_worker" == true ]] && cargo_command+=(-p a4n-character-mutation-worker)
  [[ "$build_video_record_worker" == true ]] && cargo_command+=(-p a4n-video-record-worker)
  [[ "$build_manga_record_worker" == true ]] && cargo_command+=(-p a4n-manga-record-worker)
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
    "$PROJECT_ROOT/crates/a4n-foreshadowing-worker-contract/"*)
      build_control=true
      build_foreshadowing_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-foreshadowing-worker-client/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-foreshadowing-worker/"*)
      build_foreshadowing_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-object-profile-worker-contract/"*)
      build_control=true
      build_object_profile_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-object-profile-worker-client/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-object-profile-worker/"*)
      build_object_profile_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-scene-profile-worker-contract/"*)
      build_control=true
      build_scene_profile_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-scene-profile-worker-client/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-scene-profile-worker/"*)
      build_scene_profile_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-profile-material-core/"*)
      build_control=true
      build_profile_material_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-profile-material-worker-contract/"*)
      build_control=true
      build_profile_material_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-profile-material-worker-client/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-profile-material-worker/"*)
      build_profile_material_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-project-status-worker-contract/"*)
      build_control=true
      build_project_status_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-project-status-worker-client/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-project-status-worker/"*)
      build_project_status_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-theme-core/"*)
      build_control=true
      build_theme_read_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-theme-read-worker-contract/"*)
      build_control=true
      build_theme_read_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-theme-read-worker-client/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-theme-read-worker/"*)
      build_theme_read_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-project-read-core/"*)
      build_control=true
      build_project_read_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-project-read-worker-contract/"*)
      build_control=true
      build_project_read_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-project-read-worker-client/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-project-read-worker/"*)
      build_project_read_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-import-review-core/"*)
      build_control=true
      build_import_review_read_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-import-review-worker-contract/"*)
      build_control=true
      build_import_review_read_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-import-review-worker-client/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-import-review-worker/"*)
      build_import_review_read_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-inspiration-core/"*)
      build_control=true
      build_inspiration_history_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-inspiration-worker-contract/"*)
      build_control=true
      build_inspiration_history_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-inspiration-worker-client/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-inspiration-worker/"*)
      build_inspiration_history_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-project-access-worker-contract/"*)
      build_control=true
      build_project_access_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-project-access-worker-client/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-project-access-worker/"*)
      build_project_access_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-project-collaboration-worker-contract/"*)
      build_control=true
      build_project_collaboration_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-project-collaboration-worker-client/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-project-collaboration-worker/"*)
      build_project_collaboration_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-project-mutation-worker-contract/"*)
      build_control=true
      build_project_mutation_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-project-mutation-worker-client/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-project-mutation-worker/"*)
      build_project_mutation_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-chapter-scene-worker-contract/"*)
      build_control=true
      build_chapter_scene_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-chapter-scene-worker-client/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-chapter-scene-worker/"*)
      build_chapter_scene_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-chapter-content-worker-contract/"*)
      build_control=true
      build_chapter_content_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-chapter-content-worker-client/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-chapter-content-worker/"*)
      build_chapter_content_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-chapter-content-core/"*)
      build_control=true
      build_chapter_content_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-workflow-read-worker-contract/"*)
      build_control=true
      build_workflow_read_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-workflow-read-worker-client/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-workflow-read-worker/"*)
      build_workflow_read_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-workflow-core/"*)
      build_control=true
      build_workflow_read_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-config-read-worker-contract/"*)
      build_control=true
      build_config_read_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-config-read-worker-client/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-config-read-worker/"*)
      build_config_read_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-config-read-core/"*)
      build_config_read_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-character-mutation-worker-contract/"*)
      build_control=true
      build_character_mutation_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-character-mutation-worker-client/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-character-mutation-worker/"*)
      build_character_mutation_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-video-record-worker-contract/"*)
      build_control=true
      build_video_record_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-video-record-worker-client/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-video-record-worker/"*)
      build_video_record_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-manga-record-worker-contract/"*)
      build_control=true
      build_manga_record_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-manga-record-worker-client/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-manga-record-worker/"*)
      build_manga_record_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-manga-core/"*)
      build_control=true
      build_manga_record_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-portrait-worker-contract/"*)
      build_control=true
      build_portrait_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-portrait-worker-client/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-portrait-worker/"*)
      build_portrait_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-source-snapshot-worker-contract/"*)
      build_control=true
      build_source_snapshot_worker=true
      ;;
    "$PROJECT_ROOT/crates/a4n-source-snapshot-worker-client/"*)
      build_control=true
      ;;
    "$PROJECT_ROOT/crates/a4n-source-snapshot-worker/"*)
      build_source_snapshot_worker=true
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
      build_foreshadowing_worker=true
      build_object_profile_worker=true
      build_scene_profile_worker=true
      build_portrait_worker=true
      build_source_snapshot_worker=true
      build_profile_material_worker=true
      build_project_status_worker=true
      build_theme_read_worker=true
      build_project_read_worker=true
      build_import_review_read_worker=true
      build_inspiration_history_worker=true
      build_project_access_worker=true
      build_project_collaboration_worker=true
      build_project_mutation_worker=true
      build_chapter_scene_worker=true
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
    build_foreshadowing_worker=false
    build_object_profile_worker=false
    build_scene_profile_worker=false
    build_portrait_worker=false
    build_source_snapshot_worker=false
    build_profile_material_worker=false
    build_project_status_worker=false
    build_theme_read_worker=false
    build_project_read_worker=false
    build_import_review_read_worker=false
    build_inspiration_history_worker=false
    build_project_access_worker=false
    build_project_collaboration_worker=false
    build_project_mutation_worker=false
    build_chapter_scene_worker=false
    build_chapter_content_worker=false
    build_workflow_read_worker=false
    build_config_read_worker=false
    build_character_mutation_worker=false
    build_video_record_worker=false
    build_manga_record_worker=false
    mark_changed_targets "$changed_path"

    # 合并同一轮编辑器保存产生的 close_write/create/moved_to 事件。
    changed_paths=("$changed_path")
    while IFS= read -r -t 0.3 pending_path; do
        changed_paths+=("$pending_path")
        mark_changed_targets "$pending_path"
    done || true

    if [[ "$build_control" == false && "$build_agent_worker" == false && "$build_edge" == false && "$build_novel_planning_worker" == false && "$build_artifact_media_worker" == false && "$build_outline_worker" == false && "$build_blueprint_worker" == false && "$build_asset_graph_worker" == false && "$build_foreshadowing_worker" == false && "$build_object_profile_worker" == false && "$build_scene_profile_worker" == false && "$build_portrait_worker" == false && "$build_source_snapshot_worker" == false && "$build_profile_material_worker" == false && "$build_project_status_worker" == false && "$build_theme_read_worker" == false && "$build_project_read_worker" == false && "$build_import_review_read_worker" == false && "$build_inspiration_history_worker" == false && "$build_project_access_worker" == false && "$build_project_collaboration_worker" == false && "$build_project_mutation_worker" == false && "$build_chapter_scene_worker" == false && "$build_chapter_content_worker" == false && "$build_workflow_read_worker" == false && "$build_config_read_worker" == false && "$build_character_mutation_worker" == false && "$build_video_record_worker" == false && "$build_manga_record_worker" == false ]]; then
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
        [[ "$build_foreshadowing_worker" == true ]] && printf ' a4n-foreshadowing-worker'
        [[ "$build_object_profile_worker" == true ]] && printf ' a4n-object-profile-worker'
        [[ "$build_scene_profile_worker" == true ]] && printf ' a4n-scene-profile-worker'
        [[ "$build_portrait_worker" == true ]] && printf ' a4n-portrait-worker'
        [[ "$build_source_snapshot_worker" == true ]] && printf ' a4n-source-snapshot-worker'
        [[ "$build_profile_material_worker" == true ]] && printf ' a4n-profile-material-worker'
        [[ "$build_project_status_worker" == true ]] && printf ' a4n-project-status-worker'
        [[ "$build_theme_read_worker" == true ]] && printf ' a4n-theme-read-worker'
        [[ "$build_project_read_worker" == true ]] && printf ' a4n-project-read-worker'
        [[ "$build_import_review_read_worker" == true ]] && printf ' a4n-import-review-worker'
        [[ "$build_inspiration_history_worker" == true ]] && printf ' a4n-inspiration-worker'
        [[ "$build_project_access_worker" == true ]] && printf ' a4n-project-access-worker'
        [[ "$build_project_collaboration_worker" == true ]] && printf ' a4n-project-collaboration-worker'
        [[ "$build_project_mutation_worker" == true ]] && printf ' a4n-project-mutation-worker'
        [[ "$build_chapter_scene_worker" == true ]] && printf ' a4n-chapter-scene-worker'
        [[ "$build_chapter_content_worker" == true ]] && printf ' a4n-chapter-content-worker'
        [[ "$build_workflow_read_worker" == true ]] && printf ' a4n-workflow-read-worker'
        [[ "$build_config_read_worker" == true ]] && printf ' a4n-config-read-worker'
        [[ "$build_character_mutation_worker" == true ]] && printf ' a4n-character-mutation-worker'
        [[ "$build_video_record_worker" == true ]] && printf ' a4n-video-record-worker'
        [[ "$build_manga_record_worker" == true ]] && printf ' a4n-manga-record-worker'
    )"
    if ! build_children "$build_control" "$build_agent_worker" "$build_edge" "$build_novel_planning_worker" "$build_artifact_media_worker" "$build_outline_worker" "$build_blueprint_worker" "$build_asset_graph_worker" "$build_foreshadowing_worker" "$build_object_profile_worker" "$build_scene_profile_worker" "$build_portrait_worker" "$build_source_snapshot_worker" "$build_profile_material_worker" "$build_project_status_worker" "$build_theme_read_worker" "$build_project_read_worker" "$build_import_review_read_worker" "$build_inspiration_history_worker" "$build_project_access_worker" "$build_project_collaboration_worker" "$build_project_mutation_worker" "$build_chapter_scene_worker" "$build_chapter_content_worker" "$build_workflow_read_worker" "$build_config_read_worker" "$build_character_mutation_worker" "$build_video_record_worker" "$build_manga_record_worker"; then
        log_warn "Control/Agent worker 编译失败；保留当前运行产物，等待下一次源码变更"
    else
        publish_reload_marker
        log_ok "受影响的 Runtime worker 已重新编译"
    fi
done <"$watch_fifo"
