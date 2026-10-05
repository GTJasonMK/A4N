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

COMPONENT_PACKAGE_DIRS=()
COMPONENT_PACKAGES=()
component_package_lines="$(ruby -rjson -e '
  workers = JSON.parse(File.read(ARGV.fetch(0))).map { |entry| entry.fetch("binary") }
  services = JSON.parse(File.read(ARGV.fetch(1))).map { |entry| entry.fetch("binary") }
  puts (workers + services).uniq
' "$PROJECT_ROOT/crates/process/a4n-process-catalog/worker-catalog.json" \
  "$PROJECT_ROOT/crates/process/a4n-process-catalog/service-catalog.json")"
while IFS= read -r package; do
    [[ -n "$package" ]] || continue
    COMPONENT_PACKAGES+=("$package")
    COMPONENT_PACKAGE_DIRS+=("$(cargo_package_dir "$package")")
done <<<"$component_package_lines"

path_is_relevant() {
    case "$1" in
        *.rs|*/Cargo.toml|*/build.rs|*/Cargo.lock) return 0 ;;
        *) return 1 ;;
    esac
}

mark_changed_targets() {
    local changed_path="$1"
    path_is_relevant "$changed_path" || return 0

    case "$changed_path" in
        "$PROJECT_ROOT/Cargo.toml"|"$PROJECT_ROOT/Cargo.lock")
            build_control=true
            build_edge=true
            build_component_host=true
            ;;
        "$PROJECT_ROOT/crates/host/a4n-control/"*)
            build_control=true
            ;;
        "$PROJECT_ROOT/crates/host/a4n-edge/"*)
            build_edge=true
            ;;
        "$PROJECT_ROOT/crates/host/a4n-server/"*|"$PROJECT_ROOT/crates/host/a4n-cli/"*|"$PROJECT_ROOT/crates/host/a4n-tauri/"*)
            ;;
        "$PROJECT_ROOT/crates/"*)
            local index component_dir
            for index in "${!COMPONENT_PACKAGE_DIRS[@]}"; do
                component_dir="${COMPONENT_PACKAGE_DIRS[$index]}"
                if [[ "$changed_path" == "$component_dir/"* ]]; then
                    component_targets+=("${COMPONENT_PACKAGES[$index]}")
                    return 0
                fi
            done
            case "$changed_path" in
                *-worker-client/*)
                    build_control=true
                    ;;
                *)
                    build_control=true
                    build_component_host=true
                    ;;
            esac
            ;;
    esac
}

build_targets() {
    local build_control="$1"
    local build_edge="$2"
    local build_component_host="$3"
    local package
    local -a cargo_command=("$PROJECT_ROOT/scripts/cargo.sh" build)
    $RELEASE && cargo_command+=(--release)
    [[ "$build_control" == true ]] && cargo_command+=(-p a4n-control)
    [[ "$build_edge" == true ]] && cargo_command+=(-p a4n-edge)
    if [[ "$build_component_host" == true ]]; then
        for package in "${COMPONENT_PACKAGES[@]}"; do
            cargo_command+=(-p "$package")
        done
    else
        for package in "${component_targets[@]}"; do
            cargo_command+=(-p "$package")
        done
    fi
    if $FULL_HOST && [[ "$build_control" == true ]]; then
        cargo_command+=(--features a4n-control/embedded-capabilities)
    fi
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

watch_paths=("$PROJECT_ROOT/crates" "$PROJECT_ROOT/Cargo.toml" "$PROJECT_ROOT/Cargo.lock")
log_info "监听 Rust/Cargo 变更，按需重编译 Control、Edge 或独立组件"

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
    build_control=false
    build_edge=false
    build_component_host=false
    component_targets=()
    mark_changed_targets "$changed_path"

    changed_paths=("$changed_path")
    while IFS= read -r -t 0.3 pending_path; do
        changed_paths+=("$pending_path")
        mark_changed_targets "$pending_path"
    done || true

    if [[ "$build_control" == false && "$build_edge" == false && "$build_component_host" == false && ${#component_targets[@]} -eq 0 ]]; then
        continue
    fi
    targets=()
    [[ "$build_control" == true ]] && targets+=(a4n-control)
    [[ "$build_edge" == true ]] && targets+=(a4n-edge)
    if [[ "$build_component_host" == true ]]; then
        targets+=("${COMPONENT_PACKAGES[@]}")
    else
        targets+=("${component_targets[@]}")
    fi
    log_info "检测到源码变更: ${changed_paths[*]}；构建目标: ${targets[*]}"
    if build_targets "$build_control" "$build_edge" "$build_component_host"; then
        publish_reload_marker
        log_ok "受影响的 Runtime 进程已重新编译"
    else
        log_warn "Runtime 进程编译失败；保留当前运行产物，等待下一次源码变更"
    fi
done <"$watch_fifo"
