#!/usr/bin/env python3
"""Run the final A4N browser acceptance flow in an isolated local stack.

The browser is the only actor that creates business data.  This runner only
starts the real Server, Vite and the deterministic loopback Provider, keeps
their logs, and checks that the expected provider endpoints were exercised.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import signal
import shutil
import socket
import subprocess
import sys
import time
import uuid
from datetime import datetime, timezone
from pathlib import Path
from urllib.error import URLError
from urllib.parse import unquote, urlsplit
from urllib.request import Request, urlopen


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_PROVIDER_SCRIPT = Path(__file__).with_name("mock_openai_provider.py")
REQUIRED_PROVIDER_PATHS = {"/chat/completions", "/embeddings", "/images/generations"}


class RunnerError(RuntimeError):
    """A user-actionable setup or acceptance failure."""


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--database-url",
        default=os.environ.get("A4N_BROWSER_E2E_DATABASE_URL"),
        help="隔离 PostgreSQL URL；也可由 A4N_BROWSER_E2E_DATABASE_URL 提供",
    )
    parser.add_argument(
        "--report-dir",
        type=Path,
        help="报告目录；默认 target/browser-full-project-e2e/<run-id>",
    )
    parser.add_argument(
        "--username",
        default=os.environ.get("A4N_BROWSER_E2E_USERNAME", "browser-e2e-admin"),
        help="登录用户名（密码绝不通过命令行传入）",
    )
    parser.add_argument(
        "--password-env",
        default="A4N_BROWSER_E2E_PASSWORD",
        help="从哪个环境变量读取登录/Bootstrap 密码",
    )
    parser.add_argument(
        "--server-binary",
        type=Path,
        default=ROOT / "target/debug/a4n-server",
        help="已构建的 a4n-server（二进制需启用 embedded-capabilities）",
    )
    parser.add_argument(
        "--migration-binary",
        type=Path,
        default=ROOT / "target/debug/a4n-migration",
        help="已构建的 a4n-migration",
    )
    parser.add_argument(
        "--provider-script",
        type=Path,
        default=DEFAULT_PROVIDER_SCRIPT,
        help="确定性 OpenAI 兼容 Provider 脚本",
    )
    parser.add_argument(
        "--frontend-dir",
        type=Path,
        default=ROOT / "frontend",
        help="前端工作目录（包含 node_modules）",
    )
    parser.add_argument(
        "--spec",
        type=Path,
        help="只运行指定的 WebdriverIO spec；相对路径以 frontend-dir 为基准",
    )
    parser.add_argument("--server-port", type=int, default=0)
    parser.add_argument("--provider-port", type=int, default=0)
    parser.add_argument("--vite-port", type=int, default=0)
    parser.add_argument("--startup-timeout", type=float, default=120.0)
    parser.add_argument("--migration-timeout", type=float, default=180.0)
    parser.add_argument("--browser-timeout", type=float, default=1_900.0)
    args = parser.parse_args()
    if not args.database_url:
        parser.error("必须提供 --database-url 或 A4N_BROWSER_E2E_DATABASE_URL")
    if not args.username.strip():
        parser.error("--username 不能为空")
    if any(value < 0 or value > 65535 for value in (args.server_port, args.provider_port, args.vite_port)):
        parser.error("端口必须在 0..65535")
    if min(args.startup_timeout, args.migration_timeout, args.browser_timeout) <= 0:
        parser.error("timeout 必须为正数")
    return args


def database_name(database_url: str) -> str:
    try:
        parsed = urlsplit(database_url)
    except ValueError as error:
        raise RunnerError(f"--database-url 格式无效：{error}") from error
    if parsed.scheme not in {"postgres", "postgresql"}:
        raise RunnerError("--database-url 必须使用 postgres:// 或 postgresql://")
    name = unquote(parsed.path.lstrip("/").split("/", 1)[0]).strip()
    if not name:
        raise RunnerError("--database-url 缺少数据库名")
    if name.casefold() == "a4n":
        raise RunnerError("浏览器 E2E 拒绝连接开发数据库 a4n；请创建隔离数据库")
    return name


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])


def ensure_executable(path: Path, label: str) -> str:
    resolved = path.expanduser().resolve()
    if not resolved.is_file() or not os.access(resolved, os.X_OK):
        raise RunnerError(f"找不到可执行的 {label}: {resolved}；请先构建对应二进制")
    return str(resolved)


def ensure_file(path: Path, label: str) -> Path:
    resolved = path.expanduser().resolve()
    if not resolved.is_file():
        raise RunnerError(f"找不到 {label}: {resolved}")
    return resolved


def secure_mkdir(path: Path) -> Path:
    path.mkdir(parents=True, exist_ok=True)
    path.chmod(0o700)
    return path


def open_log(path: Path):
    secure_mkdir(path.parent)
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    return os.fdopen(descriptor, "wb", buffering=0)


def tail(path: Path, limit: int = 8_000) -> str:
    return path.read_text(encoding="utf-8", errors="replace")[-limit:]


def start_process(
    command: list[str],
    *,
    cwd: Path,
    env: dict[str, str],
    log_path: Path,
) -> tuple[subprocess.Popen[bytes], object]:
    handle = open_log(log_path)
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
    except BaseException:
        handle.close()
        raise
    return process, handle


def stop_process(process: subprocess.Popen[bytes] | None, handle: object | None) -> None:
    try:
        if process is None or process.poll() is not None:
            return
        for force in (False, True):
            try:
                if os.name == "posix":
                    os.killpg(process.pid, signal.SIGKILL if force else signal.SIGTERM)
                elif force:
                    process.kill()
                else:
                    process.terminate()
            except ProcessLookupError:
                pass
            try:
                process.wait(timeout=10)
                return
            except subprocess.TimeoutExpired:
                if force:
                    raise RunnerError(f"进程 {process.pid} 在强制停止后仍未退出")
    finally:
        if handle is not None:
            handle.close()


def wait_http(
    url: str,
    *,
    process: subprocess.Popen[bytes] | None,
    timeout: float,
    expected_status: set[int] | None = None,
) -> None:
    expected = expected_status or {200}
    deadline = time.monotonic() + timeout
    last_error: object = "未收到响应"
    while time.monotonic() < deadline:
        if process is not None and process.poll() is not None:
            raise RunnerError(f"进程在健康检查前退出(exit={process.returncode})：{url}")
        try:
            with urlopen(Request(url, method="GET"), timeout=2) as response:
                if response.status in expected:
                    return
                last_error = f"HTTP {response.status}"
        except (OSError, URLError, ValueError) as error:
            last_error = error
        time.sleep(0.25)
    raise RunnerError(f"{url} 在 {timeout:g}s 内未就绪：{last_error}")


def wait_exit(process: subprocess.Popen[bytes], timeout: float, label: str, log_path: Path) -> None:
    try:
        code = process.wait(timeout=timeout)
    except subprocess.TimeoutExpired as error:
        raise RunnerError(f"{label} 在 {timeout:g}s 内未完成") from error
    if code != 0:
        raise RunnerError(f"{label} 失败(exit={code})\n{tail(log_path)}")


def provider_path(path: object) -> str:
    value = str(path).split("?", 1)[0].rstrip("/") or "/"
    if value.startswith("/v1/"):
        value = value[3:]
    return value


def provider_audit(path: Path, *, required_paths: set[str] | None = None) -> dict[str, object]:
    required = REQUIRED_PROVIDER_PATHS if required_paths is None else required_paths
    observed: list[str] = []
    if path.is_file():
        for line_number, line in enumerate(path.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
            try:
                record = json.loads(line)
            except json.JSONDecodeError as error:
                raise RunnerError(f"Provider 请求日志第 {line_number} 行不是 JSON") from error
            if isinstance(record, dict) and "path" in record:
                observed.append(provider_path(record["path"]))
    observed_set = set(observed)
    missing = sorted(required - observed_set)
    if missing:
        raise RunnerError(f"Provider 路径审计缺少：{', '.join(missing)}；实际路径：{', '.join(sorted(observed_set)) or '<none>'}")
    return {
        "request_count": len(observed),
        "observed_paths": sorted(observed_set),
        "required_paths": sorted(required),
        "missing_paths": missing,
    }


def redacted_url(url: str) -> str:
    parsed = urlsplit(url)
    host = parsed.hostname or ""
    try:
        port = f":{parsed.port}" if parsed.port else ""
    except ValueError:
        port = ""
    return f"{parsed.scheme}://{host}{port}{parsed.path}"


def configure_onnxruntime(env: dict[str, str]) -> None:
    """Make the embedded Server's dynamic ONNX dependency loadable when present."""
    names = {
        "linux": "libonnxruntime.so.1",
        "darwin": "libonnxruntime.dylib",
        "win32": "onnxruntime.dll",
    }
    library_name = names.get(sys.platform)
    if not library_name:
        return
    candidates: list[Path] = []
    for key in ("ORT_LIB_PATH",):
        value = env.get(key, "").strip()
        if value:
            candidates.append(Path(value).expanduser())
    root = env.get("A4N_ONNXRUNTIME_ROOT", "").strip()
    if root:
        candidates.append(Path(root).expanduser() / "lib")
    version_file = ROOT / "tools" / "onnxruntime" / "VERSION"
    if version_file.is_file():
        version = version_file.read_text(encoding="utf-8").strip()
        candidates.append(Path.home() / ".local" / "share" / "a4n" / "onnxruntime" / version / "lib")
    library_dir = next((path.resolve() for path in candidates if (path / library_name).is_file()), None)
    if library_dir is None:
        return
    env["ORT_LIB_PATH"] = str(library_dir)
    if sys.platform == "linux":
        current = env.get("LD_LIBRARY_PATH", "")
        env["LD_LIBRARY_PATH"] = f"{library_dir}{os.pathsep}{current}" if current else str(library_dir)
    elif sys.platform == "darwin":
        current = env.get("DYLD_LIBRARY_PATH", "")
        env["DYLD_LIBRARY_PATH"] = f"{library_dir}{os.pathsep}{current}" if current else str(library_dir)
    else:
        env["PATH"] = f"{library_dir}{os.pathsep}{env.get('PATH', '')}"


def write_report(path: Path, report: dict[str, object]) -> None:
    secure_mkdir(path.parent)
    path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    try:
        path.chmod(0o600)
    except OSError:
        pass


def run(args: argparse.Namespace) -> int:
    db_name = database_name(args.database_url)
    password = os.environ.get(args.password_env)
    if not password:
        raise RunnerError(f"登录密码环境变量 {args.password_env} 为空；密码不接受命令行参数")

    run_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + uuid.uuid4().hex[:8]
    report_dir = secure_mkdir(
        args.report_dir.expanduser().resolve()
        if args.report_dir
        else ROOT / "target" / "browser-full-project-e2e" / run_id
    )
    data_dir = secure_mkdir(report_dir / "data")
    for child in ("cache", "tmp", "logs", "downloads", "wdio"):
        secure_mkdir(data_dir / child)
    request_log = report_dir / "provider-requests.jsonl"
    request_log.touch(mode=0o600)

    server_binary = ensure_executable(args.server_binary, "a4n-server")
    migration_binary = ensure_executable(args.migration_binary, "a4n-migration")
    provider_script = ensure_file(args.provider_script, "mock provider")
    frontend_dir = args.frontend_dir.expanduser().resolve()
    if not (frontend_dir / "package.json").is_file():
        raise RunnerError(f"前端目录缺少 package.json: {frontend_dir}")
    spec_path = None
    if args.spec is not None:
        spec_path = args.spec.expanduser()
        if not spec_path.is_absolute():
            spec_path = frontend_dir / spec_path
        spec_path = spec_path.resolve()
        if not spec_path.is_file():
            raise RunnerError(f"WebdriverIO spec 不存在：{spec_path}")

    server_port = args.server_port or free_port()
    provider_port = args.provider_port or free_port()
    vite_port = args.vite_port or free_port()
    provider_url = f"http://127.0.0.1:{provider_port}/v1"
    server_url = f"http://127.0.0.1:{server_port}"
    frontend_url = f"http://127.0.0.1:{vite_port}"
    jwt_secret = f"browser-e2e-{uuid.uuid4().hex}"
    collection = re.sub(r"[^a-zA-Z0-9_-]", "_", f"a4n_browser_{run_id}")

    common_env = os.environ.copy()
    configure_onnxruntime(common_env)
    common_env.update(
        {
            "A4N_DATABASE_URL": args.database_url,
            "A4N_JWT_SECRET": jwt_secret,
            "A4N_WEB_ADDR": f"127.0.0.1:{server_port}",
            "A4N_RUNTIME_PROFILE": "full",
            "A4N_MANAGED_SIDECARS": "0",
            "A4N_DATA_DIR": str(data_dir),
            "A4N_CACHE_DIR": str(data_dir / "cache"),
            "A4N_TEMP_DIR": str(data_dir / "tmp"),
            "A4N_LOG_DIR": str(data_dir / "logs"),
            "A4N_QDRANT_URL": "http://127.0.0.1:6334",
            "A4N_QDRANT_COLLECTION": collection,
            "A4N_BOOTSTRAP_ADMIN_USERNAME": args.username,
            "A4N_BOOTSTRAP_ADMIN_EMAIL": "browser-e2e-admin@example.invalid",
            "A4N_BOOTSTRAP_ADMIN_PASSWORD": password,
            "A4N_PUBLIC_LLM_NAME": "A4N Chromium E2E Text",
            "A4N_PUBLIC_LLM_PROVIDER": "openai",
            "A4N_PUBLIC_LLM_BASE_URL": provider_url,
            "A4N_PUBLIC_LLM_API_KEY": "a4n-e2e-provider-key",
            "A4N_PUBLIC_LLM_MODEL": "a4n-virtual-text",
            "A4N_PUBLIC_LLM_MAX_CONCURRENT": "4",
            "A4N_PUBLIC_LLM_TIMEOUT_SECS": "60",
            "A4N_PUBLIC_LLM_MAX_CONTEXT_TOKENS": "128000",
            "A4N_PUBLIC_LLM_MAX_OUTPUT_TOKENS": "8192",
            "A4N_PUBLIC_IMAGE_NAME": "A4N Chromium E2E Image",
            "A4N_PUBLIC_IMAGE_BACKEND": "openai-compatible-image",
            "A4N_PUBLIC_IMAGE_BASE_URL": provider_url,
            "A4N_PUBLIC_IMAGE_API_KEY": "a4n-e2e-provider-key",
            "A4N_PUBLIC_IMAGE_MODEL": "a4n-virtual-image",
            "A4N_PUBLIC_IMAGE_MAX_CONCURRENT": "2",
            "A4N_PUBLIC_VIDEO_NAME": "A4N Chromium E2E Video",
            "A4N_PUBLIC_VIDEO_BACKEND": "openai-compatible-video",
            "A4N_PUBLIC_VIDEO_BASE_URL": provider_url,
            "A4N_PUBLIC_VIDEO_API_KEY": "a4n-e2e-provider-key",
            "A4N_PUBLIC_VIDEO_MODEL": "a4n-virtual-video",
            "A4N_PUBLIC_VIDEO_MAX_CONCURRENT": "2",
            "A4N_PUBLIC_VIDEO_DEFAULT_WIDTH": "1280",
            "A4N_PUBLIC_VIDEO_DEFAULT_HEIGHT": "720",
            "A4N_PUBLIC_VIDEO_DEFAULT_DURATION_SECS": "1",
            "A4N_PUBLIC_VIDEO_MIN_DURATION_SECS": "1",
            "A4N_PUBLIC_VIDEO_MAX_DURATION_SECS": "10",
            "A4N_PUBLIC_VIDEO_DURATION_MODE": "range",
            "A4N_PUBLIC_VIDEO_DEFAULT_FPS": "24",
        }
    )
    provider_env = common_env.copy()
    vite_env = common_env.copy()
    vite_env.update(
        {
            "A4N_WEB_ADDR": f"127.0.0.1:{server_port}",
            "VITE_A4N_API_TRANSPORT": "web",
            "VITE_A4N_API_BASE_URL": "/api",
            "VITE_PORT": str(vite_port),
            "A4N_DISABLE_VITE_HMR": "1",
        }
    )
    browser_env = vite_env.copy()
    browser_env.update(
        {
            "A4N_BROWSER_E2E_USERNAME": args.username,
            "A4N_BROWSER_E2E_PASSWORD": password,
            "A4N_BROWSER_E2E_PROVIDER_URL": provider_url,
            "A4N_BROWSER_E2E_BASE_URL": frontend_url,
            "A4N_BROWSER_E2E_ARTIFACT_DIR": str(report_dir / "wdio"),
            "A4N_BROWSER_E2E_DOWNLOAD_DIR": str(data_dir / "downloads"),
        }
    )
    for key in (
        "http_proxy",
        "https_proxy",
        "all_proxy",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "NODE_USE_ENV_PROXY",
    ):
        browser_env.pop(key, None)

    provider_process: subprocess.Popen[bytes] | None = None
    provider_handle: object | None = None
    server_process: subprocess.Popen[bytes] | None = None
    server_handle: object | None = None
    vite_process: subprocess.Popen[bytes] | None = None
    vite_handle: object | None = None
    migration_handle: object | None = None
    migration_process: subprocess.Popen[bytes] | None = None
    result: dict[str, object] = {
        "schema_version": 1,
        "run_id": run_id,
        "started_at": datetime.now(timezone.utc).isoformat(),
        "success": False,
        "database": {"name": db_name, "url": redacted_url(args.database_url)},
        "urls": {"frontend": frontend_url, "server": f"{server_url}/api", "provider": provider_url},
        "artifacts": {"report_dir": str(report_dir), "data_dir": str(data_dir)},
    }
    try:
        provider_process, provider_handle = start_process(
            [
                sys.executable,
                str(provider_script),
                "--host",
                "127.0.0.1",
                "--port",
                str(provider_port),
                "--log",
                str(request_log),
                "--embedding-dimension",
                "32",
            ],
            cwd=ROOT,
            env=provider_env,
            log_path=report_dir / "provider-process.log",
        )
        wait_http(f"http://127.0.0.1:{provider_port}/health", process=provider_process, timeout=args.startup_timeout)

        migration_process, migration_handle = start_process(
            [migration_binary], cwd=ROOT, env=common_env, log_path=report_dir / "migration.log"
        )
        wait_exit(migration_process, args.migration_timeout, "数据库迁移", report_dir / "migration.log")
        migration_handle.close()
        migration_handle = None

        server_process, server_handle = start_process(
            [server_binary], cwd=ROOT, env=common_env, log_path=report_dir / "server.log"
        )
        wait_http(f"{server_url}/api/health", process=server_process, timeout=args.startup_timeout)

        node = shutil.which("node") or shutil.which("nodejs")
        if not node:
            raise RunnerError("找不到 Node.js；无法启动 Vite")
        vite_process, vite_handle = start_process(
            [
                node,
                str(frontend_dir / "node_modules/vite/bin/vite.js"),
                "--host",
                "127.0.0.1",
                "--port",
                str(vite_port),
                "--strictPort",
            ],
            cwd=frontend_dir,
            env=vite_env,
            log_path=report_dir / "vite.log",
        )
        wait_http(frontend_url, process=vite_process, timeout=args.startup_timeout)

        wdio_log = report_dir / "wdio.log"
        wdio_handle = open_log(wdio_log)
        try:
            browser_command = ["npm", "run", "test:browser-full-project"]
            if spec_path is not None:
                browser_command.extend(["--", "--spec", str(spec_path)])
            wdio = subprocess.Popen(
                browser_command,
                cwd=str(frontend_dir),
                env=browser_env,
                stdin=subprocess.DEVNULL,
                stdout=wdio_handle,
                stderr=subprocess.STDOUT,
                start_new_session=True,
            )
            try:
                code = wdio.wait(timeout=args.browser_timeout)
            except subprocess.TimeoutExpired as error:
                stop_process(wdio, None)
                raise RunnerError(f"Chromium E2E 在 {args.browser_timeout:g}s 内未完成；详见 {wdio_log}") from error
            if code != 0:
                raise RunnerError(f"Chromium E2E 失败(exit={code})；详见 {wdio_log}\n{tail(wdio_log)}")
        finally:
            wdio_handle.close()

        result["provider"] = provider_audit(
            request_log,
            required_paths=None if spec_path is None else set(),
        )
        if spec_path is not None:
            result["provider"]["scope_note"] = "自定义 spec 未强制要求 Provider 路径；仅记录实际请求"
        result["success"] = True
        result["exit_code"] = 0
    except Exception as error:
        result["error"] = str(error)
        result["exit_code"] = 1
    finally:
        result["finished_at"] = datetime.now(timezone.utc).isoformat()
        for process, handle in (
            (vite_process, vite_handle),
            (server_process, server_handle),
            (migration_process, migration_handle),
            (provider_process, provider_handle),
        ):
            try:
                stop_process(process, handle)
            except (OSError, RunnerError) as error:
                result["success"] = False
                result["exit_code"] = 1
                result.setdefault("cleanup_errors", []).append(str(error))
        try:
            result.setdefault(
                "provider",
                provider_audit(
                    request_log,
                    required_paths=None if spec_path is None else set(),
                ),
            )
            if spec_path is not None and isinstance(result["provider"], dict):
                result["provider"]["scope_note"] = "自定义 spec 未强制要求 Provider 路径；仅记录实际请求"
        except RunnerError as error:
            result.setdefault("provider_error", str(error))
        result["artifacts"] = {
            **result["artifacts"],
            "provider_requests": str(request_log),
            "provider_process_log": str(report_dir / "provider-process.log"),
            "migration_log": str(report_dir / "migration.log"),
            "server_log": str(report_dir / "server.log"),
            "vite_log": str(report_dir / "vite.log"),
            "wdio_log": str(report_dir / "wdio.log"),
            "downloads": str(data_dir / "downloads"),
        }
        write_report(report_dir / "report.json", result)
        print(json.dumps(result, ensure_ascii=False, indent=2))
        print(f"浏览器 E2E 报告已保留：{report_dir}", file=sys.stderr)
    return 0 if result.get("success") is True else 1


def main() -> int:
    try:
        return run(parse_args())
    except RunnerError as error:
        print(f"浏览器 E2E 未启动：{error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
