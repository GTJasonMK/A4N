#!/usr/bin/env python3
"""Run one command against a disposable PostgreSQL test database."""

from __future__ import annotations

import argparse
import json
import os
import secrets
import signal
import shutil
import socket
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import quote

from runner_support import (
    ReportDirectoryError,
    private_file,
    raise_interrupted,
    safe_report_dir,
    terminate_process,
)


ROOT = Path(__file__).resolve().parents[2]


class DisposableDatabaseError(RuntimeError):
    pass


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        listener.bind(("127.0.0.1", 0))
        return int(listener.getsockname()[1])


def build_database_url(port: int, password: str, database: str = "a4n_test") -> str:
    return f"postgres://a4n_test:{quote(password, safe='')}@127.0.0.1:{port}/{database}"


def container_command(container: str, port: int, image: str) -> list[str]:
    return [
        "docker",
        "run",
        "--detach",
        "--name",
        container,
        "--shm-size=256m",
        "--env",
        "POSTGRES_USER=a4n_test",
        "--env",
        "POSTGRES_PASSWORD",
        "--env",
        "POSTGRES_DB=a4n_test",
        "--publish",
        f"127.0.0.1:{port}:5432",
        image,
        "-c",
        "max_connections=300",
    ]


def remove_container_command(container: str) -> list[str]:
    return ["docker", "rm", "--volumes", "--force", container]


def wait_until_ready(container: str, log: object, timeout: float) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            result = subprocess.run(
                ["docker", "exec", container, "pg_isready", "-h", "127.0.0.1", "-U", "a4n_test", "-d", "a4n_test"],
                stdout=log,
                stderr=subprocess.STDOUT,
                check=False,
                timeout=5,
            )
        except subprocess.TimeoutExpired:
            continue
        if result.returncode == 0:
            return
        time.sleep(0.2)
    raise DisposableDatabaseError(f"PostgreSQL 未在 {timeout:g} 秒内就绪")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--image", default=os.environ.get("A4N_TEST_POSTGRES_IMAGE", "postgres:16"))
    parser.add_argument("--report-dir", type=Path)
    parser.add_argument("--startup-timeout", type=float, default=60)
    parser.add_argument("--timeout", type=float, default=3600)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.command[:1] == ["--"]:
        args.command = args.command[1:]
    if not args.command:
        parser.error("必须在 -- 后提供测试命令")
    if min(args.startup_timeout, args.timeout) <= 0:
        parser.error("timeout 必须为正数")
    return args


def main() -> int:
    signal.signal(signal.SIGTERM, raise_interrupted)
    args = parse_args()
    if shutil.which("docker") is None:
        print("缺少 docker，无法创建临时测试数据库", file=sys.stderr)
        return 2
    run_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S") + "-" + secrets.token_hex(4)
    try:
        report_dir = safe_report_dir(args.report_dir or ROOT / "data" / "test-runs" / f"database-{run_id}")
    except (OSError, ReportDirectoryError) as error:
        print(f"测试数据库报告目录错误：{error}", file=sys.stderr)
        return 2
    log_path = report_dir / "postgres.log"
    command_log_path = report_dir / "command.log"
    result_path = report_dir / "database.json"
    container = f"a4n-test-db-{run_id.lower()}"
    password = secrets.token_urlsafe(24)
    port = free_port()
    database_url = build_database_url(port, password)
    report: dict[str, object] = {
        "schema_version": 1,
        "container": container,
        "database_url": database_url.rsplit("@", 1)[-1],
        "command": args.command,
        "status": "failed",
        "exit_code": None,
        "report_dir": str(report_dir),
    }
    started = False
    child: subprocess.Popen[bytes] | None = None
    try:
        with private_file(log_path) as log:
            docker_env = os.environ.copy()
            docker_env["POSTGRES_PASSWORD"] = password
            subprocess.run(
                container_command(container, port, args.image),
                env=docker_env,
                stdout=log,
                stderr=subprocess.STDOUT,
                check=True,
                timeout=args.startup_timeout + 30,
            )
            started = True
            wait_until_ready(container, log, args.startup_timeout)
        env = os.environ.copy()
        env["A4N_TEST_DATABASE_URL"] = database_url
        env["A4N_DATABASE_URL"] = database_url
        with private_file(command_log_path) as log:
            child = subprocess.Popen(
                args.command,
                cwd=ROOT,
                env=env,
                stdin=subprocess.DEVNULL,
                stdout=log,
                stderr=subprocess.STDOUT,
                start_new_session=True,
            )
            try:
                exit_code = child.wait(timeout=args.timeout)
            except subprocess.TimeoutExpired:
                terminate_process(child)
                exit_code = 124
                report["status"] = "timeout"
            else:
                report["status"] = "passed" if exit_code == 0 else "failed"
            report["exit_code"] = exit_code
    except KeyboardInterrupt:
        if child is not None:
            terminate_process(child)
        report["status"] = "interrupted"
        report["exit_code"] = 130
    except (OSError, subprocess.SubprocessError, DisposableDatabaseError) as error:
        report["error"] = str(error)
    finally:
        if started:
            try:
                with private_file(log_path, append=True) as log:
                    logs = subprocess.run(
                        ["docker", "logs", container],
                        stdout=log,
                        stderr=subprocess.STDOUT,
                        check=False,
                        timeout=10,
                    )
                if logs.returncode:
                    report["cleanup_error"] = f"读取 PostgreSQL 日志失败：exit={logs.returncode}"
            except (OSError, subprocess.SubprocessError) as error:
                report["cleanup_error"] = f"读取 PostgreSQL 日志失败：{error}"
        try:
            exists = subprocess.run(
                ["docker", "container", "inspect", container],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.PIPE,
                text=True,
                check=False,
                timeout=10,
            )
            if exists.returncode == 0:
                removed = subprocess.run(
                    remove_container_command(container),
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.PIPE,
                    text=True,
                    check=False,
                    timeout=30,
                )
                if removed.returncode:
                    report["cleanup_error"] = f"删除 PostgreSQL 容器失败：{removed.stderr.strip()}"
            elif started:
                report["cleanup_error"] = (
                    f"无法确认 PostgreSQL 容器已删除：exit={exists.returncode} {exists.stderr.strip()}"
                )
        except (OSError, subprocess.SubprocessError) as error:
            report["cleanup_error"] = f"检查或删除 PostgreSQL 容器失败：{error}"
        if "cleanup_error" in report:
            report["status"] = "failed"
        with private_file(result_path) as output:
            output.write(json.dumps(report, ensure_ascii=False, indent=2).encode("utf-8") + b"\n")
    print(json.dumps({"status": report["status"], "report": str(result_path)}, ensure_ascii=False), flush=True)
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
