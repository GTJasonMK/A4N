#!/usr/bin/env python3
"""Validate the functional test catalog and repository test entry points."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import Any

from run_test_matrix import MatrixError, load_matrix, resolve_profile as resolve_matrix_profile


ROOT = Path(__file__).resolve().parents[2]
SCENARIO_SOURCE = ROOT / "crates/host/a4n-runtime/src/actions/admin_records/test_console.rs"
MATRIX_SOURCE = ROOT / "scripts/tests/test_matrix.json"
PACKAGE_SOURCE = ROOT / "frontend/package.json"
SCENARIO_RE = re.compile(r'\bscenario\(\s*"([a-z][a-z0-9_]*)"')
HANDLER_RE = re.compile(r'"([a-z][a-z0-9_]*)"\s*=>')


class CatalogError(RuntimeError):
    pass


def read_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise CatalogError(f"无法读取 {path.relative_to(ROOT)}：{error}") from error
    if not isinstance(value, dict):
        raise CatalogError(f"{path.relative_to(ROOT)} 必须是对象")
    return value


def scenario_catalog() -> list[str]:
    try:
        source = SCENARIO_SOURCE.read_text(encoding="utf-8")
    except OSError as error:
        raise CatalogError(f"无法读取场景注册表：{error}") from error
    declared = SCENARIO_RE.findall(source)
    handler_source = source.split("async fn run_scenario", 1)
    if len(handler_source) != 2:
        raise CatalogError("找不到 run_scenario handler")
    handler_source = handler_source[1].split("async fn run_repository_test_group", 1)
    handlers = HANDLER_RE.findall(handler_source[0])
    if not declared:
        raise CatalogError("场景注册表没有声明任何 scenario")
    duplicates = sorted({item for item in declared if declared.count(item) > 1})
    if duplicates:
        raise CatalogError(f"场景 ID 重复：{duplicates}")
    missing_handlers = sorted(set(declared) - set(handlers))
    if missing_handlers:
        raise CatalogError(f"场景没有 handler 分支：{missing_handlers}")
    orphan_handlers = sorted(set(handlers) - set(declared))
    if orphan_handlers:
        raise CatalogError(f"handler 没有注册场景：{orphan_handlers}")
    matrix = read_json(MATRIX_SOURCE)
    test_center = matrix.get("test_center")
    groups = test_center.get("groups", []) if isinstance(test_center, dict) else []
    dynamic = [
        f"repository_test_{group['id']}"
        for group in groups
        if isinstance(group, dict) and isinstance(group.get("id"), str)
    ]
    if dynamic and (
        "strip_prefix(REPOSITORY_TEST_PREFIX)" not in source
        or "run_repository_test_group" not in source
    ):
        raise CatalogError("仓库功能测试分组缺少通用执行 handler")
    return [*declared, *dynamic]


def resolve_profile(profiles: dict[str, Any], suites: dict[str, Any], name: str, stack: tuple[str, ...] = ()) -> list[str]:
    try:
        return resolve_matrix_profile(profiles, suites, name, stack)
    except MatrixError as error:
        raise CatalogError(str(error)) from error


def check_matrix() -> dict[str, Any]:
    try:
        matrix = load_matrix(MATRIX_SOURCE)
    except MatrixError as error:
        raise CatalogError(str(error)) from error
    suites = matrix["suites"]
    profiles = matrix["profiles"]
    resolved = {name: resolve_profile(profiles, suites, name) for name in profiles}
    test_center = matrix.get("test_center")
    if not isinstance(test_center, dict):
        raise CatalogError("测试矩阵必须包含 test_center 对象")
    return {
        "suite_count": len(suites),
        "profiles": resolved,
        "test_center_group_count": len(test_center["groups"]),
        "functional_suite_count": len(test_center["functional_suites"]),
    }


def check_package_scripts() -> list[str]:
    package = read_json(PACKAGE_SOURCE)
    scripts = package.get("scripts")
    if not isinstance(scripts, dict):
        raise CatalogError("frontend/package.json 缺少 scripts")
    matrix = read_json(MATRIX_SOURCE)
    referenced: set[str] = set()
    for suite in matrix["suites"].values():
        command = suite["command"]
        if command[:3] == ["npm", "--prefix", "frontend"] and len(command) >= 5 and command[3] == "run":
            referenced.add(command[4])
    missing = sorted(name for name in referenced if name not in scripts)
    if missing:
        raise CatalogError(f"测试矩阵引用了不存在的 npm script：{missing}")
    return sorted(referenced)


def build_report() -> dict[str, Any]:
    scenarios = scenario_catalog()
    matrix = check_matrix()
    scripts = check_package_scripts()
    return {
        "scenario_count": len(scenarios),
        "scenario_ids": scenarios,
        "matrix": matrix,
        "matrix_npm_scripts": scripts,
    }


def main() -> int:
    try:
        print(json.dumps(build_report(), ensure_ascii=False, indent=2))
        return 0
    except CatalogError as error:
        print(f"测试目录检查失败：{error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
