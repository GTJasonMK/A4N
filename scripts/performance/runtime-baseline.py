#!/usr/bin/env python3
"""Measure real Server + all catalog workers using an isolated synthetic database."""

import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import gzip
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
    parser.add_argument("--qdrant-url", default="http://127.0.0.1:6334", help="loopback Qdrant gRPC endpoint")
    parser.add_argument("--chapter-history", type=int, default=0, help="completed content-sync runs per fixture chapter")
    parser.add_argument("--chapter-count", type=int, default=100)
    parser.add_argument("--concurrency", type=int, nargs="+", default=[1, 4, 8])
    parser.add_argument("--cli-checks", action="store_true", help="run existing CLI/Mock Provider suites after measurement; use a separate disposable database")
    parser.add_argument("--chapter-index-checks", action="store_true", help="verify narrow chapter reads against every full response and reject wrong scope")
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


def command(connection, name, args, token=None, accept_gzip=False):
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    if accept_gzip:
        headers["Accept-Encoding"] = "gzip"
    body = json.dumps(args, ensure_ascii=False).encode()
    started = time.perf_counter()
    connection.request("POST", f"/api/commands/{name}", body, headers)
    response = connection.getresponse()
    payload = response.read()
    elapsed_ms = (time.perf_counter() - started) * 1000
    if response.status != 200:
        raise RuntimeError(f"{name}: HTTP {response.status}: {payload[:500].decode(errors='replace')}")
    encoding = response.getheader("Content-Encoding")
    if encoding not in (None, "gzip"):
        raise RuntimeError(f"Unsupported response encoding: {encoding}")
    return json.loads(gzip.decompress(payload) if encoding == "gzip" else payload), elapsed_ms, len(payload)


def cli_checks(binaries, output, env, server_url, credentials):
    from run_cli_contract_e2e import start_provider

    temporary = output / "tmp"
    temporary.mkdir()
    cli_env = {**env, "A4N_PERF_PASSWORD": credentials["password"], "TMPDIR": str(temporary)}
    common = ["--a4n", str(binaries / "a4n"), "--server", server_url, "--username", credentials["username"],
              "--password-env", "A4N_PERF_PASSWORD"]
    checks = []
    try:
        for name, dimension, arguments in [
            ("run_cli_contract_e2e", 8, ["--ledger", str(ROOT / "scripts/tests/cli_contract_core_workflow.json"),
                                         "--report-dir", str(output / "cli-core"), "--command-timeout", "60", "--fail-fast"]),
            # Scene import uses the project's default 384-dimensional embedding before its model scenarios.
            ("run_scene_sync_cli_e2e", 384, ["--embedding-dimension", "384", "--report", str(output / "cli-scene.json")]),
            ("run_llm_lifecycle_cli_e2e", 384, ["--report", str(output / "cli-llm.json")]),
        ]:
            provider, provider_log = start_provider(argparse.Namespace(
                provider_url=None, provider_host="127.0.0.1", provider_port=0, embedding_dimension=dimension,
                provider_script=ROOT / "scripts/tests/mock_openai_provider.py", provider_start_timeout=10,
            ), output, name)
            if name != "run_cli_contract_e2e":
                arguments += ["--provider-log", str(provider_log)]
            cli_env["A4N_E2E_EMBEDDING_DIMENSION"] = str(dimension)
            try:
                started = time.perf_counter()
                with (output / f"{name}.log").open("w") as log:
                    result = subprocess.run([sys.executable, str(ROOT / "scripts/tests" / (name + ".py")),
                                             *common, "--provider-url", provider.base_url, *arguments],
                                            env=cli_env, stdout=log, stderr=subprocess.STDOUT, timeout=360)
            finally:
                provider.stop()
            checks.append({"suite": name, "exit_code": result.returncode, "elapsed_ms": (time.perf_counter() - started) * 1000})
            if result.returncode:
                raise RuntimeError(f"{name} failed: exit {result.returncode}; see its log")
        return checks
    finally:
        (output / "cli-checks.json").write_text(json.dumps(checks, indent=2))


def main():
    args = arguments()
    if not 0 <= args.chapter_history <= 20:
        raise RuntimeError("chapter-history must be between 0 and 20")
    if not 1 <= args.chapter_count <= 1000 or any(n not in (1, 4, 8) for n in args.concurrency):
        raise RuntimeError("chapter-count must be 1..1000 and concurrency must use 1, 4 or 8")
    qdrant_url = urlsplit(args.qdrant_url)
    if qdrant_url.scheme != "http" or qdrant_url.hostname != "127.0.0.1" or not qdrant_url.port:
        raise RuntimeError("Requires a loopback Qdrant gRPC endpoint")
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
    services = json.loads((ROOT / "crates/process/a4n-process-catalog/service-catalog.json").read_text())
    web_port, control_port, *ports = free_ports(len(workers) + len(services) + 2)
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
        "A4N_MANAGED_SIDECARS": "1",
        "A4N_SIDECAR_BIN_DIR": str(binaries),
        "A4N_SIDECAR_AUTH_TOKEN": secrets.token_urlsafe(32),
        "A4N_PORTABLE_HOME": str(state / "portable"),
        "A4N_FRONTEND_DIST": str(ROOT / "frontend/dist"),
        "A4N_PROMPT_SERVICE_PROMPTS_DIR": str(ROOT / "prompts"),
        "A4N_ONNXRUNTIME_ROOT": str(Path.home() / ".local/share/a4n/onnxruntime" / (ROOT / "tools/onnxruntime/VERSION").read_text().strip()),
        "A4N_QDRANT_URL": args.qdrant_url,
        "A4N_QDRANT_COLLECTION": database.path[1:],
        "A4N_RAG_SERVICE_DATA_DIR": str(state / "data/rag"),
        "A4N_BOOTSTRAP_ADMIN_USERNAME": credentials["username"],
        "A4N_BOOTSTRAP_ADMIN_PASSWORD": credentials["password"],
        "A4N_BOOTSTRAP_ADMIN_EMAIL": "performance-audit@example.invalid",
        "RUST_LOG": "warn",
    })
    for kind in ["DATA", "CACHE", "TEMP", "LOG"]:
        directory = state / kind.lower()
        directory.mkdir(exist_ok=True)
        env[f"A4N_{kind}_DIR"] = str(directory)
    for worker, port in zip(workers, ports):
        env[worker["addr_env"]] = f"127.0.0.1:{port}"
        prefix = "A4N_" + worker["name"].replace("-", "_").upper()
        env[prefix + "_BINARY"] = str(binaries / worker["binary"])
        env[prefix + "_TOKEN"] = secrets.token_urlsafe(24)
    for service, port in zip(services, ports[len(workers):]):
        env[service["addr_env"]] = f"127.0.0.1:{port}"

    report = {"success": False, "database": database.path[1:], "catalog_workers": len(workers),
              "configured_services": len(services), "metrics": []}
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
FROM generate_series(1, :'chapter_count'::integer) i
WHERE NOT EXISTS (SELECT 1 FROM chapter WHERE project_id = :'project_id'::bigint);
"""
        seeded = subprocess.run(["docker", "exec", "-i", args.postgres_container, "psql", "-U", database.username,
                                 "-d", database.path[1:], "-v", "ON_ERROR_STOP=1", "-v", f"project_id={project_id}",
                                 "-v", f"chapter_count={args.chapter_count}"],
                                input=sql, text=True, capture_output=True, timeout=60)
        if seeded.returncode:
            raise RuntimeError(seeded.stderr)
        report["project_id"] = project_id
        report["chapter_history"] = args.chapter_history
        report["chapter_count"] = args.chapter_count
        if args.chapter_history:
            history_sql = """
INSERT INTO workflow_run (business_key, workflow_kind, project_id, chapter_id,
    input_fingerprint_json, status, current_step_key, can_resume, created_at, updated_at, completed_at)
SELECT 'perf-content-sync:' || c.id || ':' || revision, 'chapter_content_sync', c.project_id, c.id,
    jsonb_build_object('schema_version', 3, 'workflow_kind', 'chapter_content_sync',
      'sync_revision', 'perf-' || revision,
      'source_fingerprint', jsonb_build_object('schema_version', 1, 'workflow_kind', 'chapter_content_sync',
        'project_id', c.project_id, 'chapter_id', c.id, 'chapter_number', c.chapter_number,
        'title', c.title, 'content_sha256', encode(sha256(convert_to(c.content, 'UTF8')), 'hex'))),
    'completed', 'chapter_content_sync.current_rag', false,
    now() - (:'history'::integer - revision + 1) * interval '1 day',
    now() - (:'history'::integer - revision + 1) * interval '1 day', now()
FROM chapter c CROSS JOIN generate_series(1, :'history'::integer) revision
WHERE c.project_id = :'project_id'::bigint
  AND NOT EXISTS (SELECT 1 FROM workflow_run w WHERE w.business_key = 'perf-content-sync:' || c.id || ':' || revision);
INSERT INTO workflow_checkpoint (workflow_run_id, step_key, step_order, status, checkpoint_json, created_at, updated_at, completed_at)
SELECT w.id, step.key, step.ordinal, 'completed',
       jsonb_build_object('completed', true, 'result', jsonb_build_object(),
          'context', (SELECT string_agg(md5(i::text), '') FROM generate_series(1, 2000) i)), now(), now(), now()
FROM workflow_run w CROSS JOIN (VALUES ('chapter_content_sync.current_rag', 1), ('llm.persisted.call', 2)) AS step(key, ordinal)
WHERE w.project_id = :'project_id'::bigint AND w.business_key LIKE 'perf-content-sync:%'
  AND NOT EXISTS (SELECT 1 FROM workflow_checkpoint c WHERE c.workflow_run_id = w.id AND c.step_key = step.key);
"""
            seeded = subprocess.run(["docker", "exec", "-i", args.postgres_container, "psql", "-U", database.username,
                                     "-d", database.path[1:], "-v", "ON_ERROR_STOP=1", "-v", f"project_id={project_id}",
                                     "-v", f"history={args.chapter_history}"], input=history_sql, text=True,
                                    capture_output=True, timeout=60)
            if seeded.returncode:
                raise RuntimeError(seeded.stderr)
        fixture_sql = """
WITH runs AS (SELECT id, workflow_kind FROM workflow_run WHERE project_id = :'project_id'::bigint
              AND workflow_kind IN ('chapter_analysis', 'chapter_content_sync'))
SELECT json_build_object(
    'content_sync_runs', (SELECT count(*) FROM runs WHERE workflow_kind = 'chapter_content_sync'),
    'analysis_runs', (SELECT count(*) FROM runs WHERE workflow_kind = 'chapter_analysis'),
    'checkpoints', (SELECT count(*) FROM workflow_checkpoint WHERE workflow_run_id IN (SELECT id FROM runs)),
    'checkpoint_json_bytes', (SELECT coalesce(sum(octet_length(checkpoint_json::text)), 0)
                             FROM workflow_checkpoint WHERE workflow_run_id IN (SELECT id FROM runs)));
"""
        fixture_result = subprocess.run(["docker", "exec", "-i", args.postgres_container, "psql", "-At", "-U", database.username,
                                         "-d", database.path[1:], "-v", "ON_ERROR_STOP=1", "-v", f"project_id={project_id}"],
                                        input=fixture_sql, text=True, capture_output=True, timeout=60)
        if fixture_result.returncode:
            raise RuntimeError(fixture_result.stderr)
        report["fixture"] = json.loads(fixture_result.stdout)
        if (report["fixture"]["content_sync_runs"] != args.chapter_count * args.chapter_history
                or report["fixture"]["analysis_runs"] != 0
                or report["fixture"]["checkpoints"] != 2 * args.chapter_count * args.chapter_history):
            raise RuntimeError("Fixture history differs from --chapter-history; use a fresh isolated database")
        plain, _, plain_bytes = command(connection, "list_chapters", {"projectId": project_id}, token)
        encoded, _, encoded_bytes = command(connection, "list_chapters", {"projectId": project_id}, token, accept_gzip=True)
        assert encoded == plain, "Negotiated compression changed the real chapter response"
        report["compression_probe"] = {"identity_bytes": plain_bytes, "gzip_wire_bytes": encoded_bytes,
                                       "payloads_identical": True}
        if args.chapter_index_checks:
            index, index_ms, index_bytes = command(connection, "list_chapter_index", {"projectId": project_id}, token)
            heavy_fields = {"content", "edit_revision", "style_tags", "analysis_data"}
            assert index == [{**{key: value for key, value in chapter.items() if key not in heavy_fields},
                              "body_character_count": len(chapter["content"])} for chapter in plain]
            for chapter in [plain[0], plain[len(plain) // 2], plain[-1]]:
                detail, _, _ = command(connection, "get_chapter", {"input": {"project_id": project_id, "chapter_id": chapter["id"]}}, token)
                assert detail == chapter, "Scoped chapter differs from the full API"
            other_project = next(item["id"] for item in page["items"] if item["id"] != project_id)
            rejections = []
            for name, payload, headers, expected in [
                ("get_chapter", {"input": {"project_id": other_project, "chapter_id": plain[0]["id"]}}, {"Authorization": f"Bearer {token}"}, 404),
                ("get_chapter", {"input": {"project_id": 0, "chapter_id": plain[0]["id"]}}, {"Authorization": f"Bearer {token}"}, 400),
                ("get_chapter", {"input": {"project_id": project_id, "chapter_id": plain[0]["id"]}}, {}, 401),
                ("list_chapter_index", {"projectId": project_id}, {}, 401),
            ]:
                connection.request("POST", f"/api/commands/{name}", json.dumps(payload), {"Content-Type": "application/json", **headers})
                response = connection.getresponse()
                error = json.loads(response.read())
                assert response.status == expected, (name, response.status, error)
                rejections.append({"command": name, "status": response.status})
            report["chapter_index"] = {"elapsed_ms": index_ms, "response_bytes": index_bytes,
                                        "all_metadata_equal": True, "sampled_bodies_equal": True, "rejections": rejections}
        connection.close()
        cases = [
            ("get_current_user", {}, lambda value: value["username"] == credentials["username"]),
            ("list_projects", listing, lambda value: value["total"] == 20 and len(value["items"]) == 20),
            ("get_project", {"projectId": project_id}, lambda value: value["id"] == project_id),
            ("list_chapters", {"projectId": project_id}, lambda value: len(value) == args.chapter_count
             and [item["chapter_number"] for item in value] == list(range(1, args.chapter_count + 1))
             and all(item["content"] == "章节性能内容。" * 512
                     and (not args.chapter_history or (item["derived_sync_status"] == "completed"
                          and item["derived_sync_workflow_run_id"] > 0
                          and item["material_sync"]["content_index"] == "completed")) for item in value)),
        ]
        for name, payload, check in cases:
            for concurrency in args.concurrency:
                def series(_):
                    client = http.client.HTTPConnection("127.0.0.1", web_port, timeout=60)
                    measurements = []
                    try:
                        for index in range(35):
                            value, elapsed, size = command(client, name, payload, token)
                            assert check(value), name
                            if index >= 5:
                                digest = hashlib.sha256(json.dumps(value, sort_keys=True, ensure_ascii=False).encode()).hexdigest() if name == "list_chapters" else None
                                measurements.append({"elapsed_ms": elapsed, "bytes": size, "response_sha256": digest})
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
        if args.cli_checks:
            report["cli_checks"] = cli_checks(binaries, output, env, f"http://127.0.0.1:{web_port}", credentials)
        report["success"] = True
    except BaseException as error:
        report["error"] = str(error)
        raise
    finally:
        stop_process(server)
        (output / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
