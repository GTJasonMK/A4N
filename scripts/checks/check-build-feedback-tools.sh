#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
cd "$PROJECT_ROOT"

runtime_measure="$SCRIPT_DIR/measure-runtime-feedback.sh"
[[ -x "$runtime_measure" ]] || {
    echo "real-edit runtime feedback tool is missing or not executable" >&2
    exit 1
}
bash -n "$runtime_measure"
"$runtime_measure" --help >/dev/null

workflow="$PROJECT_ROOT/.github/workflows/module-checks.yml"
job="$(awk '
    /^  build-feedback:/ { capture = 1 }
    capture && /^  [[:alnum:]_-]+:/ && !/^  build-feedback:/ { exit }
    capture { print }
' "$workflow")"
[[ -n "$job" ]] || {
    echo "module-checks workflow is missing the build-feedback job" >&2
    exit 1
}
while IFS= read -r required; do
    rg -F -q -- "$required" <<<"$job" || {
        echo "build-feedback workflow is missing: $required" >&2
        exit 1
    }
done <<'EOF'
runs-on: ubuntu-24.04
CARGO_INCREMENTAL: "1"
ripgrep
actions/checkout@11d5960a326750d5838078e36cf38b85af677262
fetch-depth: 0
dtolnay/rust-toolchain@f133eefe930d61f0d9371efd474daf0125ed3dd1
check-build-feedback-tools.sh
setup-onnxruntime.sh
require_onnxruntime
measure-build-feedback.sh --runs 3
check-build-feedback-budget.sh
if: always()
actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02
target/build-feedback
scripts/checks/build-feedback-baseline.tsv
EOF
if rg -q 'uses: [^[:space:]]+@(v[0-9]+|stable)([[:space:]]|$)' <<<"$job"; then
    echo "build-feedback workflow contains a movable action reference" >&2
    exit 1
fi
rg -Fq -- '-e normal,build' "$SCRIPT_DIR/measure-build-feedback.sh" || {
    echo "build-feedback dependency evidence must include normal and build dependencies" >&2
    exit 1
}

tmp_dir="$(make_target_temp_dir build-feedback-tools)"
export A4N_CACHE_TEST_ROOT="$tmp_dir"
cleanup() { rm -rf -- "$tmp_dir"; }
trap cleanup EXIT

cargo() {
    case "${1:-}" in
        --version)
            printf 'cargo 1.95.0 (build-feedback fixture)\n'
            ;;
        check)
            sleep 0.01
            mkdir -p "${CARGO_TARGET_DIR:?}/cargo-timings"
            printf '<html>fixture</html>\n' > "$CARGO_TARGET_DIR/cargo-timings/cargo-timing.html"
            ;;
        clean)
            if [[ "${2:-}" != "-p" ]]; then
                [[ "$CARGO_TARGET_DIR" == "$A4N_CACHE_TEST_ROOT/"* ]] || return 1
                rm -rf -- "$CARGO_TARGET_DIR"
            fi
            ;;
        tree)
            printf 'fixture dependency tree\n'
            ;;
        *)
            printf 'unexpected cargo command in build-feedback self-test: %s\n' "$*" >&2
            return 1
            ;;
    esac
}

rustc() {
    printf 'rustc 1.95.0 (build-feedback fixture)\n'
}

export -f cargo rustc

artifact_dir="$tmp_dir/artifacts"
measurements="$tmp_dir/measurements.tsv"
baseline="$tmp_dir/baseline.tsv"
runs=3
"$SCRIPT_DIR/measure-build-feedback.sh" \
    --runs "$runs" \
    --artifact-dir "$artifact_dir" \
    --output "$measurements" \
    --write-baseline "$baseline" >/dev/null

run_dir="$(find "$artifact_dir/runs" -mindepth 1 -maxdepth 1 -type d -print -quit)"
[[ -n "$run_dir" && -f "$run_dir/metadata.txt" && -f "$run_dir/samples.tsv" ]] || {
    echo "build-feedback run metadata or raw samples are missing" >&2
    exit 1
}
package_count="$(awk -F '\t' '$0 !~ /^#/ && NF { count++ } END { print count + 0 }' "$baseline")"
first_package="$(awk -F '\t' '$0 !~ /^#/ && NF { print $1; exit }' "$baseline")"
[[ "$package_count" -gt 0 && -n "$first_package" ]] || {
    echo "build-feedback self-test baseline has no packages" >&2
    exit 1
}
expected_timing_dirs=$((1 + package_count * (runs + 1)))
expected_reset_logs=$((package_count * runs))
[[ "$(find "$run_dir/timings" -mindepth 1 -maxdepth 1 -type d | wc -l)" -eq "$expected_timing_dirs" ]] || {
    echo "build-feedback self-test expected $expected_timing_dirs timing directories" >&2
    exit 1
}
[[ "$(find "$run_dir/logs" -maxdepth 1 -type f -name '*-reset.log' | wc -l)" -eq "$expected_reset_logs" ]] || {
    echo "build-feedback self-test expected $expected_reset_logs package reset logs" >&2
    exit 1
}
[[ "$(find "$run_dir/dependencies" -maxdepth 1 -type f -name '*.txt' | wc -l)" -eq "$package_count" ]] || {
    echo "build-feedback self-test expected $package_count dependency inventories" >&2
    exit 1
}
[[ "$(awk -F '\t' -v package="$first_package" '$1 == package && $2 ~ /^[0-9]+$/ { count++ } END { print count + 0 }' "$run_dir/samples.tsv")" -eq "$runs" ]] || {
    echo "build-feedback self-test expected $runs raw warm samples for $first_package" >&2
    exit 1
}
rg -q '^__workspace_cold__\t1\t[1-9][0-9]*$' "$run_dir/samples.tsv" || {
    echo "build-feedback self-test expected one positive isolated cold sample" >&2
    exit 1
}
if rg -q '^__workspace_cold__\t' "$baseline"; then
    echo "workspace cold sample must not be written as a warm package baseline" >&2
    exit 1
fi
if find "$artifact_dir/tmp" -mindepth 1 -type d -print -quit | rg -q .; then
    echo "isolated build-feedback target directories were not cleaned" >&2
    exit 1
fi

"$SCRIPT_DIR/check-build-feedback-budget.sh" \
    --baseline "$baseline" \
    --measurements "$measurements" >/dev/null

cp "$baseline" "$tmp_dir/unexpected-baseline.tsv"
printf 'unexpected-package\t1\t20\n' >> "$tmp_dir/unexpected-baseline.tsv"
if "$SCRIPT_DIR/check-build-feedback-budget.sh" \
    --baseline "$tmp_dir/unexpected-baseline.tsv" \
    --measurements "$measurements" >/dev/null 2>&1; then
    echo "unexpected build-feedback baseline unexpectedly passed" >&2
    exit 1
fi

sed 's/^# method.*/# method\tinvalid-method/' "$baseline" > "$tmp_dir/wrong-method.tsv"
if "$SCRIPT_DIR/check-build-feedback-budget.sh" \
    --baseline "$tmp_dir/wrong-method.tsv" \
    --measurements "$measurements" >/dev/null 2>&1; then
    echo "mismatched build-feedback methods unexpectedly passed" >&2
    exit 1
fi

sed '/^#/! s/$/\textra-field/' "$measurements" > "$tmp_dir/extra-field.tsv"
if "$SCRIPT_DIR/check-build-feedback-budget.sh" \
    --baseline "$baseline" \
    --measurements "$tmp_dir/extra-field.tsv" >/dev/null 2>&1; then
    echo "build-feedback measurement with extra fields unexpectedly passed" >&2
    exit 1
fi

fixture_root="$tmp_dir/runtime-fixture"
snapshot="$fixture_root/source"
mkdir -p "$fixture_root"
git init -q "$snapshot"
git -C "$snapshot" config user.email "build-feedback-fixture@example.invalid"
git -C "$snapshot" config user.name "build-feedback fixture"
printf '%s\n' '[workspace]' 'resolver = "2"' 'members = []' > "$snapshot/Cargo.toml"
printf 'host baseline\n' > "$snapshot/host.txt"
printf 'worker baseline\n' > "$snapshot/worker.txt"
git -C "$snapshot" add Cargo.toml host.txt worker.txt
git -C "$snapshot" commit -qm baseline

printf 'host edit\n' > "$snapshot/host.txt"
git -C "$snapshot" diff --binary -- host.txt > "$fixture_root/host.patch"
git -C "$snapshot" restore --worktree -- host.txt
printf 'worker edit\n' > "$snapshot/worker.txt"
git -C "$snapshot" diff --binary -- worker.txt > "$fixture_root/worker.patch"
git -C "$snapshot" restore --worktree -- worker.txt

runtime_cargo_log="$fixture_root/runtime-cargo.tsv"
export RUNTIME_CARGO_LOG="$runtime_cargo_log"
cargo() {
    case "${1:-}" in
        --version)
            printf 'cargo 1.95.0 (runtime fixture)\n'
            ;;
        build)
            local package=""
            while [[ $# -gt 0 ]]; do
                if [[ "$1" == "-p" ]]; then
                    package="${2:?missing package in runtime fixture}"
                    shift 2
                else
                    shift
                fi
            done
            [[ -n "$package" ]] || {
                echo "runtime fixture build has no package" >&2
                return 1
            }
            [[ "${CARGO_BUILD_BUILD_DIR:-}" == "$CARGO_TARGET_DIR" ]] || {
                echo "runtime fixture build escaped its isolated build directory" >&2
                return 1
            }
            local state=clean
            [[ -z "$(git -C "$PWD" status --porcelain=v1 --untracked-files=all)" ]] || state=dirty
            if [[ "$state" == dirty && ! -s "$CARGO_TARGET_DIR/debug/output" ]]; then
                echo "runtime fixture lost its warm cache before the edited build" >&2
                return 1
            fi
            mkdir -p "${CARGO_TARGET_DIR:?}/debug"
            printf 'disposable build output\n' > "$CARGO_TARGET_DIR/debug/output"
            if [[ "${RUNTIME_FAIL:-0}" == "1" ]]; then
                echo "runtime fixture forced build failure" >&2
                return 9
            fi
            printf '%s\t%s\t%s\t%s\n' \
                "$package" "$state" "$PWD" "${CARGO_TARGET_DIR:?}" >> "$RUNTIME_CARGO_LOG"
            mkdir -p "${CARGO_TARGET_DIR:?}/cargo-timings"
            printf '<html>runtime fixture</html>\n' > "$CARGO_TARGET_DIR/cargo-timings/cargo-timing.html"
            printf 'Compiling %s v0.0.0\n' "$package"
            ;;
        clean)
            [[ "$CARGO_TARGET_DIR" == "$A4N_CACHE_TEST_ROOT/"* ]] || return 1
            [[ "${RUNTIME_CLEAN_FAIL:-0}" != "1" ]] || return 11
            rm -rf -- "$CARGO_TARGET_DIR"
            ;;
        *)
            echo "unexpected runtime fixture cargo command: $*" >&2
            return 1
            ;;
    esac
}
export -f cargo

runtime_report="$tmp_dir/runtime-report"
"$runtime_measure" \
    --source-snapshot "$snapshot" \
    --report-dir "$runtime_report" \
    --host-patch "$fixture_root/host.patch" \
    --worker-patch "$fixture_root/worker.patch" \
    --host-package host \
    --worker-package worker >/dev/null

rg -q '^complete$' "$runtime_report/status" || {
    echo "real-edit runtime self-test did not complete" >&2
    exit 1
}
[[ "$(wc -l < "$runtime_cargo_log")" -eq 6 ]] || {
    echo "real-edit runtime self-test expected six package builds" >&2
    exit 1
}
[[ "$(awk -F '\t' 'NR == 1 { print $1 "\t" $2 }' "$runtime_cargo_log")" == $'host\tclean' ]] || {
    echo "real-edit runtime self-test missing clean Host baseline" >&2
    exit 1
}
[[ "$(awk -F '\t' 'NR == 2 { print $1 "\t" $2 }' "$runtime_cargo_log")" == $'host\tdirty' ]] || {
    echo "real-edit runtime self-test missing dirty Host edit" >&2
    exit 1
}
[[ "$(awk -F '\t' 'NR == 3 { print $1 "\t" $2 }' "$runtime_cargo_log")" == $'host\tclean' ]] || {
    echo "real-edit runtime self-test missing worker experiment Host baseline" >&2
    exit 1
}
[[ "$(awk -F '\t' 'NR == 4 { print $1 "\t" $2 }' "$runtime_cargo_log")" == $'worker\tclean' ]] || {
    echo "real-edit runtime self-test missing Worker baseline" >&2
    exit 1
}
[[ "$(awk -F '\t' 'NR == 5 { print $1 "\t" $2 }' "$runtime_cargo_log")" == $'host\tdirty' ]] || {
    echo "real-edit runtime self-test missing worker experiment Host edit" >&2
    exit 1
}
[[ "$(awk -F '\t' 'NR == 6 { print $1 "\t" $2 }' "$runtime_cargo_log")" == $'worker\tdirty' ]] || {
    echo "real-edit runtime self-test missing Worker edit" >&2
    exit 1
}
[[ "$(awk -F '\t' 'NR == 1 { print $3 "\t" $4 }' "$runtime_cargo_log")" == \
   "$(awk -F '\t' 'NR == 2 { print $3 "\t" $4 }' "$runtime_cargo_log")" ]] || {
    echo "real-edit runtime self-test Host baseline/edit did not reuse worktree and target" >&2
    exit 1
}
[[ "$(awk -F '\t' 'NR == 3 { print $3 "\t" $4 }' "$runtime_cargo_log")" == \
   "$(awk -F '\t' 'NR == 4 { print $3 "\t" $4 }' "$runtime_cargo_log")" && \
   "$(awk -F '\t' 'NR == 4 { print $3 "\t" $4 }' "$runtime_cargo_log")" == \
   "$(awk -F '\t' 'NR == 5 { print $3 "\t" $4 }' "$runtime_cargo_log")" && \
   "$(awk -F '\t' 'NR == 5 { print $3 "\t" $4 }' "$runtime_cargo_log")" == \
   "$(awk -F '\t' 'NR == 6 { print $3 "\t" $4 }' "$runtime_cargo_log")" ]] || {
    echo "real-edit runtime self-test Worker baseline/edit did not reuse worktree and target" >&2
    exit 1
}
[[ "$(awk -F '\t' 'NR == 1 { print $4 }' "$runtime_cargo_log")" != \
   "$(awk -F '\t' 'NR == 3 { print $4 }' "$runtime_cargo_log")" ]] || {
    echo "real-edit runtime self-test unexpectedly shared Host and Worker targets" >&2
    exit 1
}
[[ "$(awk -F '\t' '$1 == "host_baseline" { count++ } END { print count + 0 }' "$runtime_report/metrics.tsv")" -eq 1 && \
   "$(awk -F '\t' '$1 == "host_edit" { count++ } END { print count + 0 }' "$runtime_report/metrics.tsv")" -eq 1 ]] || {
    echo "real-edit runtime self-test metrics are incomplete" >&2
    exit 1
}
if find "$runtime_report/worktrees" -mindepth 1 -print -quit | rg -q .; then
    echo "real-edit runtime self-test left worktrees behind" >&2
    exit 1
fi
[[ ! -e "$runtime_report/target" && -s "$runtime_report/timings/host_baseline/cargo-timing.html" ]] || {
    echo "runtime cleanup must remove caches and retain timing evidence" >&2
    exit 1
}

host_test_bin="$fixture_root/host-test-bin"
mkdir -p "$host_test_bin"
printf '%s\n' '#!/usr/bin/env bash' 'printf 200' > "$host_test_bin/curl"
chmod +x "$host_test_bin/curl"
graceful_host="$fixture_root/graceful-host"
printf '%s\n' \
    '#!/usr/bin/env python3' \
    'import os, signal, time' \
    'signal.signal(signal.SIGTERM, signal.SIG_IGN)' \
    'time.sleep(0.3)' \
    'with open(os.environ["RUNTIME_HOST_STOP_FILE"], "w", encoding="utf-8") as stopped:' \
    '    stopped.write("stopped")' > "$graceful_host"
chmod +x "$graceful_host"
graceful_host_report="$tmp_dir/runtime-report-graceful-host"
RUNTIME_CARGO_LOG="$fixture_root/runtime-cargo-graceful-host.tsv"
RUNTIME_HOST_STOP_FILE="$fixture_root/graceful-host-stopped"
export RUNTIME_CARGO_LOG RUNTIME_HOST_STOP_FILE
PATH="$host_test_bin:$PATH" "$runtime_measure" \
    --source-snapshot "$snapshot" \
    --report-dir "$graceful_host_report" \
    --host-patch "$fixture_root/host.patch" \
    --worker-patch "$fixture_root/worker.patch" \
    --host-package host \
    --worker-package worker \
    --host-binary "$graceful_host" \
    --ready-url http://127.0.0.1/ready >/dev/null
[[ "$(< "$fixture_root/graceful-host-stopped")" == stopped ]] || {
    echo "runtime measurement killed Host before graceful shutdown completed" >&2
    exit 1
}

multi_runtime_cargo_log="$fixture_root/runtime-cargo-multi.tsv"
RUNTIME_CARGO_LOG="$multi_runtime_cargo_log"
export RUNTIME_CARGO_LOG
multi_runtime_report="$tmp_dir/runtime-report-multi"
"$runtime_measure" \
    --source-snapshot "$snapshot" \
    --report-dir "$multi_runtime_report" \
    --host-patch "$fixture_root/host.patch" \
    --worker-patch "$fixture_root/worker.patch" \
    --host-package host \
    --worker-package worker \
    --runs 3 >/dev/null
rg -q '^complete$' "$multi_runtime_report/status" || {
    echo "real-edit runtime multi-sample self-test did not complete" >&2
    exit 1
}
[[ "$(wc -l < "$multi_runtime_cargo_log")" -eq 18 ]] || {
    echo "real-edit runtime multi-sample self-test expected eighteen package builds" >&2
    exit 1
}
[[ "$(find "$multi_runtime_report/runs" -mindepth 1 -maxdepth 1 -type d | wc -l)" -eq 3 ]] || {
    echo "real-edit runtime multi-sample self-test expected three isolated sample reports" >&2
    exit 1
}
[[ "$(awk -F '\t' '!seen[$4]++ { count++ } END { print count + 0 }' "$multi_runtime_cargo_log")" -eq 6 ]] || {
    echo "real-edit runtime multi-sample self-test did not isolate targets per sample" >&2
    exit 1
}
[[ "$(awk -F '\t' 'NR > 1 && $2 == 3 { count++ } END { print count + 0 }' "$multi_runtime_report/metrics.tsv")" -eq 6 ]] || {
    echo "real-edit runtime multi-sample self-test expected six three-sample summaries" >&2
    exit 1
}
[[ "$(awk -F '\t' 'NR > 1 { count++ } END { print count + 0 }' "$multi_runtime_report/samples.tsv")" -eq 18 ]] || {
    echo "real-edit runtime multi-sample self-test expected eighteen raw samples" >&2
    exit 1
}
for sample in 001 002 003; do
    [[ ! -e "$multi_runtime_report/runs/$sample/target" ]] || {
        echo "runtime sample $sample retained its build cache" >&2
        exit 1
    }
    rg -q '^complete$' "$multi_runtime_report/runs/$sample/status" || {
        echo "real-edit runtime multi-sample self-test sample $sample did not complete" >&2
        exit 1
    }
done

incremental_report="$tmp_dir/runtime-incremental-disabled"
if CARGO_INCREMENTAL=0 "$runtime_measure" \
    --source-snapshot "$snapshot" \
    --report-dir "$incremental_report" \
    --host-patch "$fixture_root/host.patch" \
    --worker-patch "$fixture_root/worker.patch" \
    --host-package host \
    --worker-package worker > /dev/null 2>"$tmp_dir/runtime-incremental-disabled.err"; then
    echo "real-edit runtime accepted disabled incremental compilation" >&2
    exit 1
fi
rg -q 'CARGO_INCREMENTAL=1' "$tmp_dir/runtime-incremental-disabled.err" || {
    echo "real-edit runtime disabled-incremental failure was not explicit" >&2
    exit 1
}

bad_patch="$fixture_root/bad.patch"
printf '%s\n' 'not a patch' > "$bad_patch"
bad_report="$tmp_dir/runtime-bad-patch"
if "$runtime_measure" \
    --source-snapshot "$snapshot" \
    --report-dir "$bad_report" \
    --host-patch "$bad_patch" \
    --worker-patch "$fixture_root/worker.patch" \
    --host-package host \
    --worker-package worker > /dev/null 2>"$tmp_dir/runtime-bad-patch.err"; then
    echo "real-edit runtime invalid patch unexpectedly passed" >&2
    exit 1
fi
rg -q 'Host patch 无法应用' "$tmp_dir/runtime-bad-patch.err" || {
    echo "real-edit runtime invalid patch failure was not explicit" >&2
    exit 1
}
rg -q '^failed$' "$bad_report/status" || {
    echo "real-edit runtime invalid patch did not mark the report failed" >&2
    exit 1
}

RUNTIME_FAIL=1
export RUNTIME_FAIL
failed_build_report="$tmp_dir/runtime-build-failure"
if "$runtime_measure" \
    --source-snapshot "$snapshot" \
    --report-dir "$failed_build_report" \
    --host-patch "$fixture_root/host.patch" \
    --worker-patch "$fixture_root/worker.patch" \
    --host-package host \
    --worker-package worker > /dev/null 2>"$tmp_dir/runtime-build-failure.err"; then
    echo "real-edit runtime forced build failure unexpectedly passed" >&2
    exit 1
fi
unset RUNTIME_FAIL
rg -q 'build 失败: host_baseline' "$tmp_dir/runtime-build-failure.err" || {
    echo "real-edit runtime build failure was not explicit" >&2
    exit 1
}
rg -q '^failed$' "$failed_build_report/status" || {
    echo "real-edit runtime build failure did not mark the report failed" >&2
    exit 1
}
[[ ! -e "$failed_build_report/target" && -s "$failed_build_report/logs/host_baseline.log" ]] || {
    echo "failed build must also clean its cache and retain failure evidence" >&2
    exit 1
}

cleanup_failure_report="$tmp_dir/runtime-cleanup-failure"
if RUNTIME_CLEAN_FAIL=1 "$runtime_measure" \
    --source-snapshot "$snapshot" --report-dir "$cleanup_failure_report" \
    --host-patch "$fixture_root/host.patch" --worker-patch "$fixture_root/worker.patch" \
    --host-package host --worker-package worker > /dev/null 2>"$tmp_dir/runtime-cleanup-failure.err"; then
    echo "runtime cleanup failure incorrectly returned success" >&2
    exit 1
fi
rg -q '^failed$' "$cleanup_failure_report/status"
rg -q 'cache cleanup failed' "$tmp_dir/runtime-cleanup-failure.err"

keep_report="$tmp_dir/runtime-keep-target"
"$runtime_measure" \
    --source-snapshot "$snapshot" --report-dir "$keep_report" \
    --host-patch "$fixture_root/host.patch" --worker-patch "$fixture_root/worker.patch" \
    --host-package host --worker-package worker --keep-target >/dev/null
[[ -s "$keep_report/target/host/debug/output" ]] || {
    echo "explicit --keep-target did not retain the cache" >&2
    exit 1
}

unset -f cargo rustc
native_source="$tmp_dir/native-source"
native_report="$tmp_dir/native-report"
mkdir -p "$native_source/host/src" "$native_source/worker/src"
printf '%s\n' '[workspace]' 'resolver = "2"' 'members = ["host", "worker"]' > "$native_source/Cargo.toml"
for package in host worker; do
    printf '[package]\nname = "cache_probe_%s"\nversion = "0.1.0"\nedition = "2021"\n' "$package" > "$native_source/$package/Cargo.toml"
    printf 'fn main() { println!("baseline"); }\n' > "$native_source/$package/src/main.rs"
done
cargo generate-lockfile --offline --manifest-path "$native_source/Cargo.toml"
git init -q "$native_source"
git -C "$native_source" config user.email "cache-probe@example.invalid"
git -C "$native_source" config user.name "cache probe"
git -C "$native_source" add Cargo.toml Cargo.lock host worker
git -C "$native_source" commit -qm baseline
printf 'fn main() { println!("edited"); }\n' > "$native_source/host/src/main.rs"
git -C "$native_source" diff -- host/src/main.rs > "$tmp_dir/native-host.patch"
git -C "$native_source" restore -- host/src/main.rs
printf 'fn main() { println!("edited"); }\n' > "$native_source/worker/src/main.rs"
git -C "$native_source" diff -- worker/src/main.rs > "$tmp_dir/native-worker.patch"
git -C "$native_source" restore -- worker/src/main.rs
CARGO_BUILD_BUILD_DIR="$tmp_dir/unrelated-build-dir" "$runtime_measure" \
    --source-snapshot "$native_source" --report-dir "$native_report" \
    --host-patch "$tmp_dir/native-host.patch" --worker-patch "$tmp_dir/native-worker.patch" \
    --host-package cache_probe_host --worker-package cache_probe_worker >/dev/null
[[ ! -e "$native_report/target" && ! -e "$tmp_dir/unrelated-build-dir" ]] || {
    echo "native Cargo leaked temporary build artifacts" >&2
    exit 1
}
rg -q '^complete$' "$native_report/status"
[[ -s "$native_report/timings/host_edit/cargo-timing.html" ]]

echo "build feedback and runtime cache lifecycle checks passed (fixtures and native Cargo smoke; no performance claim)"
