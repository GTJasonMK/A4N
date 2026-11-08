"""HTTP transport and CLI entry point for the deterministic provider."""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import re
import threading
import time
from datetime import datetime, timedelta, timezone
from email import policy
from email.parser import BytesParser
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any

from mock_openai.fixtures.reverse import (
    agent_chat_fixture,
    cache_fixture_usage,
    embedding_for,
    message_text,
)
from mock_openai.protocol import (
    MP4_1S_BASE64,
    PNG_1X1_BASE64,
    UnhandledPromptError,
    VideoIdempotencyConflict,
)
from mock_openai.response import response_for
from mock_openai.contract_guard import unique_object, validate_business_request
class ProviderHandler(BaseHTTPRequestHandler):
    server: "ProviderServer"

    def log_message(self, _format: str, *_args: object) -> None:
        return

    def _json(self, status: int, value: Any) -> None:
        data = json.dumps(value, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def _bytes(self, status: int, content_type: str, data: bytes) -> None:
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def _has_bearer_token(self) -> bool:
        scheme, _, token = self.headers.get("Authorization", "").partition(" ")
        return scheme.lower() == "bearer" and bool(token.strip())

    def do_GET(self) -> None:
        normalized_path = self.path.split("?", 1)[0].rstrip("/")
        if normalized_path == "/v1beta/models":
            self._json(200, {"models": [{"name": "models/a4n-virtual-text", "displayName": "Virtual text"}]})
            return
        if normalized_path in {"/models", "/v1/models"}:
            self._json(
                200,
                {
                    "object": "list",
                    "data": [
                        {"id": "a4n-virtual-text", "object": "model"},
                        {"id": "a4n-virtual-embedding", "object": "model"},
                        {"id": "a4n-virtual-image", "object": "model"},
                        {"id": "a4n-virtual-video", "object": "model"},
                    ],
                },
            )
            return
        if normalized_path == "/health":
            self._json(200, {"status": "ok"})
            return
        if normalized_path == "/fixtures/video.mp4":
            self._bytes(200, "video/mp4", base64.b64decode(self.server.video_base64))
            return
        video_content_match = re.fullmatch(r"/(?:v1/)?videos/([^/]+)/content", normalized_path)
        if video_content_match:
            self._bytes(200, "video/mp4", base64.b64decode(self.server.video_base64))
            return
        video_match = re.fullmatch(r"/(?:v1/)?videos/(?:generations/)?([^/]+)", normalized_path)
        if video_match:
            self._get_video_generation(video_match.group(1))
            return
        self._json(404, {"error": {"message": "not found"}})

    def _get_video_generation(self, job_id: str) -> None:
        response = self.server.poll_video_job(job_id)
        if response is None:
            self._json(404, {"error": {"message": "video generation not found"}})
            return
        self._json(200, response)

    def do_DELETE(self) -> None:
        normalized_path = self.path.split("?", 1)[0].rstrip("/")
        if not normalized_path.startswith("/api/temp-media/"):
            self._json(404, {"error": {"message": "not found"}})
            return
        if not self._has_bearer_token():
            self._json(401, {"error": {"message": "missing bearer token"}})
            return
        length = int(self.headers.get("Content-Length", "0"))
        raw_body = self.rfile.read(length)
        try:
            body = json.loads(raw_body or b"{}", object_pairs_hook=unique_object)
            if not isinstance(body, dict):
                raise ValueError("request body must be a JSON object")
        except ValueError as error:
            self._json(400, {"error": {"message": f"invalid json: {error}"}})
            return
        sha256 = body.get("sha256")
        if not isinstance(sha256, str) or not re.fullmatch(r"[0-9a-fA-F]{64}", sha256):
            self._json(400, {"error": {"message": "invalid sha256"}})
            return
        self.server.record(normalized_path, {"sha256": sha256.lower()})
        self.send_response(204)
        self.end_headers()

    def do_POST(self) -> None:
        length = int(self.headers.get("Content-Length", "0"))
        raw_body = self.rfile.read(length)
        normalized_path = self.path.split("?", 1)[0].rstrip("/")

        if normalized_path in {"/images/edits", "/v1/images/edits"}:
            content_type = self.headers.get("Content-Type", "")
            if "multipart/form-data" not in content_type:
                self._json(400, {"error": {"message": "image edit requires multipart/form-data"}})
                return
            required_fields = [b'name="model"', b'name="prompt"']
            has_image_field = b'name="image"' in raw_body or b'name="image[]"' in raw_body
            if any(field not in raw_body for field in required_fields) or not has_image_field:
                self._json(400, {"error": {"message": "image edit multipart is incomplete"}})
                return
            if not any(
                signature in raw_body
                for signature in (b"\x89PNG\r\n\x1a\n", b"\xff\xd8\xff", b"RIFF")
            ):
                self._json(400, {"error": {"message": "image edit has no image bytes"}})
                return
            self.server.record(
                self.path,
                {
                    "content_type": "multipart/form-data",
                    "body_bytes": len(raw_body),
                    "has_reference_image": True,
                },
            )
            self._json(200, {"created": 0, "data": [{"b64_json": self.server.image_base64}]})
            return

        if normalized_path == "/api/temp-media":
            if not self._has_bearer_token():
                self._json(401, {"error": {"message": "missing bearer token"}})
                return
            content_type = self.headers.get("Content-Type", "")
            if not content_type.startswith("multipart/form-data;"):
                self._json(415, {"error": {"message": "temp media upload requires multipart/form-data"}})
                return
            try:
                message = BytesParser(policy=policy.default).parsebytes(
                    b"Content-Type: " + content_type.encode("ascii") + b"\r\n\r\n" + raw_body
                )
                parts = list(message.iter_parts())
                if [part.get_param("name", header="content-disposition") for part in parts] != ["metadata", "file"]:
                    raise ValueError("expected metadata and file parts in order")
                body = json.loads(parts[0].get_payload(decode=True))
                if set(body) != {"owner_user_id", "project_id", "source_media_kind", "source_media_id", "file_name", "mime_type"}:
                    raise ValueError("invalid metadata fields")
                media_bytes = parts[1].get_payload(decode=True)
                if any(type(body[key]) is not int for key in ("owner_user_id", "project_id", "source_media_id")):
                    raise ValueError("identity fields must be integers")
                if any(not isinstance(body[key], str) or not body[key].strip() for key in ("source_media_kind", "file_name", "mime_type")):
                    raise ValueError("media metadata strings must not be empty")
                owner_user_id = body["owner_user_id"]
                project_id = body["project_id"]
                source_media_id = body["source_media_id"]
                source_media_kind = body["source_media_kind"]
                file_name = body["file_name"]
                mime_type = body["mime_type"]
                if parts[1].get_content_type() != mime_type:
                    raise ValueError("file MIME does not match metadata")
            except (KeyError, TypeError, ValueError) as error:
                self._json(400, {"error": {"message": f"invalid temp media upload: {error}"}})
                return
            if min(owner_user_id, project_id, source_media_id) <= 0 or not source_media_kind:
                self._json(400, {"error": {"message": "temp media identity must be valid"}})
                return
            sha256 = hashlib.sha256(media_bytes).hexdigest()
            artifact_id = sha256[:32]
            expires_at = datetime.now(timezone.utc) + timedelta(hours=1)
            self.server.record(
                self.path,
                {
                    "owner_user_id": owner_user_id,
                    "project_id": project_id,
                    "source_media_kind": source_media_kind,
                    "source_media_id": source_media_id,
                    "content_type": "multipart/form-data",
                    "body_bytes": len(raw_body),
                    "file_name": file_name,
                    "mime_type": mime_type,
                    "byte_size": len(media_bytes),
                    "sha256": sha256,
                },
            )
            self._json(
                200,
                {
                    "artifact_id": artifact_id,
                    "url": f"https://media.example.test/{artifact_id}/{file_name}",
                    "expires_at": expires_at.isoformat().replace("+00:00", "Z"),
                    "owner_user_id": owner_user_id,
                    "project_id": project_id,
                    "source_media_kind": source_media_kind,
                    "source_media_id": source_media_id,
                    "content_type": mime_type,
                    "sha256": sha256,
                    "byte_size": len(media_bytes),
                },
            )
            return

        try:
            body = json.loads(raw_body or b"{}", object_pairs_hook=unique_object)
            if not isinstance(body, dict):
                raise ValueError("request body must be a JSON object")
        except ValueError as error:
            self._json(400, {"error": {"message": f"invalid json: {error}"}})
            return

        self.server.record(self.path, body)
        if self._native_llm(normalized_path, body):
            return
        if normalized_path in {"/embeddings", "/v1/embeddings"}:
            inputs = body.get("input")
            if isinstance(inputs, str):
                inputs = [inputs]
            if not isinstance(inputs, list) or not inputs or not all(
                isinstance(value, str) for value in inputs
            ):
                self._json(400, {"error": {"message": "embedding input must be non-empty text/list"}})
                return
            self._json(
                200,
                {
                    "object": "list",
                    "data": [
                        {
                            "object": "embedding",
                            "index": index,
                            "embedding": embedding_for(value, self.server.embedding_dimension),
                        }
                        for index, value in enumerate(inputs)
                    ],
                    "model": body.get("model", "a4n-virtual-embedding"),
                    "usage": {"prompt_tokens": len(inputs), "total_tokens": len(inputs)},
                },
            )
            return
        if normalized_path in {"/images/generations", "/v1/images/generations"}:
            self._json(200, {"created": 0, "data": [{"b64_json": self.server.image_base64}]})
            return
        if normalized_path in {
            "/videos/generations",
            "/v1/videos/generations",
            "/videos",
            "/v1/videos",
        }:
            if normalized_path in {"/videos", "/v1/videos"} and (
                not isinstance(body.get("model"), str)
                or not isinstance(body.get("prompt"), str)
                or not isinstance(body.get("seconds"), str)
                or not isinstance(body.get("size"), str)
            ):
                self._json(400, {"error": {"message": "invalid Seedance video request"}})
                return
            idempotency_key = self.headers.get("Idempotency-Key", "").strip() or None
            try:
                response = self.server.create_video_job(body, idempotency_key)
            except VideoIdempotencyConflict as error:
                self._json(409, {"error": {"message": str(error)}})
                return
            self._json(200, response)
            return
        if normalized_path not in {"/chat/completions", "/v1/chat/completions"}:
            self._json(404, {"error": {"message": "not found"}})
            return

        try:
            lifecycle_fixture = str(body.get("model", "")).removeprefix("a4n-llm-lifecycle-")
            if lifecycle_fixture in {"ok", "length", "missing", "slow"}:
                content = "LLM lifecycle fixture output"
                if lifecycle_fixture == "slow":
                    marker = "partial answer"
                    for message in reversed(body.get("messages") or []):
                        if message.get("role") != "user":
                            continue
                        match = re.search(
                            r"A4N_TAURI_GENERATION_INPUT:([A-Za-z0-9_-]+)",
                            message_text(message.get("content", "")),
                        )
                        if match:
                            marker = f"A4N_TAURI_GENERATION_REPLY:{match.group(1)}"
                            break
                    system = "\n".join(
                        message_text(message.get("content", ""))
                        for message in body.get("messages") or []
                        if message.get("role") == "system"
                    )
                    if "小说概念大师·文思" in system:
                        response = json.loads(response_for({**body, "model": "a4n-creation-journey"}))
                        response["ai_message"] = marker + " 正在继续生成。" * 350
                        content = json.dumps(response, ensure_ascii=False)
                    else:
                        content = f"{marker} partial answer " * 200
                agent_fixture = {"content": content}
            elif body.get("a4n_test_cache_usage") is True:
                agent_fixture = {"content": "雨夜里，角色谨慎地推开仓库门。"}
            else:
                agent_fixture = agent_chat_fixture(body)
        except (ValueError, KeyError, TypeError) as error:
            self._json(422, {"error": {"message": str(error), "type": "invalid_agent_fixture"}})
            return
        if agent_fixture is None:
            try:
                content = response_for(body)
                # Deliberate fault responses must reach the production recovery path.
                # Validate their normal fixture, then send the injected response unchanged.
                normal_body = {**body, "model": "a4n-virtual-text"} if str(body.get("model", "")).startswith(("a4n-output-", "a4n-anchor-", "a4n-checkpoint-")) or body.get("model") == "a4n-fact-feedback-invalid" else body
                normal = response_for(normal_body) if normal_body is not body else content
                validate_business_request(body, body.get("messages", []), normal)
            except (UnhandledPromptError, ValueError) as error:
                self._json(422, {"error": {"message": str(error), "type": "unhandled_prompt"}})
                return
            tool_calls = []
        else:
            content = str(agent_fixture.get("content", ""))
            tool_calls = agent_fixture.get("tool_calls") or ([agent_fixture["tool_call"]] if agent_fixture.get("tool_call") else [])
        if body.get("stream") is True:
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream; charset=utf-8")
            self.send_header("Cache-Control", "no-cache")
            self.end_headers()
            if tool_calls:
                events = []
                if content:
                    events.append({"choices": [{"index": 0, "delta": {"content": content}, "finish_reason": None}]})
                for tool_index, tool_call in enumerate(tool_calls):
                    arguments = json.dumps(tool_call["arguments"], ensure_ascii=False, separators=(",", ":"))
                    split_at = max(1, len(arguments) // 2)
                    deltas = [
                        {"id": tool_call["id"], "type": "function"},
                        {"function": {"name": tool_call["name"]}},
                        *({"function": {"arguments": fragment}} for fragment in (arguments[:split_at], arguments[split_at:]) if fragment),
                    ]
                    events.extend({"choices": [{"index": 0, "delta": {"tool_calls": [{"index": tool_index, **delta}]}, "finish_reason": None}]} for delta in deltas)
                events.append(
                    {
                        "choices": [
                            {"index": 0, "delta": {}, "finish_reason": "tool_calls"}
                        ],
                        "usage": {
                            "prompt_tokens": 10,
                            "completion_tokens": 20,
                            "total_tokens": 30,
                        },
                    }
                )
                for event in events:
                    self.wfile.write(
                        ("data: " + json.dumps(event, ensure_ascii=False) + "\n\n").encode(
                            "utf-8"
                        )
                    )
                self.wfile.write(b"data: [DONE]\n\n")
                self.wfile.flush()
                return
            chunk_size = 32 if lifecycle_fixture == "slow" else 1024
            for index in range(0, len(content), chunk_size):
                event = {
                    "choices": [
                        {
                            "index": 0,
                            "delta": {"content": content[index : index + chunk_size]},
                            "finish_reason": None,
                        }
                    ]
                }
                try:
                    self.wfile.write(("data: " + json.dumps(event, ensure_ascii=False) + "\n\n").encode("utf-8"))
                    if lifecycle_fixture == "slow":
                        self.wfile.flush()
                        time.sleep(0.05)
                except (BrokenPipeError, ConnectionResetError):
                    # Cancellation fixtures deliberately close an active response.
                    return
            final = {
                "choices": [{"index": 0, "delta": {}, "finish_reason": {"length": "length", "missing": None}.get(lifecycle_fixture, "stop")}],
                "usage": {"prompt_tokens": 10, "completion_tokens": 20, "total_tokens": 30},
            }
            if body.get("a4n_test_cache_usage") is True:
                final["usage"] = cache_fixture_usage("chat")
            self.wfile.write(("data: " + json.dumps(final) + "\n\n").encode("utf-8"))
            self.wfile.write(b"data: [DONE]\n\n")
            self.wfile.flush()
            return

        if tool_calls:
            self._json(
                200,
                {
                    "id": "chatcmpl-a4n-e2e",
                    "object": "chat.completion",
                    "choices": [
                        {
                            "index": 0,
                            "message": {
                                "role": "assistant",
                                "content": content,
                                "tool_calls": [
                                    {
                                        "id": tool_call["id"],
                                        "type": "function",
                                        "function": {
                                            "name": tool_call["name"],
                                            "arguments": json.dumps(
                                                tool_call["arguments"],
                                                ensure_ascii=False,
                                                separators=(",", ":"),
                                            ),
                                        },
                                    } for tool_call in tool_calls
                                ],
                            },
                            "finish_reason": "tool_calls",
                        }
                    ],
                    "usage": {
                        "prompt_tokens": 10,
                        "completion_tokens": 20,
                        "total_tokens": 30,
                    },
                },
            )
            return

        self._json(
            200,
            {
                "id": "chatcmpl-a4n-e2e",
                "object": "chat.completion",
                "choices": [
                    {
                        "index": 0,
                        "message": {"role": "assistant", "content": content},
                        "finish_reason": "stop",
                    }
                ],
                "usage": {"prompt_tokens": 10, "completion_tokens": 20, "total_tokens": 30},
            },
        )


    def _native_llm(self, path: str, body: dict[str, Any]) -> bool:
        """Wire fixtures for native text protocols; reuse the prompt fixture owner."""
        if path not in {"/v1/responses", "/v1/messages", "/v1beta/models/a4n-virtual-text:streamGenerateContent"}:
            return False
        try:
            if path == "/v1/responses":
                if not self._has_bearer_token() or body.get("stream") is not True or not isinstance(body.get("input"), list):
                    raise ValueError("Responses requires bearer auth, stream and input[]")
                messages = [{"role": item["role"], "content": item["content"]} for item in body["input"] if "role" in item]
            elif path == "/v1/messages":
                if not self.headers.get("x-api-key") or not self.headers.get("anthropic-version") or body.get("stream") is not True:
                    raise ValueError("Messages requires native auth/version and stream")
                messages = [{"role": "system", "content": message_text(body.get("system", []))}]
                messages += [{"role": item["role"], "content": message_text(item["content"])} for item in body["messages"]]
            else:
                if not self.headers.get("x-goog-api-key") or "alt=sse" not in self.path:
                    raise ValueError("Gemini requires native auth and alt=sse")
                body = {**body, "model": path.split("/models/", 1)[1].split(":", 1)[0]}
                messages = [{"role": "system", "content": "\n".join(part["text"] for part in body.get("systemInstruction", {}).get("parts", []))}]
                messages += [{"role": "assistant" if item["role"] == "model" else "user", "content": "\n".join(part["text"] for part in item["parts"])} for item in body["contents"]]
            content = "雨夜里，角色谨慎地推开仓库门。" if body.get("a4n_test_cache_usage") is True else response_for({"messages": messages, "model": body.get("model", "")})
            validate_business_request(body, messages, content)
        except (KeyError, TypeError, ValueError, UnhandledPromptError) as error:
            self._json(422, {"error": {"message": str(error)}})
            return True
        if path == "/v1/responses":
            events = [
                {"type": "response.output_text.delta", "delta": content},
                {"type": "response.completed", "response": {"status": "completed", "output": [
                    {"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": content}]}
                ], "usage": {"input_tokens": 2, "output_tokens": 1, "total_tokens": 3}}},
            ]
        elif path == "/v1/messages":
            events = [
                {"type": "message_start", "message": {"usage": {"input_tokens": 2, "output_tokens": 0}}},
                {"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}},
                {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": content}},
                {"type": "content_block_stop", "index": 0},
                {"type": "message_delta", "delta": {"stop_reason": "end_turn"}, "usage": {"output_tokens": 1}},
                {"type": "message_stop"},
            ]
        else:
            events = [{"candidates": [{"content": {"role": "model", "parts": [{"text": content}]}, "finishReason": "STOP"}],
                       "usageMetadata": {"promptTokenCount": 2, "candidatesTokenCount": 1, "totalTokenCount": 3}}]
        if body.get("a4n_test_cache_usage") is True:
            if path == "/v1/responses":
                events[-1]["response"]["usage"] = cache_fixture_usage("responses")
            elif path == "/v1/messages":
                events[0]["message"]["usage"] = cache_fixture_usage("anthropic")
                events[-2]["usage"]["output_tokens"] = 100
            else:
                events[0]["usageMetadata"] = cache_fixture_usage("gemini")
        data = "".join(f"data: {json.dumps(event, ensure_ascii=False)}\n\n" for event in events).encode("utf-8")
        self._bytes(200, "text/event-stream", data)
        return True


class ProviderServer(ThreadingHTTPServer):
    def __init__(
        self,
        address: tuple[str, int],
        log_path: Path,
        image_base64: str,
        video_base64: str,
        embedding_dimension: int,
    ):
        super().__init__(address, ProviderHandler)
        self.log_path = log_path
        self.image_base64 = image_base64
        self.video_base64 = video_base64
        self.embedding_dimension = embedding_dimension
        self.log_lock = threading.Lock()
        self.request_index = 0
        self.video_lock = threading.Lock()
        self.video_jobs: dict[str, dict[str, Any]] = {}
        self.video_idempotency: dict[str, tuple[str, str]] = {}
        self.video_job_index = 0

    def create_video_job(
        self, body: dict[str, Any], idempotency_key: str | None
    ) -> dict[str, Any]:
        fingerprint = json.dumps(
            body, ensure_ascii=False, sort_keys=True, separators=(",", ":")
        )
        with self.video_lock:
            if idempotency_key is not None:
                existing = self.video_idempotency.get(idempotency_key)
                if existing is not None:
                    job_id, existing_fingerprint = existing
                    if existing_fingerprint != fingerprint:
                        raise VideoIdempotencyConflict(
                            "Idempotency-Key 已用于不同的视频请求"
                        )
                    return self._video_job_response_locked(job_id)

            self.video_job_index += 1
            job_id = (
                "a4n-e2e-video-job"
                if self.video_job_index == 1
                else f"a4n-e2e-video-job-{self.video_job_index}"
            )
            self.video_jobs[job_id] = {
                "body": body,
                "polls": 0,
                "status": "pending",
            }
            if idempotency_key is not None:
                self.video_idempotency[idempotency_key] = (job_id, fingerprint)
            return self._video_job_response_locked(job_id)

    def poll_video_job(self, job_id: str) -> dict[str, Any] | None:
        with self.video_lock:
            if job_id not in self.video_jobs:
                return None
            job = self.video_jobs[job_id]
            job["polls"] += 1
            job["status"] = (
                "pending"
                if job["polls"] == 1
                else "running"
                if job["polls"] == 2
                else "completed"
            )
            return self._video_job_response_locked(job_id)

    def _video_job_response_locked(self, job_id: str) -> dict[str, Any]:
        job = self.video_jobs[job_id]
        response: dict[str, Any] = {
            "id": job_id,
            "status": job["status"],
        }
        if job["status"] == "completed":
            response["data"] = [
                {"b64_json": self.video_base64, "format": "mp4"}
            ]
        return response

    def record(self, path: str, body: dict[str, Any]) -> None:
        with self.log_lock:
            self.request_index += 1
            entry = {"index": self.request_index, "path": path, "body": body}
            with self.log_path.open("a", encoding="utf-8") as log:
                log.write(json.dumps(entry, ensure_ascii=False) + "\n")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=18765)
    parser.add_argument("--log", type=Path, required=True)
    parser.add_argument("--image-file", type=Path)
    parser.add_argument("--video-file", type=Path)
    parser.add_argument("--embedding-dimension", type=int, default=8)
    args = parser.parse_args()
    if args.embedding_dimension <= 0:
        parser.error("--embedding-dimension must be positive")
    args.log.parent.mkdir(parents=True, exist_ok=True)
    image_base64 = (
        base64.b64encode(args.image_file.read_bytes()).decode("ascii")
        if args.image_file
        else PNG_1X1_BASE64
    )
    video_base64 = (
        base64.b64encode(args.video_file.read_bytes()).decode("ascii")
        if args.video_file
        else MP4_1S_BASE64
    )
    ProviderServer(
        (args.host, args.port),
        args.log,
        image_base64,
        video_base64,
        args.embedding_dimension,
    ).serve_forever()
