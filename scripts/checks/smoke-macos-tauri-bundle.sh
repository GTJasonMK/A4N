#!/usr/bin/env bash
# Smoke an unsigned macOS Tauri .app with its bundled sidecars and ONNX Runtime.
# Usage: A4N_QDRANT_URL=... A4N_DATABASE_URL=... A4N_JWT_SECRET=... \
#   ./scripts/checks/smoke-macos-tauri-bundle.sh target/.../bundle/macos/A4N.app

set -euo pipefail

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
for command_name in curl ditto grep pgrep; do
    command -v "$command_name" >/dev/null 2>&1 || {
        printf 'missing command: %s\n' "$command_name" >&2
        exit 2
    }
done

base_port="${A4N_RELEASE_SMOKE_BASE_PORT:-48661}"
[[ "$base_port" =~ ^[0-9]+$ && "$base_port" -ge 1024 && "$base_port" -le 65528 ]] || {
    printf 'invalid A4N_RELEASE_SMOKE_BASE_PORT: %s\n' "$base_port" >&2
    exit 2
}

tmp_root="$(mktemp -d "${TMPDIR:-/tmp}/a4n-macos-smoke.XXXXXX")"
install_root="$tmp_root/安装 前缀"
installed_app="$install_root/A4N.app"
launcher_log="$tmp_root/launcher.log"
host_pid=""

finish() {
    local status=$?
    trap - EXIT
    if [[ -n "$host_pid" ]] && kill -0 "$host_pid" 2>/dev/null; then
        kill -TERM "$host_pid" 2>/dev/null || true
        wait "$host_pid" 2>/dev/null || true
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
host="$macos_dir/A4N"
packages=(
    a4n-prompt-service
    a4n-llm-gateway
    a4n-embedding-service
    a4n-rag-service
    a4n-image-service
    a4n-video-service
    a4n-import-service
    a4n-export-service
)
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
[[ -f "$resources_dir/libonnxruntime.dylib" ]] || {
    printf 'Tauri bundle is missing libonnxruntime.dylib\n' >&2
    exit 1
}

modules=(prompt llm embedding rag image video import export)
addr_envs=(
    A4N_PROMPT_SERVICE_ADDR
    A4N_LLM_GATEWAY_ADDR
    A4N_EMBEDDING_SERVICE_ADDR
    A4N_RAG_SERVICE_ADDR
    A4N_IMAGE_SERVICE_ADDR
    A4N_VIDEO_SERVICE_ADDR
    A4N_IMPORT_SERVICE_ADDR
    A4N_EXPORT_SERVICE_ADDR
)
url_envs=(
    A4N_PROMPT_SERVICE_URL
    A4N_LLM_GATEWAY_URL
    A4N_EMBEDDING_SERVICE_URL
    A4N_RAG_SERVICE_URL
    A4N_IMAGE_SERVICE_URL
    A4N_VIDEO_SERVICE_URL
    A4N_IMPORT_SERVICE_URL
    A4N_EXPORT_SERVICE_URL
)
urls=()
for index in "${!modules[@]}"; do
    port=$((base_port + index))
    url="http://127.0.0.1:$port"
    printf -v "${addr_envs[index]}" '%s' "127.0.0.1:$port"
    printf -v "${url_envs[index]}" '%s' "$url"
    export "${addr_envs[index]}" "${url_envs[index]}"
    urls+=("$url")
    if curl -fsS --max-time 1 "$url/healthz" >/dev/null 2>&1; then
        printf 'macOS smoke port is already in use: %s\n' "$port" >&2
        exit 1
    fi
done

export A4N_PORTABLE_HOME="$tmp_root/用户 数据"
export A4N_DATA_DIR="$tmp_root/data"
export A4N_CACHE_DIR="$tmp_root/cache"
export A4N_TEMP_DIR="$tmp_root/tmp"
export A4N_LOG_DIR="$tmp_root/logs"
export A4N_MANAGED_SIDECARS=1
unset A4N_ONNXRUNTIME_ROOT ORT_DYLIB_PATH ORT_LIB_PATH DYLD_LIBRARY_PATH

"$host" >"$launcher_log" 2>&1 &
host_pid=$!

for _ in {1..300}; do
    all_healthy=true
    for index in "${!modules[@]}"; do
        payload="$(curl -fsS --max-time 1 "${urls[index]}/healthz" 2>/dev/null || true)"
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
    payload="$(curl -fsS --max-time 2 "${urls[index]}/healthz")"
    grep -Eq '"service"[[:space:]]*:[[:space:]]*"'"${modules[index]}"'"' <<<"$payload"
    grep -Eq '"status"[[:space:]]*:[[:space:]]*"healthy"' <<<"$payload"
    matches=()
    while IFS= read -r match; do
        [[ -z "$match" ]] || matches+=("$match")
    done < <(pgrep -P "$host_pid" -f "${packages[index]}" || true)
    [[ "${#matches[@]}" -eq 1 ]] || {
        printf 'expected one managed %s process, found %s\n' "${packages[index]}" "${#matches[@]}" >&2
        exit 1
    }
    sidecar_pids+=("${matches[0]}")
done
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
    for url in "${urls[@]}"; do
        curl -fsS --max-time 1 "$url/healthz" >/dev/null 2>&1 && all_stopped=false
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
for url in "${urls[@]}"; do
    if curl -fsS --max-time 1 "$url/healthz" >/dev/null 2>&1; then
        printf 'sidecar port survived Tauri Host shutdown: %s\n' "$url" >&2
        exit 1
    fi
done

printf 'macOS Tauri smoke passed: .app Host + 8 sidecars, bundled ONNX Runtime, logs, cleanup, non-ASCII path\n'
