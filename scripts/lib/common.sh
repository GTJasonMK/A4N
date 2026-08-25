#!/usr/bin/env bash

if [[ -n "${A4N_SCRIPT_COMMON_SOURCED:-}" ]]; then
    return 0
fi
A4N_SCRIPT_COMMON_SOURCED=1

SCRIPT_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_LIB_DIR/../.." && pwd)"

# 所有入口共用一套主缓存，避免同一依赖图被编进多个 target 目录。
# 临时输出、实验和计时产物也应放在仓库 target 子目录。
export CARGO_TARGET_DIR="$PROJECT_ROOT/target"

make_target_temp_dir() {
    local temp_root="$CARGO_TARGET_DIR/tmp"
    mkdir -p "$temp_root"
    mktemp -d "$temp_root/$1.XXXXXX"
}

ensure_frontend_dependencies() {
    local frontend_dir="$PROJECT_ROOT/frontend"
    [[ -d "$frontend_dir/node_modules" ]] && return
    log_warn "前端依赖未安装，执行 npm ci"
    (cd "$frontend_dir" && npm ci --silent)
    log_ok "前端依赖安装完成"
}

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

sha256_file() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | awk '{print $1}'
    else
        die "未找到 SHA-256 工具: sha256sum 或 shasum"
    fi
}

qdrant_release_metadata() {
    local platform="$1"
    local version_file="$PROJECT_ROOT/tools/qdrant/VERSION"
    local releases_file="$PROJECT_ROOT/tools/qdrant/releases.tsv"
    local version matches asset sha256 extra

    [[ -r "$version_file" ]] || die "缺少 Qdrant 版本文件: $version_file"
    [[ -r "$releases_file" ]] || die "缺少 Qdrant release 清单: $releases_file"
    version="$(tr -d '[:space:]' < "$version_file")"
    [[ "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "无效 Qdrant 版本: $version"
    matches="$(awk -F '\t' -v platform="$platform" '$0 !~ /^#/ && $1 == platform { print $2 "\t" $3 }' "$releases_file")"
    [[ -n "$matches" && "$matches" != *$'\n'* ]] || die "Qdrant release 清单必须包含唯一平台: $platform"
    IFS=$'\t' read -r asset sha256 extra <<<"$matches"
    [[ "$asset" =~ ^qdrant-[A-Za-z0-9._-]+\.(tar\.gz|zip)$ && "$sha256" =~ ^[0-9a-f]{64}$ && -z "$extra" ]] \
        || die "Qdrant release 清单条目无效: $platform"
    printf '%s\t%s\t%s\n' "$version" "$asset" "$sha256"
}

wait_for_qdrant() {
    local pid="$1"
    local log="$2"
    for _ in {1..300}; do
        if curl -fsS --max-time 1 http://127.0.0.1:6333/readyz >/dev/null 2>&1 \
            && (exec 3<>/dev/tcp/127.0.0.1/6334) 2>/dev/null; then
            return
        fi
        if ! kill -0 "$pid" 2>/dev/null; then
            tail -n 200 "$log" >&2
            die "Qdrant 在就绪前退出"
        fi
        sleep 0.1
    done
    tail -n 200 "$log" >&2
    die "Qdrant 启动超时"
}

onnxruntime_version() {
    local version_file="$PROJECT_ROOT/tools/onnxruntime/VERSION"
    local version
    [[ -r "$version_file" ]] || die "缺少 ONNX Runtime 版本文件: $version_file"
    version="$(tr -d '[:space:]' < "$version_file")"
    [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "无效 ONNX Runtime 版本: $version"
    printf '%s\n' "$version"
}

onnxruntime_root() {
    local data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
    local version
    version="$(onnxruntime_version)"
    printf '%s\n' "${A4N_ONNXRUNTIME_ROOT:-$data_home/a4n/onnxruntime/$version}"
}

configure_onnxruntime() {
    local root version_file version expected_version lib_dir library runtime_path_var toolchain_lib_dir
    root="$(onnxruntime_root)"
    expected_version="$(onnxruntime_version)"
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
    [[ "$version" == "$expected_version" ]] || return 1
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

    local root version
    root="$(onnxruntime_root)"
    version="$(onnxruntime_version)"
    die "缺少 ONNX Runtime $version 动态库: $root；请先运行 ./scripts/dev/setup-onnxruntime.sh"
}

cargo_graph_uses_onnxruntime() {
    local edge_kinds=$1
    shift
    local dependency_tree
    if ! dependency_tree=$(cd "$PROJECT_ROOT" && cargo tree -e "$edge_kinds" --prefix none "$@"); then
        die "无法解析 Cargo 依赖图，不能判断当前命令是否需要 ONNX Runtime"
    fi
    grep -Fq 'a4n-embedding-fastembed v' <<<"$dependency_tree"
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

sidecar_catalog_rows() {
    require_command ruby
    ruby "$PROJECT_ROOT/scripts/release_manifest.rb" service-rows
}

sidecar_catalog_binaries() {
    sidecar_catalog_rows | cut -f2
}

release_tool_rows() {
    require_command ruby
    ruby "$PROJECT_ROOT/scripts/release_manifest.rb" tool-rows
}

release_tool_binaries() {
    release_tool_rows | cut -f2
}

release_tool_binary_for_package() {
    local requested_package="$1" rows package binary
    rows="$(release_tool_rows)" || return
    while IFS=$'\t' read -r package binary; do
        if [[ "$package" == "$requested_package" ]]; then
            printf '%s\n' "$binary"
            return
        fi
    done <<<"$rows"
    die "release tool catalog 缺少 package: $requested_package"
}

release_manifest_schema_version() {
    require_command ruby
    ruby "$PROJECT_ROOT/scripts/release_manifest.rb" schema-version
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
