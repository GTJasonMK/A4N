#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
cd "$PROJECT_ROOT"

BASELINE="$PROJECT_ROOT/scripts/checks/build-feedback-baseline.tsv"
MEASUREMENTS="$CARGO_TARGET_DIR/build-feedback/measurements.tsv"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --baseline)
            BASELINE="${2:?--baseline requires a path}"
            shift 2
            ;;
        --measurements)
            MEASUREMENTS="${2:?--measurements requires a path}"
            shift 2
            ;;
        --help|-h)
            cat <<'EOF'
用法：check-build-feedback-budget.sh [--baseline PATH] [--measurements PATH]

baseline TSV columns: package<TAB>baseline_ms<TAB>max_regression_percent
measurement TSV columns: package<TAB>median_ms
EOF
            exit 0
            ;;
        *)
            echo "未知参数: $1" >&2
            exit 2
            ;;
    esac
done

[[ -r "$BASELINE" ]] || {
    echo "缺少编译反馈基线: $BASELINE；先运行 measure-build-feedback.sh --write-baseline" >&2
    exit 1
}
[[ -r "$MEASUREMENTS" ]] || {
    echo "缺少编译反馈测量: $MEASUREMENTS；先运行 measure-build-feedback.sh" >&2
    exit 1
}

python3 - "$BASELINE" "$MEASUREMENTS" <<'PY'
from pathlib import Path
import math
import sys

baseline_path = Path(sys.argv[1])
measurement_path = Path(sys.argv[2])
method_prefix = "# method\t"


def method(path: Path):
    methods = [
        line.removeprefix(method_prefix).strip()
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.startswith(method_prefix)
    ]
    if len(methods) != 1 or not methods[0]:
        raise SystemExit(f"{path}: expected exactly one build feedback method")
    return methods[0]


def rows(path: Path, expected_fields: int):
    result = {}
    for line_number, raw in enumerate(
        path.read_text(encoding="utf-8").splitlines(), 1
    ):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        fields = line.split("\t")
        if len(fields) != expected_fields:
            raise SystemExit(
                f"{path}:{line_number}: expected exactly {expected_fields} "
                "tab-separated fields"
            )
        key = fields[0]
        if key in result:
            raise SystemExit(f"{path}:{line_number}: duplicate package {key}")
        try:
            value = float(fields[1])
        except ValueError as error:
            raise SystemExit(
                f"{path}:{line_number}: invalid milliseconds: {fields[1]}"
            ) from error
        if not math.isfinite(value) or value <= 0:
            raise SystemExit(f"{path}:{line_number}: milliseconds must be positive")
        result[key] = (value, fields[2:])
    return result


baseline = rows(baseline_path, 3)
measurements = rows(measurement_path, 2)
if not baseline:
    raise SystemExit("build feedback baseline has no packages")
baseline_method = method(baseline_path)
measurement_method = method(measurement_path)
if baseline_method != measurement_method:
    raise SystemExit(
        "build feedback method mismatch: "
        f"baseline={baseline_method}, measurement={measurement_method}"
    )
missing = sorted(set(baseline) - set(measurements))
if missing:
    raise SystemExit("missing measurements: " + ", ".join(missing))

if "__workspace_cold__" not in measurements:
    raise SystemExit(
        "measurements must include __workspace_cold__; do not mix a warm-only run into the CI budget"
    )

unexpected = sorted(set(measurements) - set(baseline) - {"__workspace_cold__"})
if unexpected:
    raise SystemExit("unexpected build feedback measurements: " + ", ".join(unexpected))

print(
    f"method={measurement_method}\n__workspace_cold__\tmeasured="
    f"{measurements['__workspace_cold__'][0]:.0f}ms"
)

failed = []
for package, (base, extra) in baseline.items():
    if not extra:
        raise SystemExit(
            f"{baseline_path}: {package} needs max_regression_percent"
        )
    try:
        percent = float(extra[0])
    except ValueError as error:
        raise SystemExit(
            f"{baseline_path}: invalid regression percent for {package}"
        ) from error
    if not math.isfinite(percent) or percent < 0:
        raise SystemExit(
            f"{baseline_path}: regression percent must be non-negative for {package}"
        )
    measured = measurements[package][0]
    budget = math.ceil(base * (1 + percent / 100))
    print(
        f"{package}\tbaseline={base:.0f}ms\t"
        f"measured={measured:.0f}ms\tbudget={budget}ms"
    )
    if measured > budget:
        failed.append(f"{package}: {measured:.0f}ms > {budget}ms")

if failed:
    raise SystemExit("build feedback budget exceeded:\n" + "\n".join(failed))
PY

echo "build feedback budget passed"
