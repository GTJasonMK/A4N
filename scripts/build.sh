#!/usr/bin/env bash
#=============================================================================
# A4N 项目构建脚本
#
# 用法:
#   ./scripts/build.sh              # 构建前端 + 默认 Rust 可执行包（Debug）
#   ./scripts/build.sh --release    # Release 构建（LTO + strip）
#   ./scripts/build.sh -r           # Release 构建（简写）
#   ./scripts/build.sh --frontend   # 仅构建前端
#   ./scripts/build.sh --backend    # 仅构建后端（Debug）
#   ./scripts/build.sh -p a4n-server # 仅构建指定 Rust package 及其依赖
#   ./scripts/build.sh --all         # 构建整个 Rust workspace
#   ./scripts/build.sh --check      # 仅检查编译（不输出二进制）
#   ./scripts/build.sh -c           # 仅检查编译（简写）
#   ./scripts/build.sh -w           # 显示所有 warning
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
EXTRA_FLAGS=""
SHOW_WARNINGS=false
FRONTEND_ONLY=false
BACKEND_ONLY=false
BUILD_ALL=false

# —— 解析参数 ——
while [[ $# -gt 0 ]]; do
    case "$1" in
        --release|-r)
            BUILD_MODE="release"
            shift
            ;;
        --check|-c)
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
        --package|-p)
            if [[ $# -lt 2 || -z "${2:-}" ]]; then
                die "--package 需要提供 Cargo package 名称，例如: --package a4n-server"
            fi
            TARGET_CRATE="$2"
            shift 2
            ;;
        --all)
            BUILD_ALL=true
            shift
            ;;
        --warnings|-w)
            SHOW_WARNINGS=true
            shift
            ;;
        --help|-h)
            print_script_usage "$0" 16
            exit 0
            ;;
        *)
            die "未知参数: $1；使用 --help 查看用法"
            ;;
    esac
done

should_build_frontend() {
    if $BACKEND_ONLY; then
        return 1
    fi
    if [[ "$BUILD_MODE" == "check" ]]; then
        return 1
    fi
    return 0
}

cargo_target_args() {
    if $BUILD_ALL; then
        return 0
    fi

    printf '%s\n' "-p" "$TARGET_CRATE"
}

run_cargo_capture() {
    local output_file=$1
    shift

    if $SHOW_WARNINGS; then
        "$@" 2>&1 | tee "$output_file"
        return "${PIPESTATUS[0]}"
    fi

    "$@" 2>&1 | tee "$output_file" | while IFS= read -r line; do
        # 过滤 warning，只在有错误时显示
        if [[ "$line" =~ ^error ]]; then
            echo -e "${RED}${line}${NC}"
        elif [[ ! "$line" =~ ^warning ]] && [[ ! "$line" =~ ^\s+--\> ]] && [[ ! "$line" =~ ^\s+\| ]] && [[ ! "$line" =~ ^\s+= ]] && [[ ! "$line" =~ ^\s+help: ]]; then
            echo "$line"
        fi
    done
    return "${PIPESTATUS[0]}"
}

# —— 前置检查 ——
check_prerequisites() {
    echo -e "${CYAN}━━━ 前置检查 ━━━${NC}"

    # Rust 工具链
    require_command cargo
    echo -e "${GREEN}✓ cargo $(cargo --version | cut -d' ' -f2)${NC}"

    # Rust 版本检查
    local rust_ver
    rust_ver=$(rustc --version | cut -d' ' -f2)
    echo -e "${GREEN}✓ rustc ${rust_ver}${NC}"

    # Node.js（前端构建必需）
    if should_build_frontend; then
        if command -v node &>/dev/null; then
            echo -e "${GREEN}✓ node $(node --version)${NC}"
        else
            echo -e "${RED}✗ Node.js 未安装（前端构建必需）${NC}"
            exit 1
        fi

        if command -v npm &>/dev/null; then
            echo -e "${GREEN}✓ npm $(npm --version)${NC}"
        else
            echo -e "${RED}✗ npm 未安装${NC}"
            exit 1
        fi

        # 检查前端依赖
        if [[ ! -d "$FRONTEND_DIR/node_modules" ]]; then
            echo -e "${YELLOW}⚠ 前端依赖未安装，正在安装...${NC}"
            cd "$FRONTEND_DIR"
            npm install --silent 2>&1
            cd "$PROJECT_ROOT"
            echo -e "${GREEN}✓ 前端依赖安装完成${NC}"
        fi
    fi

    # 检查系统库（Linux 下 Tauri 需要）
    if ! $FRONTEND_ONLY; then
        if [[ "$(uname -s)" == "Linux" ]]; then
            local missing_libs=()
            local pkg
            for pkg in webkit2gtk-4.1 gtk+-3.0 javascriptcoregtk-4.1; do
                if ! pkg-config --exists "$pkg"; then
                    missing_libs+=("$pkg")
                fi
            done
            if ! pkg-config --exists appindicator3-0.1 \
                && ! pkg-config --exists ayatana-appindicator3-0.1; then
                missing_libs+=("appindicator3-0.1")
            fi
            if [[ ${#missing_libs[@]} -gt 0 ]]; then
                echo -e "${YELLOW}⚠ 可能缺少的系统库: ${missing_libs[*]}${NC}"
                echo "  Arch: sudo pacman -S webkit2gtk-4.1 gtk3 libappindicator"
                echo "  Ubuntu: sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev"
            fi
        fi
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
    local mode_label
    local cargo_args=()
    local cargo_output
    local build_status=0

    while IFS= read -r arg; do
        cargo_args+=("$arg")
    done < <(cargo_target_args)

    cargo_output="$(mktemp)"

    if $BUILD_ALL; then
        echo -e "  范围: ${CYAN}整个 Rust workspace${NC}"
    else
        echo -e "  范围: ${CYAN}${TARGET_CRATE}${NC} package 及其依赖"
    fi
    echo ""

    case "$BUILD_MODE" in
        debug)
            mode_label="${BOLD}Debug${NC}"
            echo -e "${CYAN}━━━ Debug 构建 ━━━${NC}"
            echo ""
            cd "$PROJECT_ROOT"
            if run_cargo_capture "$cargo_output" cargo build "${cargo_args[@]}"; then
                build_status=0
            else
                build_status=$?
            fi
            ;;
        release)
            mode_label="${BOLD}Release (LTO + strip)${NC}"
            echo -e "${CYAN}━━━ Release 构建 ━━━${NC}"
            echo ""
            cd "$PROJECT_ROOT"
            if run_cargo_capture "$cargo_output" cargo build --release "${cargo_args[@]}"; then
                build_status=0
            else
                build_status=$?
            fi
            ;;
        check)
            mode_label="${BOLD}检查${NC}"
            echo -e "${CYAN}━━━ 编译检查 ━━━${NC}"
            echo ""
            cd "$PROJECT_ROOT"
            if run_cargo_capture "$cargo_output" cargo check "${cargo_args[@]}"; then
                build_status=0
            else
                build_status=$?
            fi
            ;;
    esac

    echo ""

    if [[ $build_status -eq 0 ]]; then
        echo -e "${GREEN}${BOLD}━━━ 后端构建成功 ━━━${NC}"

        # 显示产物位置
        if [[ "$BUILD_MODE" != "check" ]]; then
            local binary_path="$PROJECT_ROOT/target/$BUILD_MODE/$TARGET_CRATE"
            if [[ -f "$binary_path" ]]; then
                local size
                size=$(du -h "$binary_path" | cut -f1)
                echo -e "  产物: ${CYAN}${binary_path}${NC}"
                echo -e "  大小: ${CYAN}${size}${NC}"
            fi
        fi

        local total_warnings
        total_warnings=$(grep -c 'warning:' "$cargo_output" || true)
        rm -f "$cargo_output"
        if [[ "$total_warnings" -gt 0 ]]; then
            echo -e "  警告: ${YELLOW}${total_warnings} 条${NC}（使用 -w 查看详情）"
        fi

        echo ""
        exit 0
    else
        echo -e "${RED}${BOLD}━━━ 后端构建失败 ━━━${NC}"
        echo "  请检查上方错误信息"
        echo ""
        rm -f "$cargo_output"
        exit 1
    fi
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

if $BACKEND_ONLY; then
    do_build
fi

# 默认：按当前模式构建前端和 Rust 目标
if should_build_frontend; then
    build_frontend
fi
do_build

echo ""
echo -e "${GREEN}${BOLD}━━━ 全部构建完成 ━━━${NC}"
echo -e "  前端: ${FRONTEND_DIR}/dist/"
if [[ "$BUILD_MODE" != "check" ]]; then
    echo -e "  后端: ${PROJECT_ROOT}/target/${BUILD_MODE}/${TARGET_CRATE}"
fi
echo ""
