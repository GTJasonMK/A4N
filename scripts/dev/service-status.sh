#!/usr/bin/env bash
#=============================================================================
# A4N Sidecar 服务状态入口
#
# 用法:
#   ./scripts/dev/service-status.sh
#   ./scripts/dev/service-status.sh prompt,llm
#=============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$SCRIPT_DIR/run-services.sh" --status "$@"
