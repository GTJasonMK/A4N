#!/usr/bin/env python3
"""Run creation-stage acceptance against a fresh local stack and real Chromium."""

from __future__ import annotations

import argparse
import fcntl
import hashlib
import json
import os
import secrets
import signal
import shutil
import subprocess
import sys
import tempfile
import time
import uuid
from pathlib import Path
from urllib.parse import quote

from run_browser_full_project_e2e import (
    RunnerError, configure_onnxruntime, ensure_executable, free_port,
    provider_audit, secure_mkdir, start_process, stop_process, wait_http, write_report,
    tail,
)
from run_cli_contract_e2e import Cli, checked, secure_json
from run_all_workflows_e2e import FIXTURE_PATHS

ROOT = Path(__file__).resolve().parents[2]
STAGES = ("inspiration", "blueprint", "outline", "chapter", "analysis", "director", "shooting", "segments", "media")
SUITES = {
    "authoring": ("run_authoring_stage_cli_e2e.py", set(STAGES[:5])),
    "checkpoints": ("run_checkpoint_call_cli_e2e.py", {"blueprint", "shooting", "segments", "media"}),
    "facts": ("run_fact_feedback_cli_e2e.py", {"chapter", "analysis"}),
    "director": ("run_director_timeline_cli_e2e.py", set(STAGES[5:])),
    "shooting": ("run_shooting_async_cli_e2e.py", {"shooting", "segments"}),
}
FULL_GENERATION_SUITES = {
    "media_authoring": "run_media_authoring_cli_e2e.py",
    "character_sync_resume": "run_character_sync_resume_cli_e2e.py",
    "scene_sync": "run_scene_sync_cli_e2e.py",
    "agent_delegation": "run_agent_cli_e2e.py",
    "agent_domains": "run_agent_domains_cli_e2e.py",
    "specialized_agent": "run_specialized_agent_cli_e2e.py",
    "llm_lifecycle": "run_llm_lifecycle_cli_e2e.py",
    "llm_cache": "run_llm_cache_cli_e2e.py",
    "image_publication": "run_image_publication_cli_e2e.py",
    "image_checkpoint_recovery": "run_image_checkpoint_recovery_cli_e2e.py",
    "portrait_generation": "run_portrait_generation_cli_e2e.py",
    "all_workflows": "run_all_workflows_e2e.py",
}
POST_BROWSER_SUITES = {"image_checkpoint_recovery", "all_workflows"}
BINARIES = ("a4n", "a4n-server", "a4n-migration", "a4n-novel-planning-worker")
EMBEDDING_DIMENSION = 384


def source_fingerprint(root: Path = ROOT) -> str:
    paths = [root / name for name in ("Cargo.toml", "Cargo.lock", "tools/onnxruntime/VERSION", "scripts/cargo.sh", "scripts/lib/common.sh")]
    paths += [root / name for name in (".cargo/config", ".cargo/config.toml", "rust-toolchain", "rust-toolchain.toml") if (root / name).is_file()]
    paths += [path for path in (root / "crates").rglob("*") if path.is_file()]
    paths += [path for path in (root / "prompts").rglob("*") if path.is_file()]
    return files_fingerprint(root, paths)


def frontend_fingerprint(root: Path = ROOT) -> str:
    paths = [path for directory in ("src", "public", "resources") for path in (root / "frontend" / directory).rglob("*") if path.is_file()]
    paths += [path for path in (root / "frontend").iterdir() if path.is_file() and path.suffix in {".json", ".ts", ".html"}]
    return files_fingerprint(root, paths)


def files_fingerprint(root: Path, paths: list[Path]) -> str:
    digest = hashlib.sha256()
    for path in sorted(paths):
        digest.update(str(path.relative_to(root)).encode() + b"\0")
        digest.update(hashlib.sha256(path.read_bytes()).digest())
    return digest.hexdigest()


def snapshot_source(report_dir: Path, application_source: Path | None = None) -> Path:
    destination = secure_mkdir(report_dir / "source")
    files = subprocess.check_output(["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"], cwd=ROOT).decode().split("\0")
    test_files = [name for name in files if name.startswith(("scripts/tests/", "frontend/e2e/"))]
    application = application_source.resolve() if application_source else ROOT
    if application_source:
        files = [path.name for path in application.iterdir() if path.is_file()]
        for directory in (".cargo", "crates", "prompts", "frontend", "scripts", "tools/onnxruntime"):
            for parent, directories, names in os.walk(application / directory):
                directories[:] = [name for name in directories if name not in {"node_modules", "dist", "__pycache__"}]
                files += [str((Path(parent) / name).relative_to(application)) for name in names]
    for name in files:
        if not name or Path(name).name.startswith(".env") or not (name.startswith((".cargo/", "crates/", "prompts/", "frontend/", "scripts/", "tools/onnxruntime/")) or "/" not in name):
            continue
        if name.startswith(("scripts/tests/", "frontend/e2e/")):
            continue
        path = application / name
        if path.is_file():
            target = destination / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, target)
    for name in test_files:
        path = ROOT / name
        if path.is_file():
            target = destination / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, target)
    (destination / "frontend/node_modules").symlink_to(ROOT / "frontend/node_modules", target_is_directory=True)
    # Stop dotenv's parent search at the test snapshot, before the user's worktree configuration.
    (destination / ".env").write_text("# Isolated creation E2E environment is supplied by the runner.\n")
    return destination


def build_binaries(report_dir: Path, supplied: Path | None, source: Path) -> Path:
    fingerprint = source_fingerprint(source)
    if supplied:
        supplied = supplied.resolve()
        receipt = json.loads((supplied / "source.json").read_text())
        if receipt["source_sha256"] != fingerprint:
            raise RunnerError("指定二进制与本次被测源码不同；请去掉 --binaries-dir 重新构建")
        for name in BINARIES:
            ensure_executable(supplied / name, name)
            if hashlib.sha256((supplied / name).read_bytes()).hexdigest() != receipt["binaries"][name]:
                raise RunnerError(f"二进制校验值变化：{name}")
        destination = secure_mkdir(report_dir / "bin")
        for name in (*BINARIES, "source.json"):
            shutil.copy2(supplied / name, destination / name)
        return destination
    common = Path(subprocess.check_output(["git", "rev-parse", "--path-format=absolute", "--git-common-dir"], cwd=ROOT, text=True).strip())
    work = common.parent / "work"
    if not ROOT.is_relative_to(work):
        raise RunnerError("必须从 work/debug、work/integration 或 work/agent 工作树运行")
    env = os.environ.copy()
    env.update(CARGO_TARGET_DIR=str(work / ".build/agent-target"), CARGO_INCREMENTAL="0",
               CARGO_ARTIFACTS_DISABLED="1", CARGO_BUILD_JOBS="2",
               CARGO_PROFILE_DEV_DEBUG="0", CARGO_PROFILE_TEST_DEBUG="0")
    destination = secure_mkdir(report_dir / "bin")
    with (work / ".agent-build.lock").open("a") as lock, (report_dir / "build.log").open("w") as log:
        fcntl.flock(lock, fcntl.LOCK_EX)
        try:
            subprocess.run(["bash", "-c", 'set -euo pipefail; source "$1/scripts/lib/common.sh"; require_onnxruntime; exec "$1/scripts/cargo.sh" build --manifest-path "$1/Cargo.toml" --locked -p a4n-server -p a4n-cli -p a4n-migration -p a4n-novel-planning-worker --features a4n-server/embedded-capabilities', "creation-e2e", str(source)],
                           cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
            if source_fingerprint(source) != fingerprint:
                raise RunnerError("构建修改了输入快照，无法证明验证版本一致")
            hashes = {}
            for name in BINARIES:
                shutil.copy2(Path(env["CARGO_TARGET_DIR"]) / "debug" / name, destination / name)
                hashes[name] = hashlib.sha256((destination / name).read_bytes()).hexdigest()
            secure_json(destination / "source.json", {"source_sha256": fingerprint, "binaries": hashes})
        finally:
            subprocess.run([str(ROOT / "scripts/cargo.sh"), "clean"], cwd=ROOT, env=env,
                           stdout=log, stderr=subprocess.STDOUT, check=True)
    return destination


def selected_suites(stages: set[str]) -> list[str]:
    unknown = stages - set(STAGES)
    if unknown or not stages:
        raise RunnerError(f"无效阶段：{sorted(unknown) if unknown else '空'}")
    selected = [name for name, (_, covered) in SUITES.items() if stages & covered]
    if stages == set(STAGES):
        selected.extend(FULL_GENERATION_SUITES)
    return selected


def full_generation_matrix_report(suites: dict) -> dict:
    failed = [
        name
        for name in FULL_GENERATION_SUITES
        if suites.get(name, {}).get("success") is not True
    ]
    return {
        "status": "failed" if failed else "passed",
        "required_suites": list(FULL_GENERATION_SUITES),
        "failed_suites": failed,
    }


def stop_owned_workers(binary_dir: Path, browser_artifacts: Path | None = None) -> None:
    def workers():
        owned = []
        for process in Path("/proc").iterdir():
            if not process.name.isdigit():
                continue
            try:
                executable = (process / "cmdline").read_bytes().split(b"\0", 1)[0].decode()
            except (FileNotFoundError, PermissionError, UnicodeDecodeError):
                continue
            is_browser = False
            if browser_artifacts is not None:
                try:
                    marker = f"A4N_BROWSER_E2E_ARTIFACT_DIR={browser_artifacts}".encode()
                    is_browser = marker in (process / "environ").read_bytes().split(b"\0")
                except (FileNotFoundError, PermissionError, ProcessLookupError):
                    pass
            if executable and (Path(executable).parent == binary_dir or is_browser):
                owned.append(int(process.name))
        return owned
    for sig in (signal.SIGTERM, signal.SIGKILL):
        for pid in workers():
            try:
                os.kill(pid, sig)
            except ProcessLookupError:
                pass
        deadline = time.monotonic() + 5
        while workers() and time.monotonic() < deadline:
            time.sleep(.1)
    if workers():
        raise RunnerError("本轮 Worker 尚未退出")


def acceptance_result(stages: set[str], suites: dict, browser: dict) -> dict:
    result = {}
    for stage in STAGES:
        if stage not in stages:
            continue
        required = selected_suites({stage})
        checks = {}
        for name in required:
            suite = suites.get(name, {})
            if name == "authoring":
                checks[name] = suite.get("stages", {}).get(stage, {}).get("status") == "passed" and any(
                    check.get("stage") == stage for check in suite.get("checks", []) if isinstance(check, dict)
                )
            else:
                checks[name] = suite.get("success") is True and bool(suite.get("checks"))
        checks["browser"] = browser.get(stage, {}).get("status") == "passed"
        browser_status = browser.get(stage, {}).get("status")
        status = "passed" if all(checks.values()) else "blocked" if browser_status in (None, "not_run") else "failed"
        result[stage] = {"status": status, "checks": checks}
    return result


def acceptance_passed(report: dict) -> bool:
    generation_matrix = report.get("generation_matrix", {}).get("status", "not_requested")
    return (report.get("browser", {}).get("exit_code") == 0
            and bool(report.get("stages")) and bool(report.get("suites"))
            and all(stage["status"] == "passed" for stage in report["stages"].values())
            and all(suite["success"] is True for suite in report["suites"].values())
            and generation_matrix in {"passed", "not_requested"})


def read_suite_report(path: Path, exit_code: int) -> dict:
    result = {"success": False, "exit_code": exit_code, "report": str(path), "checks": [], "stages": {}}
    try:
        value = json.loads(path.read_text())
        if not isinstance(value, dict) or not isinstance(value.get("checks"), list) or not isinstance(value.get("stages", {}), dict):
            raise RunnerError("测试组报告格式错误")
        result.update(checks=value["checks"], stages=value.get("stages", {}))
        result["success"] = exit_code == 0 and value.get("success") is True and bool(value["checks"])
        if not result["success"]:
            result["error"] = "测试组超过 1800 秒" if exit_code == 124 else value.get("error") or "测试组未成功完成或未产生检查"
    except (OSError, ValueError, RunnerError) as error:
        result["error"] = f"无法读取测试组结果：{error}"
    return result


def read_all_workflows_report(path: Path, exit_code: int) -> dict:
    result = {"success": False, "exit_code": exit_code, "report": str(path), "checks": []}
    try:
        value = json.loads(path.read_text())
        fixtures = value.get("fixtures") if isinstance(value, dict) else None
        coverage = fixtures.get("paths") if isinstance(fixtures, dict) else None
        paths = coverage.get("paths") if isinstance(coverage, dict) else None
        if not isinstance(coverage, dict) or not isinstance(paths, dict):
            raise RunnerError("全工作流报告缺少 fixtures.paths.paths")
        missing = sorted(set(FIXTURE_PATHS) - set(paths))
        if missing:
            raise RunnerError(f"全工作流报告缺少业务路径：{missing}")
        malformed = [name for name in FIXTURE_PATHS if not isinstance(paths[name], dict)]
        if malformed:
            raise RunnerError(f"全工作流报告业务路径格式错误：{malformed}")
        result["checks"] = [
            {"path": name, "status": paths[name].get("status")}
            for name in FIXTURE_PATHS
        ]
        failed = [
            name
            for name in FIXTURE_PATHS
            if paths[name].get("status") != "passed"
        ]
        result["success"] = (
            exit_code == 0
            and value.get("success") is True
            and value.get("mode") == "fixture_only"
            and fixtures.get("enabled") is True
            and fixtures.get("only") is True
            and coverage.get("complete") is True
            and not failed
        )
        if not result["success"]:
            result["error"] = value.get("fatal_error") or f"全工作流 fixture 未完整通过：{failed}"
    except (OSError, ValueError, RunnerError) as error:
        result["error"] = f"无法读取全工作流结果：{error}"
    return result


def stop_postgres(container: str, log_path: Path) -> None:
    try:
        with log_path.open("a") as log:
            subprocess.run(["docker", "logs", container], stdout=log, stderr=subprocess.STDOUT, check=True, timeout=10)
    finally:
        subprocess.run(["docker", "rm", "-f", container], stdout=subprocess.DEVNULL, check=True, timeout=30)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--stage", action="append", choices=("all", *STAGES), default=[])
    parser.add_argument("--binaries-dir", type=Path, help="复用本工具构建且校验值仍匹配的 bin 目录")
    parser.add_argument("--application-source", type=Path, help="重现某份已保存的源码快照；报告明确记录被测版本，测试脚本使用当前版本")
    parser.add_argument("--report-dir", type=Path)
    parser.add_argument("--postgres-image", default=os.environ.get("A4N_E2E_POSTGRES_IMAGE", "postgres:16"))
    parser.add_argument("--list", action="store_true", help="列出阶段和实际运行的测试组")
    args = parser.parse_args()
    stages = set(STAGES) if not args.stage or "all" in args.stage else set(args.stage)
    suites = selected_suites(stages)
    if args.list:
        print(json.dumps({"stages": [s for s in STAGES if s in stages], "cli_suites": suites, "browser": "generation-stages.e2e.mjs"}, ensure_ascii=False, indent=2))
        return 0
    run_id = time.strftime("%Y%m%dT%H%M%S") + "-" + uuid.uuid4().hex[:8]
    if args.report_dir and args.report_dir.exists() and any(args.report_dir.iterdir()):
        raise RunnerError("--report-dir 必须是新的空目录，避免覆盖证据或复用旧数据库")
    report_dir = (args.report_dir or ROOT / "data/creation-e2e" / run_id).resolve()
    if not report_dir.is_relative_to(ROOT):
        raise RunnerError("报告和运行数据必须位于当前工作树内")
    secure_mkdir(report_dir)
    full_generation_run = stages == set(STAGES)
    report = {
        "success": False,
        "run_id": run_id,
        "requested_stages": sorted(stages),
        "suites": {},
        "stages": {stage: {"status": "not_run"} for stage in sorted(stages)},
        "generation_matrix": {
            "status": "not_run" if full_generation_run else "not_requested",
            "required_suites": list(FULL_GENERATION_SUITES) if full_generation_run else [],
            "failed_suites": [],
        },
        "failures": [],
    }
    processes = []
    browser_tmp = None
    container = "a4n-creation-e2e-" + run_id.lower()
    postgres_started = False
    try:
        for program in ("cargo", "node", "npm", "docker", "ffmpeg"):
            if shutil.which(program) is None:
                raise RunnerError(f"缺少运行依赖：{program}")
        ensure_executable(ROOT / "tools/qdrant/bin/qdrant", "Qdrant")
        ensure_executable(Path(os.environ.get("A4N_WDIO_CHROMIUM_BINARY", "/usr/bin/chromium")), "Chromium")
        ensure_executable(Path(os.environ.get("A4N_WDIO_CHROMEDRIVER_BINARY", "/usr/bin/chromedriver")), "ChromeDriver")
        if not (ROOT / "frontend/node_modules/vite/bin/vite.js").is_file():
            raise RunnerError("前端依赖未安装，请先在 frontend 运行 npm ci")
        source = snapshot_source(report_dir, args.application_source)
        report["source"] = {"path": str(source), "sha256": source_fingerprint(source), "frontend_sha256": frontend_fingerprint(source)}
        binaries = build_binaries(report_dir, args.binaries_dir, source)
        report["binaries"] = str(binaries)
        frontend = source / "frontend"
        env = {k: v for k, v in os.environ.items() if not k.startswith("A4N_") or k.startswith("A4N_WDIO_") or k == "A4N_ONNXRUNTIME_ROOT"}
        for key in ("HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy"):
            env.pop(key, None)
        env["NO_PROXY"] = "localhost,127.0.0.1"
        env.update(NODE_ENV="production", VITE_A4N_API_TRANSPORT="web", VITE_A4N_API_BASE_URL="/api", VITE_A4N_BASE_PATH="/")
        with (report_dir / "frontend-build.log").open("w") as log:
            subprocess.run(["npm", "run", "build"], cwd=frontend, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
        password = secrets.token_urlsafe(32)
        pg_env = {**env, "POSTGRES_PASSWORD": password}
        data = secure_mkdir(report_dir / "data")
        pgdata = secure_mkdir(data / "postgres")
        with (report_dir / "postgres.log").open("w") as log:
            subprocess.run(["docker", "run", "-d", "--name", container, "-e", "POSTGRES_PASSWORD", "-e", "POSTGRES_USER=a4n_e2e", "-e", "POSTGRES_DB=a4n_e2e",
                            "-p", "127.0.0.1::5432", "--mount", f"type=bind,src={pgdata},dst=/var/lib/postgresql/data", args.postgres_image,
                            "-c", "max_connections=300"],
                           env=pg_env, stdout=log, stderr=subprocess.STDOUT, check=True)
            postgres_started = True
            for _ in range(100):
                ready = subprocess.run(["docker", "exec", container, "pg_isready", "-h", "127.0.0.1", "-U", "a4n_e2e", "-d", "a4n_e2e"], stdout=log, stderr=subprocess.STDOUT)
                if ready.returncode == 0:
                    break
                time.sleep(.2)
            else:
                raise RunnerError("隔离 PostgreSQL 未就绪")
        pg_port = subprocess.check_output(["docker", "inspect", "--format", '{{(index (index .NetworkSettings.Ports "5432/tcp") 0).HostPort}}', container], text=True).strip()
        ports = set()
        def port():
            value = free_port()
            while value in ports:
                value = free_port()
            ports.add(value)
            return value
        api_port, provider_port, qdrant_http, qdrant_grpc = [port() for _ in range(4)]
        server_url, provider_url = f"http://127.0.0.1:{api_port}", f"http://127.0.0.1:{provider_port}/v1"
        request_log = report_dir / "provider.jsonl"
        username = "creation-e2e"
        env.update({
            "A4N_DATABASE_URL": f"postgres://a4n_e2e:{quote(password, safe='')}@127.0.0.1:{pg_port}/a4n_e2e",
            "A4N_JWT_SECRET": secrets.token_urlsafe(32), "A4N_RUNTIME_PROFILE": "full",
            "A4N_MANAGED_SIDECARS": "0", "A4N_WEB_ADDR": f"127.0.0.1:{api_port}", "A4N_CONTROL_ADDR": f"127.0.0.1:{port()}",
            "A4N_DATA_DIR": str(data), "A4N_CACHE_DIR": str(data / "cache"), "A4N_TEMP_DIR": str(data / "tmp"), "TMPDIR": str(data / "tmp"),
            "A4N_LOG_DIR": str(data / "logs"), "A4N_QDRANT_URL": f"http://127.0.0.1:{qdrant_grpc}", "A4N_QDRANT_COLLECTION": "creation_e2e",
            "A4N_BOOTSTRAP_ADMIN_USERNAME": username, "A4N_BOOTSTRAP_ADMIN_EMAIL": "creation-e2e@example.invalid", "A4N_BOOTSTRAP_ADMIN_PASSWORD": password,
            "A4N_PASSWORD": password, "A4N_PROMPT_SERVICE_PROMPTS_DIR": str(source / "prompts"), "A4N_FRONTEND_DIST": str(frontend / "dist"),
            "A4N_E2E_EMBEDDING_DIMENSION": str(EMBEDDING_DIMENSION),
            "A4N_CONTROL_BINARY": str(binaries / "a4n-server"), "A4N_EDGE_BINARY": str(binaries / "a4n-server"),
            "A4N_PUBLIC_LLM_NAME": "Creation E2E LLM", "A4N_PUBLIC_LLM_PROVIDER": "openai", "A4N_PUBLIC_LLM_BASE_URL": provider_url,
            "A4N_PUBLIC_LLM_API_KEY": "a4n-e2e-provider-key", "A4N_PUBLIC_LLM_MODEL": "a4n-creation-journey", "A4N_PUBLIC_LLM_MAX_CONTEXT_TOKENS": "128000", "A4N_PUBLIC_LLM_MAX_OUTPUT_TOKENS": "8192",
            "A4N_PUBLIC_IMAGE_NAME": "Creation E2E Image", "A4N_PUBLIC_IMAGE_BACKEND": "openai-compatible-image-edit", "A4N_PUBLIC_IMAGE_BASE_URL": provider_url,
            "A4N_PUBLIC_IMAGE_API_KEY": "a4n-e2e-provider-key", "A4N_PUBLIC_IMAGE_MODEL": "a4n-virtual-image",
            "A4N_PUBLIC_VIDEO_NAME": "Creation E2E Video", "A4N_PUBLIC_VIDEO_BACKEND": "openai-compatible-video", "A4N_PUBLIC_VIDEO_BASE_URL": provider_url,
            "A4N_PUBLIC_VIDEO_API_KEY": "a4n-e2e-provider-key", "A4N_PUBLIC_VIDEO_MODEL": "a4n-virtual-video", "A4N_PUBLIC_VIDEO_DEFAULT_DURATION_SECS": "6",
            "RUST_LOG": "warn",
        })
        for name in ("cache", "tmp", "logs", "downloads"):
            secure_mkdir(data / name)
        for worker in json.loads((source / "crates/process/a4n-process-catalog/worker-catalog.json").read_text()):
            env[worker["addr_env"]] = f"127.0.0.1:{port()}"
            env[worker["addr_env"].removesuffix("_ADDR") + "_BINARY"] = str(binaries / ("a4n-novel-planning-worker" if worker["binary"] == "a4n-novel-planning-worker" else "a4n-server"))
        configure_onnxruntime(env)
        secure_json(report_dir / "qdrant.yaml", {"storage": {"storage_path": str(data / "qdrant"), "snapshots_path": str(data / "qdrant-snapshots")}, "service": {"host": "127.0.0.1", "http_port": qdrant_http, "grpc_port": qdrant_grpc}, "telemetry_disabled": True})
        video = report_dir / "fixture.mp4"
        subprocess.run(["ffmpeg", "-v", "error", "-f", "lavfi", "-i", "color=c=navy:s=320x180:r=24", "-t", "6", "-c:v", "libx264", "-pix_fmt", "yuv420p", str(video)], check=True)
        def start(name, command):
            process, handle = start_process(command, cwd=source, env=env, log_path=report_dir / (name + ".log"))
            processes.append((process, handle))
            return process
        qdrant = start("qdrant", [ensure_executable(ROOT / "tools/qdrant/bin/qdrant", "Qdrant"), "--config-path", str(report_dir / "qdrant.yaml")])
        wait_http(f"http://127.0.0.1:{qdrant_http}/healthz", process=qdrant, timeout=60)
        mock = start("provider", [sys.executable, str(source / "scripts/tests/mock_openai_provider.py"), "--port", str(provider_port), "--log", str(request_log), "--embedding-dimension", str(EMBEDDING_DIMENSION), "--video-file", str(video)])
        wait_http(provider_url.removesuffix("/v1") + "/health", process=mock, timeout=60)
        with (report_dir / "migration.log").open("w") as log:
            subprocess.run([str(binaries / "a4n-migration")], cwd=source, env=env, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=60)
        server = start("server", [str(binaries / "a4n-server")])
        wait_http(server_url + "/api/health", process=server, timeout=90)
        cli = Cli(str(binaries / "a4n"), server_url, data / "bootstrap.json", data / "bootstrap-context.json", 60, env)
        checked(cli, ["health"], "health")
        checked(cli, ["capabilities", "check"], "CLI/Server contract")
        checked(cli, ["auth", "login", "--username", username, "--password-stdin"], "login", password + "\n")
        for package in ("ai-workflow", "vector-rag", "video", "manga", "agent", "canvas", "export", "external-import", "galgame"):
            checked(cli, ["call", "start_package_lifecycle", "--args", "-", "--allow-destructive"], "enable " + package,
                    json.dumps({"input": {"package_id": package, "action": "enable", "operation_id": str(uuid.uuid4())}}))
        def execute_suite(name: str) -> None:
            script = SUITES[name][0] if name in SUITES else FULL_GENERATION_SUITES[name]
            output = report_dir / (name + ".json")
            command = [sys.executable, str(source / "scripts/tests" / script)]
            if name == "all_workflows":
                command += [
                    "--a4n", str(binaries / "a4n"), "--server", server_url,
                    "--username", username, "--password-env", "A4N_PASSWORD",
                    "--report", str(output), "--execute-writes", "--allow-destructive", "--fixture-only",
                    "--embedding-dimension", str(EMBEDDING_DIMENSION),
                    "--timeout", "120",
                ]
            elif name == "image_checkpoint_recovery":
                command += [
                    "--a4n", str(binaries / "a4n"), "--server", server_url,
                    "--username", username, "--data-dir", str(data), "--report", str(output),
                ]
            else:
                command += [
                    "--a4n", str(binaries / "a4n"), "--server", server_url,
                    "--username", username, "--provider-url", provider_url,
                    "--provider-log", str(request_log), "--report", str(output),
                ]
            if name in {"checkpoints", "facts", "scene_sync", "agent_delegation", "media_authoring"}:
                command += ["--embedding-dimension", str(EMBEDDING_DIMENSION)]
            if name == "authoring":
                command += ["--through-stage", max(stages & SUITES[name][1], key=STAGES.index)]
            with (report_dir / (name + ".log")).open("w") as log:
                try:
                    exit_code = subprocess.run(command, cwd=source, env=env, stdout=log, stderr=subprocess.STDOUT, timeout=1800).returncode
                except subprocess.TimeoutExpired:
                    exit_code = 124
            report["suites"][name] = (
                read_all_workflows_report(output, exit_code)
                if name == "all_workflows"
                else read_suite_report(output, exit_code)
            )
            write_report(report_dir / "report.json", report)
            print(json.dumps({"suite": name, "success": report["suites"][name]["success"]}), flush=True)

        for name in suites:
            if name not in POST_BROWSER_SUITES:
                execute_suite(name)
        browser_tmp = Path(tempfile.mkdtemp(prefix="ce-", dir=ROOT.parent))
        browser_env = {**env, "TMPDIR": str(browser_tmp),
                       "A4N_BROWSER_E2E_USERNAME": username, "A4N_BROWSER_E2E_PASSWORD": password,
                       "A4N_BROWSER_E2E_PROVIDER_URL": provider_url, "A4N_BROWSER_E2E_BASE_URL": server_url,
                       "A4N_BROWSER_E2E_ARTIFACT_DIR": str(report_dir / "browser"), "A4N_BROWSER_E2E_DOWNLOAD_DIR": str(data / "downloads"),
                       "A4N_CREATION_E2E_STAGES": ",".join(s for s in STAGES if s in stages)}
        wait_http(server_url, process=server, timeout=60)
        browser, browser_log = start_process(["npm", "run", "test:browser-full-project", "--", "--spec", "./e2e/full-project/generation-stages.e2e.mjs"],
                                            cwd=frontend, env=browser_env, log_path=report_dir / "browser.log")
        processes.append((browser, browser_log))
        browser_exit = browser.wait(timeout=1800)
        browser_path = report_dir / "browser" / "stages.json"
        browser_checks = json.loads(browser_path.read_text()) if browser_path.exists() else {}
        report["browser"] = {"exit_code": browser_exit, "report": str(browser_path)}
        if browser_exit != 0:
            report["failures"].append("浏览器验收失败：" + tail(report_dir / "browser.log", 2000))
        for name in suites:
            if name in POST_BROWSER_SUITES:
                execute_suite(name)
        report["stages"] = acceptance_result(stages, report["suites"], browser_checks)
        report["provider"] = provider_audit(request_log, required_paths={"/chat/completions"})
        if full_generation_run:
            report["generation_matrix"] = full_generation_matrix_report(report["suites"])
        report["success"] = acceptance_passed(report)
    except (Exception, KeyboardInterrupt) as error:
        report["failures"].append(f"{type(error).__name__}: {error}")
        for state in report["stages"].values():
            if state["status"] == "not_run":
                state.update(status="blocked", reason="上游或运行环境失败，未执行不能算通过")
    finally:
        def cleanup(label, action):
            try:
                action()
            except Exception as error:
                report["success"] = False
                report["failures"].append(f"清理 {label} 失败：{type(error).__name__}: {error}")

        for process, handle in reversed(processes):
            cleanup(f"进程 {process.pid}", lambda: stop_process(process, handle))
        if "binaries" in report:
            cleanup("Worker/Chromium", lambda: stop_owned_workers(Path(report["binaries"]), report_dir / "browser"))
        if postgres_started:
            cleanup("PostgreSQL", lambda: stop_postgres(container, report_dir / "postgres.log"))
        if browser_tmp is not None:
            cleanup("Chromium 临时目录", lambda: shutil.rmtree(browser_tmp))
        if "source" in report:
            cleanup("源码版本核对", lambda: report["source"].update(
                matches_current_worktree=source_fingerprint() == report["source"]["sha256"] and frontend_fingerprint() == report["source"]["frontend_sha256"]))
        write_report(report_dir / "report.json", report)
        print(json.dumps({"success": report["success"], "stages": report["stages"], "report": str(report_dir / "report.json")}, ensure_ascii=False), flush=True)
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
