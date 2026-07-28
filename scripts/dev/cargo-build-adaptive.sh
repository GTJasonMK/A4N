#!/usr/bin/env bash
#=============================================================================
# 自适应并发 cargo build：高并发起步，OOM/SIGKILL 后降并发并续编（复用 target）
#
# 用法:
#   ./scripts/dev/cargo-build-adaptive.sh --release --workspace
#   ./scripts/dev/cargo-build-adaptive.sh --release -p a4n-server
#   ./scripts/dev/cargo-build-adaptive.sh -- cargo test -p a4n-db --lib
#
# 环境变量:
#   A4N_CARGO_JOBS_START   起始 -j（默认: min(nproc, 12)；release 默认 min(nproc, 10)）
#   A4N_CARGO_JOBS_MIN     最低 -j（默认 1）
#   A4N_CARGO_JOBS_FACTOR  失败后 jobs = max(min, jobs / factor)（默认 2）
#   A4N_CARGO_ADAPTIVE_LOG 日志路径（默认 .claude/compile-timings/adaptive-cargo.log）
#=============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"

NPROC="$(nproc 2>/dev/null || echo 4)"
MIN_JOBS="${A4N_CARGO_JOBS_MIN:-1}"
FACTOR="${A4N_CARGO_JOBS_FACTOR:-2}"
if [[ ! "$MIN_JOBS" =~ ^[1-9][0-9]*$ ]] || [[ ! "$FACTOR" =~ ^[1-9][0-9]*$ ]]; then
    die "A4N_CARGO_JOBS_MIN / A4N_CARGO_JOBS_FACTOR 必须是正整数"
fi

LOG_PATH="${A4N_CARGO_ADAPTIVE_LOG:-$PROJECT_ROOT/.claude/compile-timings/adaptive-cargo.log}"
mkdir -p "$(dirname "$LOG_PATH")"

# 解析：默认走 cargo build，-- 之后为完整命令
CARGO_SUBCMD=(build)
PASSTHRU=()
EXPLICIT_CMD=0
WANT_RELEASE=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --)
            shift
            EXPLICIT_CMD=1
            PASSTHRU=("$@")
            break
            ;;
        --release|-r)
            WANT_RELEASE=1
            PASSTHRU+=(--release)
            shift
            ;;
        --workspace|--all)
            PASSTHRU+=(--workspace)
            shift
            ;;
        --locked|--offline|--frozen)
            PASSTHRU+=("$1")
            shift
            ;;
        -p|--package)
            PASSTHRU+=("$1" "$2")
            shift 2
            ;;
        -j|--jobs)
            # 用户强制起始并发
            A4N_CARGO_JOBS_START="$2"
            shift 2
            ;;
        -h|--help)
            sed -n '2,20p' "$0" | sed 's/^# \?//'
            exit 0
            ;;
        *)
            PASSTHRU+=("$1")
            shift
            ;;
    esac
done

if [[ -n "${A4N_CARGO_JOBS_START:-}" ]]; then
    if [[ ! "$A4N_CARGO_JOBS_START" =~ ^[1-9][0-9]*$ ]]; then
        die "A4N_CARGO_JOBS_START 必须是正整数"
    fi
    JOBS="$A4N_CARGO_JOBS_START"
else
    if (( WANT_RELEASE )); then
        CAP=10
    else
        CAP=12
    fi
    if (( NPROC < CAP )); then
        JOBS=$NPROC
    else
        JOBS=$CAP
    fi
fi
if (( JOBS < MIN_JOBS )); then
    JOBS=$MIN_JOBS
fi

# ONNX：整仓/含 embedding 时需要
if [[ -x "$PROJECT_ROOT/scripts/dev/setup-onnxruntime.sh" ]] || true; then
    if ! configure_onnxruntime 2>/dev/null; then
        log_warn "未配置 ONNX Runtime；若编 embedding/rag 相关可能链接失败"
    else
        log_ok "ONNX: $ORT_LIB_PATH"
    fi
fi

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PROJECT_ROOT/target}"
# release 大包增量收益小且占盘；允许用户覆盖
export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-0}"

is_oom_failure() {
    local log_file=$1
    # rustc SIGKILL、系统 OOM、链接器被杀
    if rg -q 'signal: 9|SIGKILL|Killed|Cannot allocate memory|std::bad_alloc|LLVM ERROR: out of memory|failed to spawn|os error 12' "$log_file" 2>/dev/null; then
        return 0
    fi
    return 1
}

run_once() {
    local jobs=$1
    local attempt_log
    attempt_log="$(mktemp)"
    local -a cmd

    if (( EXPLICIT_CMD )); then
        # 用户完整命令：在末尾追加/覆盖 -j
        cmd=("${PASSTHRU[@]}")
        # 去掉已有 -j N
        local -a filtered=()
        local i=0
        while (( i < ${#cmd[@]} )); do
            if [[ "${cmd[i]}" == "-j" || "${cmd[i]}" == "--jobs" ]]; then
                i=$((i + 2))
                continue
            fi
            if [[ "${cmd[i]}" == -j* && "${cmd[i]}" != "-j" ]]; then
                i=$((i + 1))
                continue
            fi
            filtered+=("${cmd[i]}")
            i=$((i + 1))
        done
        cmd=("${filtered[@]}" -j "$jobs")
    else
        cmd=(cargo "${CARGO_SUBCMD[@]}" --locked -j "$jobs" "${PASSTHRU[@]}")
    fi

    {
        echo "---- $(date -Iseconds) attempt jobs=$jobs ----"
        printf 'cmd:'
        printf ' %q' "${cmd[@]}"
        printf '\n'
        free -h | head -2 || true
    } | tee -a "$LOG_PATH" >&2

    set +e
    "${cmd[@]}" 2>&1 | tee -a "$LOG_PATH" | tee "$attempt_log"
    local ec=${PIPESTATUS[0]}
    set -e

    if (( ec == 0 )); then
        rm -f "$attempt_log"
        return 0
    fi

    if is_oom_failure "$attempt_log" || is_oom_failure "$LOG_PATH"; then
        rm -f "$attempt_log"
        return 137
    fi

    # 非 OOM：原样失败
    rm -f "$attempt_log"
    return "$ec"
}

log_section "自适应 cargo 构建"
echo -e "  目标目录: ${CYAN}$CARGO_TARGET_DIR${NC}"
echo -e "  起始并发: ${CYAN}-j $JOBS${NC}（机器 nproc=$NPROC，下限 $MIN_JOBS）"
echo -e "  日志: ${CYAN}$LOG_PATH${NC}"
echo -e "  策略: OOM/SIGKILL → jobs=max(min, jobs/$FACTOR) 并续编（产物可复用）"
echo ""

{
    echo "ADAPTIVE_START $(date -Iseconds) start_jobs=$JOBS min=$MIN_JOBS"
} >>"$LOG_PATH"

while true; do
    log_info "尝试: -j $JOBS"
    set +e
    run_once "$JOBS"
    ec=$?
    set -e

    if (( ec == 0 )); then
        log_ok "构建成功（最终 -j $JOBS）"
        echo "ADAPTIVE_OK $(date -Iseconds) final_jobs=$JOBS" >>"$LOG_PATH"
        exit 0
    fi

    if (( ec == 137 )); then
        if (( JOBS <= MIN_JOBS )); then
            echo "已是最低并发 -j $MIN_JOBS 仍 OOM/SIGKILL，放弃。" | tee -a "$LOG_PATH" >&2
            echo "ADAPTIVE_FAIL $(date -Iseconds) oom_at_min_jobs=$JOBS" >>"$LOG_PATH"
            exit 137
        fi
        next=$((JOBS / FACTOR))
        if (( next < MIN_JOBS )); then
            next=$MIN_JOBS
        fi
        if (( next >= JOBS )); then
            next=$((JOBS - 1))
        fi
        if (( next < MIN_JOBS )); then
            next=$MIN_JOBS
        fi
        log_warn "检测到 OOM/SIGKILL，将并发 $JOBS → $next 并续编（已成功单元会复用）"
        echo "ADAPTIVE_DOWN $(date -Iseconds) $JOBS -> $next" >>"$LOG_PATH"
        JOBS=$next
        continue
    fi

    echo "构建失败（非 OOM），exit=$ec" | tee -a "$LOG_PATH" >&2
    echo "ADAPTIVE_FAIL $(date -Iseconds) exit=$ec jobs=$JOBS" >>"$LOG_PATH"
    exit "$ec"
done
