#!/usr/bin/env python3
"""Measure real Server + all catalog workers using an isolated synthetic database."""

import argparse
from concurrent.futures import ThreadPoolExecutor
import http.client
import json
import math
import os
from pathlib import Path
import secrets
import socket
import statistics
import subprocess
import sys
import time
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/tests"))
from run_export_worker_runtime_e2e import proc_rss_tree, start_process, stop_process, wait_process


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bin-dir", type=Path, required=True)
    parser.add_argument("--state-dir", type=Path, required=True)
    parser.add_argument("--report-dir", type=Path, required=True)
    parser.add_argument("--database-url-env", default="A4N_PERF_DATABASE_URL")
    parser.add_argument("--postgres-container", default="debug-postgres-1")
    return parser.parse_args()


def free_ports(count):
    sockets = []
    try:
        for _ in range(count):
            sock = socket.socket()
            sock.bind(("127.0.0.1", 0))
            sockets.append(sock)
        return [sock.getsockname()[1] for sock in sockets]
    finally:
        for sock in sockets:
            sock.close()


def resources(pid):
    tree = proc_rss_tree(pid)
    if tree is None:
        raise RuntimeError("Server exited before resource measurement")
    rows = []
    for child in tree["pids"]:
        proc = Path("/proc") / str(child)
        status = dict(line.split(":", 1) for line in (proc / "status").read_text().splitlines())
        smaps = dict(line.split(":", 1) for line in (proc / "smaps_rollup").read_text().splitlines()[1:])
        rows.append({"pid": child, "binary": Path(os.readlink(proc / "exe")).name,
                     "rss_kib": int(status["VmRSS"].split()[0]), "pss_kib": int(smaps["Pss"].split()[0]),
                     "threads": int(status["Threads"])})
    return {"processes": rows, "total_pss_kib": sum(row["pss_kib"] for row in rows),
            "total_threads": sum(row["threads"] for row in rows)}


def command(connection, name, args, token=None):
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    body = json.dumps(args, ensure_ascii=False).encode()
    started = time.perf_counter()
    connection.request("POST", f"/api/commands/{name}", body, headers)
    response = connection.getresponse()
    payload = response.read()
    elapsed_ms = (time.perf_counter() - started) * 1000
    if response.status != 200:
        raise RuntimeError(f"{name}: HTTP {response.status}: {payload[:500].decode(errors='replace')}")
    return json.loads(payload), elapsed_ms, len(payload)


def main():
    args = arguments()
    database_url = os.environ[args.database_url_env]
    database = urlsplit(database_url)
    if database.hostname != "127.0.0.1" or database.port != 5432 or not database.path.startswith("/a4n_perf_"):
        raise RuntimeError("Requires an explicitly isolated loopback a4n_perf_* database")
    state, output, binaries = args.state_dir.resolve(), args.report_dir.resolve(), args.bin_dir.resolve()
    for directory in (state, output):
        if not any(directory.is_relative_to(ROOT / base) for base in ("data", ".dev-runtime")):
            raise RuntimeError("State and reports must stay inside this worktree's data or .dev-runtime directory")
    if not any(binaries.is_relative_to(ROOT / base) for base in ("data", "target", ".dev-runtime")):
        raise RuntimeError("Benchmark binaries must be inside this worktree")
    state.mkdir(parents=True, exist_ok=True)
    output.mkdir(parents=True, exist_ok=False)
    state.chmod(0o700)
    output.chmod(0o700)
    credentials_path = state / "credentials.json"
    if credentials_path.exists():
        credentials = json.loads(credentials_path.read_text())
    else:
        credentials = {"username": "performance-audit", "password": secrets.token_urlsafe(32)}
        credentials_path.write_text(json.dumps(credentials))
        credentials_path.chmod(0o600)

    workers = json.loads((ROOT / "crates/process/a4n-process-catalog/worker-catalog.json").read_text())
    web_port, control_port, *worker_ports = free_ports(len(workers) + 2)
    env = {key: value for key, value in os.environ.items()
           if not key.startswith("A4N_") and key.lower() not in {"http_proxy", "https_proxy", "all_proxy"}}
    env.update({
        "A4N_DATABASE_URL": database_url,
        "A4N_JWT_SECRET": secrets.token_urlsafe(48),
        "A4N_WEB_ADDR": f"127.0.0.1:{web_port}",
        "A4N_CONTROL_ADDR": f"127.0.0.1:{control_port}",
        "A4N_CONTROL_BINARY": str(binaries / "a4n-control"),
        "A4N_EDGE_BINARY": str(binaries / "a4n-edge"),
        "A4N_RUNTIME_PROFILE": "sidecar",
        "A4N_MANAGED_SIDECARS": "0",
        "A4N_SIDECAR_AUTH_TOKEN": secrets.token_urlsafe(32),
        "A4N_PORTABLE_HOME": str(state / "portable"),
        "A4N_FRONTEND_DIST": str(ROOT / "frontend/dist"),
        "A4N_PROMPT_SERVICE_PROMPTS_DIR": str(ROOT / "prompts"),
        "A4N_BOOTSTRAP_ADMIN_USERNAME": credentials["username"],
        "A4N_BOOTSTRAP_ADMIN_PASSWORD": credentials["password"],
        "A4N_BOOTSTRAP_ADMIN_EMAIL": "performance-audit@example.invalid",
        "RUST_LOG": "warn",
    })
    for kind in ["DATA", "CACHE", "TEMP", "LOG"]:
        directory = state / kind.lower()
        directory.mkdir(exist_ok=True)
        env[f"A4N_{kind}_DIR"] = str(directory)
    for worker, port in zip(workers, worker_ports):
        env[worker["addr_env"]] = f"127.0.0.1:{port}"
        prefix = "A4N_" + worker["name"].replace("-", "_").upper()
        env[prefix + "_BINARY"] = str(binaries / worker["binary"])
        env[prefix + "_TOKEN"] = secrets.token_urlsafe(24)

    report = {"success": False, "database": database.path[1:], "catalog_workers": len(workers), "metrics": []}
    server = None
    try:
        migration = start_process("migration", [str(binaries / "a4n-migration")], env=env, cwd=state, log_path=output / "migration.log")
        try:
            wait_process(migration, 60, "migration")
        finally:
            stop_process(migration)
        started = time.perf_counter()
        server = start_process("server", [str(binaries / "a4n-server")], env=env, cwd=state, log_path=output / "server.log")
        deadline = time.monotonic() + 90
        while True:
            if server.process.poll() is not None:
                raise RuntimeError(f"Server exited: {server.process.returncode}; see server.log")
            connection = http.client.HTTPConnection("127.0.0.1", web_port, timeout=1)
            try:
                connection.request("GET", "/api/health")
                response = connection.getresponse()
                if response.status == 200 and response.read() == b"ok":
                    break
            except (OSError, http.client.HTTPException):
                pass
            finally:
                connection.close()
            if time.monotonic() >= deadline:
                raise RuntimeError("Server readiness timed out; see server.log")
            time.sleep(0.05)
        report["startup_ms"] = (time.perf_counter() - started) * 1000
        report["idle"] = resources(server.process.pid)
        (output / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2))
        print("Ready", round(report["startup_ms"], 1), "ms; PSS", report["idle"]["total_pss_kib"], "KiB", flush=True)

        connection = http.client.HTTPConnection("127.0.0.1", web_port, timeout=60)
        auth, _, _ = command(connection, "login_user", {"input": credentials})
        token = auth["tokens"]["access_token"]
        listing = {"page": 1, "pageSize": 100, "accessScope": "all"}
        page, _, _ = command(connection, "list_projects", listing, token)
        if not page["items"]:
            for index in range(20):
                command(connection, "create_project", {"input": {"title": f"性能基准 {index}", "description": "synthetic performance fixture", "project_type": "novel_long"}}, token)
            page, _, _ = command(connection, "list_projects", listing, token)
        assert page["total"] == 20 and all(item["title"].startswith("性能基准 ") for item in page["items"])
        project_id = page["items"][0]["id"]
        assert isinstance(project_id, int) and project_id > 0
        sql = """
INSERT INTO chapter (project_id, chapter_number, title, content, word_count, status, created_at, updated_at)
SELECT :'project_id'::bigint, i, '性能基准章节 ' || i, repeat('章节性能内容。', 512),
       char_length(repeat('章节性能内容。', 512)), 'draft', now(), now()
FROM generate_series(1, 100) i
WHERE NOT EXISTS (SELECT 1 FROM chapter WHERE project_id = :'project_id'::bigint);
"""
        seeded = subprocess.run(["docker", "exec", "-i", args.postgres_container, "psql", "-U", database.username,
                                 "-d", database.path[1:], "-v", "ON_ERROR_STOP=1", "-v", f"project_id={project_id}"],
                                input=sql, text=True, capture_output=True, timeout=60)
        if seeded.returncode:
            raise RuntimeError(seeded.stderr)
        connection.close()
        cases = [
            ("get_current_user", {}, lambda value: value["username"] == credentials["username"]),
            ("list_projects", listing, lambda value: value["total"] == 20 and len(value["items"]) == 20),
            ("get_project", {"projectId": project_id}, lambda value: value["id"] == project_id),
            ("list_chapters", {"projectId": project_id}, lambda value: len(value) == 100 and all(item["content"] == "章节性能内容。" * 512 for item in value)),
        ]
        for name, payload, check in cases:
            for concurrency in [1, 4, 8]:
                def series(_):
                    client = http.client.HTTPConnection("127.0.0.1", web_port, timeout=60)
                    measurements = []
                    try:
                        for index in range(35):
                            value, elapsed, size = command(client, name, payload, token)
                            assert check(value), name
                            if index >= 5:
                                measurements.append({"elapsed_ms": elapsed, "bytes": size})
                    finally:
                        client.close()
                    return measurements
                with ThreadPoolExecutor(max_workers=concurrency) as pool:
                    measurements = [row for series_rows in pool.map(series, range(concurrency)) for row in series_rows]
                values = sorted(row["elapsed_ms"] for row in measurements)
                metric = {"command": name, "concurrency": concurrency, "requests": len(values),
                          "p50_ms": statistics.median(values), "p95_ms": values[math.ceil(len(values) * 0.95) - 1],
                          "max_ms": values[-1], "response_bytes": measurements[-1]["bytes"], "samples": measurements}
                report["metrics"].append(metric)
                (output / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2))
                print(name, concurrency, "p95", round(metric["p95_ms"], 2), "ms", flush=True)
        report["after_load"] = resources(server.process.pid)
        report["success"] = True
    except BaseException as error:
        report["error"] = str(error)
        raise
    finally:
        stop_process(server)
        (output / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
