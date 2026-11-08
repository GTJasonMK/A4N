"""A bounded, loopback-only relay for opt-in live model evaluations."""

from __future__ import annotations

import hmac
import json
import math
import secrets
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.error import HTTPError, URLError
from urllib.parse import urlsplit
from urllib.request import Request, urlopen


class BudgetedLlmProxy:
    def __init__(self, upstream, api_key, *, max_calls, max_output_tokens, timeout_seconds, allow_local_http=False):
        target = urlsplit(upstream)
        if target.username or target.password or target.query or target.fragment or not target.hostname:
            raise ValueError("模型地址必须为无凭证、查询参数或片段的 URL")
        if target.port is not None and not 0 < target.port <= 65535:
            raise ValueError("模型地址端口无效")
        if target.scheme != "https" and not (allow_local_http and target.scheme == "http" and target.hostname in {"127.0.0.1", "localhost", "::1"}):
            raise ValueError("真实模型地址必须使用 HTTPS；本地测试需显式允许 loopback HTTP")
        if not api_key or type(max_calls) is not int or max_calls <= 0 or type(max_output_tokens) is not int or max_output_tokens <= 0 or not math.isfinite(timeout_seconds) or timeout_seconds <= 0:
            raise ValueError("凭证和正数调用/token/时间预算必须明确提供")
        self.origin = f"{target.scheme}://{target.netloc}"
        self.prefix = target.path.rstrip("/")
        self.api_key = api_key
        self.token = secrets.token_urlsafe(32)
        self.max_calls = max_calls
        self.max_output_tokens = max_output_tokens
        self.deadline = time.monotonic() + timeout_seconds
        self.lock = threading.Lock()
        self.calls = []
        self.active = set()
        self.forwarded_calls = 0
        proxy = self

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def do_POST(self):
                supplied = self.headers.get("Authorization", "").removeprefix("Bearer ") or self.headers.get("x-api-key", "") or self.headers.get("x-goog-api-key", "")
                if not hmac.compare_digest(supplied, proxy.token):
                    self.send_error(401, "evaluation relay credential required")
                    return
                path = urlsplit(self.path).path
                if not (path.endswith(("/chat/completions", "/responses", "/messages")) or path.endswith(":streamGenerateContent")):
                    self.send_error(404, "only model generation endpoints are allowed")
                    return
                try:
                    length = int(self.headers.get("Content-Length", "0"))
                    if not 0 < length <= 262144:
                        raise ValueError("model request must be between 1 and 262144 bytes")
                    body = json.loads(self.rfile.read(length))
                    if path.endswith(":streamGenerateContent") and isinstance(body, dict):
                        body.setdefault("generationConfig", {})
                    cap_output_tokens(body, proxy.max_output_tokens, default_field="max_output_tokens" if path.endswith("/responses") else "max_tokens")
                except (ValueError, TypeError, KeyError) as error:
                    self.send_error(400, str(error))
                    return
                row = {"path": path, "status": None, "correction": "model_output_correction" in json.dumps(body), "usage": None, "stream_completed": False}
                started = time.monotonic()
                with proxy.lock:
                    proxy.calls.append(row)
                    remaining = proxy.deadline - time.monotonic()
                    if proxy.forwarded_calls >= proxy.max_calls or remaining <= 0:
                        row.update(status=429, error="evaluation budget exhausted", duration_ms=0)
                        self.send_error(429, "evaluation budget exhausted")
                        return
                    proxy.forwarded_calls += 1
                headers = {"Content-Type": "application/json", "Accept": "text/event-stream"}
                if path.endswith("/messages"):
                    headers.update({"x-api-key": proxy.api_key, "anthropic-version": self.headers.get("anthropic-version", "2023-06-01")})
                elif path.endswith(":streamGenerateContent"):
                    headers["x-goog-api-key"] = proxy.api_key
                else:
                    headers["Authorization"] = "Bearer " + proxy.api_key
                remote = None
                try:
                    request = Request(proxy.origin + self.path, json.dumps(body).encode(), headers)
                    try:
                        remote = urlopen(request, timeout=min(90, remaining))
                    except HTTPError as error:
                        remote = error
                    with proxy.lock:
                        proxy.active.add(remote)
                    row["status"] = remote.status
                    self.send_response(remote.status)
                    self.send_header("Content-Type", remote.headers.get("Content-Type", "application/json"))
                    self.end_headers()
                    pending = b""
                    while chunk := remote.read1(65536):
                        if time.monotonic() >= proxy.deadline:
                            raise TimeoutError("evaluation time budget exhausted")
                        self.wfile.write(chunk)
                        self.wfile.flush()
                        pending += chunk
                        lines = pending.split(b"\n")
                        pending = lines.pop()[-262144:]
                        for line in lines:
                            if line.startswith(b"data:"):
                                usage = event_usage(line[5:].strip())
                                if usage is not None:
                                    row["usage"] = {**(row["usage"] or {}), **usage}
                    row["stream_completed"] = True
                except (OSError, URLError, TimeoutError) as error:
                    row["error"] = type(error).__name__
                    if row["status"] is None:
                        row["status"] = 502
                        self.send_error(502, "upstream model transport failed")
                finally:
                    row["duration_ms"] = round((time.monotonic() - started) * 1000)
                    if remote is not None:
                        with proxy.lock:
                            proxy.active.discard(remote)
                        remote.close()

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.server.daemon_threads = True
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.base_url = f"http://127.0.0.1:{self.server.server_address[1]}" + self.prefix

    def __enter__(self):
        self.thread.start()
        return self

    def __exit__(self, *_):
        self.server.shutdown()
        with self.lock:
            active = list(self.active)
        for remote in active:
            remote.close()
        self.server.server_close()
        self.thread.join(timeout=5)


def cap_output_tokens(body, maximum, *, default_field="max_tokens"):
    if not isinstance(body, dict):
        raise ValueError("model request must be an object")
    if "generationConfig" in body:
        if any(key in body for key in ("max_output_tokens", "max_completion_tokens", "max_tokens")):
            raise ValueError("ambiguous output token budget fields")
        config = body["generationConfig"]
        if not isinstance(config, dict):
            raise ValueError("generationConfig must be an object")
        target, field = config, "maxOutputTokens"
    else:
        target = body
        if sum(key in body for key in ("max_output_tokens", "max_completion_tokens", "max_tokens")) > 1:
            raise ValueError("ambiguous output token budget fields")
        field = next((key for key in ("max_output_tokens", "max_completion_tokens", "max_tokens") if key in body), default_field)
    value = target.get(field, maximum)
    if type(value) is not int or value <= 0:
        raise ValueError("output token cap must be a positive integer")
    target[field] = min(value, maximum)


def event_usage(raw):
    try:
        value = json.loads(raw)
    except (ValueError, UnicodeError):
        return None
    if not isinstance(value, dict):
        return None
    usage = value.get("usage") or value.get("usageMetadata")
    for key in ("response", "message"):
        nested = value.get(key)
        if not usage and isinstance(nested, dict):
            usage = nested.get("usage")
    return usage if isinstance(usage, dict) else None
