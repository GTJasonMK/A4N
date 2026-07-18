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
WAIT_FOR_HEALTH=true
HEALTH_TIMEOUT_SECONDS="${A4N_SERVICE_HEALTH_TIMEOUT_SECONDS:-30}"
STATUS_ONLY=false
SERVICE_MODULES=(prompt llm embedding rag image video import export)

module_package() {
    case "$1" in
        prompt) printf '%s\n' "a4n-prompt-service" ;;
        llm) printf '%s\n' "a4n-llm-gateway" ;;
        embedding) printf '%s\n' "a4n-embedding-service" ;;
        rag) printf '%s\n' "a4n-rag-service" ;;
        image) printf '%s\n' "a4n-image-service" ;;
        video) printf '%s\n' "a4n-video-service" ;;
        import) printf '%s\n' "a4n-import-service" ;;
        export) printf '%s\n' "a4n-export-service" ;;
        demo) printf '%s\n' "a4n-demo-sidecar" ;;
        *)
            return 1
            ;;
    esac
}

module_addr_env() {
    case "$1" in
        prompt) printf '%s\n' "A4N_PROMPT_SERVICE_ADDR" ;;
        llm) printf '%s\n' "A4N_LLM_GATEWAY_ADDR" ;;
        embedding) printf '%s\n' "A4N_EMBEDDING_SERVICE_ADDR" ;;
        rag) printf '%s\n' "A4N_RAG_SERVICE_ADDR" ;;
        image) printf '%s\n' "A4N_IMAGE_SERVICE_ADDR" ;;
        video) printf '%s\n' "A4N_VIDEO_SERVICE_ADDR" ;;
        import) printf '%s\n' "A4N_IMPORT_SERVICE_ADDR" ;;
        export) printf '%s\n' "A4N_EXPORT_SERVICE_ADDR" ;;
        demo) printf '%s\n' "A4N_DEMO_SIDECAR_ADDR" ;;
        *) return 1 ;;
    esac
}

module_default_addr() {
    case "$1" in
        prompt) printf '%s\n' "127.0.0.1:47641" ;;
        llm) printf '%s\n' "127.0.0.1:47642" ;;
        embedding) printf '%s\n' "127.0.0.1:47643" ;;
        rag) printf '%s\n' "127.0.0.1:47644" ;;
        image) printf '%s\n' "127.0.0.1:47645" ;;
        video) printf '%s\n' "127.0.0.1:47646" ;;
        import) printf '%s\n' "127.0.0.1:47647" ;;
        export) printf '%s\n' "127.0.0.1:47648" ;;
        demo) printf '%s\n' "127.0.0.1:47631" ;;
        *) return 1 ;;
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

print_services() {
    cat <<'EOF'
可启动服务:
  prompt     -> a4n-prompt-service
  llm        -> a4n-llm-gateway
  embedding  -> a4n-embedding-service
  rag        -> a4n-rag-service
  image      -> a4n-image-service
  video      -> a4n-video-service
  import     -> a4n-import-service
  export     -> a4n-export-service
  demo       -> a4n-demo-sidecar
EOF
}

health_check() {
    local addr="$1"
    local host="${addr%:*}"
    local port="${addr##*:}"
    if [[ -z "$host" || -z "$port" || "$host" == "$port" ]]; then
        return 1
    fi

    if command -v curl >/dev/null 2>&1; then
        curl -fsS --max-time 1 "http://${addr}/healthz" >/dev/null 2>&1
    else
        timeout 1 bash -c "</dev/tcp/${host}/${port}" >/dev/null 2>&1
    fi
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
    local service_version protocol_version capability_version

    package="$(module_package "$module")" || die "未知服务: $module；使用 --list 查看可用服务"
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

    health_state="$(json_string_field "$payload" status)"
    service_version="$(json_string_field "$payload" service_version)"
    protocol_version="$(json_number_field "$payload" protocol_version)"
    capability_version="$(json_string_field "$payload" capability_version)"
    [[ -n "$health_state" ]] || health_state="invalid-response"
    [[ -n "$service_version" ]] || service_version="unknown"
    [[ -n "$protocol_version" ]] || protocol_version="unknown"
    [[ -n "$capability_version" ]] || capability_version="unknown"
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
        if health_check "$addr"; then
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
    local package pid_file log_file addr_env addr

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

    if [[ -f "$pid_file" ]]; then
        local old_pid
        old_pid="$(cat "$pid_file" 2>/dev/null || true)"
        if [[ -n "$old_pid" ]] && kill -0 "$old_pid" 2>/dev/null; then
            log_warn "$module 已在运行，PID: $old_pid"
            if $WAIT_FOR_HEALTH; then
                wait_for_health "$module" "$old_pid" "$addr"
            fi
            return 0
        fi
    fi

    if [[ "$module" == "embedding" ]]; then
        require_onnxruntime
        log_ok "ONNX Runtime 1.24.2: $A4N_ONNXRUNTIME_ROOT"
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
guard_build_cache

IFS=',' read -r -a modules <<<"$modules_arg"
for module in "${modules[@]}"; do
    module="${module//[[:space:]]/}"
    [[ -n "$module" ]] || continue
    start_service "$module"
done
