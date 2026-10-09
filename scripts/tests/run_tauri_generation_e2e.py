#!/usr/bin/env python3
"""Run the Tauri generation user flow against a disposable database and mock LLM."""

from __future__ import annotations

import argparse
import json
import os
import secrets
import shutil
import subprocess
import sys
import time
import uuid
from pathlib import Path

from run_browser_full_project_e2e import (
    RunnerError,
    free_port,
    provider_audit,
    secure_mkdir,
    start_process,
    stop_process,
    tail,
    wait_http,
    write_report,
)


ROOT = Path(__file__).resolve().parents[2]
PROVIDER_SCRIPT = ROOT / "scripts/tests/mock_openai_provider.py"
RESULT_PREFIX = "A4N_TAURI_WRY_RESULT="


def parse_functional_result(log_path: Path) -> dict[str, object]:
    lines = [
        line.removeprefix(RESULT_PREFIX)
        for line in log_path.read_text(encoding="utf-8", errors="replace").splitlines()
        if line.startswith(RESULT_PREFIX)
    ]
    if len(lines) != 1:
        raise RunnerError(f"Tauri UI 用例没有写出唯一结果记录：{log_path}")
    try:
        result = json.loads(lines[0])
    except json.JSONDecodeError as error:
        raise RunnerError(f"Tauri UI 结果记录不是合法 JSON：{error}") from error
    if (
        not isinstance(result, dict)
        or result.get("remountCompleted") is not True
        or type(result.get("remountOccurrences")) is not int
        or result["remountOccurrences"] != 1
        or type(result.get("remountFirstTextAt")) is not int
        or result["remountFirstTextAt"] <= 0
    ):
        raise RunnerError(f"Tauri UI 功能结果不完整：{result!r}")
    return result


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--app-binary",
        type=Path,
        default=os.environ.get("A4N_WDIO_APP_BINARY"),
        help="已构建的 Tauri .app；默认读取 A4N_WDIO_APP_BINARY",
    )
    parser.add_argument("--report-dir", type=Path)
    parser.add_argument("--provider-port", type=int, default=0)
    args = parser.parse_args()
    if not args.app_binary:
        parser.error("必须提供 --app-binary 或 A4N_WDIO_APP_BINARY")
    if not os.environ.get("A4N_DATABASE_URL"):
        parser.error("必须提供隔离数据库 A4N_DATABASE_URL")
    if not os.environ.get("A4N_QDRANT_URL"):
        parser.error("必须提供 A4N_QDRANT_URL")
    if not os.environ.get("A4N_JWT_SECRET"):
        parser.error("必须提供 A4N_JWT_SECRET")
    if not 0 <= args.provider_port <= 65535:
        parser.error("--provider-port 必须在 0..65535")
    return args


def run(args: argparse.Namespace) -> int:
    app_binary = args.app_binary.expanduser().resolve()
    if not app_binary.exists():
        raise RunnerError(f"找不到 Tauri app：{app_binary}")
    if shutil.which("npm") is None:
        raise RunnerError("运行 Tauri 功能 E2E 需要 npm")

    run_id = time.strftime("%Y%m%dT%H%M%S") + "-" + uuid.uuid4().hex[:8]
    report_dir = (args.report_dir or ROOT / "data/tauri-generation-e2e" / run_id).resolve()
    if not report_dir.is_relative_to(ROOT):
        raise RunnerError("报告与运行数据必须位于当前工作树内")
    if report_dir.exists() and any(report_dir.iterdir()):
        raise RunnerError("--report-dir 必须是新的空目录")
    secure_mkdir(report_dir)
    data_dir = secure_mkdir(report_dir / "data")

    ports = {args.provider_port} if args.provider_port else set()

    def next_port() -> int:
        while (candidate := free_port()) in ports:
            continue
        ports.add(candidate)
        return candidate

    provider_port = args.provider_port or next_port()
    web_port = next_port()
    control_port = next_port()
    provider_url = f"http://127.0.0.1:{provider_port}/v1"
    request_log = report_dir / "provider.jsonl"
    provider_process = None
    provider_handle = None
    wdio_process = None
    wdio_handle = None
    report: dict[str, object] = {
        "schema_version": 1,
        "run_id": run_id,
        "app_binary": str(app_binary),
        "report_dir": str(report_dir),
        "success": False,
    }

    env = os.environ.copy()
    username = f"tauri-e2e-{run_id}"
    password = secrets.token_urlsafe(24)
    env.update(
        {
            "A4N_BOOTSTRAP_ADMIN_USERNAME": username,
            "A4N_BOOTSTRAP_ADMIN_EMAIL": f"{username}@example.invalid",
            "A4N_BOOTSTRAP_ADMIN_PASSWORD": password,
            "A4N_RUNTIME_PROFILE": "sidecar",
            "A4N_MANAGED_SIDECARS": "1",
            "A4N_WEB_ADDR": f"127.0.0.1:{web_port}",
            "A4N_CONTROL_ADDR": f"127.0.0.1:{control_port}",
            "A4N_DATA_DIR": str(data_dir),
            "A4N_CACHE_DIR": str(data_dir / "cache"),
            "A4N_TEMP_DIR": str(data_dir / "tmp"),
            "A4N_LOG_DIR": str(data_dir / "logs"),
            "A4N_QDRANT_COLLECTION": f"tauri_generation_{run_id.replace('-', '_')}",
            "A4N_PUBLIC_LLM_NAME": "Tauri Functional E2E Slow LLM",
            "A4N_PUBLIC_LLM_PROVIDER": "openai",
            "A4N_PUBLIC_LLM_BASE_URL": provider_url,
            "A4N_PUBLIC_LLM_API_KEY": "a4n-e2e-provider-key",
            "A4N_PUBLIC_LLM_MODEL": "a4n-llm-lifecycle-slow",
            "A4N_PUBLIC_LLM_MAX_CONCURRENT": "2",
            "A4N_PUBLIC_LLM_TIMEOUT_SECS": "120",
            "A4N_PUBLIC_LLM_MAX_CONTEXT_TOKENS": "128000",
            "A4N_PUBLIC_LLM_MAX_OUTPUT_TOKENS": "8192",
            "A4N_WDIO_APP_BINARY": str(app_binary),
            "A4N_WDIO_USERNAME": username,
            "A4N_WDIO_PASSWORD": password,
            "A4N_BROWSER_E2E_ARTIFACT_DIR": str(report_dir / "browser"),
        }
    )
    worker_catalog = json.loads(
        (ROOT / "crates/process/a4n-process-catalog/worker-catalog.json").read_text(encoding="utf-8")
    )
    for worker in worker_catalog:
        env[worker["addr_env"]] = f"127.0.0.1:{next_port()}"
    for key in ("http_proxy", "https_proxy", "all_proxy", "HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY"):
        env.pop(key, None)
    env["NO_PROXY"] = "localhost,127.0.0.1"
    secure_mkdir(report_dir / "browser")

    try:
        provider_process, provider_handle = start_process(
            [sys.executable, str(PROVIDER_SCRIPT), "--host", "127.0.0.1", "--port", str(provider_port), "--log", str(request_log), "--embedding-dimension", "32"],
            cwd=ROOT,
            env=env,
            log_path=report_dir / "provider-process.log",
        )
        wait_http(f"http://127.0.0.1:{provider_port}/health", process=provider_process, timeout=30)

        wdio_log = report_dir / "wdio.log"
        wdio_process, wdio_handle = start_process(
            ["npm", "run", "test:tauri-generation"],
            cwd=ROOT / "frontend",
            env=env,
            log_path=wdio_log,
        )
        try:
            browser_exit = wdio_process.wait(timeout=900)
        except subprocess.TimeoutExpired as error:
            stop_process(wdio_process, None)
            report["browser_exit_code"] = 124
            raise RunnerError(f"Tauri UI 功能 E2E 超时：{wdio_log}") from error
        report["browser_exit_code"] = browser_exit
        if browser_exit != 0:
            raise RunnerError(f"Tauri UI 功能 E2E 失败(exit={browser_exit})：\n{tail(wdio_log, 2000)}")
        report["functional_result"] = parse_functional_result(wdio_log)
        report["provider"] = provider_audit(request_log, required_paths={"/chat/completions"})
        report["success"] = True
    except subprocess.TimeoutExpired as error:
        report["error"] = f"功能 E2E 超时：{error.cmd}"
    except Exception as error:
        report["error"] = f"{type(error).__name__}: {error}"
    finally:
        if wdio_process is not None:
            try:
                stop_process(wdio_process, wdio_handle)
            except (OSError, RunnerError) as error:
                report["success"] = False
                report["wdio_cleanup_error"] = str(error)
        if provider_process is not None:
            try:
                stop_process(provider_process, provider_handle)
            except (OSError, RunnerError) as error:
                report["success"] = False
                report["cleanup_error"] = str(error)
        report["artifacts"] = {
            "provider_log": str(report_dir / "provider-process.log"),
            "provider_requests": str(request_log),
            "wdio_log": str(report_dir / "wdio.log"),
        }
        write_report(report_dir / "report.json", report)
        print(json.dumps(report, ensure_ascii=False, indent=2))
    return 0 if report["success"] is True else 1


def main() -> int:
    try:
        return run(parse_args())
    except RunnerError as error:
        print(f"Tauri 功能 E2E 未启动：{error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
