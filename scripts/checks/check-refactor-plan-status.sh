#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

python3 - <<'PY'
from pathlib import Path
from collections import Counter
import re

plan = Path("docs/重构/模块化重构执行计划-v2.md")
progress = Path("docs/重构/模块化重构进度记录.md")
workpack_readme = Path("docs/重构/剩余重构/README.md")
allowed = {"完成", "未完成"}
status_re = re.compile(r"^\s*(?:当前整体)?状态：\s*`([^`]+)`\s*$")


def check_status_values(path: Path) -> None:
    text = path.read_text(encoding="utf-8")
    invalid = [
        (line_number, match.group(1))
        for line_number, line in enumerate(text.splitlines(), 1)
        for match in [status_re.fullmatch(line)]
        if match and match.group(1) not in allowed
    ]
    if invalid:
        details = ", ".join(
            f"{path}:{line_number}={value}" for line_number, value in invalid
        )
        raise SystemExit(f"invalid refactor status value: {details}")


def body_after_heading(text: str, match: re.Match[str]) -> str:
    next_heading = re.search(r"(?m)^#{1,4} ", text[match.end() :])
    end = match.end() + next_heading.start() if next_heading else len(text)
    return text[match.end() : end]


def sections(text: str):
    targets = list(
        re.finditer(r"(?m)^### (?P<name>(?:3\.\d+|阶段 [A-M])\b.*)$", text)
    )
    for match in targets:
        yield match.group("name"), body_after_heading(text, match)


def unique_section_status(text: str, pattern: str, label: str) -> str:
    matches = list(re.finditer(pattern, text, re.MULTILINE))
    if len(matches) != 1:
        raise SystemExit(f"{label} must appear exactly once; found {len(matches)}")
    statuses = [
        match.group(1)
        for line in body_after_heading(text, matches[0]).splitlines()
        for match in [status_re.fullmatch(line)]
        if match
    ]
    if len(statuses) != 1 or statuses[0] not in allowed:
        raise SystemExit(
            f"{label} must contain exactly one valid status; found {statuses}"
        )
    return statuses[0]


plan_text = plan.read_text(encoding="utf-8")
required = {f"3.{number}" for number in range(1, 30)}
required_stages = {f"阶段 {letter}" for letter in "ABCDEFGHIJKLM"}
found = []
found_stages = []
definition_statuses = {}
definition_bodies = {}
stage_statuses = {}
for name, body in sections(plan_text):
    key = name.split()[0]
    if key.startswith("3."):
        found.append(key)
    elif name.startswith("阶段 "):
        stage = re.match(r"^(阶段 [A-M])\b", name)
        if stage:
            found_stages.append(stage.group(1))
    statuses = [
        match.group(1)
        for line in body.splitlines()
        for match in [status_re.fullmatch(line)]
        if match
    ]
    if len(statuses) != 1:
        raise SystemExit(
            f"{name} must contain exactly one status line; found {len(statuses)}"
        )
    if statuses[0] not in allowed:
        raise SystemExit(f"{name} has invalid status: {statuses[0]}")
    if key.startswith("3."):
        definition_statuses[key] = statuses[0]
        definition_bodies[key] = body
    else:
        stage_statuses[re.match(r"^阶段 [A-M]", name).group(0)] = statuses[0]

found_counts = Counter(found)
stage_counts = Counter(found_stages)
duplicates = sorted(
    (name for name, count in found_counts.items() if count != 1),
    key=lambda value: int(value.split(".")[1]),
)
if duplicates:
    raise SystemExit("duplicate V2 sections: " + ", ".join(duplicates))
duplicate_stages = sorted(name for name, count in stage_counts.items() if count != 1)
if duplicate_stages:
    raise SystemExit("duplicate refactor stages: " + ", ".join(duplicate_stages))

unexpected = sorted(
    set(found_counts) - required,
    key=lambda value: int(value.split(".")[1]),
)
if unexpected:
    raise SystemExit("unexpected V2 sections: " + ", ".join(unexpected))

missing = sorted(required - set(found_counts), key=lambda value: int(value.split(".")[1]))
if missing:
    raise SystemExit("missing V2 sections: " + ", ".join(missing))

missing_stages = sorted(required_stages - set(stage_counts))
if missing_stages:
    raise SystemExit("missing refactor stages: " + ", ".join(missing_stages))

check_status_values(plan)
check_status_values(progress)

a1_status = unique_section_status(
    plan_text, r"^#### A1\. 取消 host_api 通配导出$", "阶段 A1"
)
final_status = unique_section_status(
    plan_text, r"^## 7\. 最终完成判定$", "最终完成判定"
)
status_entries = [
    (line_number, match.group(1), line.strip().startswith("当前整体状态："))
    for line_number, line in enumerate(plan_text.splitlines(), 1)
    for match in [status_re.fullmatch(line)]
    if match
]
expected_total = len(required) + len(required_stages) + 2
if len(status_entries) != expected_total:
    raise SystemExit(
        f"refactor plan must contain exactly {expected_total} current statuses; "
        f"found {len(status_entries)}"
    )
overall_entries = [entry for entry in status_entries if entry[2]]
if len(overall_entries) != 1:
    raise SystemExit(
        f"refactor plan must contain exactly one current overall status; "
        f"found {len(overall_entries)}"
    )

prerequisites = [
    definition_statuses[f"3.{number}"] for number in range(1, 29)
] + [stage_statuses[f"阶段 {letter}"] for letter in "ABCDEFGHIJKLM"]
expected_329 = "完成" if all(value == "完成" for value in prerequisites) else "未完成"
if definition_statuses["3.29"] != expected_329:
    raise SystemExit(
        f"3.29 status must be {expected_329} based on definitions 3.1-3.28 and stages A-M"
    )
expected_incomplete_definitions = [
    f"3.{number}"
    for number in range(1, 29)
    if definition_statuses[f"3.{number}"] == "未完成"
]
listed_incomplete_definitions = re.findall(
    r"(?m)^(3\.\d+)\s", definition_bodies["3.29"]
)
if listed_incomplete_definitions != expected_incomplete_definitions:
    raise SystemExit(
        "3.29 incomplete definition list is stale: expected "
        + ", ".join(expected_incomplete_definitions)
        + "; found "
        + ", ".join(listed_incomplete_definitions)
    )
if stage_statuses["阶段 A"] == "完成" and a1_status != "完成":
    raise SystemExit("阶段 A cannot be complete while A1 is incomplete")

non_overall = [value for _, value, is_overall in status_entries if not is_overall]
expected_final = "完成" if all(value == "完成" for value in non_overall) else "未完成"
if final_status != expected_final:
    raise SystemExit(
        f"final overall status must be {expected_final} based on the other statuses"
    )

completed = sum(value == "完成" for _, value, _ in status_entries)
incomplete = expected_total - completed
summary_markers = {
    plan: f"V2：{completed} / {expected_total} 完成，{incomplete} 条未完成",
    progress: (
        f"| **V2 完成定义条目** | **{completed} / {expected_total} 完成，"
        f"还差 {incomplete}** |"
    ),
    workpack_readme: (
        f"| V2 完成定义 | {completed} / {expected_total}，未完成 {incomplete} 条 |"
    ),
}
for path, marker in summary_markers.items():
    marker_count = path.read_text(encoding="utf-8").count(marker)
    if marker_count != 1:
        raise SystemExit(
            f"refactor status summary is stale or duplicated in {path}: "
            f"expected exactly one {marker}; found {marker_count}"
        )

source = "当前执行依据：`docs/重构/模块化重构执行计划-v2.md`"
if source not in progress.read_text(encoding="utf-8"):
    raise SystemExit("progress record must name the V2 plan as its current source")

print("refactor plan sections, status values, and current-source marker are valid")
PY

"$SCRIPT_DIR/check-refactor-docs.sh"
