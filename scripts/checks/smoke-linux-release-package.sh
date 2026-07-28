#!/usr/bin/env bash
# Smoke a built Linux portable archive from a non-ASCII install prefix.
# Usage: A4N_QDRANT_URL=... A4N_DATABASE_URL=... A4N_JWT_SECRET=... \
#   ./scripts/checks/smoke-linux-release-package.sh deploy/a4n-tauri-*-linux-*.tar.gz

set -euo pipefail

archive="${1:-}"
[[ -n "$archive" && -f "$archive" ]] || {
    printf 'usage: %s <linux release archive>\n' "$0" >&2
    exit 2
}

for name in A4N_QDRANT_URL A4N_JWT_SECRET; do
    [[ -n "${!name:-}" ]] || {
        printf '%s is required for release smoke\n' "$name" >&2
        exit 2
    }
done
export A4N_DATABASE_URL="${A4N_DATABASE_URL:-${A4N_TEST_DATABASE_URL:-}}"
[[ -n "$A4N_DATABASE_URL" ]] || {
    printf 'A4N_DATABASE_URL or A4N_TEST_DATABASE_URL is required for release smoke\n' >&2
    exit 2
}

for command_name in curl rg tar; do
    command -v "$command_name" >/dev/null 2>&1 || {
        printf 'missing command: %s\n' "$command_name" >&2
        exit 2
    }
done

base_port="${A4N_RELEASE_SMOKE_BASE_PORT:-48641}"
[[ "$base_port" =~ ^[0-9]+$ && "$base_port" -ge 1024 && "$base_port" -le 65528 ]] || {
    printf 'invalid A4N_RELEASE_SMOKE_BASE_PORT: %s\n' "$base_port" >&2
    exit 2
}

tmp_root="$(mktemp -d "${TMPDIR:-/tmp}/a4n-release-smoke.XXXXXX")"
install_root="$tmp_root/安装 前缀"
launcher_log="$tmp_root/launcher.log"
launcher_pid=""

finish() {
    local status=$?
    trap - EXIT
    if [[ -n "$launcher_pid" ]] && kill -0 "$launcher_pid" 2>/dev/null; then
        kill -TERM "$launcher_pid" 2>/dev/null || true
        wait "$launcher_pid" 2>/dev/null || true
    fi
    if [[ -n "${A4N_RELEASE_SMOKE_ARTIFACT_DIR:-}" ]]; then
        mkdir -p "$A4N_RELEASE_SMOKE_ARTIFACT_DIR"
        [[ ! -f "$launcher_log" ]] || cp "$launcher_log" "$A4N_RELEASE_SMOKE_ARTIFACT_DIR/"
        [[ ! -d "${package_root:-}/data/logs" ]] || cp -R "${package_root}/data/logs" "$A4N_RELEASE_SMOKE_ARTIFACT_DIR/"
    fi
    if [[ "$status" -ne 0 && -f "$launcher_log" ]]; then
        tail -n 200 "$launcher_log" >&2
    fi
    if [[ "${A4N_RELEASE_SMOKE_KEEP_TMP:-0}" == 1 ]]; then
        printf 'release smoke files kept at %s\n' "$tmp_root" >&2
    else
        rm -rf "$tmp_root"
    fi
    exit "$status"
}
trap finish EXIT

while IFS= read -r entry; do
    case "/$entry/" in
        *"/../"*|"//"*)
            printf 'release archive contains unsafe path: %s\n' "$entry" >&2
            exit 1
            ;;
    esac
done < <(tar -tzf "$archive")

mkdir -p "$install_root"
tar -xzf "$archive" -C "$install_root"
mapfile -t package_roots < <(find "$install_root" -mindepth 1 -maxdepth 1 -type d)
[[ "${#package_roots[@]}" -eq 1 ]] || {
    printf 'release archive must contain exactly one package root\n' >&2
    exit 1
}
package_root="${package_roots[0]}"
[[ -x "$package_root/start.sh" ]] || {
    printf 'release package is missing executable start.sh\n' >&2
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
        printf 'release smoke port is already in use: %s\n' "$port" >&2
        exit 1
    fi
done

mkdir -p "$package_root/data/services"
printf '999999\n' >"$package_root/data/services/prompt.pid"

"$package_root/start.sh" >"$launcher_log" 2>&1 &
launcher_pid=$!

for _ in {1..300}; do
    all_healthy=true
    for index in "${!modules[@]}"; do
        payload="$(curl -fsS --max-time 1 "${urls[index]}/healthz" 2>/dev/null || true)"
        if ! rg -q '"service"[[:space:]]*:[[:space:]]*"'"${modules[index]}"'"' <<<"$payload" ||
            ! rg -q '"status"[[:space:]]*:[[:space:]]*"healthy"' <<<"$payload"; then
            all_healthy=false
            break
        fi
    done
    if $all_healthy && rg -q 'A4N 应用启动完成' "$launcher_log"; then
        break
    fi
    kill -0 "$launcher_pid" 2>/dev/null || {
        printf 'release launcher exited before becoming healthy\n' >&2
        exit 1
    }
    sleep 0.1
done

for index in "${!modules[@]}"; do
    payload="$(curl -fsS --max-time 2 "${urls[index]}/healthz")"
    rg -q '"service"[[:space:]]*:[[:space:]]*"'"${modules[index]}"'"' <<<"$payload"
    rg -q '"status"[[:space:]]*:[[:space:]]*"healthy"' <<<"$payload"
done
rg -q 'A4N 应用启动完成' "$launcher_log"

pid_files=("$package_root"/data/services/*.pid)
[[ "${#pid_files[@]}" -eq 8 ]] || {
    printf 'expected 8 sidecar PID files, found %s\n' "${#pid_files[@]}" >&2
    exit 1
}
sidecar_pids=()
for pid_file in "${pid_files[@]}"; do
    pid="$(<"$pid_file")"
    [[ "$pid" =~ ^[0-9]+$ && "$pid" != 999999 ]] || {
        printf 'invalid or stale sidecar PID: %s\n' "$pid" >&2
        exit 1
    }
    kill -0 "$pid"
    sidecar_pids+=("$pid")
done
find "$package_root/data/logs/services" -type f -size +0c -print -quit | rg -q .

kill -TERM "$launcher_pid"
set +e
wait "$launcher_pid"
launcher_status=$?
set -e
launcher_pid=""
[[ "$launcher_status" -eq 0 || "$launcher_status" -eq 143 ]] || {
    printf 'release launcher exited with unexpected status: %s\n' "$launcher_status" >&2
    exit 1
}

for pid in "${sidecar_pids[@]}"; do
    if kill -0 "$pid" 2>/dev/null; then
        printf 'sidecar process survived host shutdown: %s\n' "$pid" >&2
        exit 1
    fi
done
if compgen -G "$package_root/data/services/*.pid" >/dev/null; then
    printf 'sidecar PID files survived host shutdown\n' >&2
    exit 1
fi
for url in "${urls[@]}"; do
    if curl -fsS --max-time 1 "$url/healthz" >/dev/null 2>&1; then
        printf 'sidecar port survived host shutdown: %s\n' "$url" >&2
        exit 1
    fi
done

printf 'linux release package smoke passed: host + 8 sidecars, logs, stale PID, cleanup, non-ASCII path\n'
