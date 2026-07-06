#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
cd "$PROJECT_ROOT"

COMMAND="${1:-up}"

require_command docker

case "$COMMAND" in
  up)
    docker compose up -d postgres
    ;;
  down)
    docker compose down
    ;;
  status)
    docker compose ps postgres
    ;;
  *)
    die "用法: $0 {up|down|status}"
    ;;
esac
