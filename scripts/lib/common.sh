#!/usr/bin/env bash

if [[ -n "${A4N_SCRIPT_COMMON_SOURCED:-}" ]]; then
    return 0
fi
A4N_SCRIPT_COMMON_SOURCED=1

SCRIPT_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_LIB_DIR/../.." && pwd)"

# 所有入口共用一套主缓存，避免同一依赖图被编进多个 target 目录。
# 实验/计时请用 --target-dir /tmp/... 或 target-*，由 guard-build-cache 回收。
export CARGO_TARGET_DIR="$PROJECT_ROOT/target"

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

onnxruntime_root() {
    local data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
    printf '%s\n' "${A4N_ONNXRUNTIME_ROOT:-$data_home/a4n/onnxruntime/1.24.2}"
}

configure_onnxruntime() {
    local root version_file version lib_dir library runtime_path_var toolchain_lib_dir
    root="$(onnxruntime_root)"
    version_file="$root/VERSION_NUMBER"
    lib_dir="$root/lib"

    case "$(uname -s)" in
        Linux)
            library="$lib_dir/libonnxruntime.so"
            runtime_path_var="LD_LIBRARY_PATH"
            ;;
        Darwin)
            library="$lib_dir/libonnxruntime.dylib"
            runtime_path_var="DYLD_LIBRARY_PATH"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            library="$lib_dir/onnxruntime.dll"
            runtime_path_var="PATH"
            ;;
        *) return 1 ;;
    esac

    [[ -r "$version_file" ]] || return 1
    version="$(tr -d '[:space:]' < "$version_file")"
    [[ "$version" == "1.24.2" ]] || return 1
    [[ -r "$library" ]] || return 1

    export A4N_ONNXRUNTIME_ROOT="$root"
    toolchain_lib_dir="$lib_dir"
    if [[ "$runtime_path_var" == "PATH" ]] && command -v cygpath >/dev/null 2>&1; then
        toolchain_lib_dir="$(cygpath -w "$lib_dir")"
    fi
    export ORT_LIB_PATH="$toolchain_lib_dir"
    export ORT_PREFER_DYNAMIC_LINK=1
    local runtime_path="${!runtime_path_var:-}"
    case ":$runtime_path:" in
        *":$lib_dir:"*) ;;
        *) export "$runtime_path_var=$lib_dir${runtime_path:+:$runtime_path}" ;;
    esac
}

require_onnxruntime() {
    if configure_onnxruntime; then
        return 0
    fi

    local root
    root="$(onnxruntime_root)"
    die "缺少 ONNX Runtime 1.24.2 动态库: $root；请先运行 ./scripts/dev/setup-onnxruntime.sh"
}

guard_build_cache() {
    # 软上限内：只扔过期 target-*；超限：incremental → sweep → 硬上限才 cargo clean
    "$PROJECT_ROOT/scripts/dev/guard-build-cache.sh"
}

cargo_package_exists() {
    local package="$1"
    local pattern="^[[:space:]]*name[[:space:]]*=[[:space:]]*\"${package}\""
    local manifests=("$PROJECT_ROOT"/crates/*/Cargo.toml)

    [[ -e "${manifests[0]}" ]] || return 1
    if command -v rg >/dev/null 2>&1; then
        rg -q -- "$pattern" "${manifests[@]}"
    else
        grep -Eq -- "$pattern" "${manifests[@]}"
    fi
}

load_dotenv() {
    local env_file="${1:-$PROJECT_ROOT/.env}"
    if [[ -f "$env_file" ]]; then
        local -A existing_env=()
        local name value
        while IFS='=' read -r name value; do
            existing_env["$name"]="$value"
        done < <(env)

        set -a
        # shellcheck disable=SC1090
        source "$env_file"
        set +a

        for name in "${!existing_env[@]}"; do
            export "$name=${existing_env[$name]}"
        done
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
