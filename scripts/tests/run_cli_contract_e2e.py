#!/usr/bin/env python3
"""Run manifest-driven A4N CLI contract scenarios against the real Server stack."""

from __future__ import annotations

import argparse
import base64
import json
import os
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
import uuid
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from urllib.parse import urlsplit, urlunsplit


ROOT = Path(__file__).resolve().parents[2]
PLACEHOLDER = re.compile(r"\$\{([A-Za-z_][A-Za-z0-9_.-]*)\}")
TERMINAL_STATUSES = {"completed", "succeeded", "failed", "cancelled", "interrupted", "superseded"}


class RunnerError(RuntimeError):
    pass


def parse_args() -> argparse.Namespace:
    default_cli = os.environ.get("A4N_CLI", str(ROOT / "target/debug/a4n"))
    parser = argparse.ArgumentParser(
        description="Discover the current A4N command manifest and run a JSON scenario ledger.",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
        epilog=(
            'Ledger: {"schema_version":1,"variables":{},"scenarios":['
            '{"name":"list","command":"list_projects","args":{},'
            '"capture":{"project_id":"/data/items/0/id"}}],'
            '"skips":{"command_name":"explicit reason"}}. '
            "capture pointers read the agent-json envelope; watch also accepts an object with "
            "kind/id/attempt/interval_ms/timeout_seconds. files maps set_base64/set_text input "
            "paths relative to the ledger and save_base64/save_text paths under report artifacts."
        ),
    )
    parser.add_argument("--a4n", default=default_cli, help="current a4n CLI binary")
    parser.add_argument("--ledger", type=Path, help="JSON scenario/skip ledger")
    parser.add_argument("--inventory-only", action="store_true", help="only export/classify the manifest")
    parser.add_argument("--report-dir", type=Path, help="directory for coverage.json and coverage.md")
    parser.add_argument("--server", help="real A4N Server URL; omit to use the CLI default/config")
    parser.add_argument("--config", type=Path, help="CLI config path; default is an isolated temporary file")
    parser.add_argument("--context", type=Path, help="CLI context path; default is an isolated temporary file")
    parser.add_argument("--username", help="optional Server login username")
    parser.add_argument(
        "--password-env",
        default="A4N_PASSWORD",
        help="environment variable read for --username login (password is sent through stdin)",
    )
    parser.add_argument("--provider-url", help="existing mock provider base URL; /v1 is appended when absent")
    parser.add_argument(
        "--provider-script",
        type=Path,
        default=Path(__file__).with_name("mock_openai_provider.py"),
    )
    parser.add_argument("--provider-host", default="127.0.0.1")
    parser.add_argument("--provider-port", type=int, default=0, help="0 selects a free loopback port")
    parser.add_argument("--provider-start-timeout", type=float, default=10.0)
    parser.add_argument("--command-timeout", type=float, default=300.0)
    parser.add_argument(
        "--allow-destructive",
        action="store_true",
        help="authorize ledger scenarios whose manifest effect is destructive",
    )
    parser.add_argument("--fail-fast", action="store_true")
    args = parser.parse_args()
    if not args.inventory_only and not args.ledger:
        parser.error("非 inventory 模式必须提供 --ledger")
    if not 0 <= args.provider_port <= 65535:
        parser.error("--provider-port 必须在 0..65535")
    if args.provider_start_timeout <= 0 or args.command_timeout <= 0:
        parser.error("timeout 必须为正数")
    return args


def resolve_executable(value: str) -> str:
    path = Path(value).expanduser()
    if path.is_file():
        return str(path.resolve())
    resolved = shutil.which(value)
    if resolved:
        return resolved
    raise RunnerError(f"找不到 a4n CLI: {value}；请先构建或使用 --a4n 指定当前二进制")


def json_documents(text: str) -> list[Any]:
    decoder = json.JSONDecoder()
    documents: list[Any] = []
    offset = 0
    while offset < len(text):
        while offset < len(text) and text[offset].isspace():
            offset += 1
        if offset >= len(text):
            break
        try:
            value, offset = decoder.raw_decode(text, offset)
        except json.JSONDecodeError:
            return []
        documents.append(value)
    return documents


def command_error(result: dict[str, Any]) -> dict[str, Any]:
    documents = result["stderr_documents"] or result["stdout_documents"]
    if documents and isinstance(documents[-1], dict):
        error = documents[-1].get("error")
        if isinstance(error, dict):
            return {
                key: error.get(key)
                for key in ("code", "kind", "message", "retryable", "http_status")
                if error.get(key) is not None
            }
    detail = (result["stderr"] or result["stdout"]).strip()
    return {"code": "invalid_cli_output", "message": detail[-800:] or "CLI 未返回错误包络"}


class Cli:
    def __init__(
        self,
        executable: str,
        server: str | None,
        config: Path,
        context: Path,
        timeout: float,
        env: dict[str, str],
    ) -> None:
        self.executable = executable
        self.server = server
        self.config = config
        self.context = context
        self.timeout = timeout
        self.env = env

    def run(self, parts: list[str], stdin: str | None = None) -> dict[str, Any]:
        command = [self.executable, "--output", "agent-json", "--no-input"]
        if self.server:
            command += ["--server", self.server]
        command += ["--config", str(self.config), "--context", str(self.context), *parts]
        started = time.monotonic()
        try:
            process = subprocess.run(
                command,
                input=stdin,
                text=True,
                capture_output=True,
                timeout=self.timeout,
                cwd=ROOT,
                env=self.env,
                check=False,
            )
            return_code = process.returncode
            stdout = process.stdout
            stderr = process.stderr
        except subprocess.TimeoutExpired as error:
            return_code = 124
            stdout = error.stdout or ""
            stderr = error.stderr or f"CLI command timed out after {self.timeout:g}s"
            if isinstance(stdout, bytes):
                stdout = stdout.decode("utf-8", "replace")
            if isinstance(stderr, bytes):
                stderr = stderr.decode("utf-8", "replace")
        return {
            "exit_code": return_code,
            "duration_ms": round((time.monotonic() - started) * 1000),
            "stdout": stdout,
            "stderr": stderr,
            "stdout_documents": json_documents(stdout),
            "stderr_documents": json_documents(stderr),
        }


def checked(cli: Cli, parts: list[str], label: str, stdin: str | None = None) -> tuple[Any, dict[str, Any]]:
    result = cli.run(parts, stdin)
    documents = result["stdout_documents"]
    envelope = documents[-1] if documents else None
    if (
        result["exit_code"] != 0
        or not isinstance(envelope, dict)
        or envelope.get("ok") is not True
    ):
        error = command_error(result)
        raise RunnerError(f"{label} 失败(exit={result['exit_code']}): {error.get('message', error)}")
    return envelope, result


def read_ledger(path: Path | None) -> dict[str, Any]:
    if path is None:
        return {"variables": {}, "scenarios": [], "skips": {}}
    try:
        ledger = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise RunnerError(f"读取场景账本失败 {path}: {error}") from error
    if not isinstance(ledger, dict):
        raise RunnerError("场景账本根节点必须是 JSON object")
    unknown = set(ledger) - {"schema_version", "variables", "scenarios", "skips"}
    if unknown:
        raise RunnerError(f"场景账本包含未知字段: {', '.join(sorted(unknown))}")
    if ledger.get("schema_version", 1) != 1:
        raise RunnerError("场景账本 schema_version 只支持 1")
    variables = ledger.get("variables", {})
    scenarios = ledger.get("scenarios", [])
    skips = ledger.get("skips", {})
    if not isinstance(variables, dict) or not isinstance(scenarios, list) or not isinstance(skips, dict):
        raise RunnerError("variables/skips 必须是 object，scenarios 必须是 array")
    for key in variables:
        if not PLACEHOLDER.fullmatch(f"${{{key}}}"):
            raise RunnerError(f"无效变量名: {key}")
    names: set[str] = set()
    for index, scenario in enumerate(scenarios):
        if not isinstance(scenario, dict):
            raise RunnerError(f"scenarios[{index}] 必须是 object")
        extra = set(scenario) - {
            "name",
            "description",
            "command",
            "args",
            "capture",
            "watch",
            "files",
        }
        if extra:
            raise RunnerError(f"scenarios[{index}] 包含未知字段: {', '.join(sorted(extra))}")
        command = scenario.get("command")
        name = scenario.get("name", command)
        if not isinstance(command, str) or not command.strip():
            raise RunnerError(f"scenarios[{index}].command 必须是非空字符串")
        if not isinstance(name, str) or not name.strip() or name in names:
            raise RunnerError(f"scenarios[{index}].name 必须非空且唯一: {name!r}")
        names.add(name)
        if not isinstance(scenario.get("args", {}), dict):
            raise RunnerError(f"scenarios[{index}].args 必须是 object")
        capture = scenario.get("capture", {})
        if not isinstance(capture, dict):
            raise RunnerError(f"scenarios[{index}].capture 必须是 object")
        for variable, pointer in capture.items():
            if not PLACEHOLDER.fullmatch(f"${{{variable}}}") or not isinstance(pointer, str):
                raise RunnerError(f"scenarios[{index}].capture 必须是 variable -> JSON Pointer")
            validate_pointer(pointer)
        validate_files(scenario.get("files"), f"scenarios[{index}].files")
        validate_watch(scenario.get("watch"), f"scenarios[{index}].watch")
    for command, reason in skips.items():
        if not isinstance(command, str) or not command.strip():
            raise RunnerError("skips 的 command 必须是非空字符串")
        if not isinstance(reason, str) or not reason.strip():
            raise RunnerError(f"skip `{command}` 必须提供非空原因")
    return {"variables": variables, "scenarios": scenarios, "skips": skips}


def validate_files(files: Any, field: str) -> None:
    if files is None:
        return
    if not isinstance(files, dict):
        raise RunnerError(f"{field} 必须是 object")
    unknown = set(files) - {"set_base64", "set_text", "save_base64", "save_text"}
    if unknown:
        raise RunnerError(f"{field} 包含未知字段: {', '.join(sorted(unknown))}")
    for kind in ("set_base64", "set_text", "save_base64", "save_text"):
        assignments = files.get(kind, {})
        if not isinstance(assignments, dict):
            raise RunnerError(f"{field}.{kind} 必须是 JSON Pointer -> 相对路径 object")
        for pointer, path in assignments.items():
            if not isinstance(pointer, str) or not isinstance(path, str) or not path.strip():
                raise RunnerError(f"{field}.{kind} 必须是 JSON Pointer -> 非空相对路径")
            validate_pointer(pointer)


def validate_watch(watch: Any, field: str) -> None:
    if watch is None:
        return
    if isinstance(watch, str):
        if watch not in {"task", "workflow"}:
            raise RunnerError(f"{field} 只支持 task/workflow")
        return
    if not isinstance(watch, dict):
        raise RunnerError(f"{field} 必须是 task/workflow 或 object")
    unknown = set(watch) - {"kind", "id", "attempt", "interval_ms", "timeout_seconds"}
    if unknown or watch.get("kind") not in {"task", "workflow"}:
        raise RunnerError(f"{field} 无效，未知字段: {', '.join(sorted(unknown)) or '<none>'}")


def validate_pointer(pointer: str) -> None:
    if pointer and not pointer.startswith("/"):
        raise RunnerError(f"JSON Pointer 必须为空或以 / 开头: {pointer}")


def json_pointer(value: Any, pointer: str) -> Any:
    validate_pointer(pointer)
    current = value
    if not pointer:
        return current
    for raw in pointer[1:].split("/"):
        token = raw.replace("~1", "/").replace("~0", "~")
        if isinstance(current, dict) and token in current:
            current = current[token]
        elif isinstance(current, list) and token.isdigit() and int(token) < len(current):
            current = current[int(token)]
        else:
            raise RunnerError(f"JSON Pointer 不存在: {pointer}")
    return current


def substitute(value: Any, variables: dict[str, Any]) -> Any:
    if isinstance(value, list):
        return [substitute(item, variables) for item in value]
    if isinstance(value, dict):
        return {key: substitute(item, variables) for key, item in value.items()}
    if not isinstance(value, str):
        return value
    exact = PLACEHOLDER.fullmatch(value)
    if exact:
        name = exact.group(1)
        if name not in variables:
            raise RunnerError(f"缺少场景变量: {name}")
        return variables[name]

    def replace(match: re.Match[str]) -> str:
        name = match.group(1)
        if name not in variables:
            raise RunnerError(f"缺少场景变量: {name}")
        replacement = variables[name]
        if isinstance(replacement, (dict, list)):
            raise RunnerError(f"复合变量 {name} 只能作为完整字符串占位符")
        if replacement is None:
            return "null"
        if isinstance(replacement, bool):
            return str(replacement).lower()
        return str(replacement)

    return PLACEHOLDER.sub(replace, value)


def scenario_file_args(
    files: Any,
    variables: dict[str, Any],
    input_root: Path,
    artifact_root: Path,
    scenario_index: int,
) -> tuple[list[str], list[dict[str, Any]], list[dict[str, Any]]]:
    if files is None:
        return [], [], []
    output_root = artifact_root / f"scenario-{scenario_index + 1:04d}"
    parts: list[str] = []
    inputs: list[dict[str, Any]] = []
    outputs: list[dict[str, Any]] = []
    for kind, option in (
        ("set_base64", "--set-base64"),
        ("set_text", "--set-text"),
        ("save_base64", "--save-base64"),
        ("save_text", "--save-text"),
    ):
        for pointer, raw_path in files.get(kind, {}).items():
            relative = Path(substitute(raw_path, variables))
            if relative.is_absolute() or not relative.parts or any(
                part in {"", ".", ".."} for part in relative.parts
            ):
                raise RunnerError(f"files.{kind} 只允许规范相对路径: {raw_path}")
            if kind.startswith("set_"):
                path = input_root.joinpath(*relative.parts).resolve()
                if not path.is_relative_to(input_root) or not path.is_file():
                    raise RunnerError(f"files.{kind} 输入文件不存在或越界: {relative}")
                inputs.append(
                    {
                        "kind": kind,
                        "pointer": pointer,
                        "path": str(path.relative_to(input_root)),
                        "bytes": path.stat().st_size,
                    }
                )
            else:
                path = output_root.joinpath(*relative.parts).resolve()
                if not path.is_relative_to(artifact_root):
                    raise RunnerError(f"files.{kind} 输出文件越界: {relative}")
                path.parent.mkdir(parents=True, exist_ok=True)
                outputs.append({"kind": kind, "pointer": pointer, "path": path})
            parts += [option, f"{pointer}={path}"]
    return parts, inputs, outputs


def output_file_records(outputs: list[dict[str, Any]], artifact_root: Path) -> list[dict[str, Any]]:
    records = []
    for output in outputs:
        path = output["path"]
        if not path.is_file():
            raise RunnerError(f"CLI 未生成声明的输出文件: {path.relative_to(artifact_root)}")
        data = path.read_bytes()
        records.append(
            {
                "kind": output["kind"],
                "pointer": output["pointer"],
                "path": str(path.relative_to(artifact_root)),
                "bytes": len(data),
            }
        )
    return records


def classify(manifest: dict[str, Any], ledger: dict[str, Any]) -> dict[str, Any]:
    descriptors = {item["name"]: item for item in manifest["commands"]}
    scenario_names: dict[str, list[str]] = {}
    stale: list[dict[str, str]] = []
    for scenario in ledger["scenarios"]:
        command = scenario["command"]
        if command not in descriptors:
            stale.append({"kind": "scenario", "name": scenario.get("name", command), "command": command})
        else:
            scenario_names.setdefault(command, []).append(scenario.get("name", command))
    for command in ledger["skips"]:
        if command not in descriptors:
            stale.append({"kind": "skip", "name": command, "command": command})
    conflicts = sorted(set(scenario_names) & set(ledger["skips"]))
    commands = []
    for name, descriptor in descriptors.items():
        if name in conflicts:
            classification = "conflict"
        elif name in scenario_names:
            classification = "scenario"
        elif name in ledger["skips"]:
            classification = "skipped"
        else:
            classification = "uncovered"
        commands.append(
            {
                "name": name,
                "domain": descriptor.get("domain"),
                "effect": descriptor.get("effect"),
                "execution": descriptor.get("execution"),
                "classification": classification,
                "scenarios": scenario_names.get(name, []),
                "skip_reason": ledger["skips"].get(name),
            }
        )
    uncovered = [item["name"] for item in commands if item["classification"] == "uncovered"]
    return {
        "descriptors": descriptors,
        "commands": commands,
        "uncovered": uncovered,
        "stale": stale,
        "conflicts": conflicts,
        "counts": {
            "total": len(commands),
            "scenario": sum(item["classification"] == "scenario" for item in commands),
            "skipped": sum(item["classification"] == "skipped" for item in commands),
            "uncovered": len(uncovered),
            "conflict": len(conflicts),
            "stale": len(stale),
        },
    }


def free_port(host: str) -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        # ponytail: tiny bind/launch race; add an inherited listening fd only if it flakes in CI.
        listener.bind((host, 0))
        return int(listener.getsockname()[1])


def normalize_provider_url(value: str) -> str:
    parsed = urlsplit(value)
    if parsed.scheme not in {"http", "https"} or not parsed.hostname:
        raise RunnerError(f"无效 provider URL: {value}")
    if parsed.username or parsed.password or parsed.query or parsed.fragment:
        raise RunnerError("provider URL 不能包含凭证、query 或 fragment")
    path = parsed.path.rstrip("/")
    if not path:
        path = "/v1"
    elif not path.endswith("/v1"):
        raise RunnerError("provider URL path 必须为空或以 /v1 结尾")
    return urlunsplit((parsed.scheme, parsed.netloc, path, "", ""))


def provider_health_url(base_url: str) -> str:
    parsed = urlsplit(base_url)
    root = parsed.path.removesuffix("/v1").rstrip("/")
    return urlunsplit((parsed.scheme, parsed.netloc, f"{root}/health", "", ""))


class Provider:
    def __init__(self, base_url: str, process: subprocess.Popen[bytes] | None, log_handle: Any) -> None:
        self.base_url = base_url
        self.process = process
        self.log_handle = log_handle

    def stop(self) -> None:
        if self.process and self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=5)
        if self.log_handle:
            self.log_handle.close()


def wait_for_provider(base_url: str, timeout: float, process: subprocess.Popen[bytes] | None) -> None:
    deadline = time.monotonic() + timeout
    url = provider_health_url(base_url)
    last_error: Exception | None = None
    while time.monotonic() < deadline:
        if process and process.poll() is not None:
            raise RunnerError(f"mock provider 提前退出: {process.returncode}")
        try:
            with urllib.request.urlopen(url, timeout=1) as response:
                payload = json.load(response)
            if response.status == 200 and payload.get("status") == "ok":
                return
        except (OSError, ValueError, urllib.error.URLError) as error:
            last_error = error
        time.sleep(0.1)
    raise RunnerError(f"mock provider 未在 {timeout:g}s 内就绪: {last_error}")


def check_provider_contract(base_url: str, timeout: float) -> dict[str, Any]:
    def request(path: str, body: dict[str, Any] | None = None) -> Any:
        url = f"{base_url}{path}"
        headers = {"Authorization": "Bearer a4n-e2e-provider-key"}
        data = None
        if body is not None:
            data = json.dumps(body).encode("utf-8")
            headers["Content-Type"] = "application/json"
        try:
            with urllib.request.urlopen(
                urllib.request.Request(url, data=data, headers=headers), timeout=timeout
            ) as response:
                return json.load(response)
        except (OSError, ValueError, urllib.error.URLError) as error:
            raise RunnerError(f"mock provider 合同检查失败 {url}: {error}") from error

    models = request("/models")
    chat = request(
        "/chat/completions",
        {
            "model": "a4n-virtual-text",
            "messages": [{"role": "user", "content": "ping"}],
        },
    )
    embeddings = request(
        "/embeddings", {"model": "a4n-virtual-embedding", "input": ["test"]}
    )
    image = request(
        "/images/generations",
        {"model": "a4n-virtual-image", "prompt": "test", "size": "1x1"},
    )
    video_job = request(
        "/videos/generations",
        {"model": "a4n-virtual-video", "prompt": "test", "duration": 1},
    )
    video_job_id = video_job.get("id") if isinstance(video_job, dict) else None
    if not isinstance(video_job_id, str) or not video_job_id:
        raise RunnerError("mock provider 视频创建响应缺少有效 id")
    deadline = time.monotonic() + timeout
    video: dict[str, Any] = video_job
    while video.get("status") != "completed" and time.monotonic() < deadline:
        status = video.get("status")
        if status not in {"pending", "running"}:
            raise RunnerError(f"mock provider 视频任务进入异常状态: {status!r}")
        time.sleep(0.1)
        polled = request(f"/videos/generations/{video_job_id}")
        if not isinstance(polled, dict):
            raise RunnerError("mock provider 视频轮询响应必须是 JSON object")
        video = polled
    if video.get("status") != "completed":
        raise RunnerError(f"mock provider 视频任务未在 {timeout:g}s 内完成")
    for name, payload in {
        "models": models,
        "chat completions": chat,
        "embeddings": embeddings,
        "image generation": image,
        "video generation": video,
    }.items():
        if not isinstance(payload, dict):
            raise RunnerError(f"mock provider {name} 响应必须是 JSON object")
    model_data = models.get("data")
    if not isinstance(model_data, list):
        raise RunnerError("mock provider /models 响应缺少 data array")
    model_ids = {item.get("id") for item in model_data if isinstance(item, dict)}
    expected_models = {
        "a4n-virtual-text",
        "a4n-virtual-embedding",
        "a4n-virtual-image",
        "a4n-virtual-video",
    }
    vectors = embeddings.get("data")
    image_data = image.get("data")
    video_data = video.get("data")
    if not expected_models <= model_ids:
        raise RunnerError("mock provider /models 缺少虚拟 text/embedding/image/video 模型")
    try:
        chat_content = chat["choices"][0]["message"]["content"]
    except (IndexError, KeyError, TypeError) as error:
        raise RunnerError(f"mock provider 文本响应不符合 chat completions 合同: {error}") from error
    if chat_content != "pong":
        raise RunnerError(f"mock provider 文本探测应返回 pong，实际为 {chat_content!r}")
    if (
        not isinstance(vectors, list)
        or len(vectors) != 1
        or not isinstance(vectors[0], dict)
        or not isinstance(vectors[0].get("embedding"), list)
        or len(vectors[0]["embedding"]) != 8
        or not all(
            isinstance(value, (int, float)) and not isinstance(value, bool)
            for value in vectors[0]["embedding"]
        )
    ):
        raise RunnerError("mock provider embedding 合同不是单个 8 维向量")
    if (
        not isinstance(image_data, list)
        or not image_data
        or not isinstance(image_data[0], dict)
        or not isinstance(video_data, list)
        or not video_data
        or not isinstance(video_data[0], dict)
    ):
        raise RunnerError("mock provider 媒体响应缺少 data object")
    try:
        image_bytes = base64.b64decode(image_data[0]["b64_json"], validate=True)
        video_bytes = base64.b64decode(video_data[0]["b64_json"], validate=True)
    except (IndexError, KeyError, TypeError, ValueError) as error:
        raise RunnerError(f"mock provider 媒体响应缺少有效 b64_json: {error}") from error
    if not image_bytes.startswith(b"\x89PNG\r\n\x1a\n"):
        raise RunnerError("mock provider 图片不是有效 PNG 签名")
    if len(video_bytes) < 12 or video_bytes[4:8] != b"ftyp" or video_data[0].get("format") != "mp4":
        raise RunnerError("mock provider 视频不是带 format=mp4 的 MP4")
    return {
        "models": len(model_ids),
        "text": "chat-completions",
        "embedding_dimension": 8,
        "image": "png",
        "video": "mp4",
    }


def start_provider(
    args: argparse.Namespace,
    report_dir: Path,
    run_id: str,
    request_log_path: Path | None = None,
) -> tuple[Provider, Path | None]:
    if args.provider_url:
        base_url = normalize_provider_url(args.provider_url)
        wait_for_provider(base_url, args.provider_start_timeout, None)
        return Provider(base_url, None, None), None
    if args.provider_host not in {"127.0.0.1", "localhost"}:
        raise RunnerError("内置 mock provider 只允许绑定 loopback host")
    script = args.provider_script.expanduser().resolve()
    if not script.is_file():
        raise RunnerError(f"找不到 mock provider: {script}")
    port = args.provider_port or free_port(args.provider_host)
    host_for_url = "127.0.0.1" if args.provider_host == "localhost" else args.provider_host
    base_url = f"http://{host_for_url}:{port}/v1"
    request_log = (request_log_path or report_dir / f"provider-requests-{run_id}.jsonl").expanduser().resolve()
    request_log.parent.mkdir(parents=True, exist_ok=True)
    process_log = report_dir / f"provider-process-{run_id}.log"
    request_log.touch(mode=0o600)
    log_handle = os.fdopen(
        os.open(process_log, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600),
        "ab",
        buffering=0,
    )
    try:
        process = subprocess.Popen(
            [
                sys.executable,
                str(script),
                "--host",
                args.provider_host,
                "--port",
                str(port),
                "--log",
                str(request_log),
                "--embedding-dimension",
                "8",
            ],
            cwd=ROOT,
            stdout=subprocess.DEVNULL,
            stderr=log_handle,
        )
    except BaseException:
        log_handle.close()
        raise
    provider = Provider(base_url, process, log_handle)
    try:
        wait_for_provider(base_url, args.provider_start_timeout, process)
    except BaseException:
        provider.stop()
        raise
    return provider, request_log


def runtime_snapshot(base_url: str) -> dict[str, Any]:
    key = "a4n-e2e-provider-key"
    llm = {
        "id": 900001,
        "name": "A4N virtual text",
        "provider": "openai",
        "base_url": base_url,
        "api_key_encrypted": key,
        "model": "a4n-virtual-text",
        "max_concurrent": 4,
        "timeout_secs": 60,
        "max_context_tokens": 128000,
        "max_output_tokens": 8192,
        "extra_body_json": None,
        "is_default": True,
    }
    image = {
        "id": 900002,
        "name": "A4N virtual image",
        "backend": "openai-compatible-image-edit",
        "base_url": base_url,
        "api_key_encrypted": key,
        "model": "a4n-virtual-image",
        "max_concurrent": 4,
        "supported_sizes": [{"width": 1024, "height": 1024}],
        "is_default": True,
    }
    video = {
        "id": 900003,
        "name": "A4N virtual video",
        "backend": "openai-compatible-video",
        "base_url": base_url,
        "api_key_encrypted": key,
        "model": "a4n-virtual-video",
        "max_concurrent": 2,
        "default_width": 1280,
        "default_height": 720,
        "default_duration_secs": 1,
        "min_duration_secs": 1,
        "max_duration_secs": 10,
        "duration_mode": "range",
        "supported_durations_json": None,
        "default_fps": 24,
        "is_default": True,
    }
    embedding = {
        "id": 900004,
        "name": "A4N virtual embedding",
        "backend": "api",
        "base_url": base_url,
        "api_key_encrypted": key,
        "model": "a4n-virtual-embedding",
        "dimension": 8,
        "batch_size": 32,
        "local_model_path": None,
        "is_default": True,
    }
    return {
        "schema_version": 1,
        "llm": llm,
        "llm_configs": [llm],
        "image": image,
        "image_configs": [image],
        "video": video,
        "video_configs": [video],
        "embedding": embedding,
        "llm_default_source": "local",
        "image_default_source": "local",
        "video_default_source": "local",
        "client_context": {
            "frontend_kind": "web",
            "backend_kind": "remote_http",
            "execution_location": "server",
        },
    }


def secure_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    flags = os.O_WRONLY | os.O_CREAT | os.O_TRUNC
    descriptor = os.open(path, flags, 0o600)
    with os.fdopen(descriptor, "w", encoding="utf-8") as output:
        json.dump(value, output, ensure_ascii=False, separators=(",", ":"))
        output.write("\n")


def preflight(
    cli: Cli,
    args: argparse.Namespace,
    ledger: dict[str, Any],
    descriptors: dict[str, Any],
) -> list[dict[str, Any]]:
    checks: list[dict[str, Any]] = []

    def run(parts: list[str], label: str, stdin: str | None = None) -> None:
        _, result = checked(cli, parts, label, stdin)
        checks.append({"name": label, "status": "passed", "duration_ms": result["duration_ms"]})

    run(["health"], "health")
    run(["capabilities", "check"], "capabilities check")
    requires_auth = any(
        descriptors.get(scenario["command"], {}).get("auth") != "public"
        for scenario in ledger["scenarios"]
        if scenario["command"] in descriptors
    )
    if args.username:
        password = os.environ.get(args.password_env)
        if not password:
            raise RunnerError(f"--username 已提供，但环境变量 {args.password_env} 为空")
        run(
            ["auth", "login", "--username", args.username, "--password-stdin"],
            "auth login",
            f"{password}\n",
        )
        requires_auth = True
    if requires_auth:
        run(["auth", "status"], "auth status")
    return checks


def terminal_status(envelope: dict[str, Any]) -> str | None:
    for pointer in ("/data/status", "/data/run/status", "/data/task/status"):
        try:
            status = json_pointer(envelope, pointer)
        except RunnerError:
            continue
        if isinstance(status, str) and status in TERMINAL_STATUSES:
            return status
    return None


def first_reference(envelope: dict[str, Any], kind: str) -> Any:
    key = "task_ids" if kind == "task" else "workflow_run_ids"
    values = envelope.get("meta", {}).get("references", {}).get(key, [])
    if not isinstance(values, list) or not values:
        raise RunnerError(f"响应没有可供 {kind} watch 使用的 meta.references.{key}")
    return values[0]


def run_watch(
    cli: Cli,
    watch_spec: Any,
    variables: dict[str, Any],
    envelope: dict[str, Any],
) -> dict[str, Any] | None:
    if watch_spec is None:
        return None
    status = terminal_status(envelope)
    if status:
        return {
            "status": "not_needed" if status in {"completed", "succeeded"} else "failed",
            "terminal_status": status,
        }
    spec = {"kind": watch_spec} if isinstance(watch_spec, str) else substitute(watch_spec, variables)
    kind = spec["kind"]
    identifier = spec["id"] if "id" in spec else first_reference(envelope, kind)
    if kind == "task":
        attempt = int(spec.get("attempt", 1))
        if attempt <= 0:
            raise RunnerError("task watch attempt 必须为正整数")
        parts = ["task", "watch", str(identifier), "--attempt", str(attempt)]
    else:
        workflow_id = int(identifier)
        if workflow_id <= 0:
            raise RunnerError("workflow watch id 必须为正整数")
        interval = int(spec.get("interval_ms", 500))
        if not 200 <= interval <= 60000:
            raise RunnerError("workflow watch interval_ms 必须在 200..60000")
        parts = ["workflow", "watch", str(workflow_id), "--interval-ms", str(interval)]
        if spec.get("timeout_seconds") is not None:
            timeout = int(spec["timeout_seconds"])
            if timeout <= 0:
                raise RunnerError("workflow watch timeout_seconds 必须为正整数")
            parts += ["--timeout-seconds", str(timeout)]
    result = cli.run(parts)
    if result["exit_code"] != 0:
        return {
            "kind": kind,
            "id": identifier,
            "status": "failed",
            "exit_code": result["exit_code"],
            "duration_ms": result["duration_ms"],
            "error": command_error(result),
        }
    if not result["stdout_documents"] or any(
        not isinstance(document, dict) or document.get("ok") is not True
        for document in result["stdout_documents"]
    ):
        return {
            "kind": kind,
            "id": identifier,
            "status": "failed",
            "exit_code": result["exit_code"],
            "duration_ms": result["duration_ms"],
            "error": {"code": "invalid_cli_output", "message": "watch 未返回有效 agent-json 包络"},
        }
    return {
        "kind": kind,
        "id": identifier,
        "status": "passed",
        "exit_code": 0,
        "duration_ms": result["duration_ms"],
        "events": len(result["stdout_documents"]),
    }


def execute_scenarios(
    cli: Cli,
    args: argparse.Namespace,
    ledger: dict[str, Any],
    coverage: dict[str, Any],
    work_dir: Path,
    input_root: Path,
    artifact_root: Path,
    builtins: dict[str, Any],
) -> list[dict[str, Any]]:
    variables = {**ledger["variables"], **builtins}
    results: list[dict[str, Any]] = []
    conflicts = set(coverage["conflicts"])
    descriptors = coverage["descriptors"]
    stop = False
    for index, scenario in enumerate(ledger["scenarios"]):
        name = scenario.get("name", scenario["command"])
        command = scenario["command"]
        record: dict[str, Any] = {"name": name, "command": command}
        if stop:
            record.update(status="not_run", reason="fail_fast")
            results.append(record)
            continue
        descriptor = descriptors.get(command)
        if descriptor is None:
            record.update(status="invalid", reason="stale scenario command")
            results.append(record)
            stop = args.fail_fast
            continue
        if command in conflicts:
            record.update(status="invalid", reason="command 同时存在于 scenarios 与 skips")
            results.append(record)
            stop = args.fail_fast
            continue
        if descriptor.get("effect") == "destructive" and not args.allow_destructive:
            record.update(status="blocked", reason="需要 --allow-destructive")
            results.append(record)
            stop = args.fail_fast
            continue
        try:
            scenario_variables = {**variables, "scenario_uuid": str(uuid.uuid4())}
            call_args = substitute(scenario.get("args", {}), scenario_variables)
            args_path = work_dir / f"scenario-{index + 1:04d}.json"
            secure_json(args_path, call_args)
            parts = ["call", command, "--args", f"@{args_path}"]
            file_parts, file_inputs, file_outputs = scenario_file_args(
                scenario.get("files"),
                scenario_variables,
                input_root,
                artifact_root,
                index,
            )
            parts += file_parts
            if descriptor.get("effect") == "destructive":
                parts.append("--allow-destructive")
            result = cli.run(parts)
            record["exit_code"] = result["exit_code"]
            record["duration_ms"] = result["duration_ms"]
            documents = result["stdout_documents"]
            envelope = documents[-1] if documents else None
            if (
                result["exit_code"] != 0
                or not isinstance(envelope, dict)
                or envelope.get("ok") is not True
            ):
                record.update(status="failed", error=command_error(result))
            else:
                saved_files = file_inputs + output_file_records(file_outputs, artifact_root)
                captured = []
                for variable, pointer in scenario.get("capture", {}).items():
                    variables[variable] = json_pointer(envelope, pointer)
                    captured.append(variable)
                watch = run_watch(cli, scenario.get("watch"), variables, envelope)
                record.update(status="passed", captures=captured)
                if saved_files:
                    record["files"] = saved_files
                if watch:
                    record["watch"] = watch
                    if watch["status"] == "failed":
                        record["status"] = "failed"
        except (RunnerError, TypeError, ValueError) as error:
            record.update(status="failed", error={"code": "runner_error", "message": str(error)})
        results.append(record)
        if args.fail_fast and record["status"] != "passed":
            stop = True
    return results


def provider_log_entries(path: Path | None) -> list[dict[str, Any]]:
    if path is None or not path.is_file():
        return []
    entries = []
    for line in path.read_text(encoding="utf-8").splitlines():
        try:
            entry = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(entry, dict):
            entries.append(entry)
    return entries


def provider_request_summary(entries: list[dict[str, Any]]) -> dict[str, Any]:
    counts: dict[str, int] = {}
    for entry in entries:
        request_path = str(entry.get("path", "<missing>"))
        counts[request_path] = counts.get(request_path, 0) + 1
    return {"requests": len(entries), "paths": dict(sorted(counts.items()))}


def provider_log_summary(
    path: Path | None,
    scenario_start_index: int | None,
) -> dict[str, Any]:
    if path is None:
        unknown = {"requests": None, "paths": {}}
        return {"path": None, **unknown, "preflight": unknown, "scenarios": unknown}
    entries = provider_log_entries(path)
    boundary = len(entries) if scenario_start_index is None else scenario_start_index
    boundary = min(max(boundary, 0), len(entries))
    return {
        "path": str(path),
        **provider_request_summary(entries),
        "preflight": provider_request_summary(entries[:boundary]),
        "scenarios": provider_request_summary(entries[boundary:]),
    }


def completion_flags(coverage: dict[str, Any]) -> tuple[bool, bool]:
    classified_complete = (
        not coverage["uncovered"]
        and not coverage["stale"]
        and not coverage["conflicts"]
    )
    tested_complete = classified_complete and coverage["counts"]["skipped"] == 0
    return classified_complete, tested_complete


def scenario_execution_success(
    fatal_error: str | None,
    scenario_results: list[dict[str, Any]],
) -> bool:
    return (
        fatal_error is None
        and bool(scenario_results)
        and all(result.get("status") == "passed" for result in scenario_results)
    )


def atomic_write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as output:
            output.write(text)
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def markdown(report: dict[str, Any]) -> str:
    coverage = report["coverage"]
    counts = coverage["counts"]
    lines = [
        "# A4N CLI contract E2E coverage",
        "",
        f"- Generated: `{report['generated_at']}`",
        f"- Manifest version: `{report['manifest']['version']}`",
        f"- Commands: `{counts['total']}`",
        f"- Scenario-classified: `{counts['scenario']}`",
        f"- Explicitly skipped: `{counts['skipped']}`",
        f"- Uncovered: `{counts['uncovered']}`",
        f"- Stale ledger entries: `{counts['stale']}`",
        f"- Classification conflicts: `{counts['conflict']}`",
        f"- Fully classified: `{str(report['classified_complete']).lower()}`",
        f"- Fully scenario-covered: `{str(report['tested_complete']).lower()}`",
        f"- Provider requests: `{report['provider']['requests']}`",
        f"- Provider preflight requests: `{report['provider']['preflight']['requests']}`",
        f"- Provider scenario requests: `{report['provider']['scenarios']['requests']}`",
        "",
        "## Preflight",
        "",
        "| Check | Status | Time (ms) |",
        "| --- | --- | ---: |",
    ]
    for check in report["preflight"]:
        lines.append(
            f"| {check['name']} | {check['status']} | {check.get('duration_ms', '')} |"
        )
    if not report["preflight"]:
        lines.append("| _none_ |  |  |")
    lines += [
        "",
        "## Scenario execution",
        "",
        "| Scenario | Command | Status | Time (ms) | Detail |",
        "| --- | --- | --- | ---: | --- |",
    ]
    for result in report["scenario_results"]:
        detail = (
            result.get("reason")
            or result.get("error", {}).get("message")
            or result.get("watch", {}).get("error", {}).get("message")
            or ""
        )
        cells = [
            result.get("name"),
            result.get("command"),
            result.get("status"),
            result.get("duration_ms", ""),
            detail,
        ]
        lines.append("| " + " | ".join(str(cell).replace("|", "\\|").replace("\n", " ") for cell in cells) + " |")
    if not report["scenario_results"]:
        lines.append("| _none_ |  |  |  |  |")
    lines += ["", "## Explicit skips", ""]
    skipped = [item for item in coverage["commands"] if item["classification"] == "skipped"]
    if skipped:
        lines += ["| Command | Reason |", "| --- | --- |"]
        for item in skipped:
            reason = str(item["skip_reason"]).replace("|", "\\|").replace("\n", " ")
            lines.append(f"| `{item['name']}` | {reason} |")
    else:
        lines.append("_None._")
    lines += ["", "## Uncovered commands", ""]
    lines += [f"- `{command}`" for command in coverage["uncovered"]] or ["_None._"]
    lines += ["", "## Stale ledger entries", ""]
    lines += [
        f"- `{entry['kind']}` `{entry['name']}` → `{entry['command']}`"
        for entry in coverage["stale"]
    ] or ["_None._"]
    lines += ["", "## Classification conflicts", ""]
    lines += [f"- `{command}`" for command in coverage["conflicts"]] or ["_None._"]
    if report.get("fatal_error"):
        lines += ["", "## Fatal error", "", str(report["fatal_error"])]
    return "\n".join(lines) + "\n"


def main() -> int:
    args = parse_args()
    run_id = f"{datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')}-{os.getpid()}"
    report_dir = (args.report_dir or ROOT / "data/cli-contract-e2e" / run_id).expanduser().resolve()
    report_dir.mkdir(parents=True, exist_ok=True)
    input_root = args.ledger.expanduser().resolve().parent if args.ledger else ROOT
    artifact_root = (report_dir / "artifacts").resolve()
    artifact_root.mkdir(parents=True, exist_ok=True)
    executable = resolve_executable(args.a4n)
    ledger = read_ledger(args.ledger)
    environment = os.environ.copy()
    if args.username:
        environment.pop("A4N_ACCESS_TOKEN", None)
        environment.pop(args.password_env, None)
    generated_at = datetime.now(timezone.utc).isoformat()
    provider: Provider | None = None
    provider_log: Path | None = None
    provider_scenario_start_index: int | None = None
    provider_base_url: str | None = None
    preflight_results: list[dict[str, Any]] = []
    scenario_results: list[dict[str, Any]] = []
    fatal_error: str | None = None

    with tempfile.TemporaryDirectory(prefix="a4n-cli-contract-e2e-") as temporary:
        work_dir = Path(temporary)
        config = args.config.expanduser().resolve() if args.config else work_dir / "cli-config.json"
        context = args.context.expanduser().resolve() if args.context else work_dir / "cli-context.json"
        cli = Cli(executable, args.server, config, context, args.command_timeout, environment)
        manifest_envelope, manifest_result = checked(
            cli, ["capabilities", "export"], "capabilities export"
        )
        manifest = manifest_envelope.get("data")
        if not isinstance(manifest, dict) or not isinstance(manifest.get("commands"), list):
            raise RunnerError("capabilities export 缺少 data.commands")
        coverage = classify(manifest, ledger)

        if not args.inventory_only:
            try:
                provider, provider_log = start_provider(args, report_dir, run_id)
                provider_base_url = provider.base_url
                provider_contract = check_provider_contract(provider.base_url, args.command_timeout)
                preflight_results.append(
                    {
                        "name": "virtual provider contract",
                        "status": "passed",
                        "details": provider_contract,
                    }
                )
                runtime_path = work_dir / "runtime-config.json"
                secure_json(runtime_path, runtime_snapshot(provider.base_url))
                _, runtime_result = checked(
                    cli,
                    ["config", "runtime", "set", "--input", f"@{runtime_path}"],
                    "config runtime set",
                )
                preflight_results.append(
                    {
                        "name": "config runtime set",
                        "status": "passed",
                        "duration_ms": runtime_result["duration_ms"],
                    }
                )
                preflight_results += preflight(cli, args, ledger, coverage["descriptors"])
                provider_scenario_start_index = len(provider_log_entries(provider_log))
                scenario_results = execute_scenarios(
                    cli,
                    args,
                    ledger,
                    coverage,
                    work_dir,
                    input_root,
                    artifact_root,
                    {
                        "provider_base_url": provider.base_url,
                        "server_url": args.server,
                        "run_id": run_id,
                    },
                )
            except RunnerError as error:
                fatal_error = str(error)
            finally:
                if provider:
                    provider.stop()

    public_coverage = {key: value for key, value in coverage.items() if key != "descriptors"}
    failed_scenarios = [result for result in scenario_results if result.get("status") != "passed"]
    classified_complete, tested_complete = completion_flags(coverage)
    complete = tested_complete
    success = (
        fatal_error is None
        and not coverage["stale"]
        and not coverage["conflicts"]
        and (
            args.inventory_only
            or scenario_execution_success(fatal_error, scenario_results)
        )
    )
    report = {
        "schema_version": 1,
        "generated_at": generated_at,
        "run_id": run_id,
        "mode": "inventory" if args.inventory_only else "execute",
        "success": success,
        "complete": complete,
        "classified_complete": classified_complete,
        "tested_complete": tested_complete,
        "cli": executable,
        "server": args.server or "<CLI default/config>",
        "ledger": str(args.ledger.resolve()) if args.ledger else None,
        "manifest": {
            "version": manifest.get("manifest_version"),
            "commands": manifest.get("total_commands", len(manifest["commands"])),
            "export_duration_ms": manifest_result["duration_ms"],
        },
        "provider": {
            "base_url": provider_base_url,
            **provider_log_summary(provider_log, provider_scenario_start_index),
        },
        "preflight": preflight_results,
        "coverage": public_coverage,
        "scenario_results": scenario_results,
        "fatal_error": fatal_error,
    }
    json_path = report_dir / "coverage.json"
    markdown_path = report_dir / "coverage.md"
    atomic_write(json_path, json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    atomic_write(markdown_path, markdown(report))
    print(
        json.dumps(
            {
                "success": success,
                "complete": complete,
                "classified_complete": classified_complete,
                "tested_complete": tested_complete,
                "counts": coverage["counts"],
                "failed_scenarios": len(failed_scenarios),
                "json_report": str(json_path),
                "markdown_report": str(markdown_path),
                "fatal_error": fatal_error,
            },
            ensure_ascii=False,
        )
    )
    return 0 if success else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (RunnerError, OSError) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2)
