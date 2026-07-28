#!/usr/bin/env bash
#=============================================================================
# A4N 项目测试脚本
#
# 用法:
#   ./scripts/test.sh                 # 运行全部测试
#   ./scripts/test.sh --unit          # 仅单元测试
#   ./scripts/test.sh --integration   # 仅集成测试
#   ./scripts/test.sh --crate <NAME>  # 指定 crate（如 a4n-core）
#   ./scripts/test.sh --nocapture     # 显示测试输出（不截获 stdout）
#   ./scripts/test.sh --verbose       # 详细输出
#   ./scripts/test.sh --release       # Release 模式测试
#   ./scripts/test.sh --help          # 显示帮助
#
# 跨平台：Linux / macOS / Windows(Git Bash / WSL)
#=============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"
load_dotenv

# —— 配置 ——
TEST_MODE="all"           # all | unit | integration
TARGET_CRATE=""           # 空 = 全部 crate
CARGO_RELEASE=false
NOCAPTURE=false
VERBOSE=false
SHOW_OUTPUT=false         # 默认简要，--verbose 时显示详情

# —— 解析参数 ——
while [[ $# -gt 0 ]]; do
    case "$1" in
        --unit)
            TEST_MODE="unit"
            shift
            ;;
        --integration|--it)
            TEST_MODE="integration"
            shift
            ;;
        --crate)
            [[ $# -ge 2 && -n "${2:-}" ]] || die "--crate 需要提供 crate 名称"
            TARGET_CRATE="$2"
            shift 2
            ;;
        --nocapture)
            NOCAPTURE=true
            shift
            ;;
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        --release)
            CARGO_RELEASE=true
            shift
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

# —— 前置检查 ——
check_prerequisites() {
    require_command cargo
    if [[ -z "$TARGET_CRATE" || "$TARGET_CRATE" =~ ^a4n-(embedding-fastembed|embedding-service|runtime|server|tauri)$ ]]; then
        require_onnxruntime
    fi
}

# —— 获取所有 crate 列表 ——
get_all_crates() {
    cd "$PROJECT_ROOT"
    cargo metadata --no-deps --format-version 1 2>/dev/null \
        | grep '"name"' | head -20 | sed 's/.*"name":"\([^"]*\)".*/\1/'
}

# —— 运行测试 ——
run_tests() {
    local crate_label
    if [[ -n "$TARGET_CRATE" ]]; then
        crate_label="$TARGET_CRATE"
    else
        crate_label="所有 crate"
    fi

    echo -e "${BOLD}${CYAN}╔══════════════════════════════════╗${NC}"
    echo -e "${BOLD}${CYAN}║     A4N 项目测试脚本            ║${NC}"
    echo -e "${BOLD}${CYAN}╚══════════════════════════════════╝${NC}"
    echo ""
    echo -e "  目标: ${CYAN}${crate_label}${NC}"
    echo -e "  模式: ${CYAN}${TEST_MODE}${NC}"
    echo ""

    check_prerequisites
    guard_build_cache

    # 构建测试标志
    local test_flags=""
    if $NOCAPTURE; then
        test_flags="$test_flags --nocapture"
    fi

    # cargo test 参数
    local cargo_args=("test")
    if $CARGO_RELEASE; then
        cargo_args+=("--release")
    fi

    # 指定 crate
    if [[ -n "$TARGET_CRATE" ]]; then
        cargo_args+=("-p" "$TARGET_CRATE")
    fi

    # 测试过滤
    case "$TEST_MODE" in
        unit)
            # 单元测试通常在 crate 内部
            ;;
        integration)
            # 集成测试通常在 tests/ 目录
            cargo_args+=("--test" "*")
            ;;
        all)
            # 运行全部
            ;;
    esac

    if $VERBOSE; then
        cargo_args+=("-v")
    fi

    echo -e "${CYAN}━━━ 正在运行测试 ━━━${NC}"
    echo ""

    cd "$PROJECT_ROOT"

    # 运行测试并收集结果
    local test_output=""
    local test_exit_code

    if $VERBOSE; then
        # 详细模式：直接显示全部输出
        cargo "${cargo_args[@]}" 2>&1
        test_exit_code=$?
    else
        # 简要模式：只显示摘要
        if test_output=$(cargo "${cargo_args[@]}" 2>&1); then
            test_exit_code=0
        else
            test_exit_code=$?
        fi

        # 提取测试结果摘要
        echo "$test_output" | grep -E '^running|^test result|^test.*(ok|FAILED|ignored)' || true
    fi

    echo ""

    # 统计结果
    local total_passed=0
    local total_failed=0
    local total_ignored=0
    local total_measured=0
    local total_runtime=0

    while IFS= read -r line; do
        if [[ "$line" =~ test\ result:\ ok\.\ ([0-9]+)\ passed\;\ ([0-9]+)\ failed\;\ ([0-9]+)\ ignored\;\ ([0-9]+)\ measured ]]; then
            total_passed=$((total_passed + BASH_REMATCH[1]))
            total_failed=$((total_failed + BASH_REMATCH[2]))
            total_ignored=$((total_ignored + BASH_REMATCH[3]))
            total_measured=$((total_measured + BASH_REMATCH[4]))
        elif [[ "$line" =~ test\ result:\ ok\.\ ([0-9]+)\ passed\;\ ([0-9]+)\ failed ]]; then
            total_passed=$((total_passed + BASH_REMATCH[1]))
            total_failed=$((total_failed + BASH_REMATCH[2]))
        fi
        if [[ "$line" =~ finished\ in\ ([0-9.]+)s ]]; then
            total_runtime=$(echo "$total_runtime + ${BASH_REMATCH[1]}" | bc 2>/dev/null || echo 0)
        fi
    done <<< "$test_output"

    # 结果判断
    echo -e "${BOLD}━━━ 测试结果 ━━━${NC}"
    if [[ $test_exit_code -eq 0 ]]; then
        echo -e "${GREEN}${BOLD}✓ 全部通过${NC}"
    else
        echo -e "${RED}${BOLD}✗ 有测试失败${NC}"
    fi

    # 汇总
    if $VERBOSE || ! $SHOW_OUTPUT; then
        local test_lines
        test_lines=$(echo "$test_output" | grep -c '^test ' || true)
        echo -e "  通过:   ${GREEN}${total_passed}${NC}"
        echo -e "  失败:   ${RED}${total_failed}${NC}"
        echo -e "  忽略:   ${YELLOW}${total_ignored}${NC}"
        if [[ $total_measured -gt 0 ]]; then
            echo -e "  基准:   ${CYAN}${total_measured}${NC}"
        fi
    fi
    echo ""

    exit $test_exit_code
}

# —— 主流程 ——
run_tests
