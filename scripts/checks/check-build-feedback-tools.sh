#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

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

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/a4n-build-feedback-tools.XXXXXX")"
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
"$SCRIPT_DIR/measure-build-feedback.sh" \
    --runs 3 \
    --artifact-dir "$artifact_dir" \
    --output "$measurements" \
    --write-baseline "$baseline" >/dev/null

run_dir="$(find "$artifact_dir/runs" -mindepth 1 -maxdepth 1 -type d -print -quit)"
[[ -n "$run_dir" && -f "$run_dir/metadata.txt" && -f "$run_dir/samples.tsv" ]] || {
    echo "build-feedback run metadata or raw samples are missing" >&2
    exit 1
}
[[ "$(find "$run_dir/timings" -mindepth 1 -maxdepth 1 -type d | wc -l)" -eq 33 ]] || {
    echo "build-feedback self-test expected cold + eight warmups + 24 measured timing directories" >&2
    exit 1
}
[[ "$(find "$run_dir/logs" -maxdepth 1 -type f -name '*-reset.log' | wc -l)" -eq 24 ]] || {
    echo "build-feedback self-test expected one package reset before every measured warm sample" >&2
    exit 1
}
[[ "$(find "$run_dir/dependencies" -maxdepth 1 -type f -name '*.txt' | wc -l)" -eq 8 ]] || {
    echo "build-feedback self-test expected one direct dependency inventory per package" >&2
    exit 1
}
[[ "$(awk -F '\t' '$1 == "a4n-prompt-service" && $2 ~ /^[1-3]$/ { count++ } END { print count + 0 }' "$run_dir/samples.tsv")" -eq 3 ]] || {
    echo "build-feedback self-test expected three raw warm samples" >&2
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

sed 's/package-rebuild-v1/noop-v0/' "$baseline" > "$tmp_dir/wrong-method.tsv"
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

echo "build-feedback measurement and budget self-test passed"
