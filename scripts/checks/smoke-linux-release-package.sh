#!/usr/bin/env bash
# Smoke a built Linux portable archive from a non-ASCII install prefix.
# Usage: A4N_QDRANT_URL=... A4N_DATABASE_URL=... A4N_JWT_SECRET=... \
#   ./scripts/checks/smoke-linux-release-package.sh deploy/a4n-tauri-*-linux-*.tar.gz

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"

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

for command_name in curl pgrep rg sort tar wc; do
    require_command "$command_name"
done

catalog_rows="$(sidecar_catalog_rows)"
worker_rows="$(ruby "$PROJECT_ROOT/scripts/release_manifest.rb" worker-rows)"
tool_binaries="$(release_tool_binaries)"
cli_binary="$(release_tool_binary_for_package a4n-cli)"
modules=()
addr_envs=()
url_envs=()
health_paths=()
while IFS=$'\t' read -r module _package addr_env url_env health_path; do
    [[ -n "$module" ]] || continue
    modules+=("$module")
    addr_envs+=("$addr_env")
    url_envs+=("$url_env")
    health_paths+=("$health_path")
done <<<"$catalog_rows"
[[ "${#modules[@]}" -gt 0 ]] || {
    printf 'sidecar service catalog is empty\n' >&2
    exit 2
}

base_port="${A4N_RELEASE_SMOKE_BASE_PORT:-48641}"
max_base_port=$((65535 - ${#modules[@]}))
[[ "$base_port" =~ ^[0-9]+$ && "$base_port" -ge 1024 && "$base_port" -le "$max_base_port" ]] || {
    printf 'invalid A4N_RELEASE_SMOKE_BASE_PORT: %s\n' "$base_port" >&2
    exit 2
}

tmp_root="$(make_target_temp_dir linux-release-smoke)"
install_root="$tmp_root/安装 前缀"
launcher_log="$tmp_root/launcher.log"
launcher_pid=""
local_cli=""
local_config=""
local_server=""

finish() {
    local status=$?
    trap - EXIT
    if [[ -n "$launcher_pid" ]] && kill -0 "$launcher_pid" 2>/dev/null; then
        kill -TERM "$launcher_pid" 2>/dev/null || true
        wait "$launcher_pid" 2>/dev/null || true
    fi
    if [[ -n "$local_cli" && -x "$local_cli" && -n "$local_config" && -n "$local_server" ]]; then
        "$local_cli" --config "$local_config" --server "$local_server" local stop --force >/dev/null 2>&1 || true
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
while IFS= read -r tool; do
    [[ -x "$package_root/bin/$tool" ]] || {
        printf 'release package is missing executable tool: %s\n' "$tool" >&2
        exit 1
    }
done <<<"$tool_binaries"
local_cli="$package_root/bin/$cli_binary"
local_config="$tmp_root/cli.json"
local_server="http://127.0.0.1:$((base_port + ${#modules[@]}))"
capabilities="$(cd "$tmp_root" && "$local_cli" --config "$local_config" --agent capabilities export)"
ruby -rjson -e '
  data = JSON.parse(STDIN.read).fetch("data")
  commands = data.fetch("commands")
  abort "capability manifest is inconsistent" unless data.fetch("total_commands") == data.fetch("matched_commands") && data.fetch("total_commands") == commands.length
' <<<"$capabilities"

urls=()
for index in "${!modules[@]}"; do
    port=$((base_port + index))
    url="http://127.0.0.1:$port"
    printf -v "${addr_envs[index]}" '%s' "127.0.0.1:$port"
    printf -v "${url_envs[index]}" '%s' "$url"
    export "${addr_envs[index]}" "${url_envs[index]}"
    urls+=("$url")
    if curl -fsS --max-time 1 "$url${health_paths[index]}" >/dev/null 2>&1; then
        printf 'release smoke port is already in use: %s\n' "$port" >&2
        exit 1
    fi
done

"$package_root/start.sh" >"$launcher_log" 2>&1 &
launcher_pid=$!

for _ in {1..300}; do
    all_healthy=true
    for index in "${!modules[@]}"; do
        payload="$(curl -fsS --max-time 1 "${urls[index]}${health_paths[index]}" 2>/dev/null || true)"
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
    payload="$(curl -fsS --max-time 2 "${urls[index]}${health_paths[index]}")"
    rg -q '"service"[[:space:]]*:[[:space:]]*"'"${modules[index]}"'"' <<<"$payload"
    rg -q '"status"[[:space:]]*:[[:space:]]*"healthy"' <<<"$payload"
done
rg -q 'A4N 应用启动完成' "$launcher_log"

find "$package_root/data/logs/services" -type f -size +0c -print -quit | rg -q .

declare -A sidecar_pid_by_service=()
for index in "${!modules[@]}"; do
    mapfile -t process_ids < <(pgrep -P "$launcher_pid" -f -- "a4n-component-host --service ${modules[index]}" || true)
    [[ "${#process_ids[@]}" -eq 1 ]] || {
        printf 'expected one independent %s process, found %s\n' "${modules[index]}" "${#process_ids[@]}" >&2
        exit 1
    }
    sidecar_pid_by_service["${modules[index]}"]="${process_ids[0]}"
done
if [[ "$(printf '%s\n' "${sidecar_pid_by_service[@]}" | sort -u | wc -l)" -ne "${#modules[@]}" ]]; then
    printf 'multiple service roles share one component-host process\n' >&2
    exit 1
fi
declare -A sidecar_pid_by_worker=()
while IFS=$'\t' read -r worker_name _launcher; do
    [[ -n "$worker_name" ]] || continue
    mapfile -t process_ids < <(pgrep -P "$launcher_pid" -f -- "a4n-component-host --worker ${worker_name}" || true)
    [[ "${#process_ids[@]}" -eq 1 ]] || {
        printf 'expected one independent %s process, found %s\n' "$worker_name" "${#process_ids[@]}" >&2
        exit 1
    }
    sidecar_pid_by_worker["$worker_name"]="${process_ids[0]}"
done <<<"$worker_rows"
if [[ "$(printf '%s\n' "${sidecar_pid_by_worker[@]}" | sort -u | wc -l)" -ne "${#sidecar_pid_by_worker[@]}" ]]; then
    printf 'multiple worker roles share one component-host process\n' >&2
    exit 1
fi

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

for index in "${!urls[@]}"; do
    if curl -fsS --max-time 1 "${urls[index]}${health_paths[index]}" >/dev/null 2>&1; then
        printf 'sidecar port survived host shutdown: %s\n' "${urls[index]}" >&2
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

printf 'linux release package smoke passed: host + CLI tools + local Server + %s sidecars, logs, cleanup, non-ASCII path\n' "${#modules[@]}"
