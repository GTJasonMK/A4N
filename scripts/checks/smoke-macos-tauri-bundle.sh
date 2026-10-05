#!/usr/bin/env bash
# Smoke an unsigned macOS Tauri .app with its bundled sidecars and ONNX Runtime.
# Usage: A4N_QDRANT_URL=... A4N_DATABASE_URL=... A4N_JWT_SECRET=... \
#   ./scripts/checks/smoke-macos-tauri-bundle.sh target/.../bundle/macos/A4N.app

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"

source_app="${1:-}"
[[ -n "$source_app" && -d "$source_app" ]] || {
    printf 'usage: %s <A4N.app>\n' "$0" >&2
    exit 2
}

for name in A4N_QDRANT_URL A4N_DATABASE_URL A4N_JWT_SECRET; do
    [[ -n "${!name:-}" ]] || {
        printf '%s is required for macOS Tauri smoke\n' "$name" >&2
        exit 2
    }
done
for command_name in curl ditto grep pgrep sort wc; do
    require_command "$command_name"
done

catalog_rows="$(sidecar_catalog_rows)"
worker_rows="$(ruby "$PROJECT_ROOT/scripts/release_manifest.rb" worker-rows)"
tool_binaries="$(release_tool_binaries)"
cli_binary="$(release_tool_binary_for_package a4n-cli)"
packages=()
modules=()
addr_envs=()
url_envs=()
health_paths=()
while IFS=$'\t' read -r module package addr_env url_env health_path; do
    [[ -n "$module" ]] || continue
    modules+=("$module")
    packages+=("$package")
    addr_envs+=("$addr_env")
    url_envs+=("$url_env")
    health_paths+=("$health_path")
done <<<"$catalog_rows"
[[ "${#packages[@]}" -gt 0 ]] || {
    printf 'sidecar service catalog is empty\n' >&2
    exit 2
}

base_port="${A4N_RELEASE_SMOKE_BASE_PORT:-48661}"
max_base_port=$((65535 - ${#packages[@]}))
[[ "$base_port" =~ ^[0-9]+$ && "$base_port" -ge 1024 && "$base_port" -le "$max_base_port" ]] || {
    printf 'invalid A4N_RELEASE_SMOKE_BASE_PORT: %s\n' "$base_port" >&2
    exit 2
}

tmp_root="$(make_target_temp_dir macos-release-smoke)"
tmp_root="$(cd "$tmp_root" && pwd -P)"
install_root="$tmp_root/安装 前缀"
installed_app="$install_root/A4N.app"
launcher_log="$tmp_root/launcher.log"
host_pid=""
local_cli=""
local_config=""
local_server=""
missing_sidecar_path=""
missing_sidecar_backup=""
missing_sidecar_moved=0

restore_missing_sidecar() {
    if [[ "$missing_sidecar_moved" -eq 1 ]]; then
        mv "$missing_sidecar_backup" "$missing_sidecar_path"
        missing_sidecar_moved=0
    fi
}

finish() {
    local status=$?
    trap - EXIT
    restore_missing_sidecar
    if [[ -n "$host_pid" ]] && kill -0 "$host_pid" 2>/dev/null; then
        kill -TERM "$host_pid" 2>/dev/null || true
        wait "$host_pid" 2>/dev/null || true
    fi
    if [[ -n "$local_cli" && -x "$local_cli" && -n "$local_config" && -n "$local_server" ]]; then
        "$local_cli" --config "$local_config" --server "$local_server" local stop --force >/dev/null 2>&1 || true
    fi
    if [[ -n "${A4N_RELEASE_SMOKE_ARTIFACT_DIR:-}" ]]; then
        mkdir -p "$A4N_RELEASE_SMOKE_ARTIFACT_DIR"
        [[ ! -f "$launcher_log" ]] || cp "$launcher_log" "$A4N_RELEASE_SMOKE_ARTIFACT_DIR/"
        [[ ! -d "$tmp_root/logs" ]] || cp -R "$tmp_root/logs" "$A4N_RELEASE_SMOKE_ARTIFACT_DIR/"
    fi
    if [[ "$status" -ne 0 && -f "$launcher_log" ]]; then
        tail -n 200 "$launcher_log" >&2
    fi
    if [[ "${A4N_RELEASE_SMOKE_KEEP_TMP:-0}" == 1 ]]; then
        printf 'macOS smoke files kept at %s\n' "$tmp_root" >&2
    else
        rm -rf "$tmp_root"
    fi
    exit "$status"
}
trap finish EXIT

mkdir -p "$install_root"
ditto "$source_app" "$installed_app"

macos_dir="$installed_app/Contents/MacOS"
resources_dir="$installed_app/Contents/Resources"
host="$macos_dir/a4n-tauri"
local_cli="$macos_dir/$cli_binary"
local_config="$tmp_root/cli.json"
local_server="http://127.0.0.1:$((base_port + ${#packages[@]}))"
for package in "${packages[@]}"; do
    [[ -x "$macos_dir/$package" ]] || {
        printf 'Tauri bundle is missing executable sidecar: %s\n' "$package" >&2
        exit 1
    }
done
[[ -x "$host" ]] || {
    printf 'Tauri bundle is missing executable Host: %s\n' "$host" >&2
    exit 1
}
while IFS= read -r tool; do
    [[ -x "$macos_dir/$tool" ]] || {
        printf 'Tauri bundle is missing executable tool: %s\n' "$tool" >&2
        exit 1
    }
done <<<"$tool_binaries"
capabilities="$(cd "$tmp_root" && "$local_cli" --config "$local_config" --agent capabilities export)"
ruby -rjson -e '
  data = JSON.parse(STDIN.read).fetch("data")
  commands = data.fetch("commands")
  abort "capability manifest is inconsistent" unless data.fetch("total_commands") == data.fetch("matched_commands") && data.fetch("total_commands") == commands.length
' <<<"$capabilities"
[[ -f "$resources_dir/libonnxruntime.dylib" ]] || {
    printf 'Tauri bundle is missing libonnxruntime.dylib\n' >&2
    exit 1
}
[[ -f "$resources_dir/prompts/_registry.yaml" ]] || {
    printf 'Tauri bundle is missing prompts/_registry.yaml\n' >&2
    exit 1
}

urls=()
for index in "${!modules[@]}"; do
    port=$((base_port + index))
    url="http://127.0.0.1:$port"
    printf -v "${addr_envs[index]}" '%s' "127.0.0.1:$port"
    printf -v "${url_envs[index]}" '%s' "$url"
    export "${addr_envs[index]}" "${url_envs[index]}"
    urls+=("$url")
    if curl -fsS --max-time 1 "$url${health_paths[index]}" >/dev/null 2>&1; then
        printf 'macOS smoke port is already in use: %s\n' "$port" >&2
        exit 1
    fi
done

export A4N_DATA_DIR="$tmp_root/data"
export A4N_CACHE_DIR="$tmp_root/cache"
export A4N_TEMP_DIR="$tmp_root/tmp"
export A4N_LOG_DIR="$tmp_root/logs"
export A4N_MANAGED_SIDECARS=1
unset A4N_ONNXRUNTIME_ROOT ORT_DYLIB_PATH ORT_LIB_PATH DYLD_LIBRARY_PATH

missing_sidecar_path="$macos_dir/${packages[0]}"
missing_sidecar_backup="$missing_sidecar_path.missing"
[[ ! -e "$missing_sidecar_backup" ]] || {
    printf 'Tauri bundle contains unexpected sidecar test path: %s\n' "$missing_sidecar_backup" >&2
    exit 1
}
mv "$missing_sidecar_path" "$missing_sidecar_backup"
missing_sidecar_moved=1
set +e
"$host" >/dev/null 2>&1
missing_host_status=$?
set -e
restore_missing_sidecar
[[ "$missing_host_status" -ne 0 ]] || {
    printf 'Tauri bundle accepted a missing declared sidecar: %s\n' "${packages[0]}" >&2
    exit 1
}

"$host" >"$launcher_log" 2>&1 &
host_pid=$!

for _ in {1..300}; do
    all_healthy=true
    for index in "${!modules[@]}"; do
        payload="$(curl -fsS --max-time 1 "${urls[index]}${health_paths[index]}" 2>/dev/null || true)"
        if ! grep -Eq '"service"[[:space:]]*:[[:space:]]*"'"${modules[index]}"'"' <<<"$payload" ||
            ! grep -Eq '"status"[[:space:]]*:[[:space:]]*"healthy"' <<<"$payload"; then
            all_healthy=false
            break
        fi
    done
    if $all_healthy && grep -q 'A4N 应用启动完成' "$launcher_log"; then
        break
    fi
    kill -0 "$host_pid" 2>/dev/null || {
        printf 'Tauri Host exited before becoming healthy\n' >&2
        exit 1
    }
    sleep 0.1
done

sidecar_pids=()
for index in "${!modules[@]}"; do
    payload="$(curl -fsS --max-time 2 "${urls[index]}${health_paths[index]}")"
    grep -Eq '"service"[[:space:]]*:[[:space:]]*"'"${modules[index]}"'"' <<<"$payload"
    grep -Eq '"status"[[:space:]]*:[[:space:]]*"healthy"' <<<"$payload"
    matches=()
    while IFS= read -r match; do
        [[ -z "$match" ]] || matches+=("$match")
    done < <(pgrep -P "$host_pid" -f "a4n-component-host --service ${modules[index]}" || true)
    [[ "${#matches[@]}" -eq 1 ]] || {
        printf 'expected one independent %s process, found %s\n' "${modules[index]}" "${#matches[@]}" >&2
        exit 1
    }
    sidecar_pids+=("${matches[0]}")
done
if [[ "$(printf '%s\n' "${sidecar_pids[@]}" | sort -u | wc -l)" -ne "${#modules[@]}" ]]; then
    printf 'multiple service roles share one component-host process\n' >&2
    exit 1
fi
worker_pids=()
while IFS=$'\t' read -r worker_name _launcher; do
    [[ -n "$worker_name" ]] || continue
    matches=()
    while IFS= read -r match; do
        [[ -z "$match" ]] || matches+=("$match")
    done < <(pgrep -P "$host_pid" -f "a4n-component-host --worker ${worker_name}" || true)
    [[ "${#matches[@]}" -eq 1 ]] || {
        printf 'expected one independent %s process, found %s\n' "$worker_name" "${#matches[@]}" >&2
        exit 1
    }
    worker_pids+=("${matches[0]}")
done <<<"$worker_rows"
if [[ "$(printf '%s\n' "${worker_pids[@]}" | sort -u | wc -l)" -ne "${#worker_pids[@]}" ]]; then
    printf 'multiple worker roles share one component-host process\n' >&2
    exit 1
fi
grep -q 'A4N 应用启动完成' "$launcher_log"
find "$A4N_LOG_DIR/services" -type f -size +0c -print -quit | grep -q .

kill -TERM "$host_pid"
set +e
wait "$host_pid"
host_status=$?
set -e
host_pid=""
[[ "$host_status" -eq 0 || "$host_status" -eq 143 ]] || {
    printf 'Tauri Host exited with unexpected status: %s\n' "$host_status" >&2
    exit 1
}

for _ in {1..50}; do
    all_stopped=true
    for pid in "${sidecar_pids[@]}"; do
        kill -0 "$pid" 2>/dev/null && all_stopped=false
    done
    for index in "${!urls[@]}"; do
        curl -fsS --max-time 1 "${urls[index]}${health_paths[index]}" >/dev/null 2>&1 && all_stopped=false
    done
    $all_stopped && break
    sleep 0.1
done
for pid in "${sidecar_pids[@]}"; do
    if kill -0 "$pid" 2>/dev/null; then
        printf 'managed sidecar survived Tauri Host shutdown: %s\n' "$pid" >&2
        exit 1
    fi
done
for index in "${!urls[@]}"; do
    if curl -fsS --max-time 1 "${urls[index]}${health_paths[index]}" >/dev/null 2>&1; then
        printf 'sidecar port survived Tauri Host shutdown: %s\n' "${urls[index]}" >&2
        exit 1
    fi
done

(
    cd "$tmp_root"
    "$local_cli" --config "$local_config" --server "$local_server" --agent local start --no-qdrant
    "$local_cli" --config "$local_config" --server "$local_server" --agent health
    "$local_cli" --config "$local_config" --server "$local_server" --agent capabilities check
    "$local_cli" --config "$local_config" --server "$local_server" --agent local stop
) >>"$launcher_log" 2>&1
for index in "${!urls[@]}"; do
    if curl -fsS --max-time 1 "${urls[index]}${health_paths[index]}" >/dev/null 2>&1; then
        printf 'local managed sidecar survived local stop: %s\n' "${urls[index]}" >&2
        exit 1
    fi
done
if curl -sS --max-time 1 "$local_server/api/health" >/dev/null 2>&1; then
    printf 'local Server port survived local stop: %s\n' "$local_server" >&2
    exit 1
fi

printf 'macOS Tauri smoke passed: .app Host + CLI tools + local Server + %s sidecars, bundled ONNX Runtime, logs, cleanup, non-ASCII path\n' "${#packages[@]}"
