#!/usr/bin/env bash
set -euo pipefail

# Measurements must retain their own warm cache between baseline and edited builds.
export CARGO_ARTIFACTS_DISABLED=1

# 01 的真实编辑口径：固定干净 commit，在同一 worktree/target 先构建基线，
# 再应用真实 patch，观察 Host 与 Worker 的编译/链接失效面。

REPORT_DIR=""
SOURCE_DIR=""
HOST_PACKAGE="a4n-tauri"
WORKER_PACKAGE="a4n-export-service"
HOST_PATCH=""
WORKER_PATCH=""
HOST_BINARY=""
READY_URL=""
REQUEST_URL=""
REQUEST_BODY=""
STARTUP_TIMEOUT=30
REQUEST_TIMEOUT=60
HOST_STOP_GRACE_SECONDS=5
RUNS=1
KEEP_TARGET=0
SELF_TEST=0
EXPORT_WORKER_RUNTIME_E2E=0
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/build-cache.sh
source "$SCRIPT_DIR/../lib/build-cache.sh"
EXPORT_WORKER_RUNTIME_SCRIPT="$SCRIPT_DIR/../tests/run_export_worker_runtime_e2e.py"
PROTECTED_PIDS=(2377237 3425752 3424452)

usage() {
    cat <<'EOF'
用法：measure-runtime-feedback.sh --source-snapshot PATH --report-dir PATH --host-patch PATH --worker-patch PATH [选项]

  --source-snapshot PATH  必须是干净 git worktree，用于固定源码/commit
  --report-dir PATH       必须是专用空目录；原始样本、timings、日志和 metrics 输出目录
  --host-patch PATH       Host-only 真实 patch
  --worker-patch PATH     候选 Worker 真实 patch
  --host-package NAME     Host Cargo package（默认 a4n-tauri）
  --worker-package NAME   Worker Cargo package（默认 a4n-export-service）
  --host-binary PATH      可选：启动该二进制测 readiness/request/RSS
  --ready-url URL         可选：Host readiness URL
  --request-url URL       可选：就绪后调用一次的 URL
  --request-body PATH     可选：请求 body 文件；必须与 --request-url 一起提供
  --startup-timeout SEC   readiness 超时（默认 30）
  --request-timeout SEC   固定请求超时（默认 60）
  --runs N                重复完整测量；默认 1，N 只能是 1 或不少于 3
                          N>1 时每个样本使用独立 report/target，写入
                          samples.tsv（原始值）和 metrics.tsv（中位数/范围）
  --keep-target           保留临时构建缓存供排错；默认每个样本结束即回收，只保留报告
  --export-worker-runtime-e2e
                          编译测量完成后运行真实 Host+export Worker 验收；
                          读取 A4N_RUNTIME_E2E_DATABASE_URL 或 A4N_TEST_DATABASE_URL，
                          密码只从 A4N_RUNTIME_E2E_PASSWORD 读取
  --self-test             运行本脚本及 export Worker runner 的参数/状态码自测
EOF
}

die() {
    echo "runtime feedback: $*" >&2
    exit 1
}

require_command() {
    command -v "$1" >/dev/null 2>&1 || die "缺少命令: $1"
}

is_positive_integer() {
    local value="$*"
    [[ "$value" =~ ^[1-9][0-9]*$ ]]
}

is_valid_run_count() {
    local value="$*"
    is_positive_integer "$value" && (( 10#$value == 1 || 10#$value >= 3 ))
}

is_http_2xx() {
    local value="$*"
    [[ "$value" =~ ^2[0-9][0-9]$ ]]
}

is_successful_http_response() {
    local curl_status="$1" http_code="$2"
    [[ "$curl_status" == "0" ]] && is_http_2xx "$http_code"
}

write_matching_lines() {
    local pattern="$1" input="$2" output="$3" status
    if rg -- "$pattern" "$input" > "$output"; then
        return 0
    else
        status=$?
    fi
    if (( status == 1 )); then
        : > "$output"
        return 0
    fi
    die "无法读取或解析 build log: $input"
}

self_test() {
    local value
    for value in 1 30 999; do
        is_positive_integer "$value" || die "self-test: 正整数判定失败: $value"
    done
    for value in "" 0 01 1.0 abc; do
        if is_positive_integer "$value"; then
            if [[ -n "$value" ]]; then
                die "self-test: 非正整数被接受: $value"
            fi
            die "self-test: 空值被接受"
        fi
    done
    for value in 200 299; do
        is_http_2xx "$value" || die "self-test: 2xx 判定失败: $value"
    done
    for value in 000 199 300 404; do
        if is_http_2xx "$value"; then
            die "self-test: 非 2xx 被接受: $value"
        fi
    done
    for value in 200 299; do
        is_successful_http_response 0 "$value" || die "self-test: 成功 HTTP 响应判定失败: $value"
    done
    if is_successful_http_response 7 200 || is_successful_http_response 0 500 || \
       is_successful_http_response 0 ""; then
        die "self-test: 失败 HTTP 响应被接受"
    fi
    for value in 1 3 999; do
        is_valid_run_count "$value" || die "self-test: 有效样本数判定失败: $value"
    done
    for value in 0 2 02 abc; do
        if is_valid_run_count "$value"; then
            die "self-test: 无效样本数被接受: $value"
        fi
    done
    command -v rg >/dev/null 2>&1 || die "self-test: 缺少 rg"
    write_matching_lines '^#!/' "${BASH_SOURCE[0]}" /dev/null
    write_matching_lines '^definitely-not-a-build-target$' "${BASH_SOURCE[0]}" /dev/null
    [[ -f "$EXPORT_WORKER_RUNTIME_SCRIPT" ]] || die "缺少 export Worker runtime 验收器: $EXPORT_WORKER_RUNTIME_SCRIPT"
    python3 "$EXPORT_WORKER_RUNTIME_SCRIPT" --self-test
    printf 'runtime feedback self-test passed\n'
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --source-snapshot) SOURCE_DIR="${2:?--source-snapshot requires a path}"; shift 2 ;;
        --report-dir) REPORT_DIR="${2:?--report-dir requires a path}"; shift 2 ;;
        --host-patch) HOST_PATCH="${2:?--host-patch requires a path}"; shift 2 ;;
        --worker-patch) WORKER_PATCH="${2:?--worker-patch requires a path}"; shift 2 ;;
        --host-package) HOST_PACKAGE="${2:?--host-package requires a package}"; shift 2 ;;
        --worker-package) WORKER_PACKAGE="${2:?--worker-package requires a package}"; shift 2 ;;
        --host-binary) HOST_BINARY="${2:?--host-binary requires a path}"; shift 2 ;;
        --ready-url) READY_URL="${2:?--ready-url requires a URL}"; shift 2 ;;
        --request-url) REQUEST_URL="${2:?--request-url requires a URL}"; shift 2 ;;
        --request-body) REQUEST_BODY="${2:?--request-body requires a path}"; shift 2 ;;
        --startup-timeout) STARTUP_TIMEOUT="${2:?--startup-timeout requires seconds}"; shift 2 ;;
        --request-timeout) REQUEST_TIMEOUT="${2:?--request-timeout requires seconds}"; shift 2 ;;
        --runs) RUNS="${2:?--runs requires a sample count}"; shift 2 ;;
        --keep-target) KEEP_TARGET=1; shift ;;
        --export-worker-runtime-e2e) EXPORT_WORKER_RUNTIME_E2E=1; shift ;;
        --self-test) SELF_TEST=1; shift ;;
        --help|-h) usage; exit 0 ;;
        *) die "未知参数: $1" ;;
    esac
done

is_valid_run_count "$RUNS" || die "--runs 必须为 1 或不少于 3 的正整数"

if (( SELF_TEST )); then
    self_test
    exit 0
fi

[[ -n "$SOURCE_DIR" && -n "$REPORT_DIR" && -n "$HOST_PATCH" && -n "$WORKER_PATCH" ]] || {
    usage >&2
    exit 2
}
[[ -d "$SOURCE_DIR" && -f "$SOURCE_DIR/Cargo.toml" ]] || die "源码快照不是 Cargo workspace: $SOURCE_DIR"
[[ -f "$HOST_PATCH" && -f "$WORKER_PATCH" ]] || die "patch 文件不存在"
is_positive_integer "$STARTUP_TIMEOUT" || die "--startup-timeout 必须是正整数"
is_positive_integer "$REQUEST_TIMEOUT" || die "--request-timeout 必须是正整数"
[[ -z "$REQUEST_BODY" || -n "$REQUEST_URL" ]] || die "--request-body 必须与 --request-url 一起使用"
[[ -z "$HOST_BINARY" || -n "$READY_URL" ]] || die "--host-binary 必须与 --ready-url 一起使用"
[[ -z "$READY_URL" || -n "$HOST_BINARY" ]] || die "--ready-url 必须与 --host-binary 一起使用"
[[ -z "$REQUEST_URL" || -n "$HOST_BINARY" ]] || die "--request-url 必须与 --host-binary 一起使用"

for command in cargo rustc git python3 realpath sha256sum awk rg cp mkdir find mktemp uname sort; do
    require_command "$command"
done
CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-1}"
[[ "$CARGO_INCREMENTAL" == "1" ]] || die "真实编辑测量要求 CARGO_INCREMENTAL=1"
export CARGO_INCREMENTAL

SOURCE_DIR="$(realpath "$SOURCE_DIR")"
HOST_PATCH="$(realpath "$HOST_PATCH")"
WORKER_PATCH="$(realpath "$WORKER_PATCH")"
if [[ -n "$HOST_BINARY" ]]; then
    [[ -f "$HOST_BINARY" && -x "$HOST_BINARY" ]] || die "Host binary 不可执行文件: $HOST_BINARY"
    HOST_BINARY="$(realpath "$HOST_BINARY")"
fi
if [[ -n "$REQUEST_BODY" ]]; then
    [[ -f "$REQUEST_BODY" ]] || die "请求 body 文件不存在: $REQUEST_BODY"
    REQUEST_BODY="$(realpath "$REQUEST_BODY")"
fi

SOURCE_ROOT="$(git -C "$SOURCE_DIR" rev-parse --show-toplevel 2>/dev/null)" || die "源码快照不是 git worktree"
SOURCE_ROOT="$(realpath "$SOURCE_ROOT")"
[[ "$SOURCE_ROOT" == "$SOURCE_DIR" ]] || die "--source-snapshot 必须指向 worktree 根目录"
git -C "$SOURCE_ROOT" diff --quiet || die "源码快照存在未提交修改"
git -C "$SOURCE_ROOT" diff --cached --quiet || die "源码快照存在暂存修改"
[[ -z "$(git -C "$SOURCE_ROOT" status --porcelain=v1 --untracked-files=all)" ]] || die "源码快照存在未提交或未跟踪文件"
COMMIT="$(git -C "$SOURCE_ROOT" rev-parse HEAD)"

REPORT_DIR="$(realpath -m -- "$REPORT_DIR")" || die "无法解析 report 目录: $REPORT_DIR"
case "$REPORT_DIR/" in
    "$SOURCE_ROOT/"*) die "--report-dir 不能位于源码快照内" ;;
esac
[[ "$REPORT_DIR" != "/" ]] || die "--report-dir 不能是根目录"
if [[ -e "$REPORT_DIR" ]]; then
    [[ -d "$REPORT_DIR" ]] || die "--report-dir 不是目录: $REPORT_DIR"
else
    mkdir -p -- "$REPORT_DIR"
fi
[[ -z "$(find "$REPORT_DIR" -mindepth 1 -maxdepth 1 -print -quit)" ]] || die "--report-dir 必须是空的专用目录"

multi_write_status() {
    local exit_status="$1"
    if (( exit_status == 0 )); then
        printf 'complete\n' > "$MULTI_STATUS_FILE"
    else
        printf 'failed\nexit_status: %s\n' "$exit_status" > "$MULTI_STATUS_FILE"
    fi
}

run_multiple_samples() {
    local sample sample_id sample_dir runner_log sample_status child_status
    local -a child_args

    MULTI_STATUS_FILE="$REPORT_DIR/status"
    printf 'running\n' > "$MULTI_STATUS_FILE"
    trap 'multi_exit_status=$?; trap - EXIT; multi_write_status "$multi_exit_status"; exit "$multi_exit_status"' EXIT
    mkdir -p -- "$REPORT_DIR/runs"
    printf '# sample\tmetric\tvalue_ms\n' > "$REPORT_DIR/samples.tsv"
    {
        printf 'commit: %s\n' "$COMMIT"
        printf 'source_root: %s\n' "$SOURCE_ROOT"
        printf 'git_dirty: 0\n'
        printf 'rustc: %s\n' "$(rustc --version)"
        printf 'cargo: %s\n' "$(cargo --version)"
        printf 'runner: %s\n' "$(uname -a)"
        printf 'cpu_count: %s\n' "$(getconf _NPROCESSORS_ONLN 2>/dev/null || printf '<unknown>')"
        printf 'cargo_incremental: %s\n' "$CARGO_INCREMENTAL"
        printf 'cargo_build_jobs: %s\n' "${CARGO_BUILD_JOBS:-<default>}"
        printf 'cargo_profile_dev_debug: %s\n' "${CARGO_PROFILE_DEV_DEBUG:-<default>}"
        printf 'cargo_profile_dev_incremental: %s\n' "${CARGO_PROFILE_DEV_INCREMENTAL:-<default>}"
        printf 'cargo_profile_dev_opt_level: %s\n' "${CARGO_PROFILE_DEV_OPT_LEVEL:-<default>}"
        printf 'host_package: %s\n' "$HOST_PACKAGE"
        printf 'worker_package: %s\n' "$WORKER_PACKAGE"
        printf 'host_patch_sha256: %s\n' "$(sha256sum "$HOST_PATCH" | awk '{print $1}')"
        printf 'worker_patch_sha256: %s\n' "$(sha256sum "$WORKER_PATCH" | awk '{print $1}')"
        printf 'method: repeated complete real-edit samples; one isolated target/worktree per sample\n'
        printf 'requested_sample_count: %s\n' "$RUNS"
        printf 'host_binary: %s\n' "${HOST_BINARY:-<not measured>}"
        printf 'request_timeout_sec: %s\n' "$REQUEST_TIMEOUT"
        printf 'rss_sampling: readiness loop and in-flight request; host descendants included\n'
        printf 'protected_pids: %s\n' "${PROTECTED_PIDS[*]}"
        printf 'export_worker_runtime_e2e: %s\n' "$([[ "$EXPORT_WORKER_RUNTIME_E2E" -eq 1 ]] && printf requested || printf not-requested)"
        printf 'export_worker_runtime_script: %s\n' "$EXPORT_WORKER_RUNTIME_SCRIPT"
        printf 'raw_samples: %s\n' "$REPORT_DIR/samples.tsv"
        printf 'summary_metrics: %s\n' "$REPORT_DIR/metrics.tsv"
    } > "$REPORT_DIR/metadata.txt"

    for ((sample = 1; sample <= RUNS; sample++)); do
        printf -v sample_id '%03d' "$sample"
        sample_dir="$REPORT_DIR/runs/$sample_id"
        runner_log="$REPORT_DIR/runs/$sample_id.runner.log"
        child_args=(
            --source-snapshot "$SOURCE_DIR"
            --report-dir "$sample_dir"
            --host-patch "$HOST_PATCH"
            --worker-patch "$WORKER_PATCH"
            --host-package "$HOST_PACKAGE"
            --worker-package "$WORKER_PACKAGE"
            --startup-timeout "$STARTUP_TIMEOUT"
            --request-timeout "$REQUEST_TIMEOUT"
            --runs 1
        )
        [[ -n "$HOST_BINARY" ]] && child_args+=(--host-binary "$HOST_BINARY")
        [[ -n "$READY_URL" ]] && child_args+=(--ready-url "$READY_URL")
        [[ -n "$REQUEST_URL" ]] && child_args+=(--request-url "$REQUEST_URL")
        [[ -n "$REQUEST_BODY" ]] && child_args+=(--request-body "$REQUEST_BODY")
        (( EXPORT_WORKER_RUNTIME_E2E )) && child_args+=(--export-worker-runtime-e2e)
        (( KEEP_TARGET )) && child_args+=(--keep-target)

        child_status=0
        if "$SCRIPT_DIR/measure-runtime-feedback.sh" "${child_args[@]}" > "$runner_log" 2>&1; then
            :
        else
            child_status=$?
            cat "$runner_log" >&2
            die "样本 $sample_id 失败 (exit=$child_status)；详见 $runner_log"
        fi
        [[ -f "$sample_dir/status" ]] || die "样本 $sample_id 缺少 status；详见 $runner_log"
        sample_status="$(< "$sample_dir/status")"
        [[ "$sample_status" == complete ]] || die "样本 $sample_id 未完成 (status=$sample_status)；详见 $runner_log"
        [[ -f "$sample_dir/metrics.tsv" ]] || die "样本 $sample_id 缺少 metrics.tsv；详见 $runner_log"
        if awk -F '\t' -v sample="$sample_id" '
            BEGIN { valid = 0; invalid = 0 }
            /^#/ { next }
            NF != 2 {
                printf "runtime feedback: invalid sample metric row in %s\n", FILENAME > "/dev/stderr"
                invalid = 1
                next
            }
            $1 == "export_worker_runtime_e2e_report" { next }
            $2 !~ /^[0-9]+$/ {
                printf "runtime feedback: non-numeric sample metric %s in %s\n", $1, FILENAME > "/dev/stderr"
                invalid = 1
                next
            }
            { printf "%s\t%s\t%s\n", sample, $1, $2; valid = 1 }
            END { exit (invalid || !valid) }
        ' "$sample_dir/metrics.tsv" >> "$REPORT_DIR/samples.tsv"; then
            :
        else
            die "样本 $sample_id 的 metrics.tsv 无有效数值记录；详见 $runner_log"
        fi
    done

    printf '# metric\tvalid_samples\tmedian_ms\tmin_ms\tmax_ms\n' > "$REPORT_DIR/metrics.tsv"
    if ! awk -F '\t' -v expected="$RUNS" '
        BEGIN { bad = 0; metric_count = 0 }
        NR == 1 && $1 == "# sample" { next }
        NF != 3 || $2 == "" || $3 !~ /^[0-9]+$/ {
            printf "runtime feedback: invalid raw sample row at line %d\n", NR > "/dev/stderr"
            bad = 1
            next
        }
        {
            metric = $2
            value = $3 + 0
            count[metric]++
            values[metric, count[metric]] = value
            if (!(metric in minimum) || value < minimum[metric]) minimum[metric] = value
            if (!(metric in maximum) || value > maximum[metric]) maximum[metric] = value
            if (count[metric] == 1) metric_count++
        }
        END {
            if (metric_count == 0) {
                print "runtime feedback: no numeric metrics were collected" > "/dev/stderr"
                bad = 1
            }
            for (metric in count) {
                n = count[metric]
                if (n != expected) {
                    printf "runtime feedback: metric %s has %d/%d valid samples\n", metric, n, expected > "/dev/stderr"
                    bad = 1
                    continue
                }
                for (i = 1; i <= n; i++) ordered[i] = values[metric, i]
                for (i = 2; i <= n; i++) {
                    current = ordered[i]
                    j = i - 1
                    while (j >= 1 && ordered[j] > current) {
                        ordered[j + 1] = ordered[j]
                        j--
                    }
                    ordered[j + 1] = current
                }
                if (n % 2) median = ordered[(n + 1) / 2]
                else median = (ordered[n / 2] + ordered[n / 2 + 1]) / 2
                printf "%s\t%d\t%g\t%g\t%g\n", metric, n, median, minimum[metric], maximum[metric]
            }
            exit bad
        }
    ' "$REPORT_DIR/samples.tsv" | LC_ALL=C sort -t $'\t' -k1,1 >> "$REPORT_DIR/metrics.tsv"; then
        die "多样本统计失败；详见 $REPORT_DIR/samples.tsv"
    fi

    trap - EXIT
    multi_write_status 0
    printf 'runtime feedback report: %s\n' "$REPORT_DIR"
}

if (( RUNS > 1 )); then
    run_multiple_samples
    exit 0
fi

mkdir -p "$REPORT_DIR/logs" "$REPORT_DIR/timings" "$REPORT_DIR/worktrees"
STATUS_FILE="$REPORT_DIR/status"
TARGET_ROOT="$REPORT_DIR/target"
TARGET_CREATED=0
HOST_TARGET_DIR="$TARGET_ROOT/host"
WORKER_TARGET_DIR="$TARGET_ROOT/worker"
[[ "$HOST_TARGET_DIR" != "$WORKER_TARGET_DIR" ]] || die "Host 与 Worker 必须使用不同 target 目录"
METRICS="$REPORT_DIR/metrics.tsv"
printf '# metric\tvalue\n' > "$METRICS"
printf 'running\n' > "$STATUS_FILE"

WORKTREES=()
HOST_PID=""
HOST_PGID=""
HOST_WAITED=0
HOST_EXIT_STATUS=""
REQUEST_PID=""
RSS_SAMPLES=0
RSS_SAMPLE_FAILURES=0
READINESS_RSS_SAMPLES=0
REQUEST_RSS_SAMPLES=0
RSS_PHASE=""
RSS_ERROR_LOG=""
PROCESS_LOG=""

descendant_pids() {
    local parent="$1" child
    printf '%s\n' "$parent"
    while read -r child; do
        [[ -n "$child" ]] || continue
        descendant_pids "$child"
    done < <(pgrep -P "$parent" 2>/dev/null || true)
}

pid_is_live() {
    local pid="$1" stat
    kill -0 "$pid" 2>/dev/null || return 1
    stat="$(ps -o stat= -p "$pid" 2>/dev/null | awk '{$1=$1; print}')" || return 1
    [[ -n "$stat" && "$stat" != Z* ]]
}

host_group_has_process() {
    ps -e -o pgid= 2>/dev/null | awk -v expected="$1" '$1 == expected { found=1; exit } END { exit !found }'
}

host_group_has_live_process() {
    local process_table
    process_table="$(ps -e -o pgid= -o stat= 2>/dev/null)" || return 2
    awk -v expected="$1" '$1 == expected && $2 !~ /^Z/ { found=1; exit } END { exit !found }' \
        <<<"$process_table"
}

host_group_contains_protected_pid() {
    local protected pgid
    [[ -n "$HOST_PGID" ]] || return 1
    for protected in "${PROTECTED_PIDS[@]}"; do
        if kill -0 "$protected" 2>/dev/null; then
            pgid="$(ps -o pgid= -p "$protected" 2>/dev/null | awk '{$1=$1; print}')"
            [[ "$pgid" == "$HOST_PGID" ]] && return 0
        fi
    done
    return 1
}

reap_host() {
    local status=0
    [[ -n "$HOST_PID" && "$HOST_WAITED" -eq 0 ]] || return 0
    if wait "$HOST_PID" 2>/dev/null; then
        status=0
    else
        status=$?
    fi
    HOST_EXIT_STATUS="$status"
    HOST_WAITED=1
}

host_is_alive() {
    [[ -n "$HOST_PID" ]] && pid_is_live "$HOST_PID"
}

fail_if_host_exited() {
    local phase="$1" status
    if ! host_is_alive; then
        reap_host
        status="$HOST_EXIT_STATUS"
        [[ -n "$status" ]] || status=unknown
        die "Host 进程在 $phase 阶段提前退出 (exit=$status)；详见 $PROCESS_LOG"
    fi
}

wait_for_host_shutdown() {
    local attempt group_status
    for ((attempt = 0; attempt < HOST_STOP_GRACE_SECONDS * 10; attempt++)); do
        group_status=1
        if [[ -n "$HOST_PGID" ]]; then
            if host_group_has_live_process "$HOST_PGID"; then
                group_status=0
            else
                group_status=$?
            fi
            ((group_status <= 1)) || return 1
        fi
        if ! host_is_alive && ((group_status == 1)); then
            return 0
        fi
        sleep 0.1
    done
    return 1
}

send_host_signal() {
    local signal="$1" pid
    if host_group_contains_protected_pid; then
        printf 'runtime feedback: refusing to signal host group %s because it contains a protected PID\n' "$HOST_PGID" >&2
        return 1
    fi
    if [[ -n "$HOST_PGID" ]] && host_group_has_process "$HOST_PGID"; then
        kill "-$signal" -- "-$HOST_PGID" 2>/dev/null || true
    fi
    if [[ -n "$HOST_PID" ]]; then
        while read -r pid; do
            [[ "$pid" == "$HOST_PID" ]] || kill "-$signal" "$pid" 2>/dev/null || true
        done < <(descendant_pids "$HOST_PID")
        kill "-$signal" "$HOST_PID" 2>/dev/null || true
    fi
    return 0
}

stop_host_process() {
    local stop_failed=0
    if [[ -n "$REQUEST_PID" ]]; then
        kill -TERM "$REQUEST_PID" 2>/dev/null || true
        kill -KILL "$REQUEST_PID" 2>/dev/null || true
        wait "$REQUEST_PID" 2>/dev/null || true
        REQUEST_PID=""
    fi
    if [[ -n "$HOST_PID" || -n "$HOST_PGID" ]]; then
        if ! send_host_signal TERM; then
            stop_failed=1
        elif ! wait_for_host_shutdown; then
            if ! send_host_signal KILL || ! wait_for_host_shutdown; then
                stop_failed=1
            fi
        fi
        if (( stop_failed == 0 )); then
            reap_host
            if host_is_alive; then
                stop_failed=1
            fi
        fi
        if (( stop_failed != 0 )); then
            printf 'runtime feedback: failed to stop Host safely; report cannot be complete\n' >&2
            return 1
        fi
        HOST_PID=""
        HOST_PGID=""
        HOST_WAITED=0
        HOST_EXIT_STATUS=""
    fi
}

cleanup() {
    local exit_status="$1" worktree directory host_stopped=1
    if ! stop_host_process; then
        host_stopped=0
        if [[ "$exit_status" -eq 0 ]]; then
            exit_status=1
        fi
    fi
    if (( host_stopped && TARGET_CREATED && ! KEEP_TARGET )); then
        if [[ ! -d "$TARGET_ROOT" || -L "$TARGET_ROOT" ]]; then
            printf 'failed\nexit_status: 1\n' > "$STATUS_FILE"
            printf 'runtime feedback: refusing changed target root: %s\n' "$TARGET_ROOT" >&2
            return 1
        fi
        for directory in "$HOST_TARGET_DIR" "$WORKER_TARGET_DIR" "$TARGET_ROOT/runtime-e2e"; do
            if ! clean_build_cache "$directory" "$SOURCE_ROOT/Cargo.toml" >>"$REPORT_DIR/logs/cache-cleanup.log" 2>&1; then
                printf 'runtime feedback: cache cleanup failed; see %s\n' "$REPORT_DIR/logs/cache-cleanup.log" >&2
                exit_status=1
            fi
        done
        if [[ -d "$TARGET_ROOT" ]] && ! rmdir -- "$TARGET_ROOT"; then
            printf 'runtime feedback: temporary target was not fully reclaimed: %s\n' "$TARGET_ROOT" >&2
            exit_status=1
        fi
    elif (( KEEP_TARGET )); then
        printf 'runtime feedback: --keep-target retained %s\n' "$TARGET_ROOT" >&2
    fi
    for worktree in "${WORKTREES[@]:-}"; do
        [[ -n "$worktree" ]] || continue
        if ! git -C "$SOURCE_ROOT" worktree remove --force "$worktree" >/dev/null 2>&1; then
            printf 'runtime feedback: failed to remove measurement worktree: %s\n' "$worktree" >&2
            exit_status=1
        fi
    done
    if (( exit_status == 0 )); then
        printf 'complete\n' > "$STATUS_FILE"
    else
        printf 'failed\nexit_status: %s\n' "$exit_status" > "$STATUS_FILE"
    fi
    return "$exit_status"
}

trap 'exit_status=$?; trap - EXIT; cleanup "$exit_status" || exit_status=$?; exit "$exit_status"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

create_build_cache "$TARGET_ROOT"
TARGET_CREATED=1

git -C "$SOURCE_ROOT" apply --check "$HOST_PATCH" || die "Host patch 无法应用到固定 commit"
git -C "$SOURCE_ROOT" apply --check "$WORKER_PATCH" || die "Worker patch 无法应用到固定 commit"

{
    printf 'commit: %s\n' "$COMMIT"
    printf 'source_root: %s\n' "$SOURCE_ROOT"
    printf 'git_dirty: 0\n'
    printf 'rustc: %s\n' "$(rustc --version)"
    printf 'cargo: %s\n' "$(cargo --version)"
    printf 'runner: %s\n' "$(uname -a)"
    printf 'cpu_count: %s\n' "$(getconf _NPROCESSORS_ONLN 2>/dev/null || printf '<unknown>')"
    printf 'cargo_incremental: %s\n' "$CARGO_INCREMENTAL"
    printf 'cargo_build_jobs: %s\n' "${CARGO_BUILD_JOBS:-<default>}"
    printf 'cargo_profile_dev_debug: %s\n' "${CARGO_PROFILE_DEV_DEBUG:-<default>}"
    printf 'cargo_profile_dev_incremental: %s\n' "${CARGO_PROFILE_DEV_INCREMENTAL:-<default>}"
    printf 'cargo_profile_dev_opt_level: %s\n' "${CARGO_PROFILE_DEV_OPT_LEVEL:-<default>}"
    printf 'host_package: %s\n' "$HOST_PACKAGE"
    printf 'worker_package: %s\n' "$WORKER_PACKAGE"
    printf 'host_patch_sha256: %s\n' "$(sha256sum "$HOST_PATCH" | awk '{print $1}')"
    printf 'worker_patch_sha256: %s\n' "$(sha256sum "$WORKER_PATCH" | awk '{print $1}')"
    printf 'method: same worktree and target, baseline build then real patch, cargo timings\n'
    printf 'host_target_dir: %s\n' "$HOST_TARGET_DIR"
    printf 'worker_target_dir: %s\n' "$WORKER_TARGET_DIR"
    printf 'keep_target: %s\n' "$KEEP_TARGET"
    printf 'host_binary: %s\n' "${HOST_BINARY:-<not measured>}"
    if [[ -n "$HOST_BINARY" ]]; then
        printf 'runtime_measurement: enabled\n'
    else
        printf 'runtime_measurement: not-measured (no --host-binary)\n'
    fi
    printf 'request_timeout_sec: %s\n' "$REQUEST_TIMEOUT"
    printf 'rss_sampling: readiness loop and in-flight request; host descendants included\n'
    printf 'protected_pids: %s\n' "${PROTECTED_PIDS[*]}"
    printf 'export_worker_runtime_e2e: %s\n' "$([[ "$EXPORT_WORKER_RUNTIME_E2E" -eq 1 ]] && printf requested || printf not-requested)"
    printf 'export_worker_runtime_script: %s\n' "$EXPORT_WORKER_RUNTIME_SCRIPT"
} > "$REPORT_DIR/metadata.txt"

now_ms() { python3 -c 'import time; print(time.monotonic_ns() // 1_000_000)'; }

new_worktree() {
    local name="$1"
    local path
    path="$(mktemp -d "$REPORT_DIR/worktrees/${name}.XXXXXX")"
    rmdir -- "$path"
    git -C "$SOURCE_ROOT" worktree add --detach "$path" "$COMMIT" >/dev/null
    [[ "$(git -C "$path" rev-parse HEAD)" == "$COMMIT" ]] || die "测量 worktree 未固定到 $COMMIT"
    [[ -z "$(git -C "$path" status --porcelain=v1 --untracked-files=all)" ]] || die "新建测量 worktree 非干净"
    WORKTREES+=("$path")
    NEW_WORKTREE_PATH="$path"
}

build_package() {
    local label="$1" worktree="$2" target_dir="$3" package="$4"
    local log="$REPORT_DIR/logs/$label.log"
    local timing_source="$target_dir/cargo-timings"
    local timing_dest="$REPORT_DIR/timings/$label"
    local compiling_log post_build_status patch_label expected_status start end
    rm -rf -- "$timing_source"
    {
        printf 'command:'
        printf ' %q' cargo build --manifest-path "$worktree/Cargo.toml" -p "$package" --locked --timings -vv
        printf '\n'
        printf 'worktree: %s\n' "$worktree"
        printf 'target_dir: %s\n' "$target_dir"
    } > "$log"
    start="$(now_ms)"
    if ! (cd "$worktree" && CARGO_TARGET_DIR="$target_dir" CARGO_BUILD_BUILD_DIR="$target_dir" cargo build --manifest-path "$worktree/Cargo.toml" -p "$package" --locked --timings -vv) >>"$log" 2>&1; then
        cat "$log" >&2
        die "build 失败: $label"
    fi
    end="$(now_ms)"
    post_build_status="$(git -C "$worktree" status --porcelain=v1 --untracked-files=all)"
    printf '%s\n' "$post_build_status" > "$REPORT_DIR/logs/$label.post-build-status.txt"
    case "$label" in
        *_baseline|*_baseline_*)
            [[ -z "$post_build_status" ]] || die "基线 build 后 worktree 被污染: $label"
            ;;
        *_edit_*)
            patch_label="${label%%_edit*}"
            [[ -f "$REPORT_DIR/logs/$patch_label.patch-status.txt" ]] || die "缺少已应用 patch 状态: $patch_label"
            expected_status="$(< "$REPORT_DIR/logs/$patch_label.patch-status.txt")"
            [[ "$post_build_status" == "$expected_status" ]] || die "编辑 build 改变了 patch 外的 worktree: $label"
            ;;
        *_edit)
            patch_label="${label%%_edit}"
            [[ -f "$REPORT_DIR/logs/$patch_label.patch-status.txt" ]] || die "缺少已应用 patch 状态: $patch_label"
            expected_status="$(< "$REPORT_DIR/logs/$patch_label.patch-status.txt")"
            [[ "$post_build_status" == "$expected_status" ]] || die "编辑 build 改变了 patch 外的 worktree: $label"
            ;;
    esac
    [[ -d "$timing_source" ]] || die "缺少 cargo timings: $label"
    mkdir -p "$timing_dest"
    cp -R "$timing_source/." "$timing_dest/"
    write_matching_lines \
        '^[[:space:]]*(Fresh|Compiling|Running)([[:space:]]|$)' \
        "$log" "$REPORT_DIR/logs/$label.targets.txt"
    compiling_log="$REPORT_DIR/logs/$label.compiling.txt"
    write_matching_lines \
        '^[[:space:]]*Compiling[[:space:]]+[^[:space:]]+' \
        "$log" "$compiling_log"
    if [[ "$label" == *_baseline* ]] && ! rg -F -q -- "Compiling $package " "$compiling_log"; then
        die "基线 build 没有实际 Compiling 目标: $label ($package)"
    fi
    printf '%s\t%s\n' "$label" "$((end - start))" >> "$METRICS"
}

apply_patch_to_worktree() {
    local label="$1" worktree="$2" patch="$3" before_status
    before_status="$(git -C "$worktree" status --porcelain=v1 --untracked-files=all)"
    [[ -z "$before_status" ]] || die "$label patch 前 worktree 已被构建污染"
    printf '%s\n' "$before_status" > "$REPORT_DIR/logs/$label.before-patch-status.txt"
    git -C "$worktree" apply --check "$patch" || die "$label patch 无法应用"
    git -C "$worktree" apply "$patch" || die "$label patch 应用失败"
    [[ -n "$(git -C "$worktree" status --porcelain=v1 --untracked-files=all)" ]] || die "$label patch 应用后没有工作区变更"
    git -C "$worktree" status --porcelain=v1 --untracked-files=all > "$REPORT_DIR/logs/$label.patch-status.txt"
    git -C "$worktree" diff --binary > "$REPORT_DIR/logs/$label.applied.diff"
}

new_worktree host
host_worktree="$NEW_WORKTREE_PATH"
build_package host_baseline "$host_worktree" "$HOST_TARGET_DIR" "$HOST_PACKAGE"
apply_patch_to_worktree host "$host_worktree" "$HOST_PATCH"
build_package host_edit "$host_worktree" "$HOST_TARGET_DIR" "$HOST_PACKAGE"

new_worktree worker
worker_worktree="$NEW_WORKTREE_PATH"
build_package worker_baseline_host "$worker_worktree" "$WORKER_TARGET_DIR" "$HOST_PACKAGE"
build_package worker_baseline_worker "$worker_worktree" "$WORKER_TARGET_DIR" "$WORKER_PACKAGE"
apply_patch_to_worktree worker "$worker_worktree" "$WORKER_PATCH"
build_package worker_edit_host "$worker_worktree" "$WORKER_TARGET_DIR" "$HOST_PACKAGE"
build_package worker_edit_worker "$worker_worktree" "$WORKER_TARGET_DIR" "$WORKER_PACKAGE"

if [[ -n "$HOST_BINARY" ]]; then
    for command in curl pgrep ps awk setsid; do
        require_command "$command"
    done
    rss_log="$REPORT_DIR/rss.tsv"
    PROCESS_LOG="$REPORT_DIR/logs/process.log"
    readiness_log="$REPORT_DIR/logs/readiness.log"
    request_log="$REPORT_DIR/logs/request.log"
    RSS_ERROR_LOG="$REPORT_DIR/logs/rss-errors.tsv"
    printf '# elapsed_ms\trss_kb\tpid\n' > "$rss_log"
    printf '# elapsed_ms\tphase\treason\n' > "$RSS_ERROR_LOG"
    start="$(now_ms)"
    setsid --wait "$HOST_BINARY" >"$PROCESS_LOG" 2>&1 &
    HOST_PID=$!
    HOST_WAITED=0
    HOST_EXIT_STATUS=""
    for ((attempt = 0; attempt < 100; attempt++)); do
        HOST_PGID="$(ps -o pgid= -p "$HOST_PID" 2>/dev/null | awk '{$1=$1; print}')"
        if [[ "$HOST_PGID" != "$HOST_PID" ]]; then
            HOST_PGID=""
        fi
        [[ -n "$HOST_PGID" ]] && break
        host_is_alive || break
        sleep 0.01
    done
    rss_sample_failure() {
        local reason="$1"
        RSS_SAMPLE_FAILURES=$((RSS_SAMPLE_FAILURES + 1))
        printf '%s\t%s\t%s\n' "$(now_ms)" "${RSS_PHASE:-unknown}" "$reason" >> "$RSS_ERROR_LOG"
        return 1
    }

    sample_rss() {
        local elapsed total
        if ! total="$(ps -e -o pid= -o ppid= -o rss= -o stat= 2>/dev/null | awk -v root="$HOST_PID" '
            {
                pid = $1
                ppid = $2
                rss[pid] = $3
                state[pid] = $4
                if (pid ~ /^[0-9]+$/ && ppid ~ /^[0-9]+$/) {
                    parent[pid] = ppid
                }
            }
            END {
                if (!(root in rss) || state[root] ~ /^Z/) {
                    exit 2
                }
                tree[root] = 1
                changed = 1
                while (changed) {
                    changed = 0
                    for (pid in parent) {
                        if (tree[parent[pid]] && !tree[pid]) {
                            tree[pid] = 1
                            changed = 1
                        }
                    }
                }
                for (pid in tree) {
                    if (!tree[pid]) {
                        continue
                    }
                    if (!(pid in rss)) {
                        continue
                    }
                    if (state[pid] !~ /^Z/) {
                        if (rss[pid] !~ /^[0-9]+$/) {
                            exit 4
                        }
                        total += rss[pid] + 0
                    }
                }
                if (total <= 0) {
                    exit 3
                }
                print total
            }')"; then
            rss_sample_failure ps-tree
            return 1
        fi
        if [[ ! "$total" =~ ^[1-9][0-9]*$ ]]; then
            rss_sample_failure invalid-total
            return 1
        fi
        elapsed="$(( $(now_ms) - start ))"
        printf '%s\t%s\t%s\n' "$elapsed" "$total" "$HOST_PID" >> "$rss_log"
        RSS_SAMPLES=$((RSS_SAMPLES + 1))
        case "$RSS_PHASE" in
            readiness) READINESS_RSS_SAMPLES=$((READINESS_RSS_SAMPLES + 1)) ;;
            request) REQUEST_RSS_SAMPLES=$((REQUEST_RSS_SAMPLES + 1)) ;;
        esac
    }

    sample_rss_required() {
        local phase="$1" attempt
        for ((attempt = 0; attempt < 10; attempt++)); do
            if sample_rss; then
                return 0
            fi
            sleep 0.01
        done
        die "无法取得有效 $phase RSS；详见 $RSS_ERROR_LOG"
    }

    ready=0
    RSS_PHASE=readiness
    readiness_deadline=$((start + STARTUP_TIMEOUT * 1000))
    while (( $(now_ms) < readiness_deadline )); do
        sample_rss_required readiness
        fail_if_host_exited "readiness"
        readiness_code=""
        readiness_curl_status=0
        readiness_code="$(curl --silent --show-error --output /dev/null \
            --write-out '%{http_code}' --max-time 1 -- "$READY_URL" \
            2>>"$readiness_log")" || readiness_curl_status=$?
        fail_if_host_exited "readiness probe"
        if is_successful_http_response "$readiness_curl_status" "$readiness_code"; then
            ready=1
            break
        fi
        if [[ "$readiness_code" =~ ^[1-9][0-9][0-9]$ ]] && ! is_http_2xx "$readiness_code"; then
            die "Host readiness HTTP 状态非 2xx: $readiness_code；详见 $readiness_log"
        fi
        sleep 0.1
    done
    [[ "$ready" -eq 1 ]] || die "Host readiness 超时；详见 $PROCESS_LOG 和 $readiness_log"
    fail_if_host_exited "readiness 完成"
    sample_rss_required "readiness completion"
    if (( READINESS_RSS_SAMPLES == 0 )); then
        die "readiness 阶段没有有效 RSS 记录"
    fi
    printf 'startup_ready_ms\t%s\n' "$(( $(now_ms) - start ))" >> "$METRICS"

    if [[ -n "$REQUEST_URL" ]]; then
        fail_if_host_exited "固定请求开始"
        RSS_PHASE=request
        request_start="$(now_ms)"
        request_code_file="$REPORT_DIR/request.http-code"
        request_record="$REPORT_DIR/request.tsv"
        : > "$request_code_file"
        : > "$request_log"
        printf '# start_ms\tend_ms\tcurl_exit_status\thttp_code\n' > "$request_record"
        if [[ -n "$REQUEST_BODY" ]]; then
            curl --silent --show-error --output /dev/null \
                --write-out '%{http_code}' --max-time "$REQUEST_TIMEOUT" \
                -H 'content-type: application/json' --data-binary "@$REQUEST_BODY" \
                -- "$REQUEST_URL" >"$request_code_file" 2>"$request_log" &
        else
            curl --silent --show-error --output /dev/null \
                --write-out '%{http_code}' --max-time "$REQUEST_TIMEOUT" \
                -- "$REQUEST_URL" >"$request_code_file" 2>"$request_log" &
        fi
        REQUEST_PID=$!
        sample_rss_required "request start"
        while pid_is_live "$REQUEST_PID"; do
            sleep 0.1
            sample_rss_required "request in-flight"
            fail_if_host_exited "固定请求"
        done
        fail_if_host_exited "固定请求完成"
        request_status=0
        if wait "$REQUEST_PID"; then
            request_status=0
        else
            request_status=$?
        fi
        REQUEST_PID=""
        request_code="$(< "$request_code_file")"
        request_end="$(now_ms)"
        printf '%s\t%s\t%s\t%s\n' \
            "$request_start" "$request_end" "$request_status" "$request_code" > "$request_record"
        if (( request_status == 28 )); then
            die "固定请求超时；详见 $request_log"
        fi
        (( request_status == 0 )) || die "固定请求传输失败 (curl exit=$request_status)；详见 $request_log"
        request_code_display="$request_code"
        [[ -n "$request_code_display" ]] || request_code_display="<none>"
        is_successful_http_response "$request_status" "$request_code" || die "固定请求 HTTP 状态非 2xx: $request_code_display；详见 $request_log"
        if [[ ! -s "$request_record" ]]; then
            die "固定请求缺少状态记录"
        fi
        printf 'first_request_ms\t%s\n' "$(( $(now_ms) - request_start ))" >> "$METRICS"
        if (( REQUEST_RSS_SAMPLES == 0 )); then
            die "固定请求阶段没有有效 RSS 记录"
        fi
    fi

    fail_if_host_exited "RSS 收尾"
    RSS_PHASE=final
    sample_rss_required "RSS final"
    fail_if_host_exited "RSS 收尾采样"
    if (( RSS_SAMPLES == 0 )); then
        die "未采集到有效 Host/子进程 RSS"
    fi
    peak_rss="$(awk -F '\t' 'NR > 1 && $2 > peak { peak = $2 } END { print peak + 0 }' "$rss_log")"
    [[ "$peak_rss" =~ ^[1-9][0-9]*$ ]] || die "RSS 峰值不是有效正整数"
    printf 'host_and_children_sampled_peak_rss_kb\t%s\n' "$peak_rss" >> "$METRICS"
    printf 'rss_sample_failures\t%s\n' "$RSS_SAMPLE_FAILURES" >> "$METRICS"
    fail_if_host_exited "测量结束"
    if ! stop_host_process; then
        die "Host 未能安全停止；不生成成功测量"
    fi
fi

if (( EXPORT_WORKER_RUNTIME_E2E )); then
    [[ -f "$EXPORT_WORKER_RUNTIME_SCRIPT" ]] || die "缺少 export Worker runtime 验收器: $EXPORT_WORKER_RUNTIME_SCRIPT"
    runtime_report_dir="$REPORT_DIR/runtime-e2e"
    runtime_target_dir="$REPORT_DIR/target/runtime-e2e"
    mkdir -p -- "$runtime_report_dir" "$runtime_target_dir"
    [[ -z "$(find "$runtime_report_dir" -mindepth 1 -maxdepth 1 -print -quit)" ]] || die "runtime-e2e report 目录必须为空"
    python3 "$EXPORT_WORKER_RUNTIME_SCRIPT" \
        --build \
        --report-dir "$runtime_report_dir" \
        --target-dir "$runtime_target_dir"
    printf 'export_worker_runtime_e2e_report\t%s\n' "$runtime_report_dir" >> "$METRICS"
fi

echo "runtime feedback report: $REPORT_DIR"
