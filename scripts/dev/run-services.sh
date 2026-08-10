#!/usr/bin/env bash
#=============================================================================
# A4N Sidecar 服务启动脚本
#
# 用法:
#   ./scripts/dev/run-services.sh prompt,llm,rag
#   ./scripts/dev/run-services.sh llm
#   ./scripts/dev/run-services.sh import,export
#   ./scripts/dev/run-services.sh demo
#   ./scripts/dev/run-services.sh --list
#   ./scripts/dev/run-services.sh --status
#   ./scripts/dev/run-services.sh --status prompt,llm
#   ./scripts/dev/run-services.sh --no-wait prompt,llm
#   ./scripts/dev/run-services.sh --wait-timeout 60 prompt,llm
#
# 说明:
#   该脚本只启动已经存在的 sidecar package。目标 package 尚未创建时会明确失败。
#   默认等待每个服务的 /healthz 就绪；如需只拉起进程可使用 --no-wait。
#=============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"

load_dotenv

APP_PORTABLE_HOME="$(resolve_project_path "${A4N_PORTABLE_HOME:-$PROJECT_ROOT}")"
APP_DATA_DIR="$(resolve_project_path "${A4N_DATA_DIR:-$APP_PORTABLE_HOME/data}")"
APP_CACHE_DIR="$(resolve_project_path "${A4N_CACHE_DIR:-$APP_DATA_DIR/cache}")"
APP_LOG_DIR="$(resolve_project_path "${A4N_LOG_DIR:-$APP_DATA_DIR/logs}")"
APP_TMP_DIR="$(resolve_project_path "${A4N_TEMP_DIR:-$APP_DATA_DIR/tmp}")"
SERVICE_DIR="$APP_DATA_DIR/services"
SERVICE_LOG_DIR="$APP_LOG_DIR/services"
SERVICE_CATALOG_FILE="$PROJECT_ROOT/crates/a4n-service-protocol/service-catalog.json"
WAIT_FOR_HEALTH=true
HEALTH_TIMEOUT_SECONDS="${A4N_SERVICE_HEALTH_TIMEOUT_SECONDS:-30}"
STATUS_ONLY=false
BUILD_CACHE_GUARDED=false
SERVICE_MODULES=()
QDRANT_ENDPOINT_HOST=""
QDRANT_ENDPOINT_PORT=""

catalog_modules() {
    sed -n 's/.*"name":"\([^"]*\)".*/\1/p' "$SERVICE_CATALOG_FILE"
}

catalog_entry_line() {
    local module="$1"
    [[ "$module" =~ ^[a-z][a-z0-9_-]*$ ]] || return 1
    sed -n "/\"name\":\"${module}\"/p" "$SERVICE_CATALOG_FILE" | sed -n '1p'
}

catalog_field() {
    local module="$1"
    local field="$2"
    local line value
    case "$field" in
        addr_env|binary|capability_version|default_addr) ;;
        *) return 1 ;;
    esac
    line="$(catalog_entry_line "$module")" || return 1
    [[ -n "$line" ]] || return 1
    value="$(printf '%s\n' "$line" | sed -n "s/.*\"${field}\":\"\([^\"]*\)\".*/\1/p")"
    [[ -n "$value" ]] || return 1
    printf '%s\n' "$value"
}

[[ -r "$SERVICE_CATALOG_FILE" ]] || die "缺少 sidecar 服务目录: $SERVICE_CATALOG_FILE"
mapfile -t SERVICE_MODULES < <(catalog_modules)
[[ "${#SERVICE_MODULES[@]}" -gt 0 ]] || die "sidecar 服务目录为空: $SERVICE_CATALOG_FILE"

resolve_qdrant_endpoint() {
    local url="${A4N_QDRANT_URL:-}"
    local scheme authority
    case "$url" in
        http://*) scheme="http" ;;
        https://*) scheme="https" ;;
        *) return 1 ;;
    esac

    authority="${url#*://}"
    authority="${authority%%/*}"
    case "$authority" in
        \[*\]:*)
            QDRANT_ENDPOINT_HOST="${authority%%]*}"
            QDRANT_ENDPOINT_HOST="${QDRANT_ENDPOINT_HOST#[}"
            QDRANT_ENDPOINT_PORT="${authority##*:}"
            ;;
        *:*)
            QDRANT_ENDPOINT_HOST="${authority%:*}"
            QDRANT_ENDPOINT_PORT="${authority##*:}"
            ;;
        *)
            QDRANT_ENDPOINT_HOST="$authority"
            [[ "$scheme" == "https" ]] && QDRANT_ENDPOINT_PORT=443 || QDRANT_ENDPOINT_PORT=80
            ;;
    esac
    [[ -n "$QDRANT_ENDPOINT_HOST" && "$QDRANT_ENDPOINT_PORT" =~ ^[0-9]+$ ]]
    [[ "$QDRANT_ENDPOINT_PORT" -ge 1 && "$QDRANT_ENDPOINT_PORT" -le 65535 ]]
}

tcp_reachable() {
    local host="$1"
    local port="$2"
    if command -v nc >/dev/null 2>&1; then
        nc -z -w 1 "$host" "$port" >/dev/null 2>&1
    elif command -v timeout >/dev/null 2>&1; then
        timeout 1 bash -c '</dev/tcp/$1/$2' _ "$host" "$port" >/dev/null 2>&1
    else
        bash -c '</dev/tcp/$1/$2' _ "$host" "$port" >/dev/null 2>&1
    fi
}

qdrant_reachable() {
    resolve_qdrant_endpoint && tcp_reachable "$QDRANT_ENDPOINT_HOST" "$QDRANT_ENDPOINT_PORT"
}

wait_for_qdrant() {
    local pid="$1"
    local log_file="$2"
    local attempt
    log_info "等待 Qdrant: ${QDRANT_ENDPOINT_HOST}:${QDRANT_ENDPOINT_PORT}"
    for ((attempt = 1; attempt <= HEALTH_TIMEOUT_SECONDS; attempt++)); do
        if qdrant_reachable; then
            log_ok "Qdrant 已就绪"
            return 0
        fi
        if [[ -n "$pid" ]] && ! kill -0 "$pid" 2>/dev/null; then
            die "Qdrant 在就绪前已退出；请查看日志: $log_file"
        fi
        sleep 1
    done
    die "Qdrant 未在 ${HEALTH_TIMEOUT_SECONDS}s 内就绪: ${QDRANT_ENDPOINT_HOST}:${QDRANT_ENDPOINT_PORT}"
}

ensure_qdrant() {
    resolve_qdrant_endpoint || die "A4N_QDRANT_URL 必须是有效的 http(s) URL"
    if qdrant_reachable; then
        log_ok "Qdrant 已在运行: ${QDRANT_ENDPOINT_HOST}:${QDRANT_ENDPOINT_PORT}"
        return 0
    fi
    case "$A4N_QDRANT_URL:$QDRANT_ENDPOINT_HOST" in
        http://*:127.0.0.1|http://*:localhost) ;;
        *) die "Qdrant 不可达: ${QDRANT_ENDPOINT_HOST}:${QDRANT_ENDPOINT_PORT}" ;;
    esac

    local binary="$PROJECT_ROOT/tools/qdrant/bin/qdrant"
    local pid_file="$SERVICE_DIR/qdrant.pid"
    local log_file="$SERVICE_LOG_DIR/qdrant.log"
    [[ -x "$binary" ]] || die "未找到本地 Qdrant；请先运行 ./scripts/dev/setup-qdrant.sh"
    mkdir -p "$SERVICE_DIR" "$SERVICE_LOG_DIR"

    local pid=""
    if [[ -f "$pid_file" ]]; then
        pid="$(cat "$pid_file" 2>/dev/null || true)"
    fi
    if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
        log_warn "Qdrant 进程已存在但尚未就绪，PID: $pid"
        wait_for_qdrant "$pid" "$log_file"
        return 0
    fi

    log_section "启动 Qdrant"
    (
        export QDRANT__SERVICE__HOST="127.0.0.1"
        export QDRANT__SERVICE__GRPC_PORT="$QDRANT_ENDPOINT_PORT"
        exec nohup setsid "$PROJECT_ROOT/scripts/dev/run-qdrant.sh"
    ) </dev/null >>"$log_file" 2>&1 &
    pid=$!
    printf '%s\n' "$pid" >"$pid_file"
    log_ok "Qdrant 已启动，PID: $pid，日志: $log_file"
    wait_for_qdrant "$pid" "$log_file"
}

ensure_sidecar_auth_token() {
    local token_file="$SERVICE_DIR/auth-token"
    mkdir -p "$SERVICE_DIR"
    if [[ -n "${A4N_SIDECAR_AUTH_TOKEN:-}" ]]; then
        umask 077
        printf '%s\n' "$A4N_SIDECAR_AUTH_TOKEN" >"$token_file"
        export A4N_SIDECAR_AUTH_TOKEN
        return 0
    fi
    if [[ -s "$token_file" ]]; then
        A4N_SIDECAR_AUTH_TOKEN="$(tr -d '[:space:]' <"$token_file")"
    else
        A4N_SIDECAR_AUTH_TOKEN="$(od -An -N32 -tx1 /dev/urandom | tr -d '[:space:]')"
        [[ -n "$A4N_SIDECAR_AUTH_TOKEN" ]] || die "无法生成 sidecar 认证 token"
        umask 077
        printf '%s\n' "$A4N_SIDECAR_AUTH_TOKEN" >"$token_file"
    fi
    [[ -n "$A4N_SIDECAR_AUTH_TOKEN" ]] || die "sidecar 认证 token 为空: $token_file"
    export A4N_SIDECAR_AUTH_TOKEN
}

module_package() {
    case "$1" in
        demo) printf '%s\n' "a4n-demo-sidecar" ;;
        *) catalog_field "$1" binary ;;
    esac
}

module_addr_env() {
    case "$1" in
        demo) printf '%s\n' "A4N_DEMO_SIDECAR_ADDR" ;;
        *) catalog_field "$1" addr_env ;;
    esac
}

module_default_addr() {
    case "$1" in
        demo) printf '%s\n' "127.0.0.1:47631" ;;
        *) catalog_field "$1" default_addr ;;
    esac
}

module_addr() {
    local module="$1"
    local addr_env default_addr
    addr_env="$(module_addr_env "$module")" || return 1
    default_addr="$(module_default_addr "$module")" || return 1
    printf '%s\n' "${!addr_env:-$default_addr}"
}

service_binary_path() {
    local package="$1"
    local target_dir="${CARGO_TARGET_DIR:-target}"
    case "$target_dir" in
        /*) ;;
        *) target_dir="$PROJECT_ROOT/$target_dir" ;;
    esac
    printf '%s/debug/%s\n' "$target_dir" "$package"
}

module_expected_capability_version() {
    local module="$1"
    [[ "$module" == "demo" ]] && return 0
    catalog_field "$module" capability_version
}

print_services() {
    printf '%s\n' "可启动服务:"
    for module in "${SERVICE_MODULES[@]}"; do
        printf '  %-10s -> %s\n' "$module" "$(module_package "$module")"
    done
    printf '  %-10s -> %s\n' demo a4n-demo-sidecar
}

health_check() {
    local module="$1"
    local addr="$2"
    local payload expected_capability_version capability_version
    command -v curl >/dev/null 2>&1 || return 1
    payload="$(curl -fsS --max-time 1 "http://${addr}/healthz" 2>/dev/null)" || return 1
    expected_capability_version="$(module_expected_capability_version "$module")" || \
        die "服务目录中缺少 $module 的 capability_version: $SERVICE_CATALOG_FILE"
    [[ "$(json_string_field "$payload" service)" == "$module" ]] || return 1
    [[ "$(json_string_field "$payload" status)" == "healthy" ]] || return 1
    capability_version="$(json_string_field "$payload" capability_version)"
    [[ -z "$expected_capability_version" || "$capability_version" == "$expected_capability_version" ]] || return 2
    [[ "$module" != "rag" ]] || qdrant_reachable
}

json_string_field() {
    local json="$1"
    local field="$2"
    printf '%s\n' "$json" | sed -n "s/.*\"${field}\":\"\([^\"]*\)\".*/\1/p"
}

json_number_field() {
    local json="$1"
    local field="$2"
    printf '%s\n' "$json" | sed -n "s/.*\"${field}\":\([0-9][0-9]*\).*/\1/p"
}

service_status() {
    local module="$1"
    local package addr pid_file pid process_state payload health_state
    local reported_service service_version protocol_version capability_version expected_capability_version

    package="$(module_package "$module")" || die "未知服务: $module；使用 --list 查看可用服务"
    expected_capability_version="$(module_expected_capability_version "$module")" || \
        die "服务目录中缺少 $module 的 capability_version: $SERVICE_CATALOG_FILE"
    addr="$(module_addr "$module")"
    pid_file="$SERVICE_DIR/${module}.pid"
    pid="-"
    process_state="not-managed"
    if [[ -f "$pid_file" ]]; then
        pid="$(cat "$pid_file" 2>/dev/null || true)"
        if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
            process_state="running"
        else
            process_state="stale"
            [[ -n "$pid" ]] || pid="-"
        fi
    fi

    if ! command -v curl >/dev/null 2>&1; then
        printf '%-10s package=%-24s pid=%-8s process=%-11s addr=%s health=unknown version=unknown (缺少 curl)\n' \
            "$module" "$package" "$pid" "$process_state" "$addr"
        return 1
    fi
    if ! payload="$(curl -fsS --max-time 2 "http://${addr}/healthz" 2>/dev/null)"; then
        printf '%-10s package=%-24s pid=%-8s process=%-11s addr=%s health=unreachable version=unknown\n' \
            "$module" "$package" "$pid" "$process_state" "$addr"
        return 1
    fi

    reported_service="$(json_string_field "$payload" service)"
    health_state="$(json_string_field "$payload" status)"
    service_version="$(json_string_field "$payload" service_version)"
    protocol_version="$(json_number_field "$payload" protocol_version)"
    capability_version="$(json_string_field "$payload" capability_version)"
    [[ -n "$health_state" ]] || health_state="invalid-response"
    [[ -n "$service_version" ]] || service_version="unknown"
    [[ -n "$protocol_version" ]] || protocol_version="unknown"
    [[ -n "$capability_version" ]] || capability_version="unknown"
    if [[ "$reported_service" != "$module" ]]; then
        health_state="wrong-service"
    elif [[ -n "$expected_capability_version" && "$capability_version" != "$expected_capability_version" ]]; then
        health_state="capability-mismatch"
    elif [[ "$module" == "rag" && "$health_state" == "healthy" ]] && ! qdrant_reachable; then
        health_state="dependency-unreachable"
    fi
    printf '%-10s package=%-24s pid=%-8s process=%-11s addr=%s health=%s version=%s protocol=%s capability=%s\n' \
        "$module" "$package" "$pid" "$process_state" "$addr" "$health_state" \
        "$service_version" "$protocol_version" "$capability_version"
    [[ "$health_state" == "healthy" ]]
}

wait_for_health() {
    local module="$1"
    local pid="$2"
    local addr="$3"
    local attempt

    log_info "等待 $module 健康检查: http://${addr}/healthz"
    for ((attempt = 1; attempt <= HEALTH_TIMEOUT_SECONDS; attempt++)); do
        if health_check "$module" "$addr"; then
            log_ok "$module 健康检查通过"
            return 0
        fi
        if [[ -n "$pid" ]] && ! kill -0 "$pid" 2>/dev/null; then
            die "$module 在健康检查通过前已退出；请查看日志: $SERVICE_LOG_DIR/${module}.log"
        fi
        sleep 1
    done

    die "$module 未在 ${HEALTH_TIMEOUT_SECONDS}s 内通过健康检查: http://${addr}/healthz"
}

validate_wait_timeout() {
    if ! [[ "$HEALTH_TIMEOUT_SECONDS" =~ ^[0-9]+$ ]] || [[ "$HEALTH_TIMEOUT_SECONDS" -lt 1 ]]; then
        die "--wait-timeout/A4N_SERVICE_HEALTH_TIMEOUT_SECONDS 必须是正整数秒数"
    fi
}

start_service() {
    local module="$1"
    local package pid_file log_file addr_env addr qdrant_url
    qdrant_url="${A4N_QDRANT_URL:-}"

    if ! package="$(module_package "$module")"; then
        die "未知服务: $module；使用 --list 查看可用服务"
    fi
    if ! cargo_package_exists "$package"; then
        die "服务 '$module' 映射到 package '$package'，但该 package 尚不存在；请先完成对应 sidecar 阶段"
    fi

    mkdir -p "$SERVICE_DIR" "$SERVICE_LOG_DIR" "$APP_TMP_DIR"
    pid_file="$SERVICE_DIR/${module}.pid"
    log_file="$SERVICE_LOG_DIR/${module}.log"
    addr_env="$(module_addr_env "$module")"
    addr="$(module_addr "$module")"

    if [[ "$module" == "rag" ]]; then
        [[ -n "${qdrant_url//[[:space:]]/}" ]] || \
            die "启动 rag 需要 A4N_QDRANT_URL；请在 .env 设置，例如 http://127.0.0.1:6334"
        ensure_qdrant
    fi

    if [[ -f "$pid_file" ]]; then
        local old_pid health_result
        old_pid="$(cat "$pid_file" 2>/dev/null || true)"
        if [[ -n "$old_pid" ]] && kill -0 "$old_pid" 2>/dev/null; then
            if health_check "$module" "$addr"; then
                log_warn "$module 已在运行，PID: $old_pid"
                return 0
            else
                health_result=$?
            fi
            if [[ "$health_result" -eq 2 ]]; then
                log_warn "$module capability 版本不匹配，重启旧进程，PID: $old_pid"
                "$SCRIPT_DIR/restart-service.sh" "$module"
                return 0
            fi
            log_warn "$module 已在运行，PID: $old_pid"
            if $WAIT_FOR_HEALTH; then
                wait_for_health "$module" "$old_pid" "$addr"
            fi
            return 0
        fi
        rm -f "$pid_file"
    fi

    if health_check "$module" "$addr"; then
        log_warn "$module 已在运行但非本脚本管理: $addr"
        return 0
    fi
    if tcp_reachable "${addr%:*}" "${addr##*:}"; then
        die "$module 地址已被占用，但未返回匹配的健康响应: http://${addr}/healthz"
    fi

    if [[ "$module" == "embedding" ]]; then
        require_onnxruntime
        log_ok "ONNX Runtime 1.24.2: $A4N_ONNXRUNTIME_ROOT"
    fi

    if ! $BUILD_CACHE_GUARDED; then
        guard_build_cache
        BUILD_CACHE_GUARDED=true
    fi

    log_section "构建 $module ($package)"
    (
        cd "$PROJECT_ROOT"
        cargo build -p "$package"
    )

    local binary_path
    binary_path="$(service_binary_path "$package")"
    [[ -x "$binary_path" ]] || die "服务 '$module' 构建后未找到可执行文件: $binary_path"

    log_section "启动 $module ($package)"
    (
        cd "$PROJECT_ROOT"
        export A4N_DATA_DIR="$APP_DATA_DIR"
        export A4N_CACHE_DIR="$APP_CACHE_DIR"
        export A4N_TEMP_DIR="$APP_TMP_DIR"
        export A4N_RAG_SERVICE_DATA_DIR="${A4N_RAG_SERVICE_DATA_DIR:-$APP_DATA_DIR}"
        export A4N_EMBEDDING_CACHE_DIR="${A4N_EMBEDDING_CACHE_DIR:-$APP_CACHE_DIR/embedding-models}"
        export A4N_PROMPT_SERVICE_PROMPTS_DIR="${A4N_PROMPT_SERVICE_PROMPTS_DIR:-$PROJECT_ROOT/prompts}"
        export "$addr_env=$addr"
        exec nohup setsid "$binary_path"
    ) </dev/null >>"$log_file" 2>&1 &
    local pid=$!
    printf '%s\n' "$pid" >"$pid_file"
    log_ok "$module 已启动，PID: $pid，日志: $log_file"
    if $WAIT_FOR_HEALTH; then
        wait_for_health "$module" "$pid" "$addr"
    fi
}

modules_arg=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --list)
            print_services
            exit 0
            ;;
        --status)
            STATUS_ONLY=true
            shift
            ;;
        --help|-h)
            print_script_usage "$0" 16
            echo ""
            print_services
            exit 0
            ;;
        --no-wait)
            WAIT_FOR_HEALTH=false
            shift
            ;;
        --wait-timeout)
            if [[ $# -lt 2 || -z "${2:-}" ]]; then
                die "--wait-timeout 需要提供秒数"
            fi
            HEALTH_TIMEOUT_SECONDS="$2"
            shift 2
            ;;
        --*)
            die "未知参数: $1；使用 --help 查看用法"
            ;;
        *)
            if [[ -n "$modules_arg" ]]; then
                die "只能提供一个服务列表参数；收到额外参数: $1"
            fi
            modules_arg="$1"
            shift
            ;;
    esac
done

if $STATUS_ONLY; then
    status_failed=false
    if [[ -n "$modules_arg" ]]; then
        IFS=',' read -r -a status_modules <<<"$modules_arg"
    else
        status_modules=("${SERVICE_MODULES[@]}")
    fi
    for module in "${status_modules[@]}"; do
        module="${module//[[:space:]]/}"
        [[ -n "$module" ]] || continue
        if ! service_status "$module"; then
            status_failed=true
        fi
    done
    if $status_failed; then
        exit 1
    fi
    exit 0
fi

if [[ -z "$modules_arg" ]]; then
    print_script_usage "$0" 16
    exit 1
fi

validate_wait_timeout
ensure_sidecar_auth_token

IFS=',' read -r -a modules <<<"$modules_arg"
for module in "${modules[@]}"; do
    module="${module//[[:space:]]/}"
    [[ -n "$module" ]] || continue
    start_service "$module"
done
