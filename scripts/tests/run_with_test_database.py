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


ROOT = Path(__file__).resolve().parents[2]


class TestDatabaseError(RuntimeError):
    pass


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        listener.bind(("127.0.0.1", 0))
        return int(listener.getsockname()[1])


def build_database_url(port: int, password: str, database: str = "a4n_test") -> str:
    return f"postgres://a4n_test:{quote(password, safe='')}@127.0.0.1:{port}/{database}"


def container_command(container: str, port: int, image: str, _password: str) -> list[str]:
    return [
        "docker",
        "run",
        "--detach",
        "--name",
        container,
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


def wait_until_ready(container: str, log: object, timeout: float) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = subprocess.run(
            ["docker", "exec", container, "pg_isready", "-U", "a4n_test", "-d", "a4n_test"],
            stdout=log,
            stderr=subprocess.STDOUT,
            check=False,
        )
        if result.returncode == 0:
            return
        time.sleep(0.2)
    raise TestDatabaseError(f"PostgreSQL 未在 {timeout:g} 秒内就绪")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--image", default=os.environ.get("A4N_TEST_POSTGRES_IMAGE", "postgres:16"))
    parser.add_argument("--report-dir", type=Path)
    parser.add_argument("--startup-timeout", type=float, default=60)
    parser.add_argument("--timeout", type=float, default=3600)
    parser.add_argument("--keep-container", action="store_true")
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
    args = parse_args()
    if shutil.which("docker") is None:
        print("缺少 docker，无法创建临时测试数据库", file=sys.stderr)
        return 2
    run_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S") + "-" + secrets.token_hex(4)
    report_dir = (args.report_dir or ROOT / "data" / "test-runs" / f"database-{run_id}").resolve()
    report_dir.mkdir(parents=True, exist_ok=True)
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
    try:
        with log_path.open("wb") as log:
            docker_env = os.environ.copy()
            docker_env["POSTGRES_PASSWORD"] = password
            subprocess.run(
                container_command(container, port, args.image, password),
                env=docker_env,
                stdout=log,
                stderr=subprocess.STDOUT,
                check=True,
            )
            started = True
            wait_until_ready(container, log, args.startup_timeout)
        env = os.environ.copy()
        env["A4N_TEST_DATABASE_URL"] = database_url
        env["A4N_DATABASE_URL"] = database_url
        with command_log_path.open("wb") as log:
            process = subprocess.Popen(
                args.command,
                cwd=ROOT,
                env=env,
                stdin=subprocess.DEVNULL,
                stdout=log,
                stderr=subprocess.STDOUT,
                start_new_session=True,
            )
            try:
                exit_code = process.wait(timeout=args.timeout)
            except subprocess.TimeoutExpired:
                try:
                    os.killpg(process.pid, signal.SIGTERM)
                    process.wait(timeout=5)
                except (ProcessLookupError, subprocess.TimeoutExpired):
                    try:
                        os.killpg(process.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    process.wait()
                exit_code = 124
                report["status"] = "timeout"
            else:
                report["status"] = "passed" if exit_code == 0 else "failed"
            report["exit_code"] = exit_code
    except (OSError, subprocess.CalledProcessError, TestDatabaseError) as error:
        report["error"] = str(error)
    finally:
        if started:
            try:
                with log_path.open("ab") as log:
                    subprocess.run(["docker", "logs", container], stdout=log, stderr=subprocess.STDOUT, check=False)
            finally:
                if not args.keep_container:
                    subprocess.run(["docker", "rm", "--force", container], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)
                else:
                    report["kept_container"] = True
        result_path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"status": report["status"], "report": str(result_path)}, ensure_ascii=False), flush=True)
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
