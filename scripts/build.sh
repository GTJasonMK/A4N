#!/usr/bin/env bash
#=============================================================================
# A4N 项目构建脚本
#
# 用法:
#   ./scripts/build.sh              # 构建前端 + 默认 Rust 可执行包（Debug）
#   ./scripts/build.sh --release    # Release 构建（LTO + strip）
#   ./scripts/build.sh --frontend   # 仅构建前端
#   ./scripts/build.sh --backend    # 仅构建后端（Debug）
#   ./scripts/build.sh --package a4n-server # 仅构建指定 Rust package 及其依赖
#   ./scripts/build.sh --package a4n-server --no-default-features # 使用 minimal host feature 集检查/构建
#   ./scripts/build.sh --package a4n-server --features embedded-capabilities # 检查/构建 full host
#   ./scripts/build.sh --all         # 构建整个 Rust workspace
#   ./scripts/build.sh --check      # 仅检查编译（不输出二进制）
#   ./scripts/build.sh --help       # 显示帮助
#
# 跨平台：Linux / macOS / Windows(Git Bash / WSL)
#=============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

# —— 配置 ——
FRONTEND_DIR="$PROJECT_ROOT/frontend"
TARGET_CRATE="a4n-tauri"
BUILD_MODE="debug"
FRONTEND_ONLY=false
BACKEND_ONLY=false
BUILD_ALL=false
PACKAGE_SELECTED=false
NO_DEFAULT_FEATURES=false
FEATURES=""

# —— 解析参数 ——
while [[ $# -gt 0 ]]; do
    case "$1" in
        --release)
            [[ "$BUILD_MODE" == "debug" ]] || die "构建模式只能指定一次"
            BUILD_MODE="release"
            shift
            ;;
        --check)
            [[ "$BUILD_MODE" == "debug" ]] || die "构建模式只能指定一次"
            BUILD_MODE="check"
            shift
            ;;
        --frontend)
            FRONTEND_ONLY=true
            shift
            ;;
        --backend)
            BACKEND_ONLY=true
            shift
            ;;
        --package)
            if [[ $# -lt 2 || -z "${2:-}" ]]; then
                die "--package 需要提供 Cargo package 名称，例如: --package a4n-server"
            fi
            $PACKAGE_SELECTED && die "--package 只能指定一次"
            PACKAGE_SELECTED=true
            TARGET_CRATE="$2"
            shift 2
            ;;
        --all)
            BUILD_ALL=true
            shift
            ;;
        --no-default-features)
            NO_DEFAULT_FEATURES=true
            shift
            ;;
        --features)
            if [[ $# -lt 2 || -z "${2:-}" ]]; then
                die "--features 需要提供 Cargo feature 列表"
            fi
            FEATURES="$2"
            shift 2
            ;;
        --help|-h)
            print_script_usage "$0" 15
            exit 0
            ;;
        *)
            die "未知参数: $1；使用 --help 查看用法"
            ;;
    esac
done

$BUILD_ALL && $PACKAGE_SELECTED && die "--all 与 --package 不能同时使用"
if $FRONTEND_ONLY; then
    $BACKEND_ONLY && die "--frontend 与 --backend 不能同时使用"
    [[ "$BUILD_MODE" == "debug" ]] || die "--frontend 不能与 Rust 构建模式同时使用"
    $BUILD_ALL && die "--frontend 不能与 --all 同时使用"
    $PACKAGE_SELECTED && die "--frontend 不能与 --package 同时使用"
    $NO_DEFAULT_FEATURES && die "--frontend 不能与 --no-default-features 同时使用"
    [[ -n "$FEATURES" ]] && die "--frontend 不能与 --features 同时使用"
fi

should_build_frontend() {
    [[ "$BACKEND_ONLY" == false && "$BUILD_MODE" != "check" ]]
}

cargo_target_args() {
    if ! $BUILD_ALL; then
        printf '%s\n' "-p" "$TARGET_CRATE"
        if [[ "$TARGET_CRATE" == "a4n-server" ]]; then
            # Server supervises Edge as a sibling process; building the Server
            # alone would produce an unusable runtime bundle.
            printf '%s\n' "-p" "a4n-edge"
            printf '%s\n' "-p" "a4n-novel-planning-worker"
            printf '%s\n' "-p" "a4n-artifact-media-worker"
            printf '%s\n' "-p" "a4n-outline-worker"
            printf '%s\n' "-p" "a4n-blueprint-worker"
            printf '%s\n' "-p" "a4n-asset-graph-worker"
            printf '%s\n' "-p" "a4n-foreshadowing-worker"
        fi
    fi

    if $NO_DEFAULT_FEATURES; then
        printf '%s\n' "--no-default-features"
    fi
    if [[ -n "$FEATURES" ]]; then
        printf '%s\n' "--features" "$FEATURES"
    fi
}

# —— 前置检查 ——
check_prerequisites() {
    echo -e "${CYAN}━━━ 前置检查 ━━━${NC}"

    if ! $FRONTEND_ONLY; then
        require_command cargo
        echo -e "${GREEN}✓ cargo $(cargo --version | cut -d' ' -f2)${NC}"

        local rust_ver
        rust_ver=$(rustc --version | cut -d' ' -f2)
        echo -e "${GREEN}✓ rustc ${rust_ver}${NC}"

        local cargo_args=()
        while IFS= read -r arg; do
            cargo_args+=("$arg")
        done < <(cargo_target_args)
        if cargo_graph_uses_onnxruntime normal,build "${cargo_args[@]}"; then
            require_onnxruntime
            echo -e "${GREEN}✓ ONNX Runtime $(onnxruntime_version) ($A4N_ONNXRUNTIME_ROOT)${NC}"
        fi
    fi

    # Node.js（前端构建必需）
    if should_build_frontend; then
        require_command node
        require_command npm
        echo -e "${GREEN}✓ node $(node --version)${NC}"
        echo -e "${GREEN}✓ npm $(npm --version)${NC}"

        ensure_frontend_dependencies
    fi

    # Tauri 系统库缺失时在 Cargo 构建前失败，不把必需依赖降级成 warning。
    if ! $FRONTEND_ONLY && ($BUILD_ALL || [[ "$TARGET_CRATE" == "a4n-tauri" ]]); then
        require_tauri_system_libraries
        echo -e "${GREEN}✓ Tauri Linux 系统库已就绪${NC}"
    fi

    echo ""
}

# —— 构建前端 ——
build_frontend() {
    echo -e "${CYAN}━━━ 构建前端 ━━━${NC}"
    cd "$FRONTEND_DIR"

    if npm run build 2>&1; then
        local css_size js_size
        css_size=$(du -h "$FRONTEND_DIR/dist/assets/"*.css 2>/dev/null | cut -f1 || echo "N/A")
        js_size=$(du -h "$FRONTEND_DIR/dist/assets/"*.js 2>/dev/null | cut -f1 || echo "N/A")
        echo ""
        echo -e "  ${GREEN}✓${NC} 前端构建成功"
        echo -e "  CSS: ${css_size}  JS: ${js_size}"
        echo -e "  产物: ${FRONTEND_DIR}/dist/"
    else
        echo -e "${RED}✗ 前端构建失败${NC}"
        exit 1
    fi

    cd "$PROJECT_ROOT"
    echo ""
}

# —— 执行构建 ——
do_build() {
    local cargo_args=()
    local cargo_command=("$PROJECT_ROOT/scripts/cargo.sh")

    while IFS= read -r arg; do
        cargo_args+=("$arg")
    done < <(cargo_target_args)

    if $BUILD_ALL; then
        echo -e "  范围: ${CYAN}整个 Rust workspace${NC}"
    else
        echo -e "  范围: ${CYAN}${TARGET_CRATE}${NC} package 及其依赖"
    fi
    echo ""

    case "$BUILD_MODE" in
        debug)
            cargo_command+=(build)
            echo -e "${CYAN}━━━ Debug 构建 ━━━${NC}"
            ;;
        release)
            cargo_command+=(build --release)
            echo -e "${CYAN}━━━ Release 构建 ━━━${NC}"
            ;;
        check)
            cargo_command+=(check)
            echo -e "${CYAN}━━━ 编译检查 ━━━${NC}"
            ;;
    esac

    echo ""
    cd "$PROJECT_ROOT"

    if "${cargo_command[@]}" "${cargo_args[@]}"; then
        echo ""
        echo -e "${GREEN}${BOLD}━━━ 后端构建成功 ━━━${NC}"

        # 显示产物位置
        if [[ "$BUILD_MODE" != "check" ]]; then
            local binary_path="$CARGO_TARGET_DIR/$BUILD_MODE/$TARGET_CRATE"
            if [[ -f "$binary_path" ]]; then
                local size
                size=$(du -h "$binary_path" | cut -f1)
                echo -e "  产物: ${CYAN}${binary_path}${NC}"
                echo -e "  大小: ${CYAN}${size}${NC}"
            fi
        fi

        echo ""
        exit 0
    fi

    echo ""
    echo -e "${RED}${BOLD}━━━ 后端构建失败 ━━━${NC}"
    echo "  请检查上方错误信息"
    echo ""
    exit 1
}

# —— 主流程 ——
echo -e "${BOLD}${CYAN}╔══════════════════════════════════╗${NC}"
echo -e "${BOLD}${CYAN}║     A4N 项目构建脚本            ║${NC}"
echo -e "${BOLD}${CYAN}╚══════════════════════════════════╝${NC}"
echo ""

check_prerequisites

if $FRONTEND_ONLY; then
    build_frontend
    echo -e "${GREEN}${BOLD}前端构建完成${NC}"
    exit 0
fi

# 默认：按当前模式构建前端和 Rust 目标
if should_build_frontend; then
    build_frontend
fi
do_build
