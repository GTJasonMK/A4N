#!/usr/bin/env bash
#=============================================================================
# A4N 非热重载运行入口
#
# 用法:
#   ./scripts/dev/run-no-hot.sh          # cargo tauri dev --no-watch，并关闭 Vite HMR
#   ./scripts/dev/run-no-hot.sh --check  # 透传 run.sh 参数
#=============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$SCRIPT_DIR/../run.sh" --no-hot "$@"
