#!/usr/bin/env bash
#=============================================================================
# A4N 构建缓存守护
#
# 策略（避免「一到阈值就 cargo clean 毁掉依赖复用」）：
#   1. 始终只养一套常用缓存：CARGO_TARGET_DIR / 仓库根 target
#   2. 实验/旁路目录（target-timing、target-noble、target-*）用完即扔
#   3. 主缓存超软上限：先砍 incremental，再尝试 cargo-sweep 扫旧产物
#   4. 仍超硬上限：才 cargo clean（最后手段）
#
# 环境变量：
#   A4N_BUILD_CACHE_LIMIT_GIB       软上限 GiB，默认 30
#   A4N_BUILD_CACHE_HARD_LIMIT_GIB  硬上限 GiB，默认 max(软上限*2, 软上限+20)
#   A4N_BUILD_CACHE_SWEEP_DAYS      sweep/实验目录过期天数，默认 14
#   A4N_KEEP_EPHEMERAL_TARGETS=1    禁止自动删除 target-* 实验目录
#   A4N_BUILD_CACHE_SKIP=1          跳过本守护
#=============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# 主缓存：与 scripts/lib/common.sh 一致，强制单目录
PRIMARY_TARGET_DIR="${CARGO_TARGET_DIR:-$PROJECT_ROOT/target}"
# 若外部把 CARGO_TARGET_DIR 指到仓库外，仍只守护主路径；实验目录只扫仓库根
PRIMARY_TARGET_DIR="$(cd "$PRIMARY_TARGET_DIR" 2>/dev/null && pwd || echo "$PRIMARY_TARGET_DIR")"

LIMIT_GIB="${A4N_BUILD_CACHE_LIMIT_GIB:-30}"
SWEEP_DAYS="${A4N_BUILD_CACHE_SWEEP_DAYS:-14}"
LOCK_FILE="${XDG_RUNTIME_DIR:-/tmp}/a4n-cache-cleanup.lock"

if [[ "${A4N_BUILD_CACHE_SKIP:-0}" == "1" ]]; then
    exit 0
fi

# 进程检查与 flock 目前按 Linux 写的
if [[ "$(uname -s)" != "Linux" ]]; then
    exit 0
fi

if [[ ! "$LIMIT_GIB" =~ ^[1-9][0-9]*$ ]]; then
    echo "无效的 A4N_BUILD_CACHE_LIMIT_GIB：$LIMIT_GIB（必须是正整数）" >&2
    exit 2
fi
if [[ ! "$SWEEP_DAYS" =~ ^[1-9][0-9]*$ ]]; then
    echo "无效的 A4N_BUILD_CACHE_SWEEP_DAYS：$SWEEP_DAYS（必须是正整数）" >&2
    exit 2
fi

if [[ -n "${A4N_BUILD_CACHE_HARD_LIMIT_GIB:-}" ]]; then
    if [[ ! "$A4N_BUILD_CACHE_HARD_LIMIT_GIB" =~ ^[1-9][0-9]*$ ]]; then
        echo "无效的 A4N_BUILD_CACHE_HARD_LIMIT_GIB：$A4N_BUILD_CACHE_HARD_LIMIT_GIB" >&2
        exit 2
    fi
    HARD_GIB="$A4N_BUILD_CACHE_HARD_LIMIT_GIB"
else
    # 默认：软上限的 2 倍与「软上限+20」取大，避免 30→立刻 hard clean
    local_double=$((LIMIT_GIB * 2))
    local_plus=$((LIMIT_GIB + 20))
    if (( local_double > local_plus )); then
        HARD_GIB=$local_double
    else
        HARD_GIB=$local_plus
    fi
fi
if (( HARD_GIB < LIMIT_GIB )); then
    HARD_GIB=$LIMIT_GIB
fi

LIMIT_BYTES=$((LIMIT_GIB * 1024 * 1024 * 1024))
HARD_BYTES=$((HARD_GIB * 1024 * 1024 * 1024))

bytes_of() {
    local path=$1
    if [[ ! -e "$path" ]]; then
        echo 0
        return
    fi
    du -s -B1 "$path" 2>/dev/null | awk '{print $1}'
}

human_of() {
    numfmt --to=iec-i --suffix=B "${1:-0}" 2>/dev/null || echo "${1}B"
}

a4n_build_is_running() {
    local proc cwd cmdline

    for proc in /proc/[0-9]*; do
        [[ -r "$proc/cmdline" ]] || continue
        cmdline="$(tr '\0' ' ' < "$proc/cmdline" 2>/dev/null || true)"
        case "$cmdline" in
            *cargo*|*rustc*) ;;
            *) continue ;;
        esac

        cwd="$(readlink "$proc/cwd" 2>/dev/null || true)"
        if [[ "$cwd" == "$PROJECT_ROOT" || "$cwd" == "$PROJECT_ROOT/"* ||
            "$cmdline" == *"$PRIMARY_TARGET_DIR"* ||
            "$cmdline" == *"$PROJECT_ROOT/target"* ]]; then
            return 0
        fi
        # 实验 target-dir 上的编译也算在忙
        if [[ "$cmdline" == *"$PROJECT_ROOT/target-"* ]]; then
            return 0
        fi
    done

    return 1
}

# 仓库根下 target-*（不含主 target）：实验/旁路缓存
list_ephemeral_target_dirs() {
    local d
    shopt -s nullglob
    for d in "$PROJECT_ROOT"/target-*; do
        [[ -d "$d" ]] || continue
        # 绝不碰主缓存目录本身
        if [[ "$(cd "$d" && pwd)" == "$PRIMARY_TARGET_DIR" ]]; then
            continue
        fi
        printf '%s\n' "$d"
    done
    shopt -u nullglob
}

dir_mtime_days_ago() {
    local path=$1
    local mtime now
    mtime="$(stat -c %Y "$path" 2>/dev/null || echo 0)"
    now="$(date +%s)"
    echo $(((now - mtime) / 86400))
}

drop_ephemeral_targets() {
    local mode=$1 # stale | all
    local d age removed=0 size

    if [[ "${A4N_KEEP_EPHEMERAL_TARGETS:-0}" == "1" ]]; then
        return 0
    fi

    while IFS= read -r d; do
        [[ -n "$d" && -d "$d" ]] || continue
        if [[ "$mode" == "stale" ]]; then
            age="$(dir_mtime_days_ago "$d")"
            if (( age < SWEEP_DAYS )); then
                continue
            fi
        fi
        size="$(bytes_of "$d")"
        echo "  → 删除实验/旁路 target 目录: $d ($(human_of "$size")) [$mode]"
        rm -rf -- "$d"
        removed=1
    done < <(list_ephemeral_target_dirs)

    return 0
}

drop_incremental() {
    local base=$1
    local d removed=0 size

    for d in "$base/debug/incremental" "$base/release/incremental" \
        "$base"/*/incremental; do
        if [[ -d "$d" ]]; then
            size="$(bytes_of "$d")"
            echo "  → 删除 incremental: $d ($(human_of "$size"))"
            rm -rf -- "$d"
            removed=1
        fi
    done
    # 兼容无 nullglob 时字面量路径
    return 0
}

try_cargo_sweep() {
    local base=$1
    if ! command -v cargo-sweep >/dev/null 2>&1 && ! cargo sweep -h >/dev/null 2>&1; then
        echo "  → 未安装 cargo-sweep，跳过扫旧（可选: cargo install cargo-sweep）"
        return 0
    fi
    echo "  → cargo sweep --time ${SWEEP_DAYS}（保留 ${SWEEP_DAYS} 天内用过的产物）"
    # cargo-sweep 子命令: cargo sweep
    if cargo sweep --help >/dev/null 2>&1; then
        (
            cd "$PROJECT_ROOT"
            CARGO_TARGET_DIR="$base" cargo sweep --time "$SWEEP_DAYS" 2>/dev/null \
                || cargo sweep --time "$SWEEP_DAYS" -r 2>/dev/null \
                || true
        )
    fi
}

primary_size_bytes() {
    bytes_of "$PRIMARY_TARGET_DIR"
}

# --- 主流程 ---
exec 9>"$LOCK_FILE"
if ! flock -n 9; then
    echo "A4N 构建缓存检查正在运行，本次跳过。"
    exit 0
fi

if a4n_build_is_running; then
    # 编译进行中：只做只读报告，绝不删
    if [[ -d "$PRIMARY_TARGET_DIR" ]]; then
        SIZE_BYTES="$(primary_size_bytes)"
        if (( SIZE_BYTES > LIMIT_BYTES )); then
            echo "A4N 主缓存 $(human_of "$SIZE_BYTES") 已超软上限 ${LIMIT_GIB}GiB，但存在编译进程，本次跳过清理。"
        fi
    fi
    exit 0
fi

# 1) 实验目录：过期则扔（无论是否超限）
drop_ephemeral_targets stale

SIZE_BYTES="$(primary_size_bytes)"
if (( SIZE_BYTES <= LIMIT_BYTES )); then
    exit 0
fi

echo "A4N 主缓存 $(human_of "$SIZE_BYTES") 超过软上限 ${LIMIT_GIB}GiB，开始分级清理（保留依赖复用）。"

# 2) 超软限：实验目录全部扔掉
drop_ephemeral_targets all
SIZE_BYTES="$(primary_size_bytes)"
echo "  当前主缓存: $(human_of "$SIZE_BYTES")"
if (( SIZE_BYTES <= LIMIT_BYTES )); then
    echo "  ✓ 已回到软上限内（仅清理了旁路 target-*）。"
    exit 0
fi

# 3) 砍 incremental（保留 .rlib，第三方依赖多数仍 Fresh）
if [[ -d "$PRIMARY_TARGET_DIR" ]]; then
    drop_incremental "$PRIMARY_TARGET_DIR"
fi
SIZE_BYTES="$(primary_size_bytes)"
echo "  当前主缓存: $(human_of "$SIZE_BYTES")"
if (( SIZE_BYTES <= LIMIT_BYTES )); then
    echo "  ✓ 已回到软上限内（已砍 incremental）。"
    exit 0
fi

# 4) cargo-sweep 扫旧 fingerprint
try_cargo_sweep "$PRIMARY_TARGET_DIR"
SIZE_BYTES="$(primary_size_bytes)"
echo "  当前主缓存: $(human_of "$SIZE_BYTES")"
if (( SIZE_BYTES <= LIMIT_BYTES )); then
    echo "  ✓ 已回到软上限内（cargo sweep）。"
    exit 0
fi

# 5) 仍超硬上限才 cargo clean
if (( SIZE_BYTES <= HARD_BYTES )); then
    echo "  ⚠ 仍高于软上限 ${LIMIT_GIB}GiB，但未到硬上限 ${HARD_GIB}GiB，保留主缓存以免冷编译。"
    echo "  提示: 可手动 cargo sweep、或提高 A4N_BUILD_CACHE_LIMIT_GIB、或确认后设 A4N_BUILD_CACHE_FORCE_CLEAN=1。"
    if [[ "${A4N_BUILD_CACHE_FORCE_CLEAN:-0}" != "1" ]]; then
        exit 0
    fi
fi

echo "  → 达到硬上限 ${HARD_GIB}GiB（或 FORCE_CLEAN=1），执行 cargo clean（最后手段）。"
cargo clean --manifest-path "$PROJECT_ROOT/Cargo.toml" --target-dir "$PRIMARY_TARGET_DIR"
echo "  ✓ cargo clean 完成。下次构建将冷编译依赖。"
exit 0
