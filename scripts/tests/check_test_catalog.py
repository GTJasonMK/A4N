#!/usr/bin/env python3
"""Validate the functional test catalog and repository test entry points."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import Any


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
    handlers = HANDLER_RE.findall(source)
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
    return declared


def resolve_profile(profiles: dict[str, Any], suites: dict[str, Any], name: str, stack: tuple[str, ...] = ()) -> list[str]:
    if name in stack:
        raise CatalogError(f"测试 profile 继承循环：{' -> '.join((*stack, name))}")
    definition = profiles.get(name)
    if not isinstance(definition, dict):
        raise CatalogError(f"测试 profile 无效：{name}")
    parent = definition.get("extends")
    inherited = resolve_profile(profiles, suites, parent, (*stack, name)) if parent else []
    names = definition.get("suites", [])
    excluded = definition.get("exclude", [])
    if not isinstance(names, list) or not isinstance(excluded, list):
        raise CatalogError(f"测试 profile suite 必须是数组：{name}")
    unknown = sorted((set(names) | set(excluded)) - set(suites))
    if unknown:
        raise CatalogError(f"测试 profile 引用了未知 suite：{unknown}")
    resolved = [item for item in dict.fromkeys([*inherited, *names]) if item not in set(excluded)]
    if not resolved:
        raise CatalogError(f"测试 profile 为空：{name}")
    return resolved


def check_matrix() -> dict[str, Any]:
    matrix = read_json(MATRIX_SOURCE)
    suites = matrix.get("suites")
    profiles = matrix.get("profiles")
    if not isinstance(suites, dict) or not isinstance(profiles, dict):
        raise CatalogError("测试矩阵必须包含 suites 和 profiles")
    for name, suite in suites.items():
        if not isinstance(suite, dict) or not isinstance(suite.get("command"), list) or not suite["command"]:
            raise CatalogError(f"suite 命令无效：{name}")
    resolved = {name: resolve_profile(profiles, suites, name) for name in profiles}
    return {"suite_count": len(suites), "profiles": resolved}


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
