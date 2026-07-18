#!/usr/bin/env bash
#=============================================================================
# A4N Sidecar 服务重启脚本
#
# 用法:
#   ./scripts/dev/restart-service.sh llm
#   ./scripts/dev/restart-service.sh rag
#   ./scripts/dev/restart-service.sh --list
#=============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"

load_dotenv

APP_PORTABLE_HOME="$(resolve_project_path "${A4N_PORTABLE_HOME:-$PROJECT_ROOT}")"
APP_DATA_DIR="$(resolve_project_path "${A4N_DATA_DIR:-$APP_PORTABLE_HOME/data}")"
SERVICE_DIR="$APP_DATA_DIR/services"

if [[ $# -ne 1 ]]; then
    print_script_usage "$0" 10
    exit 1
fi

case "$1" in
    --list)
        exec "$SCRIPT_DIR/run-services.sh" --list
        ;;
    --help|-h)
        print_script_usage "$0" 10
        echo ""
        "$SCRIPT_DIR/run-services.sh" --list
        exit 0
        ;;
esac

module="$1"
pid_file="$SERVICE_DIR/${module}.pid"

if [[ -f "$pid_file" ]]; then
    pid="$(cat "$pid_file" 2>/dev/null || true)"
    if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
        log_section "停止 $module"
        kill -TERM "$pid" 2>/dev/null || true
        for _ in {1..20}; do
            if ! kill -0 "$pid" 2>/dev/null; then
                break
            fi
            sleep 0.1
        done
        if kill -0 "$pid" 2>/dev/null; then
            log_warn "$module 未在 2 秒内退出，发送 KILL"
            kill -KILL "$pid" 2>/dev/null || true
        fi
    fi
    rm -f "$pid_file"
fi

exec "$SCRIPT_DIR/run-services.sh" "$module"
