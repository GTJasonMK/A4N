#!/usr/bin/env bash
# A4N Rust 测试入口
# 用法: ./scripts/test.sh [--crate NAME] [--nocapture] [--release]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"
load_dotenv

target_crate=""
cargo_args=(test)
test_args=()

while (($# > 0)); do
    case "$1" in
        --crate)
            [[ $# -ge 2 && -n "${2:-}" ]] || die "--crate 需要提供 crate 名称"
            target_crate="$2"
            shift 2
            ;;
        --nocapture)
            test_args+=(--nocapture)
            shift
            ;;
        --release)
            cargo_args+=(--release)
            shift
            ;;
        --help|-h)
            print_script_usage "$0" 3
            exit 0
            ;;
        *) die "未知参数: $1；使用 --help 查看用法" ;;
    esac
done

require_command cargo
if [[ -z "$target_crate" || "$target_crate" =~ ^a4n-(embedding-fastembed|embedding-service|runtime|server|tauri)$ ]]; then
    require_onnxruntime
fi
guard_build_cache

[[ -z "$target_crate" ]] || cargo_args+=(-p "$target_crate")
cd "$PROJECT_ROOT"
if ((${#test_args[@]} > 0)); then
    exec cargo "${cargo_args[@]}" -- "${test_args[@]}"
fi
exec cargo "${cargo_args[@]}"
