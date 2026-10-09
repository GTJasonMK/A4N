#!/usr/bin/env python3
"""Run the repository's declared test suites with one fail-closed report."""

from __future__ import annotations

import argparse
import json
import os
import secrets
import signal
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
MATRIX_PATH = Path(__file__).with_name("test_matrix.json")


class MatrixError(RuntimeError):
    pass


def load_matrix(path: Path = MATRIX_PATH) -> dict[str, Any]:
    try:
        matrix = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise MatrixError(f"无法读取测试矩阵：{error}") from error
    if matrix.get("schema_version") != 1:
        raise MatrixError("测试矩阵 schema_version 必须是 1")
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
        if not isinstance(timeout, int) or timeout <= 0:
            raise MatrixError(f"suite {name} 的 timeout_seconds 无效")
    for profile, names in profiles.items():
        if not isinstance(names, list) or not names:
            raise MatrixError(f"profile {profile} 必须包含 suite")
        unknown = sorted(set(names) - set(suites))
        if unknown:
            raise MatrixError(f"profile {profile} 引用了未知 suite：{unknown}")
    return matrix


def selected_suites(matrix: dict[str, Any], profile: str, requested: list[str]) -> list[str]:
    suites = matrix["suites"]
    profiles = matrix["profiles"]
    if profile not in profiles:
        raise MatrixError(f"未知测试 profile：{profile}")
    names = list(profiles[profile])
    unknown = sorted(set(requested) - set(suites))
    if unknown:
        raise MatrixError(f"未知 suite：{unknown}")
    if requested:
        names = requested
    if not names:
        raise MatrixError("没有选择测试 suite")
    return names


def safe_report_dir(path: Path) -> Path:
    resolved = path.expanduser().resolve()
    if resolved == Path("/"):
        raise MatrixError("报告目录不能是根目录")
    if resolved.exists() and any(resolved.iterdir()):
        raise MatrixError(f"报告目录必须为空，避免覆盖旧证据：{resolved}")
    resolved.mkdir(parents=True, exist_ok=True)
    return resolved


def terminate_process(process: subprocess.Popen[bytes]) -> None:
    if process.poll() is not None:
        return
    try:
        os.killpg(process.pid, signal.SIGTERM)
        process.wait(timeout=5)
    except (ProcessLookupError, subprocess.TimeoutExpired):
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass


def run_suite(name: str, suite: dict[str, Any], report_dir: Path) -> dict[str, Any]:
    log_path = report_dir / "logs" / f"{name}.log"
    log_path.parent.mkdir(parents=True, exist_ok=True)
    command = list(suite["command"])
    started = time.monotonic()
    result: dict[str, Any] = {
        "name": name,
        "command": command,
        "status": "failed",
        "exit_code": None,
        "timeout_seconds": suite["timeout_seconds"],
        "log": str(log_path),
    }
    try:
        with log_path.open("wb") as log:
            process = subprocess.Popen(
                command,
                cwd=ROOT,
                env=os.environ.copy(),
                stdin=subprocess.DEVNULL,
                stdout=log,
                stderr=subprocess.STDOUT,
                start_new_session=True,
            )
            try:
                exit_code = process.wait(timeout=suite["timeout_seconds"])
            except subprocess.TimeoutExpired:
                terminate_process(process)
                result.update(status="timeout", exit_code=124)
                return result
            result["exit_code"] = exit_code
            result["status"] = "passed" if exit_code == 0 else "failed"
    except OSError as error:
        result["error"] = str(error)
    finally:
        result["duration_seconds"] = round(time.monotonic() - started, 3)
    return result


def write_report(path: Path, report: dict[str, Any]) -> None:
    path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=("fast", "nightly", "full"), default="fast")
    parser.add_argument("--suite", action="append", default=[])
    parser.add_argument("--report-dir", type=Path)
    parser.add_argument("--list", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        matrix = load_matrix()
        names = selected_suites(matrix, args.profile, args.suite)
        if args.list:
            print(json.dumps({"profile": args.profile, "suites": names}, ensure_ascii=False, indent=2))
            return 0
        if args.report_dir:
            report_dir = safe_report_dir(args.report_dir)
        else:
            run_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + secrets.token_hex(4)
            report_dir = safe_report_dir(ROOT / "data" / "test-runs" / run_id)
        if args.dry_run:
            print(json.dumps({"profile": args.profile, "suites": names, "report_dir": str(report_dir)}, ensure_ascii=False, indent=2))
            return 0
        report: dict[str, Any] = {
            "schema_version": 1,
            "profile": args.profile,
            "started_at": datetime.now(timezone.utc).isoformat(),
            "report_dir": str(report_dir),
            "suites": {},
            "success": False,
        }
        for name in names:
            result = run_suite(name, matrix["suites"][name], report_dir)
            report["suites"][name] = result
            print(json.dumps({"suite": name, "status": result["status"]}, ensure_ascii=False), flush=True)
        report["success"] = len(report["suites"]) == len(names) and all(
            item["status"] == "passed" for item in report["suites"].values()
        )
        report["finished_at"] = datetime.now(timezone.utc).isoformat()
        write_report(report_dir / "matrix.json", report)
        print(json.dumps({"success": report["success"], "report": str(report_dir / "matrix.json")}, ensure_ascii=False), flush=True)
        return 0 if report["success"] else 1
    except MatrixError as error:
        print(f"测试矩阵错误：{error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
