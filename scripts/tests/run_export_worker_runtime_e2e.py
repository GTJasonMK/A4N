#!/usr/bin/env python3
"""Run a real, isolated Host + export Worker runtime sample.

This runner deliberately reports the limits of the export slice. It does not
turn non-applicable E1-E6 cases into passing tests and never claims a net gain
without a comparable baseline.
"""

from __future__ import annotations

import argparse
import base64
import binascii
import hashlib
import http.client
import io
import json
import os
import secrets
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import threading
import time
import uuid
import zipfile
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.parse import quote, unquote, urlsplit, urlunsplit
from urllib.request import Request, ProxyHandler, build_opener

from run_browser_full_project_e2e import secure_mkdir as secure_dir, tail

ROOT = Path(__file__).resolve().parents[2]
EXPORT_CAPABILITY_VERSION = "export-render-v3"
PROJECT_FAMILY_MANIFEST_PATH = "a4n-project-family.json"
PROTOCOL_VERSION = 2
MAX_HTTP_BODY_BYTES = 128 * 1024 * 1024
FAULTS = ("E1", "E2", "E3", "E4", "E5", "E6")
HTTP = build_opener(ProxyHandler({}))


class RunnerError(RuntimeError):
    pass


@dataclass
class HttpResult:
    status: int | None
    body: bytes
    elapsed_ms: int
    error: str | None = None
    headers: dict[str, str] | None = None
    response_bytes: int | None = None


@dataclass
class ManagedProcess:
    label: str
    command: list[str]
    process: subprocess.Popen[bytes]
    log_path: Path
    pgid: int
    handle: Any
    finished: bool = False


def now_ms() -> int:
    return time.monotonic_ns() // 1_000_000


def utc_run_id() -> str:
    return datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + uuid.uuid4().hex[:8]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run the real isolated A4N export Host + Worker runtime acceptance sample.",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )
    parser.add_argument(
        "--database-url",
        default=None,
        help="isolated PostgreSQL URL; a4n is always rejected",
    )
    parser.add_argument(
        "--create-database",
        action="store_true",
        help="create a random temporary database using the URL in --database-admin-url-env",
    )
    parser.add_argument(
        "--database-admin-url-env",
        default="A4N_RUNTIME_E2E_ADMIN_DATABASE_URL",
        help="environment variable containing the PostgreSQL admin URL",
    )
    parser.add_argument(
        "--keep-created-database",
        action="store_true",
        help="keep a database created by this run for postmortem inspection",
    )
    parser.add_argument("--host-binary", type=Path, help="a4n-server; --build uses its isolated target")
    parser.add_argument("--worker-binary", type=Path, help="a4n-export-service")
    parser.add_argument("--migration-binary", type=Path, help="a4n-migration")
    parser.add_argument(
        "--target-dir",
        type=Path,
        help="independent Cargo target directory used with --build (defaults below the report)",
    )
    parser.add_argument(
        "--build",
        action="store_true",
        help="build server, export service, and migration into --target-dir with --locked",
    )
    parser.add_argument("--build-timeout", type=float, default=900.0)
    parser.add_argument("--keep-target", action="store_true", help="keep this run's default build cache for debugging; explicit --target-dir is always caller-owned")
    parser.add_argument("--report-dir", type=Path, help="empty directory for durable raw evidence")
    parser.add_argument(
        "--username",
        default=os.environ.get("A4N_RUNTIME_E2E_USERNAME", ""),
        help="bootstrap admin username; defaults to a unique disposable username",
    )
    parser.add_argument(
        "--password-env",
        default="A4N_RUNTIME_E2E_PASSWORD",
        help="environment variable containing the bootstrap password",
    )
    parser.add_argument("--request-count", type=int, default=3, help="successful export requests to measure")
    parser.add_argument("--concurrency", type=int, default=1, help="business export request concurrency")
    parser.add_argument("--startup-timeout", type=float, default=60.0)
    parser.add_argument("--request-timeout", type=float, default=120.0)
    parser.add_argument(
        "--fault",
        choices=("none", "all", *FAULTS),
        default="none",
        help="run one explicit E1-E6 injection after the normal business sample",
    )
    parser.add_argument(
        "--fault-payload-mib",
        type=int,
        default=4,
        help="bounded random ZIP payload for E3/E6 transport/concurrency evidence",
    )
    parser.add_argument("--self-test", action="store_true", help="run local validation only")
    args = parser.parse_args()
    if args.request_count <= 0 or args.concurrency <= 0:
        parser.error("--request-count and --concurrency must be positive")
    if args.concurrency > 32:
        parser.error("--concurrency must be <= 32")
    if args.fault_payload_mib <= 0 or args.fault_payload_mib > 16:
        parser.error("--fault-payload-mib must be in 1..16")
    if min(args.startup_timeout, args.request_timeout, args.build_timeout) <= 0:
        parser.error("timeouts must be positive")
    if args.build and any((args.host_binary, args.worker_binary, args.migration_binary)):
        parser.error("--build selects all binaries from --target-dir; do not combine explicit binary paths")
    if args.create_database and args.database_url:
        parser.error("--create-database cannot be combined with --database-url")
    if not args.create_database and not args.database_url:
        args.database_url = os.environ.get("A4N_RUNTIME_E2E_DATABASE_URL") or os.environ.get(
            "A4N_TEST_DATABASE_URL"
        )
    return args


def prepare_report(path: Path | None) -> Path:
    if path is None:
        return Path(tempfile.mkdtemp(prefix="a4n-export-worker-e2e-"))
    resolved = path.expanduser().resolve()
    if resolved == Path("/"):
        raise RunnerError("report directory cannot be /")
    try:
        resolved.relative_to(ROOT)
    except ValueError:
        pass
    else:
        raise RunnerError(f"report directory must be outside the source checkout: {resolved}")
    if resolved.exists() and any(resolved.iterdir()):
        raise RunnerError(f"report directory must be empty: {resolved}")
    return secure_dir(resolved)


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(descriptor, "w", encoding="utf-8") as output:
        json.dump(value, output, ensure_ascii=False, indent=2)
        output.write("\n")


def append_jsonl(path: Path, value: Any, lock: threading.Lock | None = None) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    line = json.dumps(value, ensure_ascii=False, separators=(",", ":")) + "\n"
    if lock:
        with lock:
            with path.open("a", encoding="utf-8") as output:
                output.write(line)
    else:
        with path.open("a", encoding="utf-8") as output:
            output.write(line)


def redacted_url(value: str) -> str:
    parsed = urlsplit(value)
    host = parsed.hostname or ""
    port = ""
    try:
        if parsed.port:
            port = f":{parsed.port}"
    except ValueError:
        pass
    path = parsed.path or "/"
    return f"{parsed.scheme}://{host}{port}{path}"


def database_name(value: str, *, reject_development: bool = True) -> str:
    try:
        parsed = urlsplit(value)
    except ValueError as error:
        raise RunnerError(f"database URL is invalid: {error}") from error
    if parsed.scheme not in {"postgres", "postgresql"}:
        raise RunnerError("database URL must use postgres:// or postgresql://")
    if not parsed.hostname:
        raise RunnerError("database URL has no host")
    name = unquote(parsed.path.lstrip("/").split("/", 1)[0]).strip()
    if not name:
        raise RunnerError("database URL has no database name")
    if reject_development and name.casefold() == "a4n":
        raise RunnerError("refusing the development database a4n; use an isolated database")
    return name


def database_url_for_name(admin_url: str, name: str) -> str:
    parsed = urlsplit(admin_url)
    return urlunsplit((parsed.scheme, parsed.netloc, "/" + quote(name), parsed.query, ""))


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])


def ensure_executable(path: Path, label: str) -> Path:
    resolved = path.expanduser().resolve()
    if not resolved.is_file() or not os.access(resolved, os.X_OK):
        raise RunnerError(f"executable for {label} not found: {resolved}")
    return resolved


def read_limited(response: Any, limit: int = MAX_HTTP_BODY_BYTES) -> bytes:
    chunks: list[bytes] = []
    size = 0
    while True:
        chunk = response.read(min(1024 * 1024, limit - size + 1))
        if not chunk:
            break
        chunks.append(chunk)
        size += len(chunk)
        if size > limit:
            raise RunnerError(f"HTTP response exceeded {limit} bytes")
    return b"".join(chunks)


def http_request(
    url: str,
    *,
    method: str = "GET",
    body: bytes | None = None,
    headers: dict[str, str] | None = None,
    timeout: float = 10.0,
) -> HttpResult:
    started = time.monotonic()
    request = Request(url, data=body, headers=headers or {}, method=method)
    try:
        with HTTP.open(request, timeout=timeout) as response:
            payload = read_limited(response)
            return HttpResult(
                response.status,
                payload,
                round((time.monotonic() - started) * 1000),
                headers=dict(response.headers.items()),
            )
    except HTTPError as error:
        with error:
            payload = read_limited(error)
            return HttpResult(
                error.code,
                payload,
                round((time.monotonic() - started) * 1000),
                headers=dict(error.headers.items()) if error.headers else {},
            )
    except (OSError, URLError, ValueError) as error:
        return HttpResult(
            None,
            b"",
            round((time.monotonic() - started) * 1000),
            error=f"{type(error).__name__}: {error}",
        )


def json_body(result: HttpResult, label: str) -> Any:
    if result.error:
        raise RunnerError(f"{label} transport failed: {result.error}")
    try:
        return json.loads(result.body.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise RunnerError(f"{label} returned non-JSON (HTTP {result.status})") from error


def assert_owned_process_group(pid: int, pgid: int) -> None:
    if pgid != pid or pgid == os.getpgrp() or os.getpgid(pid) != pgid or os.getsid(pid) != pid:
        raise RunnerError(f"refusing process group {pgid}; it is not the spawned process's isolated session")


def open_process_log(path: Path) -> Any:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    return os.fdopen(descriptor, "wb", buffering=0)


def start_process(
    label: str,
    command: list[str],
    *,
    env: dict[str, str],
    cwd: Path,
    log_path: Path,
) -> ManagedProcess:
    handle = open_process_log(log_path)
    process: subprocess.Popen[bytes] | None = None
    try:
        process = subprocess.Popen(
            command,
            cwd=str(cwd),
            env=env,
            stdin=subprocess.DEVNULL,
            stdout=handle,
            stderr=subprocess.STDOUT,
            start_new_session=True,
        )
        pgid = os.getpgid(process.pid)
        assert_owned_process_group(process.pid, pgid)
    except BaseException:
        try:
            if process is not None:
                process.kill()
                process.wait(timeout=10)
        finally:
            handle.close()
        raise
    assert process is not None
    return ManagedProcess(label, command, process, log_path, pgid, handle)


def finish_process(process: ManagedProcess) -> None:
    if not process.finished:
        process.handle.close()
        process.finished = True


def wait_process(process: ManagedProcess, timeout: float, label: str) -> int:
    try:
        code = process.process.wait(timeout=timeout)
    except subprocess.TimeoutExpired as error:
        raise RunnerError(f"{label} did not finish in {timeout:g}s; see {process.log_path}") from error
    finally:
        if process.process.poll() is not None:
            finish_process(process)
    if code != 0:
        raise RunnerError(f"{label} failed with exit={code}; see {process.log_path}")
    return code


def stop_process(process: ManagedProcess | None, *, force: bool = False) -> None:
    if process is None:
        return
    try:
        if process.process.poll() is None:
            assert_owned_process_group(process.process.pid, process.pgid)
            first_signal = signal.SIGKILL if force else signal.SIGTERM
            os.killpg(process.pgid, first_signal)
            try:
                process.process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                assert_owned_process_group(process.process.pid, process.pgid)
                os.killpg(process.pgid, signal.SIGKILL)
                process.process.wait(timeout=10)
    finally:
        finish_process(process)


def proc_rss_tree(root: int) -> dict[str, Any] | None:
    """Read a process tree without inventing values for an exited root."""
    proc = Path("/proc")
    if not (proc / str(root)).is_dir():
        return None
    parents: dict[int, int] = {}
    states: dict[int, str] = {}
    rss: dict[int, int] = {}
    for entry in proc.iterdir():
        if not entry.name.isdigit():
            continue
        pid = int(entry.name)
        try:
            stat = (entry / "stat").read_text(encoding="ascii")
            right = stat.rfind(")")
            fields = stat[right + 2 :].split()
            states[pid] = fields[0]
            parents[pid] = int(fields[1])
            for line in (entry / "status").read_text(encoding="ascii").splitlines():
                if line.startswith("VmRSS:"):
                    rss[pid] = int(line.split()[1])
                    break
        except (OSError, ValueError, IndexError):
            continue
    if root not in rss or states.get(root, "Z").startswith("Z"):
        return None
    tree = {root}
    changed = True
    while changed:
        changed = False
        for pid, parent in parents.items():
            if parent in tree and pid not in tree:
                tree.add(pid)
                changed = True
    live = [pid for pid in tree if pid in rss and not states.get(pid, "Z").startswith("Z")]
    root_rss = rss[root]
    descendant_rss = sum(rss[pid] for pid in live if pid != root)
    return {
        "root_pid": root,
        "pids": sorted(live),
        "root_rss_kb": root_rss,
        "descendant_rss_kb": descendant_rss,
        "tree_rss_kb": root_rss + descendant_rss,
    }


def service_request(payload: dict[str, Any], request_id: str | None = None) -> dict[str, Any]:
    request_id = request_id or f"runtime-e2e-{uuid.uuid4().hex}"
    return {
        "request_id": request_id,
        "trace_context": {"trace_id": request_id},
        "schema_version": {"protocol": PROTOCOL_VERSION},
        "secret_policy": {
            "scope": "none",
            "allow_system_secret": False,
            "allow_user_secret": False,
            "allow_upload_token": False,
        },
        "capability_version": EXPORT_CAPABILITY_VERSION,
        "payload": payload,
    }


METRIC_COUNTER_FIELDS = (
    "requests_total",
    "errors_total",
    "cancelled_total",
    "in_flight",
    "duration_us_total",
    "duration_us_max",
)
METRIC_SNAPSHOT_FIELDS = (
    "pid",
    "started_at_unix_ms",
    "uptime_ms",
    "restart_count",
    "requests_total",
    "errors_total",
    "cancelled_total",
    "in_flight",
    "duration_us_total",
    "duration_us_max",
)
RENDER_METRIC_FIELDS = (
    "active_renders",
    "peak_active_renders",
    "queue_depth",
    "peak_queue_depth",
    "queue_rejections",
    "max_concurrent_renders",
    "queue_capacity",
)


def require_metric_counter(value: Any, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise RunnerError(f"{label} must be a non-negative integer")
    return value


def validate_metrics_snapshot(metrics: dict[str, Any], label: str) -> dict[str, Any]:
    if metrics.get("service") != "export":
        raise RunnerError(f"{label} returned metrics for an unexpected service")
    for field in METRIC_SNAPSHOT_FIELDS:
        require_metric_counter(metrics.get(field), f"{label}.{field}")
    operations = metrics.get("operations", {})
    if not isinstance(operations, dict):
        raise RunnerError(f"{label}.operations must be an object")
    for path, operation in operations.items():
        if not isinstance(path, str) or not isinstance(operation, dict):
            raise RunnerError(f"{label}.operations contains an invalid entry")
        for field in METRIC_COUNTER_FIELDS:
            require_metric_counter(operation.get(field), f"{label}.operations[{path!r}].{field}")
    return metrics


def validate_render_metrics_snapshot(metrics: dict[str, Any], label: str) -> dict[str, Any]:
    if metrics.get("service") != "export":
        raise RunnerError(f"{label} returned metrics for an unexpected service")
    for field in RENDER_METRIC_FIELDS:
        require_metric_counter(metrics.get(field), f"{label}.{field}")
    if metrics["max_concurrent_renders"] == 0:
        raise RunnerError(f"{label}.max_concurrent_renders must be positive")
    return metrics


def service_response_ok(result: HttpResult) -> bool:
    if result.status != 200 or result.error:
        return False
    try:
        value = json.loads(result.body.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError):
        return False
    return isinstance(value, dict) and value.get("status") == "ok" and isinstance(value.get("payload"), dict)


def slow_consumer_observation_passed(
    *,
    slow_response_statuses: list[int],
    expected_slow_responses: int,
    held_queue_depth: int,
    queue_capacity: int,
    queued_results: list[HttpResult],
    drained_results: list[HttpResult],
) -> bool:
    return (
        expected_slow_responses > 0
        and len(slow_response_statuses) == expected_slow_responses
        and all(status == 200 for status in slow_response_statuses)
        and held_queue_depth > 0
        and len(queued_results) == queue_capacity + 1
        and sum(service_response_ok(result) for result in queued_results) == queue_capacity
        and sum(result.status == 429 for result in queued_results) == 1
        and len(drained_results) == expected_slow_responses
        and all(result.status == 200 and result.error is None for result in drained_results)
    )


def metrics_delta(before: dict[str, Any], after: dict[str, Any]) -> dict[str, Any]:
    validate_metrics_snapshot(before, "metrics before")
    validate_metrics_snapshot(after, "metrics after")
    paths = ("/export/render_json", "/export/render_zip")
    delta: dict[str, Any] = {}
    for path in paths:
        old = before.get("operations", {}).get(path, {})
        new = after.get("operations", {}).get(path, {})
        if not new:
            raise RunnerError(f"metrics after is missing operation {path!r}")
        delta[path] = {
            field: require_metric_counter(new[field], f"metrics after.operations[{path!r}].{field}")
            - require_metric_counter(old.get(field, 0), f"metrics before.operations[{path!r}].{field}")
            for field in METRIC_COUNTER_FIELDS
        }
    return delta


def redact_error_text(value: str, secrets_to_hide: tuple[str, ...] = ()) -> str:
    result = value
    for secret in secrets_to_hide:
        if secret:
            result = result.replace(secret, "[REDACTED]")
    return result[-2_000:]


class RuntimeRunner:
    def __init__(self, args: argparse.Namespace, report_dir: Path, run_id: str) -> None:
        self.args = args
        self.report_dir = report_dir
        self.run_id = run_id
        self.logs = secure_dir(report_dir / "logs")
        self.data = secure_dir(report_dir / "data")
        self.host_data = secure_dir(self.data / "host")
        self.worker_data = secure_dir(self.data / "worker")
        self.cache = secure_dir(report_dir / "cache")
        self.temp = secure_dir(report_dir / "tmp")
        self.host_cache = secure_dir(self.cache / "host")
        self.worker_cache = secure_dir(self.cache / "worker")
        self.host_temp = secure_dir(self.temp / "host")
        self.worker_temp = secure_dir(self.temp / "worker")
        self.host_logs = secure_dir(self.logs / "host")
        self.worker_logs = secure_dir(self.logs / "worker")
        self.frontend = secure_dir(report_dir / "frontend")
        self.target_dir = args.target_dir.expanduser().resolve() if args.target_dir else report_dir / "target"
        self.target_created = False
        self.http_log = report_dir / "http.jsonl"
        self.rss_jsonl = report_dir / "rss.jsonl"
        self.rss_tsv = report_dir / "rss.tsv"
        self.http_lock = threading.Lock()
        self.rss_lock = threading.Lock()
        self.processes: list[ManagedProcess] = []
        self.host: ManagedProcess | None = None
        self.worker: ManagedProcess | None = None
        self.migration: ManagedProcess | None = None
        self.phase = "setup"
        self.rss_samples: list[dict[str, Any]] = []
        self.database_url = ""
        self.created_database_name: str | None = None
        self.host_url = ""
        self.worker_url = ""
        self.host_env: dict[str, str] = {}
        self.worker_env: dict[str, str] = {}
        self.auth_token = ""
        self.worker_binary: Path | None = None
        self.last_token = ""
        self.metric_snapshot_count = 0
        self.render_metric_snapshot_count = 0
        self.project_id: int | None = None
        self.report: dict[str, Any] = {
            "schema_version": 1,
            "candidate": "export",
            "run_id": run_id,
            "status": "blocked",
            "success": False,
            "net_benefit": {
                "status": "undetermined",
                "reason": "this run has no same-workload static/host baseline; runtime data is not a net-gain claim",
            },
            "process_ownership": "isolated session per spawned process",
            "faults": {fault: {"status": "not_run"} for fault in FAULTS},
            "artifacts": {},
        }

    def record_http(self, label: str, method: str, url: str, result: HttpResult, body_fields: list[str]) -> None:
        append_jsonl(
            self.http_log,
            {
                "label": label,
                "method": method,
                "path": urlsplit(url).path,
                "status": result.status,
                "elapsed_ms": result.elapsed_ms,
                "response_bytes": (
                    result.response_bytes if result.response_bytes is not None else len(result.body)
                ),
                "body_fields": body_fields,
                "transport_error": result.error,
            },
            self.http_lock,
        )

    def sample_rss(self, phase: str | None = None) -> None:
        host = proc_rss_tree(self.host.process.pid) if self.host and self.host.process.poll() is None else None
        worker = proc_rss_tree(self.worker.process.pid) if self.worker and self.worker.process.poll() is None else None
        if host is None and worker is None:
            return
        sample = {
            "elapsed_ms": now_ms() - self.started_ms,
            "phase": phase or self.phase,
            "host": host,
            "worker": worker,
            "combined_tree_rss_kb": (
                host["tree_rss_kb"] + worker["tree_rss_kb"] if host and worker else None
            ),
        }
        with self.rss_lock:
            self.rss_samples.append(sample)
            append_jsonl(self.rss_jsonl, sample)
            if not self.rss_tsv.exists():
                self.rss_tsv.write_text(
                    "# elapsed_ms\tphase\thost_root_rss_kb\thost_descendant_rss_kb\thost_tree_rss_kb\t"
                    "worker_root_rss_kb\tworker_descendant_rss_kb\tworker_tree_rss_kb\tcombined_tree_rss_kb\n",
                    encoding="utf-8",
                )
            values = [sample["elapsed_ms"], sample["phase"]]
            for component in (host, worker):
                values.extend(
                    [
                        component.get("root_rss_kb", "") if component else "",
                        component.get("descendant_rss_kb", "") if component else "",
                        component.get("tree_rss_kb", "") if component else "",
                    ]
                )
            values.append(sample["combined_tree_rss_kb"] or "")
            with self.rss_tsv.open("a", encoding="utf-8") as output:
                output.write("\t".join(str(value) for value in values) + "\n")

    def start_rss_monitor(self) -> tuple[threading.Event, threading.Thread]:
        stop = threading.Event()

        def monitor() -> None:
            while not stop.wait(0.05):
                self.sample_rss()

        thread = threading.Thread(target=monitor, name="rss-sampler", daemon=True)
        thread.start()
        return stop, thread

    def stop_rss_monitor(self, monitor: tuple[threading.Event, threading.Thread]) -> None:
        monitor[0].set()
        monitor[1].join(timeout=2)
        self.sample_rss("收尾")

    def set_database(self) -> None:
        if self.args.create_database:
            admin_url = os.environ.get(self.args.database_admin_url_env, "").strip()
            if not admin_url:
                raise RunnerError(
                    f"--create-database requires {self.args.database_admin_url_env}; password stays in the environment"
                )
            database_name(admin_url, reject_development=False)
            if not shutil.which("psql"):
                raise RunnerError("--create-database requires psql")
            name = "a4n_export_worker_" + secrets.token_hex(8)
            self.run_psql(admin_url, f'CREATE DATABASE "{name}";', "database-create.log")
            self.created_database_name = name
            self.database_url = database_url_for_name(admin_url, name)
        else:
            if not self.args.database_url:
                raise RunnerError(
                    "an isolated database is required; set A4N_RUNTIME_E2E_DATABASE_URL or A4N_TEST_DATABASE_URL"
                )
            self.database_url = self.args.database_url
        name = database_name(self.database_url)
        self.report["database"] = {
            "name": name,
            "url": redacted_url(self.database_url),
            "created_by_runner": self.created_database_name is not None,
            "drop_after_run": bool(self.created_database_name and not self.args.keep_created_database),
        }

    def run_psql(self, admin_url: str, sql: str, log_name: str) -> None:
        log_path = self.logs / log_name
        command = [
            "psql",
            "--dbname",
            admin_url,
            "-XqAt",
            "--set",
            "ON_ERROR_STOP=1",
            "--command",
            sql,
        ]
        process = subprocess.run(command, capture_output=True, text=True, check=False)
        log_path.write_text(
            redact_error_text((process.stdout or "") + (process.stderr or "")), encoding="utf-8"
        )
        log_path.chmod(0o600)
        if process.returncode != 0:
            raise RunnerError(f"PostgreSQL command failed (exit={process.returncode}); see {log_path}")

    def drop_created_database(self) -> None:
        if not self.created_database_name or self.args.keep_created_database:
            return
        admin_url = os.environ.get(self.args.database_admin_url_env, "").strip()
        if not admin_url:
            raise RunnerError("created database cannot be dropped because the admin URL disappeared")
        self.run_psql(
            admin_url,
            f'DROP DATABASE IF EXISTS "{self.created_database_name}" WITH (FORCE);',
            "database-drop.log",
        )

    def resolve_binaries(self) -> tuple[Path, Path, Path]:
        if self.args.build:
            cargo = shutil.which("cargo")
            if not cargo:
                raise RunnerError("--build requires cargo")
            if self.args.target_dir is None:
                manager = os.environ.get("CARGO_ARTIFACTS_BIN") or shutil.which("cargo-artifacts")
                if manager:
                    subprocess.run([manager, "prepare", "--target-dir", str(self.target_dir)], stdout=sys.stderr, check=True)
                else:
                    self.target_dir.mkdir()
                    print("cargo-artifacts is not installed: temporary cache cleanup remains enabled, but parent snapshots are not isolated", file=sys.stderr)
                self.target_created = True
                self.target_dir.chmod(0o700)
            else:
                secure_dir(self.target_dir)
            log_path = self.logs / "cargo-build.log"
            command = [
                cargo,
                "build",
                "--manifest-path",
                str(ROOT / "Cargo.toml"),
                "-p",
                "a4n-server",
                "-p",
                "a4n-control",
                "-p",
                "a4n-agent-worker",
                "-p",
                "a4n-edge",
                "-p",
                "a4n-export-service",
                "-p",
                "a4n-novel-planning-worker",
                "-p",
                "a4n-artifact-media-worker",
                "-p",
                "a4n-migration",
                "--target-dir",
                str(self.target_dir),
                "--locked",
            ]
            env = os.environ.copy()
            env["CARGO_INCREMENTAL"] = "1"
            env["CARGO_BUILD_BUILD_DIR"] = str(self.target_dir)
            started = time.monotonic()
            process = subprocess.run(
                command,
                cwd=str(ROOT),
                env=env,
                capture_output=True,
                text=True,
                timeout=self.args.build_timeout,
                check=False,
            )
            log_path.write_text((process.stdout or "") + (process.stderr or ""), encoding="utf-8")
            log_path.chmod(0o600)
            self.report["build"] = {
                "status": "passed" if process.returncode == 0 else "blocked",
                "elapsed_ms": round((time.monotonic() - started) * 1000),
                "target_dir": str(self.target_dir),
                "command": [part for part in command if part != str(ROOT / "Cargo.toml")],
                "log": str(log_path),
            }
            if process.returncode != 0:
                raise RunnerError(f"isolated Cargo build failed (exit={process.returncode}); see {log_path}")
            return (
                ensure_executable(self.target_dir / "debug/a4n-server", "a4n-server"),
                ensure_executable(self.target_dir / "debug/a4n-export-service", "a4n-export-service"),
                ensure_executable(self.target_dir / "debug/a4n-migration", "a4n-migration"),
            )
        self.report["build"] = {
            "status": "not_requested",
            "target_dir": str(self.target_dir),
            "binary_isolation": "not_proven for prebuilt binaries",
        }
        return (
            ensure_executable(self.args.host_binary or ROOT / "target/debug/a4n-server", "a4n-server"),
            ensure_executable(
                self.args.worker_binary or ROOT / "target/debug/a4n-export-service", "a4n-export-service"
            ),
            ensure_executable(self.args.migration_binary or ROOT / "target/debug/a4n-migration", "a4n-migration"),
        )

    def make_env(self, host_port: int, worker_port: int, username: str, password: str) -> None:
        base = os.environ.copy()
        for key in list(base):
            if key.startswith(("A4N_PUBLIC_", "A4N_OPENAI_", "A4N_REMOTE_")):
                base.pop(key, None)
        for key in (
            "A4N_RELEASE_MANIFEST_PATH",
            "A4N_SIDECAR_BIN_DIR",
            "A4N_QDRANT_URL",
            "A4N_QDRANT_COLLECTION",
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
            "NO_PROXY",
            "http_proxy",
            "https_proxy",
            "all_proxy",
            "no_proxy",
        ):
            base.pop(key, None)
        self.auth_token = "a4n-runtime-e2e-" + secrets.token_hex(16)
        common = {
            "A4N_DATABASE_URL": self.database_url,
            "A4N_JWT_SECRET": "a4n-runtime-e2e-jwt-" + secrets.token_hex(24),
            "A4N_WEB_ADDR": f"127.0.0.1:{host_port}",
            "A4N_EXPORT_SERVICE_ADDR": f"127.0.0.1:{worker_port}",
            "A4N_SIDECAR_AUTH_TOKEN": self.auth_token,
            "A4N_RUNTIME_PROFILE": "sidecar",
            "A4N_MANAGED_SIDECARS": "0",
            "A4N_PORTABLE_HOME": str(self.report_dir / "portable"),
            "A4N_PROMPT_SERVICE_PROMPTS_DIR": str(ROOT / "prompts"),
            "A4N_DATA_DIR": str(self.host_data),
            "A4N_CACHE_DIR": str(self.host_cache),
            "A4N_TEMP_DIR": str(self.host_temp),
            "A4N_LOG_DIR": str(self.host_logs),
            "A4N_FRONTEND_DIST": str(self.frontend),
            "A4N_BOOTSTRAP_ADMIN_USERNAME": username,
            "A4N_BOOTSTRAP_ADMIN_EMAIL": f"{username}@example.invalid",
            "A4N_BOOTSTRAP_ADMIN_PASSWORD": password,
        }
        self.host_env = {**base, **common}
        self.worker_env = {
            **base,
            "A4N_EXPORT_SERVICE_ADDR": f"127.0.0.1:{worker_port}",
            "A4N_SIDECAR_AUTH_TOKEN": self.auth_token,
            "A4N_PORTABLE_HOME": str(self.report_dir / "portable"),
            "A4N_DATA_DIR": str(self.worker_data),
            "A4N_CACHE_DIR": str(self.worker_cache),
            "A4N_TEMP_DIR": str(self.worker_temp),
            "A4N_LOG_DIR": str(self.worker_logs),
        }
        for key in (
            "A4N_DATABASE_URL",
            "A4N_JWT_SECRET",
            "A4N_WEB_ADDR",
            "A4N_FRONTEND_DIST",
            "A4N_BOOTSTRAP_ADMIN_USERNAME",
            "A4N_BOOTSTRAP_ADMIN_EMAIL",
            "A4N_BOOTSTRAP_ADMIN_PASSWORD",
        ):
            self.worker_env.pop(key, None)

    def worker_metrics(self, label: str) -> dict[str, Any]:
        result = http_request(
            self.worker_url + "/metrics",
            headers={"x-a4n-sidecar-token": self.auth_token},
            timeout=self.args.request_timeout,
        )
        self.record_http(label, "GET", self.worker_url + "/metrics", result, [])
        if result.status != 200:
            raise RunnerError(f"Worker metrics failed with HTTP {result.status}: {tail(self.logs / 'worker-process.log', 4_000)}")
        metrics = json_body(result, label)
        if not isinstance(metrics, dict):
            raise RunnerError(f"{label} returned an invalid export metrics envelope")
        validate_metrics_snapshot(metrics, label)
        self.metric_snapshot_count += 1
        safe_label = "".join(char if char.isalnum() or char in "-_" else "_" for char in label)
        write_json(
            self.report_dir / "metrics" / f"{self.metric_snapshot_count:04d}-{safe_label}.json",
            metrics,
        )
        return metrics

    def worker_render_metrics(self, label: str) -> dict[str, Any]:
        result = http_request(
            self.worker_url + "/export/metrics",
            headers={"x-a4n-sidecar-token": self.auth_token},
            timeout=self.args.request_timeout,
        )
        self.record_http(label, "GET", self.worker_url + "/export/metrics", result, [])
        if result.status != 200:
            raise RunnerError(
                f"Worker render metrics failed with HTTP {result.status}: "
                f"{tail(self.logs / 'worker-process.log', 4_000)}"
            )
        metrics = json_body(result, label)
        if not isinstance(metrics, dict):
            raise RunnerError(f"{label} returned an invalid export render metrics envelope")
        validate_render_metrics_snapshot(metrics, label)
        self.render_metric_snapshot_count += 1
        safe_label = "".join(char if char.isalnum() or char in "-_" else "_" for char in label)
        write_json(
            self.report_dir / "render-metrics" / f"{self.render_metric_snapshot_count:04d}-{safe_label}.json",
            metrics,
        )
        return metrics

    def wait_http(self, label: str, url: str, process: ManagedProcess, expected: set[int]) -> HttpResult:
        deadline = time.monotonic() + self.args.startup_timeout
        last: HttpResult | None = None
        while time.monotonic() < deadline:
            if process.process.poll() is not None:
                raise RunnerError(f"{label} exited early (exit={process.process.returncode}); see {process.log_path}")
            last = http_request(url, timeout=2)
            if last.status in expected and not last.error:
                return last
            self.sample_rss("启动")
            time.sleep(0.15)
        detail = f"HTTP {last.status}" if last else "no response"
        raise RunnerError(f"{label} was not ready in {self.args.startup_timeout:g}s ({detail}); see {process.log_path}")

    def start_stack(self, binaries: tuple[Path, Path, Path], username: str, password: str) -> None:
        host_port = free_port()
        worker_port = free_port()
        self.host_url = f"http://127.0.0.1:{host_port}"
        self.worker_url = f"http://127.0.0.1:{worker_port}"
        self.make_env(host_port, worker_port, username, password)
        self.started_ms = now_ms()
        worker_started = now_ms()
        self.worker = start_process(
            "worker",
            [str(binaries[1])],
            env=self.worker_env,
            cwd=ROOT,
            log_path=self.logs / "worker-process.log",
        )
        self.processes.append(self.worker)
        migration_started = now_ms()
        self.migration = start_process(
            "migration",
            [str(binaries[2])],
            env=self.host_env,
            cwd=ROOT,
            log_path=self.logs / "migration.log",
        )
        self.processes.append(self.migration)
        worker_health = self.wait_http("export Worker", self.worker_url + "/healthz", self.worker, {200})
        worker_health_data = json_body(worker_health, "Worker health")
        if (
            not isinstance(worker_health_data, dict)
            or worker_health_data.get("service") != "export"
            or worker_health_data.get("status") != "healthy"
            or worker_health_data.get("version", {}).get("protocol_version") != PROTOCOL_VERSION
            or worker_health_data.get("version", {}).get("capability_version") != EXPORT_CAPABILITY_VERSION
        ):
            raise RunnerError("Worker health did not satisfy the export protocol identity")
        write_json(self.report_dir / "worker-health.json", worker_health_data)
        worker_ready = now_ms()
        self.sample_rss("Worker ready")
        wait_process(self.migration, self.args.startup_timeout, "database migration")
        migration_done = now_ms()
        host_started = now_ms()
        self.host = start_process(
            "host",
            [str(binaries[0])],
            env=self.host_env,
            cwd=ROOT,
            log_path=self.logs / "host-process.log",
        )
        self.processes.append(self.host)
        host_health = self.wait_http("Host", self.host_url + "/api/health", self.host, {200})
        if host_health.body.strip() != b"ok":
            raise RunnerError(f"Host health returned unexpected body: {host_health.body[:120]!r}")
        host_ready = now_ms()
        self.report["ports"] = {"host": host_port, "worker": worker_port}
        self.report["startup"] = {
            "started_at_monotonic_ms": self.started_ms,
            "worker_spawn_ms": worker_started - self.started_ms,
            "worker_ready_ms": worker_ready - self.started_ms,
            "migration_spawn_ms": migration_started - self.started_ms,
            "migration_done_ms": migration_done - self.started_ms,
            "host_spawn_ms": host_started - self.started_ms,
            "host_ready_ms": host_ready - self.started_ms,
            "host_plus_worker_ready_ms": host_ready - self.started_ms,
        }
        self.report["processes"] = {
            "worker_pid": self.worker.process.pid,
            "host_pid": self.host.process.pid,
            "migration_pid": self.migration.process.pid,
            "worker_pgid": self.worker.pgid,
            "host_pgid": self.host.pgid,
        }
        self.sample_rss("Host ready")

    def command(self, label: str, command_name: str, args: dict[str, Any], token: str | None = None) -> Any:
        url = self.host_url + "/api/commands/" + command_name
        body = json.dumps(args, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
        headers = {"content-type": "application/json", "user-agent": "a4n-export-worker-runtime-e2e/1"}
        if token:
            headers["authorization"] = f"Bearer {token}"
        result = http_request(url, method="POST", body=body, headers=headers, timeout=self.args.request_timeout)
        self.record_http(label, "POST", url, result, sorted(args.keys()))
        if result.status != 200:
            detail = result.body.decode("utf-8", "replace")[:1_000]
            raise RunnerError(f"{label} returned HTTP {result.status}: {redact_error_text(detail)}")
        return json_body(result, label)

    def run_business_setup(self, username: str, password: str) -> str:
        auth = self.command("login_user", "login_user", {"input": {"username": username, "password": password}})
        token = auth.get("tokens", {}).get("access_token") if isinstance(auth, dict) else None
        if not isinstance(token, str) or not token:
            raise RunnerError("login_user did not return tokens.access_token")
        project = self.command(
            "create_project",
            "create_project",
            {
                "input": {
                    "title": f"A4N export runtime {self.run_id}",
                    "description": "Disposable project for the real export Worker runtime sample.",
                    "project_type": "novel_long",
                }
            },
            token,
        )
        project_id = project.get("id") if isinstance(project, dict) else None
        if not isinstance(project_id, int) or project_id <= 0:
            raise RunnerError("create_project did not return a positive project id")
        self.project_id = project_id
        self.report["business_setup"] = {"login": "passed", "create_project": "passed", "project_id": project_id}
        return token

    def export_once(self, token: str, label: str, index: int, output: bool = True) -> dict[str, Any]:
        if self.project_id is None:
            raise RunnerError("export requested before project setup")
        started = now_ms()
        value = self.command(
            label,
            "export_project_family_data",
            {"input": {"root_project_id": self.project_id, "include_derived_projects": False}},
            token,
        )
        if not isinstance(value, dict):
            raise RunnerError(f"{label} did not return an export object")
        file_name = value.get("file_name")
        encoded = value.get("data_base64")
        if not isinstance(file_name, str) or not file_name or not isinstance(encoded, str) or not encoded:
            raise RunnerError(f"{label} returned incomplete package data")
        try:
            package = base64.b64decode(encoded, validate=True)
        except (ValueError, binascii.Error) as error:
            raise RunnerError(f"{label} returned invalid base64 package data") from error
        if not package.startswith(b"PK"):
            raise RunnerError(f"{label} did not return a ZIP payload")
        try:
            with zipfile.ZipFile(io.BytesIO(package)) as archive:
                names = archive.namelist()
                bad_name = archive.testzip()
        except (OSError, zipfile.BadZipFile) as error:
            raise RunnerError(f"{label} returned an unreadable ZIP payload") from error
        if bad_name is not None or PROJECT_FAMILY_MANIFEST_PATH not in names:
            raise RunnerError(
                f"{label} ZIP failed integrity/manifest validation; "
                f"expected {PROJECT_FAMILY_MANIFEST_PATH!r}, entries={names!r}"
            )
        output_path = self.data / "exports" / f"{index:04d}-{Path(file_name).name}"
        if output:
            output_path.parent.mkdir(parents=True, exist_ok=True)
            output_path.write_bytes(package)
            output_path.chmod(0o600)
        return {
            "index": index,
            "label": label,
            "status": "passed",
            "elapsed_ms": now_ms() - started,
            "file_name": Path(file_name).name,
            "bytes": len(package),
            "sha256": hashlib.sha256(package).hexdigest(),
            "zip_entries": len(names),
            "output": str(output_path) if output else None,
        }

    def run_throughput(self, token: str) -> None:
        before = self.worker_metrics("business-before")
        self.phase = "business export"
        started = time.monotonic()
        records: list[dict[str, Any]] = []
        monitor = self.start_rss_monitor()
        try:
            with ThreadPoolExecutor(max_workers=self.args.concurrency) as executor:
                futures = {
                    executor.submit(self.export_once, token, f"export_{index:04d}", index): index
                    for index in range(1, self.args.request_count + 1)
                }
                for future in as_completed(futures):
                    records.append(future.result())
        finally:
            self.stop_rss_monitor(monitor)
        elapsed_ms = round((time.monotonic() - started) * 1000)
        records.sort(key=lambda record: record["index"])
        after = self.worker_metrics("business-after")
        delta = metrics_delta(before, after)
        successful = len(records)
        expected_json = successful
        expected_zip = successful
        if (
            delta["/export/render_json"]["requests_total"] != expected_json
            or delta["/export/render_zip"]["requests_total"] != expected_zip
            or delta["/export/render_json"]["errors_total"] != 0
            or delta["/export/render_zip"]["errors_total"] != 0
        ):
            raise RunnerError(
                "Worker IPC metrics did not match the successful Host export count; "
                f"delta={json.dumps(delta, sort_keys=True)}"
            )
        self.report["throughput"] = {
            "request_count": self.args.request_count,
            "successful": successful,
            "concurrency": self.args.concurrency,
            "elapsed_ms": elapsed_ms,
            "requests_per_second": round(successful / (elapsed_ms / 1000), 3) if elapsed_ms else None,
            "output_bytes": sum(record["bytes"] for record in records),
            "requests": records,
        }
        self.report["ipc"] = {
            "before": before,
            "after": after,
            "delta": delta,
            "expected_per_success": {"/export/render_json": 1, "/export/render_zip": 1},
        }
        write_json(self.report_dir / "ipc.json", self.report["ipc"])

    def direct_worker_json(self, label: str, body: dict[str, Any], timeout: float | None = None) -> HttpResult:
        encoded = json.dumps(body, separators=(",", ":")).encode("utf-8")
        result = http_request(
            self.worker_url + "/export/render_json",
            method="POST",
            body=encoded,
            headers={
                "content-type": "application/json",
                "x-a4n-sidecar-token": self.auth_token,
                "x-a4n-request-id": label,
                "x-a4n-trace-id": label,
            },
            timeout=timeout or self.args.request_timeout,
        )
        self.record_http(label, "POST", self.worker_url + "/export/render_json", result, ["payload"])
        return result

    def large_zip_body(self, mib: int, label: str) -> bytes:
        random_bytes = os.urandom(mib * 1024 * 1024)
        payload = {
            "manifest_path": "manifest.json",
            "manifest_bytes_base64": base64.b64encode(b'{"runtime":"fault"}').decode("ascii"),
            "manifest_label": "runtime fault manifest",
            "entries": [
                {
                    "path": "payload.bin",
                    "bytes_base64": base64.b64encode(random_bytes).decode("ascii"),
                    "label": "runtime fault payload",
                }
            ],
            "package_label": label,
        }
        return json.dumps(service_request(payload, label), separators=(",", ":")).encode("utf-8")

    def direct_worker_zip(self, label: str, body: bytes, timeout: float | None = None) -> HttpResult:
        result = http_request(
            self.worker_url + "/export/render_zip",
            method="POST",
            body=body,
            headers={
                "content-type": "application/json",
                "x-a4n-sidecar-token": self.auth_token,
                "x-a4n-request-id": label,
                "x-a4n-trace-id": label,
            },
            timeout=timeout or self.args.request_timeout,
        )
        self.record_http(label, "POST", self.worker_url + "/export/render_zip", result, ["payload"])
        return result

    def open_slow_worker_zip(self, label: str, body: bytes) -> dict[str, Any]:
        parsed = urlsplit(self.worker_url)
        if not parsed.hostname or parsed.port is None:
            raise RunnerError("Worker URL has no usable host/port")
        connection = http.client.HTTPConnection(
            parsed.hostname,
            parsed.port,
            timeout=self.args.request_timeout,
        )
        started = time.monotonic()
        try:
            connection.putrequest("POST", "/export/render_zip")
            connection.putheader("content-type", "application/json")
            connection.putheader("x-a4n-sidecar-token", self.auth_token)
            connection.putheader("x-a4n-request-id", label)
            connection.putheader("x-a4n-trace-id", label)
            connection.putheader("content-length", str(len(body)))
            connection.endheaders()
            connection.send(body)
            response = connection.getresponse()
        except (OSError, http.client.HTTPException) as error:
            connection.close()
            raise RunnerError(f"slow-consumer request {label} failed: {error}") from error
        return {
            "connection": connection,
            "response": response,
            "started": started,
            "label": label,
        }

    def drain_slow_worker_zip(self, handle: dict[str, Any]) -> HttpResult:
        if handle.get("drained"):
            return handle["result"]
        response = handle["response"]
        connection = handle["connection"]
        size = 0
        error: str | None = None
        try:
            while True:
                chunk = response.read(1024 * 1024)
                if not chunk:
                    break
                size += len(chunk)
        except (OSError, http.client.HTTPException) as caught:
            error = f"{type(caught).__name__}: {caught}"
        finally:
            connection.close()
        result = HttpResult(
            response.status,
            b"",
            round((time.monotonic() - handle["started"]) * 1000),
            error=error,
            response_bytes=size,
        )
        handle["drained"] = True
        handle["result"] = result
        self.record_http(handle["label"], "POST", self.worker_url + "/export/render_zip", result, ["payload"])
        return result

    def observe_slow_consumers(self, label: str) -> dict[str, Any]:
        before = self.worker_render_metrics(f"{label}-slow-before")
        slow_count = before["max_concurrent_renders"]
        queue_capacity = before["queue_capacity"]
        body = self.large_zip_body(min(max(self.args.fault_payload_mib, 8), 16), f"{label}-slow")
        handles: list[dict[str, Any]] = []
        open_errors: list[str] = []
        samples: list[dict[str, int]] = []
        poll_errors: list[str] = []
        try:
            with ThreadPoolExecutor(max_workers=slow_count) as executor:
                futures = [
                    executor.submit(self.open_slow_worker_zip, f"{label}-held-{index}", body)
                    for index in range(slow_count)
                ]
                for future in futures:
                    try:
                        handles.append(future.result())
                    except RunnerError as error:
                        open_errors.append(str(error))

            deadline = time.monotonic() + min(self.args.request_timeout, 5.0)
            while time.monotonic() < deadline:
                try:
                    metrics = self.worker_render_metrics(f"{label}-slow-sample")
                    samples.append(
                        {
                            "active_renders": metrics["active_renders"],
                            "queue_depth": metrics["queue_depth"],
                            "queue_rejections": metrics["queue_rejections"],
                        }
                    )
                    if metrics["queue_depth"] >= len(handles):
                        break
                except RunnerError as error:
                    poll_errors.append(redact_error_text(str(error), (self.auth_token,)))
                time.sleep(0.05)

            held_queue_depth = max((sample["queue_depth"] for sample in samples), default=0)
            queue_body = self.large_zip_body(1, f"{label}-queued")
            queued_count = queue_capacity + 1
            queue_executor = ThreadPoolExecutor(max_workers=queued_count)
            try:
                queue_futures = [
                    queue_executor.submit(self.direct_worker_zip, f"{label}-queued-{index}", queue_body)
                    for index in range(queued_count)
                ]
                drained = [self.drain_slow_worker_zip(handle) for handle in handles]
                queued_results = [future.result() for future in queue_futures]
            finally:
                queue_executor.shutdown(wait=True)

            rejected = sum(result.status == 429 for result in queued_results)
            successful = sum(service_response_ok(result) for result in queued_results)
            slow_response_statuses = [handle["response"].status for handle in handles]
            observed = slow_consumer_observation_passed(
                slow_response_statuses=slow_response_statuses,
                expected_slow_responses=slow_count,
                held_queue_depth=held_queue_depth,
                queue_capacity=queue_capacity,
                queued_results=queued_results,
                drained_results=drained,
            )
            return {
                "status": "observed" if observed else "partial",
                "slow_response_count": len(handles),
                "expected_slow_response_count": slow_count,
                "slow_response_statuses": slow_response_statuses,
                "metrics_samples": len(samples),
                "metrics_poll_errors": poll_errors,
                "max_queue_depth_while_body_held": held_queue_depth,
                "queued_requests": queued_count,
                "expected_queued_successful_responses": queue_capacity,
                "queued_successful_responses": successful,
                "expected_queued_rejected_responses": 1,
                "queued_rejected_responses": rejected,
                "drained_response_statuses": [result.status for result in drained],
                "drained_error_count": sum(result.error is not None for result in drained),
                "open_errors": open_errors,
                "fairness": {
                    "status": "not_applicable",
                    "reason": "export is the only package served by this Worker; no multi-package scheduler is in scope",
                },
            }
        finally:
            for handle in handles:
                self.drain_slow_worker_zip(handle)

    def fault_not_applicable(self, fault: str, reason: str) -> dict[str, Any]:
        return {"status": "not_applicable", "reason": reason}

    def fault_e3(self) -> dict[str, Any]:
        if self.worker is None:
            return {"status": "blocked", "reason": "Worker was not started"}
        label = f"fault-E3-{self.run_id}"
        body = self.large_zip_body(self.args.fault_payload_mib, label)
        sent = 0
        sent_lock = threading.Lock()
        holder: dict[str, Any] = {"result": None}
        parsed = urlsplit(self.worker_url)

        def upload() -> None:
            nonlocal sent
            connection: http.client.HTTPConnection | None = None
            try:
                connection = http.client.HTTPConnection(parsed.hostname, parsed.port, timeout=self.args.request_timeout)
                connection.putrequest("POST", "/export/render_zip")
                connection.putheader("content-type", "application/json")
                connection.putheader("x-a4n-sidecar-token", self.auth_token)
                connection.putheader("x-a4n-request-id", label)
                connection.putheader("x-a4n-trace-id", label)
                connection.putheader("content-length", str(len(body)))
                connection.endheaders()
                for offset in range(0, len(body), 64 * 1024):
                    chunk = body[offset : offset + 64 * 1024]
                    connection.send(chunk)
                    with sent_lock:
                        sent += len(chunk)
                    time.sleep(0.003)
                response = connection.getresponse()
                holder["result"] = HttpResult(response.status, read_limited(response), 0)
            except (OSError, http.client.HTTPException) as error:
                holder["result"] = HttpResult(None, b"", 0, f"{type(error).__name__}: {error}")
            finally:
                if connection:
                    connection.close()

        thread = threading.Thread(target=upload, name="fault-e3-upload")
        thread.start()
        deadline = time.monotonic() + min(self.args.request_timeout, 10.0)
        while thread.is_alive() and time.monotonic() < deadline:
            with sent_lock:
                progress = sent
            if progress >= min(1024 * 1024, len(body)):
                break
            time.sleep(0.01)
        with sent_lock:
            sent_before_kill = sent
        killed_at_ms = now_ms()
        old_worker_pid = self.worker.process.pid
        old_worker_pgid = self.worker.pgid
        try:
            stop_process(self.worker, force=True)
        except (OSError, RunnerError) as error:
            thread.join(timeout=2)
            return {"status": "blocked", "reason": f"protected/failed Worker termination: {error}"}
        thread.join(timeout=10)
        holder_result = holder["result"]
        interrupted = bool(holder_result and holder_result.status is None and holder_result.error)
        self.record_http(
            "fault_E3_interrupted_upload",
            "POST",
            self.worker_url + "/export/render_zip",
            holder_result or HttpResult(None, b"", 0, "request thread did not finish"),
            ["payload"],
        )
        self.worker = start_process(
            "worker-restart-after-E3",
            [str(self.worker_binary)],
            env=self.worker_env,
            cwd=ROOT,
            log_path=self.logs / "worker-restart-E3.log",
        )
        self.processes.append(self.worker)
        health = self.wait_http("restarted export Worker", self.worker_url + "/healthz", self.worker, {200})
        restarted = health.status == 200
        recovery = None
        if restarted and self.last_token:
            try:
                recovery = self.export_once(self.last_token, "fault_E3_recovery_export", 9001)
            except RunnerError as error:
                recovery = {"status": "failed", "reason": str(error)}
        recovery_passed = isinstance(recovery, dict) and recovery.get("status") == "passed"
        evidence = {
            "worker_transport_interruption": {
                "status": "passed" if interrupted else "failed",
                "sent_bytes_before_kill": sent_before_kill,
                "request_error": holder_result.error if holder_result else None,
            },
            "worker_restart": {
                "status": "passed" if restarted else "failed",
                "old_worker_pid": old_worker_pid,
                "new_worker_pid": self.worker.process.pid,
            },
            "post_restart_host_export": {
                "status": "passed" if recovery_passed else "failed",
                "result": recovery,
            },
            "remote_ack": {
                "status": "not_applicable",
                "observed": False,
                "reason": "export has no remote Provider job or ACK endpoint",
            },
            "remote_occupancy": {
                "status": "not_applicable",
                "observed": False,
                "reason": "export has no remote Provider occupancy to query",
            },
            "host_checkpoint": {
                "status": "not_applicable",
                "observed": False,
                "reason": "this download path has no resumable Host checkpoint",
            },
        }
        self.report.setdefault("process_events", []).append(
            {
                "event": "E3_worker_killed_and_restarted",
                "old_worker_pid": old_worker_pid,
                "old_worker_pgid": old_worker_pgid,
                "sent_bytes_before_kill": sent_before_kill,
                "killed_at_elapsed_ms": killed_at_ms - self.started_ms,
                "new_worker_pid": self.worker.process.pid,
            }
        )
        details = {
            "evidence": evidence,
            "sent_bytes_before_kill": sent_before_kill,
            "request_error": holder_result.error if holder_result else None,
            "recovery_export": recovery,
        }
        if sent_before_kill <= 0 or not interrupted or not restarted:
            return {
                "status": "failed",
                "reason": "could not prove a real in-flight request was interrupted and the Worker recovered",
                **details,
            }
        if not recovery_passed:
            return {
                "status": "failed",
                "reason": "Worker restarted, but the post-restart Host export did not complete",
                **details,
            }
        return {
            "status": "partial",
            "reason": "real Worker transport interruption and restart were observed",
            "scope_limit": "export has no Provider ACK, remote occupancy, or Host checkpoint to validate",
            **details,
        }

    def fault_e6(self) -> dict[str, Any]:
        label = f"fault-E6-{self.run_id}"
        body = self.large_zip_body(self.args.fault_payload_mib, label)
        before = self.worker_metrics("fault-E6-before")
        before_render = self.worker_render_metrics("fault-E6-render-before")
        started = time.monotonic()
        samples: list[dict[str, Any]] = []
        poll_errors: list[str] = []
        stop_poll = threading.Event()

        def poll_metrics() -> None:
            while not stop_poll.wait(0.03):
                try:
                    metrics = self.worker_render_metrics("fault-E6-render-sample")
                    samples.append(
                        {
                            "active_renders": metrics["active_renders"],
                            "peak_active_renders": metrics["peak_active_renders"],
                            "queue_depth": metrics["queue_depth"],
                            "peak_queue_depth": metrics["peak_queue_depth"],
                            "queue_rejections": metrics["queue_rejections"],
                        }
                    )
                except RunnerError as error:
                    poll_errors.append(redact_error_text(str(error), (self.auth_token,)))

        poller = threading.Thread(target=poll_metrics, name="fault-e6-metrics", daemon=True)
        poller.start()
        results: list[HttpResult] = []
        try:
            expected_admitted = before_render["max_concurrent_renders"] + before_render["queue_capacity"]
            total_requests = expected_admitted + 2
            with ThreadPoolExecutor(max_workers=total_requests) as executor:
                futures = [
                    executor.submit(self.direct_worker_zip, f"{label}-{index}", body)
                    for index in range(total_requests)
                ]
                for future in as_completed(futures):
                    results.append(future.result())
        finally:
            stop_poll.set()
            poller.join(timeout=3)
        if poller.is_alive():
            raise RunnerError("E6 metrics polling thread did not stop")
        after = self.worker_metrics("fault-E6-after")
        after_render = self.worker_render_metrics("fault-E6-render-after")
        delta = metrics_delta(before, after)
        try:
            slow_consumer = self.observe_slow_consumers(label)
        except RunnerError as error:
            slow_consumer = {
                "status": "partial",
                "error": redact_error_text(str(error), (self.auth_token,)),
            }
        success_count = sum(service_response_ok(result) for result in results)
        rejection_count = sum(result.status == 429 for result in results)
        observed_active_renders = [
            value["active_renders"]
            for value in samples
            if isinstance(value.get("active_renders"), int)
        ]
        max_active_renders = max(observed_active_renders, default=0)
        observed_queue_depths = [
            value["queue_depth"]
            for value in samples
            if isinstance(value.get("queue_depth"), int)
        ]
        queue_rejections = after_render["queue_rejections"] - before_render["queue_rejections"]
        elapsed_ms = round((time.monotonic() - started) * 1000)
        evidence = {
            "business_requests": {
                "status": "passed" if success_count == expected_admitted else "failed",
                "successful_responses": success_count,
                "expected_admitted_responses": expected_admitted,
                "total_requests": total_requests,
                "queue_rejected_responses": rejection_count,
                "protocol_status_checked": True,
            },
            "metrics_polling": {
                "status": "observed" if samples and not poll_errors else "partial",
                "route": "/export/metrics",
                "successful_polls": len(samples),
                "failed_polls": len(poll_errors),
                "errors": poll_errors,
                "route_is_outside_global_render_concurrency": True,
            },
            "queue": {
                "status": "observed" if observed_queue_depths and queue_rejections > 0 else "partial",
                "observed": bool(observed_queue_depths),
                "max_depth_observed": max(observed_queue_depths, default=0),
                "queue_rejections": queue_rejections,
                "http_429_responses": rejection_count,
            },
            "active_render": {
                "status": "observed" if after_render["peak_active_renders"] > 0 else "partial",
                "observed": after_render["peak_active_renders"] > 0,
                "reason": (
                    "the export handler counts only the render operation after the global concurrency layer "
                    "has admitted the request"
                ),
            },
            "slow_consumer": slow_consumer,
        }
        details = {
            "success_count": success_count,
            "expected_success_count": expected_admitted,
            "total_requests": total_requests,
            "queue_rejection_count": rejection_count,
            "metrics_delta": delta,
            "render_metrics_before": before_render,
            "render_metrics_after": after_render,
            "max_active_renders_observed": max_active_renders,
            "metrics_poll_attempts": len(samples) + len(poll_errors),
            "metrics_poll_successes": len(samples),
            "metrics_poll_errors": poll_errors,
            "sample_count": len(samples),
            "elapsed_ms": elapsed_ms,
            "slow_consumer": slow_consumer,
            "evidence": evidence,
        }
        if (
            success_count != expected_admitted
            or rejection_count < 1
            or queue_rejections < 1
            or delta["/export/render_zip"]["requests_total"] != total_requests
        ):
            return {
                "status": "failed",
                "reason": "E6 bounded admission did not produce the expected admitted and rejected request counts",
                **details,
            }
        if not samples or after_render["peak_active_renders"] < 2:
            return {
                "status": "partial",
                "reason": "bounded requests completed, but active render sampling did not observe two simultaneous render operations",
                "scope_limit": "multi-package fairness is outside this single-package Worker; slow-consumer evidence is recorded separately",
                **details,
            }
        reason = "bounded Worker admission observed active renders, queue depth, and queue-full rejection"
        if poll_errors:
            reason += f"; {len(poll_errors)} metrics poll(s) failed and remain explicit"
        return {
            "status": "partial",
            "reason": reason,
            "scope_limit": "multi-package fairness is outside this single-package Worker; inspect slow_consumer evidence separately",
            **details,
        }

    def run_fault(self, fault: str) -> None:
        if fault == "E1":
            self.report["faults"][fault] = self.fault_not_applicable(
                fault, "export project-family rendering has no Provider request, ACK, or external job"
            )
        elif fault == "E2":
            self.report["faults"][fault] = self.fault_not_applicable(
                fault, "export is read-only after migration; it has no DB commit ACK to lose"
            )
        elif fault == "E3":
            self.report["faults"][fault] = self.fault_e3()
        elif fault == "E4":
            self.report["faults"][fault] = self.fault_not_applicable(
                fault, "export has no provider credential snapshot or resumable Host checkpoint"
            )
        elif fault == "E5":
            self.report["faults"][fault] = self.fault_not_applicable(
                fault, "export produces a download and does not CAS-write a mutable target"
            )
        elif fault == "E6":
            self.report["faults"][fault] = self.fault_e6()

    def fault_verdict(self) -> tuple[str, bool]:
        if self.args.fault == "none":
            return "passed", True
        statuses = [entry.get("status") for entry in self.report["faults"].values()]
        if "failed" in statuses:
            return "failed", False
        if "blocked" in statuses:
            return "blocked", False
        if any(status in {"not_applicable", "partial", "not_run"} for status in statuses):
            return "partial", False
        return "passed", True

    def summarize_rss(self) -> None:
        host_values = [sample["host"]["tree_rss_kb"] for sample in self.rss_samples if sample.get("host")]
        worker_values = [sample["worker"]["tree_rss_kb"] for sample in self.rss_samples if sample.get("worker")]
        combined_values = [
            sample["combined_tree_rss_kb"]
            for sample in self.rss_samples
            if sample.get("combined_tree_rss_kb") is not None
        ]
        self.report["rss"] = {
            "sample_count": len(self.rss_samples),
            "host_tree_peak_rss_kb": max(host_values) if host_values else None,
            "worker_tree_peak_rss_kb": max(worker_values) if worker_values else None,
            "combined_tree_peak_rss_kb": max(combined_values) if combined_values else None,
            "layers": {
                "host_root_and_descendants": "host.tree_rss_kb",
                "worker_root_and_descendants": "worker.tree_rss_kb",
                "combined": "combined_tree_rss_kb only when both roots were observed",
            },
        }

    def cleanup_build_target(self) -> None:
        if not self.target_created or self.args.keep_target:
            return
        if self.target_dir.is_symlink() or self.target_dir.resolve() != self.target_dir:
            raise RunnerError(f"refusing changed build cache path: {self.target_dir}")
        if not self.target_dir.exists():
            return
        env = os.environ.copy()
        env["CARGO_TARGET_DIR"] = str(self.target_dir)
        env["CARGO_BUILD_BUILD_DIR"] = str(self.target_dir)
        process = subprocess.run(
            ["cargo", "clean", "--offline", "--manifest-path", str(ROOT / "Cargo.toml"), "--target-dir", str(self.target_dir)],
            cwd=ROOT, env=env, capture_output=True, text=True, timeout=60,
        )
        log = self.logs / "cargo-cleanup.log"
        log.write_text(process.stdout + process.stderr, encoding="utf-8")
        log.chmod(0o600)
        if process.returncode != 0 or self.target_dir.exists():
            raise RunnerError(f"temporary Cargo cache cleanup failed; see {log}")

    def cleanup(self) -> list[str]:
        errors: list[str] = []
        for process in reversed(self.processes):
            try:
                stop_process(process)
            except (OSError, RunnerError, subprocess.TimeoutExpired) as error:
                errors.append(f"{process.label}: {error}")
        if not errors:
            try:
                self.cleanup_build_target()
            except (OSError, RunnerError, subprocess.TimeoutExpired) as error:
                errors.append(str(error))
        self.report["build_cache"] = {
            "path": str(self.target_dir),
            "created_by_run": self.target_created,
            "retained": self.target_dir.exists(),
        }
        try:
            self.drop_created_database()
        except RunnerError as error:
            errors.append(str(error))
        return errors

    def run(self) -> int:
        password = os.environ.get(self.args.password_env, "")
        username = self.args.username.strip() or f"a4n_export_worker_{self.run_id.replace('-', '')}"
        try:
            if not password:
                raise RunnerError(f"password environment variable {self.args.password_env} is empty")
            self.set_database()
            binaries = self.resolve_binaries()
            self.report["configuration"] = {
                "host_binary": str(binaries[0]),
                "worker_binary": str(binaries[1]),
                "migration_binary": str(binaries[2]),
                "target_dir": str(self.target_dir),
                "data_dir": str(self.data),
                "cache_dir": str(self.cache),
                "temp_dir": str(self.temp),
                "log_dir": str(self.logs),
                "provider": "not started; export path is expected to be provider-free",
            }
            self.worker_binary = binaries[1]
            self.start_stack(binaries, username, password)
            self.phase = "business setup"
            self.last_token = self.run_business_setup(username, password)
            self.run_throughput(self.last_token)
            self.report["business"] = {
                "status": "passed",
                "flow": ["login_user", "create_project", "export_project_family_data"],
                "host_to_worker": "verified by render_json/render_zip metrics deltas",
            }
            if self.args.fault == "all":
                for fault in FAULTS:
                    self.run_fault(fault)
            elif self.args.fault != "none":
                self.run_fault(self.args.fault)
            else:
                for fault in FAULTS:
                    self.report["faults"][fault] = {
                        "status": "not_run",
                        "reason": "use --fault E1..E6 or --fault all; no fault is silently counted as pass",
                    }
            status, success = self.fault_verdict()
            self.report["status"] = status
            self.report["success"] = success
            self.report["runtime_verdict"] = (
                "real business path and bounded runtime observations recorded"
                if success
                else "business path ran, but the requested fault scope is incomplete or failed"
            )
        except Exception as error:
            self.report["status"] = "blocked" if isinstance(error, RunnerError) else "failed"
            self.report["error"] = redact_error_text(str(error), (password, self.auth_token, self.database_url))
        finally:
            self.summarize_rss()
            cleanup_errors = self.cleanup()
            if cleanup_errors:
                self.report["cleanup_errors"] = cleanup_errors
                self.report["status"] = "blocked"
                self.report["success"] = False
            self.report["finished_at"] = datetime.now(timezone.utc).isoformat()
            self.report["artifacts"] = {
                "report": str(self.report_dir / "report.json"),
                "http_log": str(self.http_log),
                "rss_jsonl": str(self.rss_jsonl),
                "rss_tsv": str(self.rss_tsv),
                "metrics_dir": str(self.report_dir / "metrics"),
                "render_metrics_dir": str(self.report_dir / "render-metrics"),
                "logs_dir": str(self.logs),
                "data_dir": str(self.data),
            }
            write_json(self.report_dir / "report.json", self.report)
        fault_statuses = [entry.get("status") for entry in self.report["faults"].values()]
        if self.report.get("success") is not True:
            return 2
        if self.args.fault != "none" and any(status in {"not_applicable", "partial", "not_run"} for status in fault_statuses):
            return 3
        return 0


def self_test() -> None:
    assert database_name("postgres://user:pass@127.0.0.1:5432/a4n_test") == "a4n_test"
    assert database_name("postgres://user:pass@127.0.0.1:5432/a4n", reject_development=False) == "a4n"
    try:
        database_name("postgres://user:pass@127.0.0.1:5432/a4n")
    except RunnerError:
        pass
    else:
        raise AssertionError("development database must be rejected")
    assert "pass" not in redacted_url("postgres://user:pass@127.0.0.1:5432/a4n_test")
    request = service_request({"payload": {"ok": True}})
    assert request["schema_version"] == {"protocol": 2}
    assert request["secret_policy"]["scope"] == "none"

    def metric_operation(requests_total: int) -> dict[str, int]:
        return {
            field: requests_total if field == "requests_total" else 0
            for field in METRIC_COUNTER_FIELDS
        }

    def metric_snapshot(json_requests: int, zip_requests: int) -> dict[str, Any]:
        return {
            "service": "export",
            "pid": 1,
            "started_at_unix_ms": 1,
            "uptime_ms": 1,
            "restart_count": 0,
            "requests_total": json_requests + zip_requests,
            "errors_total": 0,
            "cancelled_total": 0,
            "in_flight": 0,
            "duration_us_total": 0,
            "duration_us_max": 0,
            "operations": {
                "/export/render_json": metric_operation(json_requests),
                "/export/render_zip": metric_operation(zip_requests),
            },
        }

    before = metric_snapshot(0, 1)
    del before["operations"]["/export/render_json"]
    del before["operations"]["/export/render_zip"]
    after = metric_snapshot(2, 2)
    assert metrics_delta(before, after)["/export/render_zip"]["requests_total"] == 2
    assert validate_render_metrics_snapshot(
        {
            "service": "export",
            "active_renders": 2,
            "peak_active_renders": 3,
            "queue_depth": 1,
            "peak_queue_depth": 2,
            "queue_rejections": 1,
            "max_concurrent_renders": 4,
            "queue_capacity": 4,
        },
        "render metrics",
    )["active_renders"] == 2
    broken = metric_snapshot(2, 3)
    del broken["operations"]["/export/render_json"]
    try:
        metrics_delta(before, broken)
    except RunnerError:
        pass
    else:
        raise AssertionError("missing post-run operation metrics must fail explicitly")
    assert service_response_ok(HttpResult(200, b'{"status":"ok","payload":{}}', 0))
    assert not service_response_ok(HttpResult(200, b'{"status":"error"}', 0))
    queue_capacity = 4
    queued_results = [
        HttpResult(200, b'{"status":"ok","payload":{}}', 0)
        for _ in range(queue_capacity)
    ] + [HttpResult(429, b"queue full", 0)]
    drained_results = [HttpResult(200, b"", 0, response_bytes=1) for _ in range(2)]
    observation = {
        "slow_response_statuses": [200, 200],
        "expected_slow_responses": 2,
        "held_queue_depth": 2,
        "queue_capacity": queue_capacity,
        "queued_results": queued_results,
        "drained_results": drained_results,
    }
    assert slow_consumer_observation_passed(**observation)
    assert not slow_consumer_observation_passed(
        **{
            **observation,
            "queued_results": [HttpResult(429, b"queue full", 0) for _ in range(queue_capacity + 1)],
        }
    ), "queue rejections without successful releases must not count as observed"
    assert not slow_consumer_observation_passed(
        **{
            **observation,
            "drained_results": [HttpResult(200, b"", 0, error="truncated response"), drained_results[1]],
        }
    ), "held responses with drain errors must not count as observed"
    from unittest.mock import patch
    from unittest import TestCase

    with patch.object(os, "getpgrp", return_value=10), patch.object(os, "getpgid", return_value=20), patch.object(os, "getsid", return_value=20):
        assert_owned_process_group(20, 20)
        for pid, pgid in ((10, 10), (20, 10), (21, 21)):
            with TestCase().assertRaises(RunnerError):
                assert_owned_process_group(pid, pgid)
    with patch.object(os, "getpgrp", return_value=10), patch.object(os, "getpgid", return_value=20), patch.object(os, "getsid", return_value=10):
        with TestCase().assertRaises(RunnerError):
            assert_owned_process_group(20, 20)

    error_body = io.BytesIO(b"rejected")
    error = HTTPError("http://fixture.invalid", 422, "rejected", {}, error_body)
    with patch.object(HTTP, "open", side_effect=error):
        assert http_request("http://fixture.invalid").body == b"rejected"
    assert error_body.closed, "HTTP error responses must be closed"

    with tempfile.TemporaryDirectory(prefix="a4n-cache-ownership-") as directory:
        root = Path(directory).resolve()
        child = start_process("ownership", [sys.executable, "-c", "import time; time.sleep(60)"],
                              env=os.environ.copy(), cwd=root, log_path=root / "child.log")
        try:
            child.pgid = os.getpgrp()
            with TestCase().assertRaises(RunnerError):
                stop_process(child)
            assert child.process.poll() is None, "invalid ownership must not signal even the spawned child"
        finally:
            child.pgid = child.process.pid
            stop_process(child)
        assert child.process.poll() is not None and child.handle.closed
        args = argparse.Namespace(target_dir=None, keep_target=False)
        runner = RuntimeRunner(args, root / "report", "cache-self-test")
        runner.target_dir.mkdir()
        (runner.target_dir / "output").write_text("disposable")
        runner.cleanup_build_target()
        assert runner.target_dir.exists(), "caller-owned cache must be retained"
        runner.target_created = True
        args.keep_target = True
        runner.cleanup_build_target()
        assert runner.target_dir.exists(), "--keep-target must retain the cache"
        args.keep_target = False
        with patch.object(subprocess, "run", return_value=subprocess.CompletedProcess([], 9, "", "cleanup failed")):
            try:
                runner.cleanup_build_target()
            except RunnerError:
                pass
            else:
                raise AssertionError("cache cleanup failure must be visible")

        def clean_fixture(command: list[str], **kwargs: Any) -> subprocess.CompletedProcess[str]:
            assert command[-2:] == ["--target-dir", str(runner.target_dir)]
            assert kwargs["env"]["CARGO_BUILD_BUILD_DIR"] == str(runner.target_dir)
            shutil.rmtree(runner.target_dir)
            return subprocess.CompletedProcess(command, 0, "", "")

        with patch.object(subprocess, "run", side_effect=clean_fixture):
            runner.cleanup_build_target()
        assert not runner.target_dir.exists() and runner.logs.is_dir()
        if os.name == "posix":
            outside = root / "keep"
            outside.mkdir()
            runner.target_dir.symlink_to(outside, target_is_directory=True)
            try:
                runner.cleanup_build_target()
            except RunnerError:
                pass
            else:
                raise AssertionError("linked target must never be cleaned")
            assert outside.is_dir()
    print("export worker runtime e2e self-test passed")


def main() -> int:
    args = parse_args()
    if args.self_test:
        self_test()
        return 0
    try:
        report_dir = prepare_report(args.report_dir)
        run_id = utc_run_id()
        runner = RuntimeRunner(args, report_dir, run_id)
        code = runner.run()
        print(json.dumps(runner.report, ensure_ascii=False, indent=2))
        print(f"export Worker runtime report: {report_dir}", file=sys.stderr)
        return code
    except RunnerError as error:
        print(f"export Worker runtime blocked: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
