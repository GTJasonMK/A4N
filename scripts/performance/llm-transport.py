#!/usr/bin/env python3
"""Measure the real LLM gateway against a verified loopback TLS/SSE provider."""

import argparse
import hashlib
import http.client
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import math
import os
from pathlib import Path
import secrets
import socket
import ssl
import statistics
import subprocess
import sys
import threading
import time

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/tests"))
from run_export_worker_runtime_e2e import service_request, start_process, stop_process


class Provider(ThreadingHTTPServer):
    daemon_threads = True

    def __init__(self, certificate, key):
        self.accepted = 0
        self.requests = []
        super().__init__(("127.0.0.1", 0), Handler)
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(certificate, key)
        self.socket = context.wrap_socket(self.socket, server_side=True)

    def get_request(self):
        connection, address = super().get_request()
        connection.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
        self.accepted += 1
        return connection, address


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *_):
        pass

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        self.server.requests.append({"body": body, "authorization": self.headers.get("Authorization"),
                                     "received_ns": time.perf_counter_ns()})
        chunks = [
            {"choices": [{"index": 0, "delta": {"content": "TLS fixture response"}, "finish_reason": None}]},
            {"choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}],
             "usage": {"prompt_tokens": 3, "completion_tokens": 2, "total_tokens": 5}},
        ]
        if body["model"] == "steady":
            chunks[0]["choices"][0]["delta"]["content"] = "TLS fixture "
            chunks.insert(1, {"choices": [{"index": 0, "delta": {"content": "response"}, "finish_reason": None}]})
        frames = [("data: " + json.dumps(chunk) + "\n\n").encode() for chunk in chunks] + [b"data: [DONE]\n\n"]
        payload = b"".join(frames)
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Transfer-Encoding", "chunked")
        self.end_headers()
        if body["model"] == "paused":
            time.sleep(1.2)
        if body["model"] == "steady":
            for index, frame in enumerate(frames):
                if index:
                    time.sleep(.6)
                self.wfile.write(f"{len(frame):x}\r\n".encode() + frame + b"\r\n")
                self.wfile.flush()
            self.wfile.write(b"0\r\n\r\n")
            self.wfile.flush()
            return
        self.wfile.write(f"{len(payload):x}\r\n".encode() + payload + b"\r\n0\r\n\r\n")
        self.wfile.flush()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--gateway", type=Path, required=True)
    parser.add_argument("--report-dir", type=Path, required=True)
    parser.add_argument("--requests", type=int, default=45)
    args = parser.parse_args()
    if args.requests < 10:
        parser.error("at least 10 requests are required")
    gateway, output = args.gateway.resolve(), args.report_dir.resolve()
    if not gateway.is_relative_to(ROOT) or not output.is_relative_to(ROOT / "data"):
        parser.error("gateway and reports must stay inside the worktree (reports under data)")
    output.mkdir(parents=True, exist_ok=False)
    output.chmod(0o700)
    certificate, key = output / "certificate.pem", output / "key.pem"
    with (output / "certificate.log").open("w") as log:
        subprocess.run(["openssl", "req", "-x509", "-newkey", "rsa:2048", "-sha256", "-nodes",
                        "-days", "1", "-subj", "/CN=localhost", "-addext", "subjectAltName=IP:127.0.0.1",
                        "-keyout", str(key), "-out", str(certificate)],
                       stdout=log, stderr=subprocess.STDOUT, check=True, timeout=30)
    key.chmod(0o600)
    # Keep production trust-store loading in the measurement; a single test root
    # would conceal the cost of constructing a new native-TLS client each time.
    roots = ssl.get_default_verify_paths().cafile
    if not roots:
        raise RuntimeError("The TLS benchmark requires the system CA bundle")
    trust_bundle = output / "trust-bundle.pem"
    trust_bundle.write_bytes(Path(roots).read_bytes() + b"\n" + certificate.read_bytes())
    provider = Provider(certificate, key)
    server_thread = threading.Thread(target=provider.serve_forever, daemon=True)
    server_thread.start()
    with socket.socket() as reservation:
        reservation.bind(("127.0.0.1", 0))
        gateway_port = reservation.getsockname()[1]
    token = secrets.token_urlsafe(32)
    env = {key: value for key, value in os.environ.items()
           if not key.startswith("A4N_") and key.lower() not in {"http_proxy", "https_proxy", "all_proxy"}}
    env.update({"A4N_LLM_GATEWAY_ADDR": f"127.0.0.1:{gateway_port}", "A4N_SIDECAR_AUTH_TOKEN": token,
                "A4N_LLM_GATEWAY_LOG": "warn", "SSL_CERT_FILE": str(trust_bundle)})
    process = None
    report = {"success": False, "gateway_sha256": hashlib.sha256(gateway.read_bytes()).hexdigest(), "samples": []}
    try:
        process = start_process("llm-gateway", [str(gateway)], env=env, cwd=output, log_path=output / "gateway.log")
        deadline = time.monotonic() + 10
        while True:
            if process.process.poll() is not None:
                raise RuntimeError("gateway exited; see gateway.log")
            connection = http.client.HTTPConnection("127.0.0.1", gateway_port, timeout=1)
            try:
                connection.request("GET", "/healthz", headers={"x-a4n-sidecar-token": token})
                response = connection.getresponse()
                if response.status == 200:
                    response.read()
                    break
            except (OSError, http.client.HTTPException):
                pass
            finally:
                connection.close()
            if time.monotonic() >= deadline:
                raise RuntimeError("gateway readiness timed out")
            time.sleep(0.05)
        connection = http.client.HTTPConnection("127.0.0.1", gateway_port, timeout=10)
        try:
            def call(index, model, timeout):
                request = service_request({
                    "provider": {"provider": "openai", "base_url": f"https://127.0.0.1:{provider.server_port}/v1",
                                 "model": model, "api_key": f"synthetic-{index}",
                                 "timeout_secs": timeout, "extra_body_json": None},
                    "request": {"messages": [{"role": "user", "content": "ping"}], "temperature": None,
                                "max_tokens": 16, "tools": None, "tool_choice": None, "stop_sequences": None},
                }, request_id=f"tls-{index}")
                request["capability_version"] = "llm-chat-v8"
                request["secret_policy"].update(scope="local_sidecar_only", allow_user_secret=True)
                body = json.dumps(request).encode()
                started = time.perf_counter_ns()
                connection.request("POST", "/llm/probe", body,
                                   {"Content-Type": "application/json", "x-a4n-sidecar-token": token})
                response = connection.getresponse()
                result = json.loads(response.read())
                elapsed_ms = (time.perf_counter_ns() - started) / 1_000_000
                assert response.status == 200, result
                upstream = provider.requests[-1]
                assert len(provider.requests) == index + 1
                assert upstream["authorization"] == f"Bearer synthetic-{index}"
                assert upstream["body"]["model"] == model and upstream["body"]["stream"] is True
                if result["status"] == "ok":
                    payload = result["payload"]
                    assert payload["response_preview"] == "TLS fixture response" and payload["finish_reason"] == "stop", result
                    assert payload["usage"]["total_tokens"] == 5, result
                return result, {"elapsed_ms": elapsed_ms,
                                "upstream_received_ms": (upstream["received_ns"] - started) / 1_000_000,
                                "accepted_connections": provider.accepted,
                                "payload_sha256": hashlib.sha256(json.dumps(result.get("payload"), sort_keys=True).encode()).hexdigest()}

            for index in range(args.requests):
                result, sample = call(index, f"fixture-{index}", 5)
                assert result["status"] == "ok", result
                report["samples"].append(sample)
            report.update(requests=args.requests, accepted_connections=provider.accepted, checks=[])
            for index, (model, timeout, expected) in enumerate([
                ("steady", 1, "ok"), ("paused", 1, "error"), ("paused", 2, "ok"), ("normal-again", 5, "ok"),
            ], start=args.requests):
                result, sample = call(index, model, timeout)
                assert result["status"] == expected, result
                if expected == "error":
                    assert "1 秒" in result["error"]["message"], result
                if model == "steady":
                    assert sample["elapsed_ms"] > 1000, sample
                report["checks"].append({"model": model, "timeout_secs": timeout, "status": result["status"],
                                        "elapsed_ms": sample["elapsed_ms"], "error": result.get("error")})
        finally:
            connection.close()
        values = sorted(row["elapsed_ms"] for row in report["samples"][5:])
        report.update(success=True, p50_ms=statistics.median(values), p95_ms=values[math.ceil(len(values) * .95) - 1])
        print(json.dumps({key: value for key, value in report.items() if key != "samples"}))
    except BaseException as error:
        report["error"] = str(error)
        raise
    finally:
        stop_process(process)
        provider.shutdown()
        provider.server_close()
        server_thread.join(timeout=5)
        (output / "report.json").write_text(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
