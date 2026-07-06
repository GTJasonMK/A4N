#!/usr/bin/env bash

if [[ -n "${A4N_SCRIPT_COMMON_SOURCED:-}" ]]; then
    return 0
fi
A4N_SCRIPT_COMMON_SOURCED=1

SCRIPT_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_LIB_DIR/../.." && pwd)"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
BOLD='\033[1m'
NC='\033[0m'

log_section() {
    echo -e "${CYAN}━━━ $* ━━━${NC}"
}

log_ok() {
    echo -e "  ${GREEN}✓${NC} $*"
}

log_info() {
    echo -e "  ${CYAN}→${NC} $*"
}

log_warn() {
    echo -e "  ${YELLOW}⚠${NC} $*"
}

die() {
    echo -e "${RED}✗ $*${NC}" >&2
    exit 1
}

require_command() {
    command -v "$1" >/dev/null 2>&1 || die "未找到命令: $1"
}

load_dotenv() {
    local env_file="${1:-$PROJECT_ROOT/.env}"
    if [[ -f "$env_file" ]]; then
        set -a
        # shellcheck disable=SC1090
        source "$env_file"
        set +a
    fi
}

resolve_project_path() {
    local path="$1"
    case "$path" in
        /*) printf '%s\n' "$path" ;;
        *) printf '%s/%s\n' "$PROJECT_ROOT" "$path" ;;
    esac
}

print_script_usage() {
    local script="$1"
    local end_line="${2:-40}"
    sed -n "3,${end_line}s/^# \\{0,1\\}//p" "$script"
}

