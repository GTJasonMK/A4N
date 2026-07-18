#!/usr/bin/env bash
#=============================================================================
# A4N Host 运行脚本
#
# 用法:
#   ./scripts/dev/run-host.sh tauri
#   ./scripts/dev/run-host.sh server
#   ./scripts/dev/run-host.sh tauri --no-hot
#   ./scripts/dev/run-host.sh server --check
#   ./scripts/dev/run-host.sh server --full
#=============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"

if [[ $# -lt 1 ]]; then
    print_script_usage "$0" 10
    exit 1
fi

host="$1"
shift
FULL_HOST=false
CHECK_ONLY=false

remaining=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --full|--embedded)
            FULL_HOST=true
            shift
            ;;
        --check)
            CHECK_ONLY=true
            shift
            ;;
        *)
            remaining+=("$1")
            shift
            ;;
    esac
done
set -- "${remaining[@]}"

configure_sidecar_host_env() {
    if $FULL_HOST; then
        require_onnxruntime
        export A4N_RUNTIME_PROFILE="${A4N_RUNTIME_PROFILE:-full}"
        export A4N_HOST_CARGO_RUN_ARGS="${A4N_HOST_CARGO_RUN_ARGS:---features embedded-capabilities}"
        export A4N_TAURI_DEV_ARGS="${A4N_TAURI_DEV_ARGS:---features embedded-capabilities}"
        return
    fi

    export A4N_RUNTIME_PROFILE="${A4N_RUNTIME_PROFILE:-sidecar}"
    export A4N_HOST_CARGO_RUN_ARGS="${A4N_HOST_CARGO_RUN_ARGS:---no-default-features}"
    export A4N_TAURI_DEV_ARGS="${A4N_TAURI_DEV_ARGS:---no-default-features}"
}

guard_build_cache

case "$host" in
    tauri|desktop)
        configure_sidecar_host_env
        if $CHECK_ONLY; then
            if $FULL_HOST; then
                exec cargo check -p a4n-tauri --features embedded-capabilities
            fi
            exec cargo check -p a4n-tauri --no-default-features
        fi
        exec "$PROJECT_ROOT/scripts/run.sh" "$@"
        ;;
    server|web)
        configure_sidecar_host_env
        if $CHECK_ONLY; then
            if $FULL_HOST; then
                exec cargo check -p a4n-server --features embedded-capabilities
            fi
            exec cargo check -p a4n-server --no-default-features
        fi
        if $FULL_HOST; then
            exec cargo run -p a4n-server --features embedded-capabilities "$@"
        fi
        exec cargo run -p a4n-server --no-default-features "$@"
        ;;
    --help|-h)
        print_script_usage "$0" 10
        cat <<'EOF'

默认行为:
  run-host.sh 默认设置 A4N_RUNTIME_PROFILE=sidecar，并用 --no-default-features 启动 host。
  如需旧的 embedded/full host，追加 --full。
EOF
        ;;
    *)
        die "未知 host: $host；可用值: tauri, server"
        ;;
esac
