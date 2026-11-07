#!/usr/bin/env python3
"""Run workspace tests with isolated PostgreSQL, Qdrant and process binaries."""

from __future__ import annotations

import argparse
import json
import os
import signal
import shutil
import subprocess
import sys
import uuid
from pathlib import Path

from runner_support import private_file, raise_interrupted, safe_report_dir
from run_browser_full_project_e2e import (
    RunnerError,
    configure_onnxruntime,
    ensure_executable,
    free_port,
    secure_mkdir,
    start_process,
    stop_process,
    wait_http,
)

ROOT = Path(__file__).resolve().parents[2]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report-dir", type=Path, required=True)
    return parser.parse_args()


def target_directory() -> Path:
    configured = os.environ.get("CARGO_TARGET_DIR")
    target = Path(configured) if configured else ROOT / "target"
    return target if target.is_absolute() else ROOT / target


def build_test_workers(env: dict[str, str], report_dir: Path) -> dict[str, str]:
    cargo = shutil.which("cargo")
    if cargo is None:
        raise RunnerError("缺少 cargo，无法构建 Rust workspace 测试 sidecar")
    log_path = report_dir / "test-workers-build.log"
    catalog = json.loads(
        (ROOT / "crates/process/a4n-process-catalog/worker-catalog.json").read_text(
            encoding="utf-8"
        )
    )
    workers = [entry for entry in catalog if entry.get("kind") == "stateful_worker"]
    binary_packages = {entry["binary"] for entry in workers}
    binary_packages.add("a4n-control")
    binary_packages.add("a4n-edge")
    package_args = [
        item
        for package in sorted(binary_packages)
        for item in ("-p", package)
    ]
    with private_file(log_path) as log:
        result = subprocess.run(
            [
                cargo,
                "build",
                "--locked",
                "--features",
                "a4n-control/embedded-capabilities",
                *package_args,
            ],
            cwd=ROOT,
            env=env,
            stdout=log,
            stderr=subprocess.STDOUT,
            check=False,
        )
    if result.returncode:
        raise RunnerError(
            f"Rust workspace 测试 sidecar 构建失败(exit={result.returncode})，日志：{log_path}"
        )
    debug_dir = target_directory() / "debug"
    binary_suffix = ".exe" if os.name == "nt" else ""
    binaries = {
        f"A4N_{entry['name'].replace('-', '_').upper()}_BINARY": ensure_executable(
            debug_dir / f"{entry['binary']}{binary_suffix}", entry["binary"]
        )
        for entry in workers
    }
    # The server integration tests launch the control process directly.  It is
    # part of the test process graph, so expose the same isolated build output
    # instead of making the tests fall back to the worktree's default target.
    binaries["A4N_CONTROL_BINARY"] = ensure_executable(
        debug_dir / f"a4n-control{binary_suffix}", "a4n-control"
    )
    binaries["A4N_EDGE_BINARY"] = ensure_executable(
        debug_dir / f"a4n-edge{binary_suffix}", "a4n-edge"
    )
    return binaries


def write_qdrant_config(path: Path, data_dir: Path, http_port: int, grpc_port: int) -> None:
    config = {
        "storage": {
            "storage_path": str(secure_mkdir(data_dir / "storage")),
            "snapshots_path": str(secure_mkdir(data_dir / "snapshots")),
        },
        "service": {"host": "127.0.0.1", "http_port": http_port, "grpc_port": grpc_port},
        "telemetry_disabled": True,
    }
    with private_file(path) as output:
        output.write(json.dumps(config).encode("utf-8"))


def run_workspace(env: dict[str, str], report_dir: Path) -> int:
    command = [
        sys.executable,
        str(ROOT / "scripts/tests/run_with_test_database.py"),
        "--report-dir",
        str(report_dir / "database"),
        "--timeout",
        "7200",
        "--",
        "bash",
        "-c",
        "set -euo pipefail; cargo run --locked -p a4n-migration; exec cargo test --locked --workspace --all-targets --no-fail-fast --features a4n-runtime/embedded-transforms -- --test-threads=1",
    ]
    return subprocess.run(command, cwd=ROOT, env=env, check=False).returncode


def main() -> int:
    signal.signal(signal.SIGTERM, raise_interrupted)
    args = parse_args()
    report_dir = safe_report_dir(args.report_dir)
    processes: list[tuple[subprocess.Popen[bytes], object]] = []
    env = os.environ.copy()
    env.setdefault("CARGO_BUILD_JOBS", "2")
    exit_code = 1
    cleanup_failed = False
    try:
        env["TMPDIR"] = str(secure_mkdir(report_dir / "tmp"))
        configure_onnxruntime(env)
        ort_library_dir = env.get("ORT_LIB_PATH")
        if not ort_library_dir:
            raise RunnerError("全 workspace 测试需要项目固定版本的 ONNX Runtime")
        env["A4N_ONNXRUNTIME_ROOT"] = str(Path(ort_library_dir).resolve().parent)
        env.setdefault("ORT_PREFER_DYNAMIC_LINK", "1")
        env.update(build_test_workers(env, report_dir))
        qdrant_binary = ensure_executable(ROOT / "tools/qdrant/bin/qdrant", "Qdrant")
        http_port, grpc_port = free_port(), free_port()
        qdrant_data = secure_mkdir(report_dir / "qdrant")
        config_path = report_dir / "qdrant.yaml"
        write_qdrant_config(config_path, qdrant_data, http_port, grpc_port)
        env["A4N_QDRANT_URL"] = f"http://127.0.0.1:{grpc_port}"
        env["A4N_QDRANT_COLLECTION"] = f"a4n_rust_{uuid.uuid4().hex}"
        qdrant, qdrant_log = start_process(
            [qdrant_binary, "--config-path", str(config_path)],
            cwd=ROOT,
            env=env,
            log_path=report_dir / "qdrant.log",
        )
        processes.append((qdrant, qdrant_log))
        wait_http(f"http://127.0.0.1:{http_port}/healthz", process=qdrant, timeout=60)
        exit_code = run_workspace(env, report_dir)
    except KeyboardInterrupt:
        exit_code = 130
    except (OSError, RunnerError, subprocess.SubprocessError) as error:
        print(f"Rust workspace 测试准备失败：{error}", file=sys.stderr)
    finally:
        for process, log in reversed(processes):
            try:
                stop_process(process, log)
            except RunnerError as error:
                print(f"Rust workspace 测试服务清理失败：{error}", file=sys.stderr)
                cleanup_failed = True
    return 1 if cleanup_failed else exit_code


if __name__ == "__main__":
    raise SystemExit(main())
