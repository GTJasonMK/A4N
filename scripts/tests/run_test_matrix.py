#!/usr/bin/env python3
"""Run the repository's declared test suites with one fail-closed report."""

from __future__ import annotations

import argparse
import json
import os
import re
import secrets
import signal
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from runner_support import (
    ReportDirectoryError,
    private_file,
    raise_interrupted,
    safe_report_dir,
    terminate_process,
    validate_report_dir,
)


ROOT = Path(__file__).resolve().parents[2]
MATRIX_PATH = Path(__file__).with_name("test_matrix.json")
SUITE_CLEANUP_GRACE_SECONDS = 60


class MatrixError(RuntimeError):
    pass


def load_matrix(path: Path = MATRIX_PATH) -> dict[str, Any]:
    try:
        matrix = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise MatrixError(f"无法读取测试矩阵：{error}") from error
    if not isinstance(matrix, dict) or matrix.get("schema_version") != 2:
        raise MatrixError("测试矩阵 schema_version 必须是 2")
    suites = matrix.get("suites")
    profiles = matrix.get("profiles")
    if not isinstance(suites, dict) or not isinstance(profiles, dict):
        raise MatrixError("测试矩阵必须包含 suites 和 profiles")
    for name, suite in suites.items():
        if not isinstance(name, str) or not isinstance(suite, dict):
            raise MatrixError("suite 定义必须是对象")
        command = suite.get("command")
        timeout = suite.get("timeout_seconds")
        if (
            not isinstance(command, list)
            or not command
            or not all(isinstance(part, str) and part for part in command)
        ):
            raise MatrixError(f"suite {name} 的 command 无效")
        if not isinstance(timeout, int) or isinstance(timeout, bool) or timeout <= 0:
            raise MatrixError(f"suite {name} 的 timeout_seconds 无效")
    for profile in profiles:
        resolve_profile(profiles, suites, profile)
    validate_test_center_catalog(matrix, suites)
    return matrix


def validate_test_center_catalog(matrix: dict[str, Any], suites: dict[str, Any]) -> None:
    catalog = matrix.get("test_center")
    if catalog is None:
        return
    if not isinstance(catalog, dict):
        raise MatrixError("test_center 必须是对象")

    functional_suites = catalog.get("functional_suites")
    groups = catalog.get("groups")
    if (
        not isinstance(functional_suites, list)
        or not functional_suites
        or not all(isinstance(name, str) for name in functional_suites)
    ):
        raise MatrixError("test_center.functional_suites 必须是非空 suite 列表")
    if len(functional_suites) != len(set(functional_suites)):
        raise MatrixError("test_center.functional_suites 不能重复")
    if not isinstance(groups, list) or not groups:
        raise MatrixError("test_center.groups 必须是非空数组")

    group_ids: set[str] = set()
    grouped_suites: set[str] = set()
    all_functional_suites: list[str] | None = None
    for group in groups:
        if not isinstance(group, dict):
            raise MatrixError("test_center group 必须是对象")
        group_id = group.get("id")
        suite_names = group.get("suites")
        if not isinstance(group_id, str) or not re.fullmatch(r"[a-z0-9][a-z0-9-]*", group_id):
            raise MatrixError("test_center group id 无效")
        if group_id in group_ids:
            raise MatrixError(f"test_center group id 重复：{group_id}")
        group_ids.add(group_id)
        for field in ("category", "title", "description"):
            if not isinstance(group.get(field), str) or not group[field].strip():
                raise MatrixError(f"test_center group {group_id} 的 {field} 无效")
        if (
            not isinstance(suite_names, list)
            or not suite_names
            or not all(isinstance(name, str) for name in suite_names)
        ):
            raise MatrixError(f"test_center group {group_id} 必须包含 suite")
        if len(suite_names) != len(set(suite_names)):
            raise MatrixError(f"test_center group {group_id} 的 suite 不能重复")
        unknown = sorted(set(suite_names) - set(suites))
        if unknown:
            raise MatrixError(f"test_center group {group_id} 引用了未知 suite：{unknown}")
        if group_id == "all-functional":
            all_functional_suites = suite_names
        else:
            overlap = grouped_suites.intersection(suite_names)
            if overlap:
                raise MatrixError(f"功能 suite 被多个 test_center 分组重复引用：{sorted(overlap)}")
            grouped_suites.update(suite_names)

    unknown_functional = sorted(set(functional_suites) - set(suites))
    if unknown_functional:
        raise MatrixError(f"test_center 引用了未知功能 suite：{unknown_functional}")
    if grouped_suites != set(functional_suites):
        missing = sorted(set(functional_suites) - grouped_suites)
        extra = sorted(grouped_suites - set(functional_suites))
        raise MatrixError(f"test_center 分组覆盖不一致；缺少 {missing}，多出 {extra}")
    if all_functional_suites != functional_suites:
        raise MatrixError("all-functional 必须按登记顺序完整包含 functional_suites")


def resolve_profile(
    profiles: dict[str, Any], suites: dict[str, Any], profile: str, stack: tuple[str, ...] = ()
) -> list[str]:
    if profile not in profiles:
        raise MatrixError(f"未知测试 profile：{profile}")
    if profile in stack:
        raise MatrixError(f"测试 profile 继承形成循环：{' -> '.join((*stack, profile))}")
    definition = profiles[profile]
    if not isinstance(definition, dict):
        raise MatrixError(f"profile {profile} 必须是继承定义")
    parent = definition.get("extends")
    if parent is not None and (not isinstance(parent, str) or not parent):
        raise MatrixError(f"profile {profile} 的 extends 无效")
    inherited = resolve_profile(profiles, suites, parent, (*stack, profile)) if parent else []
    names = definition.get("suites", [])
    excluded = definition.get("exclude", [])
    if not isinstance(names, list) or not isinstance(excluded, list):
        raise MatrixError(f"profile {profile} 必须包含 suite")
    if not all(isinstance(name, str) for name in (*names, *excluded)):
        raise MatrixError(f"profile {profile} 必须使用字符串 suite 名称")
    unknown = sorted((set(names) | set(excluded)) - set(suites))
    if unknown:
        raise MatrixError(f"profile {profile} 引用了未知 suite：{unknown}")
    excluded_set = set(excluded)
    resolved = [name for name in dict.fromkeys([*inherited, *names]) if name not in excluded_set]
    if not resolved:
        raise MatrixError(f"profile {profile} 必须包含 suite")
    return resolved


def selected_suites(matrix: dict[str, Any], profile: str, requested: list[str]) -> list[str]:
    suites = matrix["suites"]
    profiles = matrix["profiles"]
    names = resolve_profile(profiles, suites, profile)
    unknown = sorted(set(requested) - set(suites))
    if unknown:
        raise MatrixError(f"未知 suite：{unknown}")
    if requested:
        names = list(dict.fromkeys(requested))
    if not names:
        raise MatrixError("没有选择测试 suite")
    return names


def run_suite(name: str, suite: dict[str, Any], report_dir: Path) -> dict[str, Any]:
    log_path = report_dir / "logs" / f"{name}.log"
    command = [part.replace("{report_dir}", str(report_dir)) for part in suite["command"]]
    started = time.monotonic()
    result: dict[str, Any] = {
        "name": name,
        "command": command,
        "status": "failed",
        "exit_code": None,
        "timeout_seconds": suite["timeout_seconds"],
        "log": str(log_path),
    }
    process: subprocess.Popen[bytes] | None = None
    try:
        env = os.environ.copy()
        if name.startswith("browser-"):
            env["A4N_BROWSER_E2E_ARTIFACT_DIR"] = str(report_dir / "browser-artifacts" / name)
            for key in (
                "HTTP_PROXY",
                "HTTPS_PROXY",
                "ALL_PROXY",
                "http_proxy",
                "https_proxy",
                "all_proxy",
                "NODE_USE_ENV_PROXY",
            ):
                env.pop(key, None)
            no_proxy = [
                value.strip()
                for key in ("NO_PROXY", "no_proxy")
                for value in env.get(key, "").split(",")
                if value.strip()
            ]
            no_proxy.extend(("localhost", "127.0.0.1", "::1"))
            local_bypass = ",".join(dict.fromkeys(no_proxy))
            env["NO_PROXY"] = local_bypass
            env["no_proxy"] = local_bypass
        with private_file(log_path) as log:
            process = subprocess.Popen(
                command,
                cwd=ROOT,
                env=env,
                stdin=subprocess.DEVNULL,
                stdout=log,
                stderr=subprocess.STDOUT,
                start_new_session=True,
            )
            try:
                exit_code = process.wait(timeout=suite["timeout_seconds"])
            except subprocess.TimeoutExpired:
                terminate_process(process, grace_seconds=SUITE_CLEANUP_GRACE_SECONDS)
                result.update(status="timeout", exit_code=124)
                return result
            result["exit_code"] = exit_code
            result["status"] = "passed" if exit_code == 0 else "failed"
    except OSError as error:
        result["error"] = str(error)
    except KeyboardInterrupt:
        if process is not None:
            terminate_process(process, grace_seconds=SUITE_CLEANUP_GRACE_SECONDS)
        result.update(status="interrupted", exit_code=130)
    finally:
        result["duration_seconds"] = round(time.monotonic() - started, 3)
    return result


def write_report(path: Path, report: dict[str, Any]) -> None:
    with private_file(path) as output:
        output.write(json.dumps(report, ensure_ascii=False, indent=2).encode("utf-8") + b"\n")


def parse_args(profiles: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=profiles, default="fast")
    parser.add_argument("--suite", action="append", default=[])
    parser.add_argument("--report-dir", type=Path)
    parser.add_argument("--list", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--verify-model-report",type=Path)
    return parser.parse_args()


def main() -> int:
    signal.signal(signal.SIGTERM, raise_interrupted)
    try:
        matrix = load_matrix()
        args = parse_args(list(matrix["profiles"]))
        if args.verify_model_report:
            from model_evidence import verify_report
            try:result=verify_report(args.verify_model_report)
            except (OSError,ValueError,KeyError,TypeError) as error:raise MatrixError("模型验收报告无效："+str(error)) from error
            print(json.dumps(result,ensure_ascii=False,indent=2))
            return 0
        names = selected_suites(matrix, args.profile, args.suite)
        if args.list:
            print(json.dumps({"profile": args.profile, "suites": names}, ensure_ascii=False, indent=2))
            return 0
        if args.dry_run:
            if args.report_dir:
                report_dir = validate_report_dir(args.report_dir)
            else:
                run_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + secrets.token_hex(4)
                report_dir = ROOT / "data" / "test-runs" / run_id
            print(json.dumps({"profile": args.profile, "suites": names, "report_dir": str(report_dir)}, ensure_ascii=False, indent=2))
            return 0
        if args.report_dir:
            report_dir = safe_report_dir(args.report_dir)
        else:
            run_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + secrets.token_hex(4)
            report_dir = safe_report_dir(ROOT / "data" / "test-runs" / run_id)
        report: dict[str, Any] = {
            "schema_version": 1,
            "profile": args.profile,
            "started_at": datetime.now(timezone.utc).isoformat(),
            "report_dir": str(report_dir),
            "suites": {},
            "success": False,
        }
        try:
            for name in names:
                print(json.dumps({"suite": name, "status": "running"}, ensure_ascii=False), flush=True)
                result = run_suite(name, matrix["suites"][name], report_dir)
                report["suites"][name] = result
                print(json.dumps({"suite": name, "status": result["status"]}, ensure_ascii=False), flush=True)
                if result["status"] == "interrupted":
                    break
        except KeyboardInterrupt:
            report["interrupted"] = True
        signal.signal(signal.SIGTERM, signal.SIG_IGN)
        report["success"] = len(report["suites"]) == len(names) and all(
            item["status"] == "passed" for item in report["suites"].values()
        )
        report["finished_at"] = datetime.now(timezone.utc).isoformat()
        write_report(report_dir / "matrix.json", report)
        print(json.dumps({"success": report["success"], "report": str(report_dir / "matrix.json")}, ensure_ascii=False), flush=True)
        return 0 if report["success"] else 1
    except (MatrixError, ReportDirectoryError) as error:
        print(f"测试矩阵错误：{error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
