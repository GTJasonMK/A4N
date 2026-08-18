#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
cd "$PROJECT_ROOT"

RUNS=3
BASELINE="$PROJECT_ROOT/scripts/checks/build-feedback-baseline.tsv"
OUTPUT="$CARGO_TARGET_DIR/build-feedback/measurements.tsv"
ARTIFACT_DIR="$CARGO_TARGET_DIR/build-feedback"
BASELINE_OUTPUT=""
SKIP_WORKSPACE=0
SELECTED_PACKAGES=()

usage() {
    cat <<'EOF'
用法：measure-build-feedback.sh [选项]

  --runs N                 每个 warm package 的测量次数（不少于 3，默认 3）
  --output PATH            测量 TSV 输出路径
  --artifact-dir PATH      日志、原始样本、timings 和依赖树目录
  --write-baseline PATH    同时写入当前测量方法的 package baseline
  --skip-workspace         不测 workspace cold check（仅用于本地快速迭代）
  --package NAME           只测指定 package；可重复使用
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --runs)
            RUNS="${2:?--runs requires a positive integer}"
            shift 2
            ;;
        --output)
            OUTPUT="${2:?--output requires a path}"
            shift 2
            ;;
        --artifact-dir)
            ARTIFACT_DIR="${2:?--artifact-dir requires a path}"
            shift 2
            ;;
        --write-baseline)
            BASELINE_OUTPUT="${2:?--write-baseline requires a path}"
            shift 2
            ;;
        --skip-workspace)
            SKIP_WORKSPACE=1
            shift
            ;;
        --package)
            [[ "${2:-}" =~ ^[A-Za-z0-9_-]+$ ]] || {
                echo "--package requires a Cargo package name" >&2
                exit 2
            }
            SELECTED_PACKAGES+=("${2:?--package requires a package name}")
            shift 2
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            echo "未知参数: $1" >&2
            exit 2
            ;;
    esac
done

[[ "$RUNS" =~ ^[1-9][0-9]*$ && "$RUNS" -ge 3 ]] || {
    echo "--runs 必须是不少于 3 的整数" >&2
    exit 2
}
[[ -r "$BASELINE" ]] || {
    echo "缺少编译反馈基线: $BASELINE" >&2
    exit 1
}

METHOD="$(awk -F '\t' '$1 == "# method" { print $2 }' "$BASELINE")"
[[ -n "$METHOD" && "$METHOD" != *$'\n'* ]] || {
    echo "编译反馈基线必须包含唯一 method" >&2
    exit 1
}

BASELINE_PACKAGES=()
declare -A MAX_REGRESSION_PERCENT=()
while IFS=$'\t' read -r package _baseline_ms max_regression_percent extra; do
    [[ -z "$package" || "$package" == \#* ]] && continue
    [[ "$package" =~ ^[A-Za-z0-9_-]+$ && -n "$max_regression_percent" && -z "$extra" ]] || {
        echo "编译反馈基线行无效: $package" >&2
        exit 1
    }
    [[ -z "${MAX_REGRESSION_PERCENT[$package]+x}" ]] || {
        echo "编译反馈基线包含重复 package: $package" >&2
        exit 1
    }
    BASELINE_PACKAGES+=("$package")
    MAX_REGRESSION_PERCENT["$package"]="$max_regression_percent"
done < "$BASELINE"
[[ "${#BASELINE_PACKAGES[@]}" -gt 0 ]] || {
    echo "编译反馈基线没有 package" >&2
    exit 1
}

RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)-$$"
RUN_DIR="$ARTIFACT_DIR/runs/$RUN_ID"
LOG_DIR="$RUN_DIR/logs"
DEPENDENCY_DIR="$RUN_DIR/dependencies"
TIMINGS_DIR="$RUN_DIR/timings"
SAMPLES="$RUN_DIR/samples.tsv"
TEMP_ROOT="$ARTIFACT_DIR/tmp"
mkdir -p "$(dirname "$OUTPUT")" "$LOG_DIR" "$DEPENDENCY_DIR" "$TIMINGS_DIR" "$TEMP_ROOT"

COLD_TARGET_DIR="$(mktemp -d "$TEMP_ROOT/cold.XXXXXX")"
WARM_TARGET_DIR=""
cleanup_targets() {
    local directory
    for directory in "$COLD_TARGET_DIR" "$WARM_TARGET_DIR"; do
        if [[ -n "$directory" && "$directory" == "$TEMP_ROOT/"* ]]; then
            rm -rf -- "$directory"
        fi
    done
}
trap cleanup_targets EXIT
WARM_TARGET_DIR="$(mktemp -d "$TEMP_ROOT/warm.XXXXXX")"

: > "$OUTPUT"
printf '# method\t%s\n' "$METHOD" >> "$OUTPUT"
printf '# package\tmedian_ms\n' >> "$OUTPUT"
printf '# package\trun\tduration_ms\n' > "$SAMPLES"

now_ms() {
    python3 -c 'import time; print(time.monotonic_ns() // 1_000_000)'
}

run_timed() {
    local label="$1"
    shift
    local start end log
    log="$LOG_DIR/${label}.log"
    {
        printf 'command:'
        printf ' %q' "$@"
        printf '\n'
        printf 'cargo_target_dir: %s\n' "$CARGO_TARGET_DIR"
    } > "$log"
    start="$(now_ms)"
    if ! "$@" >>"$log" 2>&1; then
        cat "$log" >&2
        echo "编译反馈命令失败: $label" >&2
        return 1
    fi
    end="$(now_ms)"
    printf '%s\n' "$((end - start))"
}

capture_cargo_timings() {
    local label="$1"
    local target_dir="$2"
    local source="$target_dir/cargo-timings"
    local destination="$TIMINGS_DIR/$label"
    [[ -d "$source" ]] || {
        echo "Cargo timings missing after $label: $source" >&2
        return 1
    }
    mkdir -p "$destination"
    cp -R "$source/." "$destination/"
}

run_cargo_timed() {
    local label="$1"
    local target_dir="$2"
    shift 2
    local duration
    rm -rf -- "$target_dir/cargo-timings"
    duration="$(CARGO_TARGET_DIR="$target_dir" run_timed "$label" "$@")" || return 1
    capture_cargo_timings "$label" "$target_dir"
    printf '%s\n' "$duration"
}

reset_warm_package() {
    local label="$1"
    local package="$2"
    local log="$LOG_DIR/${label}-reset.log"
    printf 'command: cargo clean -p %q\n' "$package" > "$log"
    printf 'cargo_target_dir: %s\n' "$WARM_TARGET_DIR" >> "$log"
    CARGO_TARGET_DIR="$WARM_TARGET_DIR" cargo clean -p "$package" >>"$log" 2>&1 || {
        cat "$log" >&2
        echo "无法重置 warm package artifact: $package" >&2
        return 1
    }
}

{
    if command -v lscpu >/dev/null 2>&1; then
        lscpu
    else
        uname -a
        getconf _NPROCESSORS_ONLN 2>/dev/null || true
    fi
} > "$RUN_DIR/hardware.txt"

{
    echo "run_id: $RUN_ID"
    echo "git_commit: $(git rev-parse HEAD)"
    echo "git_dirty: $(git status --porcelain=v1 | wc -l | tr -d ' ') files"
    echo "rustc: $(rustc --version)"
    echo "cargo: $(cargo --version)"
    echo "runner: $(uname -a)"
    echo "runner_os: ${RUNNER_OS:-<unset>}"
    echo "runner_arch: ${RUNNER_ARCH:-<unset>}"
    echo "runner_name: ${RUNNER_NAME:-<unset>}"
    echo "github_run_id: ${GITHUB_RUN_ID:-<unset>}"
    echo "github_run_attempt: ${GITHUB_RUN_ATTEMPT:-<unset>}"
    echo "measurement_method: $METHOD"
    echo "warm_runs: $RUNS"
    echo "warmup_before_measured_runs: 1"
    echo "warm_sample_reset: cargo clean -p <package> in isolated warm target"
    echo "workspace_cold_measured: $((1 - SKIP_WORKSPACE))"
    echo "workspace_cold_target: isolated_fresh"
    echo "package_warm_target: isolated_fresh_shared_after_warmup"
    echo "shared_workspace_target_used: 0"
    echo "cargo_incremental: ${CARGO_INCREMENTAL:-<unset>}"
    echo "hardware: hardware.txt"
} | tee "$RUN_DIR/metadata.txt"

if (( ! SKIP_WORKSPACE )); then
    cold_ms="$(run_cargo_timed workspace-cold "$COLD_TARGET_DIR" cargo check --workspace --locked --timings)"
    printf '__workspace_cold__\t%s\n' "$cold_ms" >> "$OUTPUT"
    printf '__workspace_cold__\t1\t%s\n' "$cold_ms" >> "$SAMPLES"
    echo "workspace cold: ${cold_ms}ms"
fi

packages=("${BASELINE_PACKAGES[@]}")
if (( ${#SELECTED_PACKAGES[@]} > 0 )); then
    packages=("${SELECTED_PACKAGES[@]}")
fi

for package in "${packages[@]}"; do
    if [[ "$package" == "a4n-server" || "$package" == "a4n-tauri" ]]; then
        check_command=(cargo check -p "$package" --locked --no-default-features --timings)
    else
        check_command=(cargo check -p "$package" --locked --timings)
    fi
    warmup_ms="$(run_cargo_timed "${package//-/_}-warmup" "$WARM_TARGET_DIR" "${check_command[@]}")"
    printf '%s\twarmup\t%s\n' "$package" "$warmup_ms" >> "$SAMPLES"
    values=()
    for run in $(seq 1 "$RUNS"); do
        label="${package//-/_}-warm-${run}"
        reset_warm_package "$label" "$package"
        duration="$(run_cargo_timed "$label" "$WARM_TARGET_DIR" "${check_command[@]}")"
        values+=("$duration")
        printf '%s\t%s\t%s\n' "$package" "$run" "$duration" >> "$SAMPLES"
    done
    median="$(printf '%s\n' "${values[@]}" | sort -n | awk '
        { values[NR] = $1 }
        END {
            if (NR % 2) print values[(NR + 1) / 2]
            else print int((values[NR / 2] + values[NR / 2 + 1]) / 2)
        }
    ')"
    printf '%s\t%s\n' "$package" "$median" >> "$OUTPUT"
    case "$package" in
        a4n-server|a4n-tauri)
            cargo tree --locked -e normal,build --depth 1 -p "$package" --no-default-features >"$DEPENDENCY_DIR/$package.txt"
            ;;
        *)
            cargo tree --locked -e normal,build --depth 1 -p "$package" >"$DEPENDENCY_DIR/$package.txt"
            ;;
    esac
    echo "$package warm median: ${median}ms"
done

if [[ -n "$BASELINE_OUTPUT" ]]; then
    mkdir -p "$(dirname "$BASELINE_OUTPUT")"
    {
        printf '# method\t%s\n' "$METHOD"
        printf '# package\tbaseline_ms\tmax_regression_percent\n'
        while IFS=$'\t' read -r package median; do
            [[ -z "$package" || "$package" == \#* || "$package" == "__workspace_cold__" ]] && continue
            max_regression_percent="${MAX_REGRESSION_PERCENT[$package]-}"
            [[ -n "$max_regression_percent" ]] || {
                echo "编译反馈基线缺少 package 策略: $package" >&2
                exit 1
            }
            printf '%s\t%s\t%s\n' "$package" "$median" "$max_regression_percent"
        done < "$OUTPUT"
    } > "$BASELINE_OUTPUT"
    echo "baseline written: $BASELINE_OUTPUT"
fi

echo "measurements written: $OUTPUT"
echo "raw samples and timings written: $RUN_DIR"
