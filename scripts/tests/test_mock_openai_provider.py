#!/usr/bin/env python3
"""HTTP contract self-test for the deterministic virtual provider."""

from __future__ import annotations

import base64
import hashlib
import json
import subprocess
import tempfile
import threading
import unittest
import zlib
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import Request, urlopen

from mock_openai.protocol import (
    MP4_1S_BASE64,
    PNG_1X1_BASE64,
)
from mock_openai.transport import ProviderServer
from mock_openai.fixtures.entities import (
    character_discovery_response,
    character_sync_attribute_changes_response,
    character_sync_behaviors_response,
    character_sync_chapter_states_response,
    character_sync_evidence_events_response,
    object_discovery_response,
    scene_entity_state_response,
    scene_occurrence_response,
    scene_profile_candidates_response,
    scene_transition_response,
)
from mock_openai.fixtures.shooting import (
    shooting_adaptation_response,
    shooting_span_ids,
    shooting_design_response,
    video_segment_plan_response,
)
from mock_openai.fixtures.reverse import agent_chat_fixture
from mock_openai.response import response_for


def agent_observation(data):
    return {"observation_version": 1, "status": "success", "data": data,
            "text": "fixture observation", "error": None, "result_refs": [],
            "execution": {"id": "1", "complete": True, "read_tool": None},
            "stats": {"duration_ms": 1, "result_bytes": len(json.dumps(data).encode())}}


class MockOpenAIProviderTest(unittest.TestCase):
    def test_source_markers_are_read_from_actual_context_content(self) -> None:
        from mock_openai.fixtures.entities import source_span_pairs, text_anchor_id

        source = json.dumps({"content": "“⟦ch2:s3:f1⟧别走！”她说。\n⟦ch2:s4:f1⟧门关上了。"}, ensure_ascii=False)
        self.assertEqual([identifier for identifier, _ in source_span_pairs(source)], ["ch2:s3:f1", "ch2:s4:f1"])
        anchors = json.dumps({"content": "⟦41⟧林岚握住怀表。\n⟦42⟧周衡转身。"}, ensure_ascii=False)
        self.assertEqual(text_anchor_id(anchors, "握住怀表"), 41)
        with self.assertRaises(ValueError):
            text_anchor_id(anchors, "不存在的剧情")

    def test_cache_content_blocks_and_immutable_loop_observations(self) -> None:
        self.assertEqual(response_for({"messages": [{"role": "user", "content": [
            {"type": "text", "text": "pi"}, {"type": "text", "text": "ng"}
        ]}]}), "pong")
        with self.assertRaises(ValueError):
            response_for({"messages": [{"role": "user", "content": [{"type": "image_url"}]}]})
        observation = {"tool": "get_project_information", "success": True,
                       "result": {"content": {"source_spans": [{"text": "林岚走进机房。"}]}}}
        request = {"messages": [
            {"role": "system", "content": "content_optimization_chat_agent_loop"},
            {"role": "user", "content": json.dumps({"loop_observation": {"sequence": 1, "value": observation}}, ensure_ascii=False)},
            {"role": "user", "content": json.dumps({"loop_state": {"observations_ref": "previous messages", "progress": {"observation_count": 1}}})},
        ]}
        decision = json.loads(response_for(request))
        self.assertEqual(decision["arguments"]["tool"], "edit_article")
        self.assertEqual(decision["arguments"]["arguments"]["patches"][0]["old_text"], "林岚走进机房。")

    @classmethod
    def setUpClass(cls) -> None:
        cls.temp_dir = tempfile.TemporaryDirectory()
        cls.server = ProviderServer(
            ("127.0.0.1", 0),
            Path(cls.temp_dir.name) / "requests.jsonl",
            PNG_1X1_BASE64,
            MP4_1S_BASE64,
            8,
        )
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()
        host, port = cls.server.server_address
        cls.base_url = f"http://{host}:{port}"

    @classmethod
    def tearDownClass(cls) -> None:
        cls.server.shutdown()
        cls.server.server_close()
        cls.thread.join(timeout=2)
        cls.temp_dir.cleanup()

    def request_json(
        self,
        path: str,
        body: dict[str, object] | bytes | None = None,
        content_type: str = "application/json",
        headers: dict[str, str] | None = None,
    ) -> tuple[int, dict[str, object]]:
        data = body
        if isinstance(body, dict):
            data = json.dumps(body).encode("utf-8")
        request_headers = {"Content-Type": content_type} if data is not None else {}
        if headers:
            request_headers.update(headers)
        request = Request(
            self.base_url + path,
            data=data,
            headers=request_headers,
        )
        with urlopen(request, timeout=2) as response:
            return response.status, json.load(response)

    def request_sse_chunks(self, body: dict[str, object]) -> list[dict[str, object]]:
        request = Request(
            self.base_url + "/v1/chat/completions",
            data=json.dumps({**body, "stream": True}).encode("utf-8"),
            headers={"Content-Type": "application/json"},
        )
        with urlopen(request, timeout=2) as response:
            self.assertEqual(response.status, 200)
            self.assertEqual(response.headers.get_content_type(), "text/event-stream")
            events = [
                line.removeprefix("data: ")
                for line in response.read().decode("utf-8").splitlines()
                if line.startswith("data: ")
            ]
        self.assertEqual(events[-1], "[DONE]")
        return [json.loads(event) for event in events[:-1]]

    def test_native_text_protocol_http_contracts(self) -> None:
        cases = [
            ("/v1/responses", {"model": "a4n-virtual-text", "input": [{"role": "user", "content": "ping"}], "stream": True},
             {"Authorization": "Bearer fixture"}, "response.completed"),
            ("/v1/messages", {"model": "a4n-virtual-text", "messages": [{"role": "user", "content": [{"type": "text", "text": "ping"}]}], "stream": True},
             {"x-api-key": "fixture", "anthropic-version": "2023-06-01"}, "message_stop"),
            ("/v1/messages", {"model": "a4n-virtual-text", "messages": [{"role": "user", "content": "ping"}], "stream": True},
             {"x-api-key": "fixture", "anthropic-version": "2023-06-01"}, "message_stop"),
            ("/v1beta/models/a4n-virtual-text:streamGenerateContent?alt=sse", {"contents": [{"role": "user", "parts": [{"text": "ping"}]}]},
             {"x-goog-api-key": "fixture"}, None),
        ]
        for path, body, headers, terminal in cases:
            with self.subTest(path=path):
                request = Request(self.base_url + path, data=json.dumps(body).encode(), headers={"Content-Type": "application/json", **headers})
                with urlopen(request, timeout=2) as response:
                    self.assertEqual(response.headers.get_content_type(), "text/event-stream")
                    wire = response.read().decode()
                self.assertIn("pong", wire)
                events = [json.loads(line[6:]) for line in wire.splitlines() if line.startswith("data: ")]
                if terminal:
                    self.assertEqual(events[-1]["type"], terminal)
                else:
                    self.assertEqual(events[-1]["candidates"][0]["finishReason"], "STOP")

    def streamed_tool_call(
        self,
        chunks: list[dict[str, object]],
        expected_id: str,
        expected_name: str,
    ) -> dict[str, object]:
        tool_deltas = []
        for chunk in chunks:
            choice = chunk["choices"][0]
            self.assertEqual(choice["index"], 0)
            tool_deltas.extend(choice["delta"].get("tool_calls", []))

        self.assertGreaterEqual(len(tool_deltas), 4)
        self.assertTrue(all(delta["index"] == 0 for delta in tool_deltas))
        self.assertEqual(
            tool_deltas[0],
            {"index": 0, "id": expected_id, "type": "function"},
        )
        self.assertEqual(
            tool_deltas[1],
            {"index": 0, "function": {"name": expected_name}},
        )
        argument_deltas = tool_deltas[2:]
        self.assertGreaterEqual(len(argument_deltas), 2)
        self.assertTrue(
            all(set(delta["function"]) == {"arguments"} for delta in argument_deltas)
        )
        arguments = "".join(
            delta["function"]["arguments"] for delta in argument_deltas
        )
        terminal = chunks[-1]["choices"][0]
        self.assertEqual(terminal["delta"], {})
        self.assertEqual(terminal["finish_reason"], "tool_calls")
        self.assertEqual(chunks[-1]["usage"]["total_tokens"], 30)
        return {
            "id": tool_deltas[0]["id"],
            "name": tool_deltas[1]["function"]["name"],
            "arguments": json.loads(arguments),
        }

    def assert_png(self, data: bytes) -> None:
        self.assertTrue(data.startswith(b"\x89PNG\r\n\x1a\n"))
        offset = 8
        idat = bytearray()
        chunks: list[bytes] = []
        while offset < len(data):
            self.assertGreaterEqual(len(data) - offset, 12)
            length = int.from_bytes(data[offset : offset + 4], "big")
            chunk_type = data[offset + 4 : offset + 8]
            chunk_end = offset + 12 + length
            self.assertLessEqual(chunk_end, len(data))
            chunk_data = data[offset + 8 : offset + 8 + length]
            expected_crc = int.from_bytes(data[offset + 8 + length : chunk_end], "big")
            self.assertEqual(zlib.crc32(chunk_type + chunk_data) & 0xFFFFFFFF, expected_crc)
            chunks.append(chunk_type)
            if chunk_type == b"IDAT":
                idat.extend(chunk_data)
            offset = chunk_end
            if chunk_type == b"IEND":
                break
        self.assertEqual(offset, len(data))
        self.assertEqual(chunks[0], b"IHDR")
        self.assertEqual(chunks[-1], b"IEND")
        self.assertTrue(zlib.decompress(idat))

    def test_models_and_health(self) -> None:
        for path in ("/models", "/v1/models"):
            status, payload = self.request_json(path)
            self.assertEqual(status, 200)
            self.assertEqual(
                [model["id"] for model in payload["data"]],
                [
                    "a4n-virtual-text",
                    "a4n-virtual-embedding",
                    "a4n-virtual-image",
                    "a4n-virtual-video",
                ],
            )

        self.assertEqual(self.request_json("/health"), (200, {"status": "ok"}))

    def test_temporary_media_upload_transfers_original_binary(self) -> None:
        metadata = {"owner_user_id": 7, "project_id": 11, "source_media_kind": "generated_image",
                    "source_media_id": 13, "file_name": "reference.png", "mime_type": "image/png"}
        image = base64.b64decode(PNG_1X1_BASE64)
        boundary = "a4n-temp-media-test"
        body = (
            f'--{boundary}\r\nContent-Disposition: form-data; name="metadata"\r\n\r\n'.encode()
            + json.dumps(metadata).encode()
            + f'\r\n--{boundary}\r\nContent-Disposition: form-data; name="file"; filename="reference.png"\r\nContent-Type: image/png\r\n\r\n'.encode()
            + image + f'\r\n--{boundary}--\r\n'.encode()
        )
        status, response = self.request_json("/api/temp-media", body,
            f"multipart/form-data; boundary={boundary}", {"Authorization": "Bearer upload-test"})
        self.assertEqual(status, 200)
        self.assertEqual(response["byte_size"], len(image))
        self.assertEqual(response["sha256"], hashlib.sha256(image).hexdigest())
        self.assertEqual(response["source_media_id"], 13)
        self.assertNotIn("source_image_id", response)
        with self.assertRaises(HTTPError) as error:
            self.request_json("/api/temp-media", {**metadata, "data_base64": PNG_1X1_BASE64},
                              headers={"Authorization": "Bearer upload-test"})
        self.assertEqual(error.exception.code, 415)

    def test_streaming_chat_ping(self) -> None:
        request = Request(
            self.base_url + "/v1/chat/completions",
            data=json.dumps(
                {
                    "model": "a4n-virtual-text",
                    "messages": [{"role": "user", "content": "ping"}],
                    "stream": True,
                }
            ).encode("utf-8"),
            headers={"Content-Type": "application/json"},
        )
        with urlopen(request, timeout=2) as response:
            self.assertEqual(response.status, 200)
            self.assertEqual(response.headers.get_content_type(), "text/event-stream")
            events = [
                line.removeprefix("data: ")
                for line in response.read().decode("utf-8").splitlines()
                if line.startswith("data: ")
            ]
        self.assertEqual(events[-1], "[DONE]")
        chunks = [json.loads(event) for event in events[:-1]]
        self.assertEqual(
            "".join(chunk["choices"][0]["delta"].get("content", "") for chunk in chunks),
            "pong",
        )
        self.assertEqual(chunks[-1]["choices"][0]["finish_reason"], "stop")

    def test_agent_sse_locates_delegates_completes_and_summarizes(self) -> None:
        main_system = (
            "你是 A4N 项目 Agent。你是掌管项目全局的主 Agent。\n"
            "当前能力域：main\n当前目标：优化最近写作项目的最新章节"
        )
        user_message = {
            "role": "user",
            "content": "帮我优化一下最近在进行的写作项目的最新一章正文内容",
        }
        locate_chunks = self.request_sse_chunks(
            {
                "model": "a4n-virtual-text",
                "messages": [{"role": "system", "content": main_system}, user_message],
            }
        )
        locate = self.streamed_tool_call(
            locate_chunks,
            "call_a4n_locate_writing_target",
            "locate_writing_target",
        )
        self.assertEqual(locate["arguments"], {})
        self.assertTrue(any("A4N_FIXTURE_CALLS" in choice.get("delta", {}).get("content", "") for chunk in locate_chunks for choice in chunk.get("choices", [])))

        locate_assistant = {
            "role": "assistant",
            "content": "",
            "tool_calls": [
                {
                    "id": locate["id"],
                    "type": "function",
                    "function": {
                        "name": locate["name"],
                        "arguments": json.dumps(locate["arguments"], ensure_ascii=False),
                    },
                }
            ],
        }
        locate_observation = {
            "project": {"kind": "project", "id": "7", "label": "逆行时钟"},
            "chapter": {"kind": "chapter", "id": "11", "label": "雨夜来信"},
            "chapter_number": 1,
            "selection_reason": "选择最近更新的写作项目和章节",
        }
        locate_tool = {
            "role": "tool",
            "tool_call_id": locate["id"],
            "content": json.dumps(agent_observation(locate_observation), ensure_ascii=False),
        }
        delegate_chunks = self.request_sse_chunks(
            {
                "model": "a4n-virtual-text",
                "messages": [
                    {"role": "system", "content": main_system},
                    user_message,
                    locate_assistant,
                    locate_tool,
                ],
            }
        )
        delegate = self.streamed_tool_call(
            delegate_chunks,
            "call_a4n_delegate_content_optimization",
            "delegate",
        )
        self.assertEqual(delegate["arguments"]["capability"], "content_optimization")
        self.assertEqual(delegate["arguments"]["projectId"], 7)
        self.assertEqual(delegate["arguments"]["chapterId"], 11)
        self.assertTrue(delegate["arguments"]["successCriteria"])

        optimization = agent_chat_fixture({"messages": [{"role": "system", "content":
            "你是 A4N 项目 Agent。\n当前能力域：content_optimization"}]})
        self.assertEqual(optimization["tool_call"]["name"], "inspect_project_context")

        delegate_assistant = {
            "role": "assistant",
            "content": "",
            "tool_calls": [
                {
                    "id": delegate["id"],
                    "type": "function",
                    "function": {
                        "name": delegate["name"],
                        "arguments": json.dumps(delegate["arguments"], ensure_ascii=False),
                    },
                }
            ],
        }
        delivered = {"kind": "optimization_patch_set", "id": "31", "revision": "2026-09-21T12:00:00Z", "hash": "a" * 64}
        parent_messages = [
                    {"role": "system", "content": main_system},
                    user_message,
                    locate_assistant,
                    locate_tool,
                    delegate_assistant,
                    {
                        "role": "tool", "tool_call_id": delegate["id"],
                        "content": json.dumps({"delegation_id": "3", "child_run_id": "5",
                            "result_refs": [delivered]}),
                    },
                ]
        for name, call_id, arguments, result in [
            ("list_optimize_sessions", "sessions_5", {"input": {"project_id": 7, "chapter_id": 11}}, [{"id": 21}]),
            ("list_optimize_patch_sets", "call_a4n_list_optimize_patch_sets_21_5", {"input": {"session_id": 21}}, [
                {"id": 31, "updated_at": delivered["revision"], "base_content_hash": delivered["hash"], "status": "pending", "patches": [{"old_text": "旧句", "new_text": "新句"}]},
            ]),
        ]:
            chunks = self.request_sse_chunks({"model": "a4n-virtual-text", "messages": parent_messages})
            read = self.streamed_tool_call(chunks, call_id, name)
            self.assertEqual(read["arguments"], arguments)
            parent_messages.extend([
                {"role": "assistant", "content": "", "tool_calls": [{"id": call_id, "type": "function", "function": {"name": name, "arguments": json.dumps(arguments)}}]},
                {"role": "tool", "tool_call_id": call_id, "content": json.dumps(agent_observation(result))},
            ])
        chunks = self.request_sse_chunks({"model": "a4n-virtual-text", "messages": parent_messages})
        review = self.streamed_tool_call(chunks, "review_5", "review_agent_delegation")
        self.assertEqual(review["arguments"]["input"]["reviewed_refs"], [delivered])
        self.assertEqual(review["arguments"]["input"]["decision"], "accept")

        changed = json.loads(parent_messages[-1]["content"])
        changed["data"][0]["updated_at"] = "2026-09-21T13:00:00Z"
        parent_messages[-1]["content"] = json.dumps(changed)
        with self.assertRaisesRegex(ValueError, "changed versions"):
            agent_chat_fixture({"messages": parent_messages})

    def test_summary_keeps_current_tool_results_and_does_not_reuse_a_previous_turn(self) -> None:
        from specialized_agent_fixture import fixture_call, summary_fixture, summarized_observations
        task = {"role": "user", "content": "A4N_SPECIALIZED {\"mode\":\"image_generate\"}"}
        call = fixture_call("start", "start_image_generation", {})
        call["tool_call"]["id"] = "agent-call-4-0"
        source = {
            "previous_summary": None,
            "current_request": [{"message_id": 10, "content": task["content"]}],
            "covered_through_sequence": 12,
            "messages": [
                {"message_id": 11, "message": {"role": "assistant", "content": call["content"], "tool_calls": [call["tool_call"]]}},
                {"message_id": 12, "message": {"role": "tool", "tool_call_id": "agent-call-4-0", "content": json.dumps(agent_observation({"task_id": "t1", "attempt": 1}))}},
            ],
        }
        summary = json.loads(summary_fixture(source)["content"])
        messages = [{"role": "system", "content": "<context_summary>" + json.dumps(summary) + "</context_summary>"}, task, {"role": "assistant", "content": "retained batch"}]
        self.assertEqual(summarized_observations(messages, 1), {"start": {"task_id": "t1", "attempt": 1}})
        self.assertEqual(summarized_observations([*messages, task], 3), {})
        source["previous_summary"] = summary
        source["current_request"][0]["message_id"] = 20
        self.assertEqual(json.loads(summary_fixture(source)["content"])["tool_state"], [])

    def test_agent_fixture_scopes_decisions_to_latest_turn(self) -> None:
        system = {
            "role": "system",
            "content": "你是 A4N 项目 Agent。你是掌管项目全局的主 Agent。\n当前能力域：main",
        }
        historical_user = {"role": "user", "content": "上一轮任务"}
        historical_delegate = {
            "role": "assistant",
            "content": "",
            "tool_calls": [
                {
                    "id": "old-delegate",
                    "type": "function",
                    "function": {"name": "delegate", "arguments": "{}"},
                }
            ],
        }
        historical_result = {
            "role": "tool",
            "tool_call_id": "old-delegate",
            "content": json.dumps(agent_observation({"summary": "旧任务已完成"})),
        }
        historical_inbox = {
            "role": "user",
            "content": "[Agent inbox:child_result] 旧任务已完成",
        }
        current_user = {"role": "user", "content": "开始新的正文优化任务"}

        first = agent_chat_fixture(
            {"messages": [system, historical_user, historical_delegate, historical_result, historical_inbox, current_user]}
        )
        self.assertEqual(first["tool_call"]["name"], "locate_writing_target")

        locate = {
            "role": "assistant",
            "content": "",
            "tool_calls": [
                {
                    "id": "new-locate",
                    "type": "function",
                    "function": {"name": "locate_writing_target", "arguments": "{}"},
                }
            ],
        }
        observation = {
            "role": "tool",
            "tool_call_id": "new-locate",
            "content": json.dumps(agent_observation({"project": {"id": 7}, "chapter": {"id": 11}})),
        }
        second = agent_chat_fixture(
            {"messages": [system, historical_user, historical_delegate, historical_result, historical_inbox, current_user, locate, observation]}
        )
        self.assertEqual(second["tool_call"]["name"], "delegate")
        self.assertEqual(second["tool_call"]["arguments"]["projectId"], 7)

    def test_optimization_agent_uses_observed_text_and_requires_a_saved_proposal(self) -> None:
        messages = [{"role": "system", "content": "你是 A4N 项目 Agent。\n当前能力域：content_optimization"},
                    {"role": "user", "content": "优化当前章节"}]
        inspect = agent_chat_fixture({"messages": messages})["tool_call"]
        self.assertEqual(inspect["name"], "inspect_project_context")
        messages.extend([
            {"role": "assistant", "tool_calls": [{"id": inspect["id"], "function": {"name": inspect["name"]}}]},
            {"role": "tool", "tool_call_id": inspect["id"], "content": json.dumps(agent_observation({"content": {"paragraphs": [{"index": 1}]}}))},
        ])
        read = agent_chat_fixture({"messages": messages})["tool_call"]
        self.assertEqual(read["name"], "read_project_context")
        messages.extend([
            {"role": "assistant", "tool_calls": [{"id": read["id"], "function": {"name": read["name"]}}]},
            {"role": "tool", "tool_call_id": read["id"], "content": json.dumps(agent_observation({
                "content": {"paragraphs": [{"content": "林岚读完了信。"}]},
            }))},
        ])
        edit = agent_chat_fixture({"messages": messages})["tool_call"]
        self.assertEqual(edit["name"], "edit_article")
        self.assertEqual(edit["arguments"]["patches"][0]["old_text"], "林岚读完了信。")
        messages.extend([
            {"role": "assistant", "tool_calls": [{"id": edit["id"], "function": {"name": edit["name"]}}]},
            {"role": "tool", "tool_call_id": edit["id"], "content": json.dumps(agent_observation({
                "mode": "pending_confirmation", "patch_set_id": 31,
            }))},
        ])
        self.assertIn("正文尚未写回", agent_chat_fixture({"messages": messages})["content"])
        messages[-1]["content"] = json.dumps(agent_observation({"mode": "pending_confirmation"}))
        with self.assertRaisesRegex(ValueError, "persisted pending proposal"):
            agent_chat_fixture({"messages": messages})

    def test_agent_fixture_handles_context_envelope_and_opaque_call_ids(self) -> None:
        messages = [
            {"role": "system", "content": "你是 A4N 项目 Agent。\n当前能力域：general"},
            {"role": "user", "content": '[可变上下文]\n元数据\n\n[当前任务]\nA4N_GENERAL {"project_id":7}'},
        ]
        first = agent_chat_fixture({"messages": messages})
        self.assertEqual(first["tool_call"]["name"], "search_agent_tools")
        messages.extend([
            {"role": "assistant", "content": first["content"], "tool_calls": [{"id": "opaque-runtime-id", "function": {"name": "search_agent_tools", "arguments": "{}"}}]},
            {"role": "tool", "tool_call_id": "opaque-runtime-id", "content": json.dumps(agent_observation({"tools": []}))},
        ])
        second = agent_chat_fixture({"messages": messages})
        self.assertEqual(second["tool_call"]["name"], "describe_agent_tool")

    def test_content_optimization_chat_fixture_matches_current_workflow(self) -> None:
        response = json.loads(
            response_for(
                {
                    "messages": [
                        {
                            "role": "user",
                            "content": "content_optimization_chat_agent_loop",
                        }
                    ]
                }
            )
        )
        self.assertEqual(response["action"], "run_tool")
        self.assertEqual(response["arguments"]["tool"], "get_project_information")

        response = json.loads(
            response_for(
                {
                    "messages": [
                        {
                            "role": "user",
                            "content": (
                                "content_optimization_chat_agent_loop\n"
                                '{"loop_state":{"observations":[{"tool":"get_project_information",'
                                '"success":true,"result":{"content":{"source_spans":[{"id":"ch1:s1:f1","text":"原文。"}]}}}]}}'
                            ),
                        }
                    ]
                }
            )
        )
        self.assertEqual(response["action"], "run_tool")
        self.assertEqual(response["arguments"]["tool"], "edit_article")
        self.assertEqual(response["arguments"]["arguments"]["patches"][0]["old_text"], "原文。")

    def test_unmatched_prompt_is_rejected(self) -> None:
        with self.assertRaises(HTTPError) as caught:
            self.request_json(
                "/v1/chat/completions",
                {
                    "model": "a4n-virtual-text",
                    "messages": [{"role": "user", "content": "definitely-unmatched"}],
                },
            )
        error = caught.exception
        try:
            self.assertEqual(error.code, 422)
            payload = json.load(error)
        finally:
            error.close()
        self.assertEqual(payload["error"]["type"], "unhandled_prompt")
        self.assertIn("No deterministic fixture matched prompt", payload["error"]["message"])

    def test_reverse_contract_dispatch(self) -> None:
        cases = [
            (
                "reverse_blueprint",
                {"title", "world_setting", "characters", "relationships"},
            ),
            (
                "reverse_outline",
                {"outlines"},
            ),
            (
                "reverse_part_outline",
                {"parts"},
            ),
        ]
        for prompt_name, expected_keys in cases:
            _, payload = self.request_json(
                "/v1/chat/completions",
                {
                    "model": "a4n-virtual-text",
                    "messages": [
                        {
                            "role": "user",
                            "content": json.dumps(
                                {
                                    "source_context": {
                                        "total_chapters": 1,
                                        "summaries": [
                                            {"chapter_number": 1, "title": "雨夜来信"}
                                        ],
                                    },
                                    "task": {"prompt_name": prompt_name},
                                },
                                ensure_ascii=False,
                            ),
                        }
                    ],
                },
            )
            content = json.loads(payload["choices"][0]["message"]["content"])
            self.assertTrue(expected_keys.issubset(content))

    def test_shooting_design_uses_the_structured_target_shot(self) -> None:
        spans = [
            {"id": "ch1:s1:f1", "paragraph_index": 1, "text": "建立空间"},
            {"id": "ch1:s2:f1", "paragraph_index": 2, "text": "进入地下机房"},
        ]
        locked_shots = [
            {
                "segment_id": f"shot_{index}",
                "title": f"画面 {index}",
                "start_ms": (index - 1) * 1000,
                "end_ms": index * 1000,
                "frame": {"width": 1920, "height": 1080},
                "prompt": span["text"],
                "scene_binding": {"kind": "scene", "scene_profile_id": 1},
                "keyframes": [],
                "end_frame": None,
            }
            for index, span in enumerate(spans, 1)
        ]
        prompt = "\n".join(
            [
                json.dumps({"source_spans": spans}, ensure_ascii=False),
                json.dumps(
                    {
                        "locked_visual_unit": {
                            "id": "unit_1",
                            "location": "地下通道",
                            "time_of_day": "夜",
                            "participant_entity_ids": [],
                            "user_addition_ids": [],
                            "shots": locked_shots,
                        },
                        "target_shot": locked_shots[1],
                        "screenplay_plan_unit": {
                            "scene_identity": "unit_1",
                            "sequence_index": 1,
                            "narrative_unit_index": 1,
                            "timeline_position": 1,
                            "intercut_group_id": None,
                            "editing_mode": "continuity",
                            "transition_contracts": [],
                            "title": "两镜头合同",
                            "slugline": "内景 地下通道 夜",
                            "source_span_ids": [span["id"] for span in spans],
                            "participants": ["林岚"],
                            "dramatic_question": "是否继续前进？",
                            "scene_objective": "进入机房",
                            "scene_obstacle": "通道昏暗",
                            "entry_state": "停在入口",
                            "exit_state": "进入机房",
                            "performance_facts": [
                                {
                                    "fact_index": 1,
                                    "shot_id": "shot_2",
                                    "source_span_ids": ["ch1:s2:f1"],
                                    "user_addition_ids": ["director:shot_2"],
                                    "actor": "林岚",
                                    "objective": "进入机房",
                                    "obstacle": "通道昏暗",
                                    "visible_action": "向前迈步",
                                    "reaction": None,
                                    "dialogue_intent": None,
                                    "entry_state": "停在入口",
                                    "exit_state": "进入机房",
                                    "required": True,
                                }
                            ],
                        },
                    },
                    ensure_ascii=False,
                ),
            ]
        )

        response = shooting_design_response(prompt)
        self.assertEqual(
            [shot["shot_id"] for shot in response["shots"]],
            ["shot_2"],
        )
        self.assertEqual(
            [shot["source_span_ids"] for shot in response["shots"]],
            [["ch1:s2:f1"]],
        )
        self.assertEqual(
            response["shots"][0]["performance_beats"][0]["source_span_ids"],
            ["ch1:s2:f1"],
        )
        self.assertEqual(response["shots"][0]["performance_beats"][0]["actor"], "林岚")
        targeted = shooting_design_response(f"{prompt}\n只设计 target_shot_id=shot_2。响应只能包含目标镜头。")
        self.assertEqual([shot["shot_id"] for shot in targeted["shots"]], ["shot_2"])
        self.assertEqual(
            targeted["shots"][0]["performance_beats"][0]["source_span_ids"],
            ["ch1:s2:f1"],
        )

    def test_shooting_adaptation_cites_only_matching_sources_in_the_unit(self) -> None:
        context = {
            "source_spans": [
                {"id": "ch1:s1:f1", "paragraph_index": 1, "text": "走进机房"},
                {"id": "ch1:s2:f1", "paragraph_index": 2, "text": "观察钟表"},
                {"id": "ch1:s3:f1", "paragraph_index": 3, "text": "走进机房"},
            ],
            "unit": {"narrative_unit_index": 1, "participants": ["林岚"],
                     "source_span_ids": ["ch1:s1:f1", "ch1:s2:f1", "ch1:s99:f1"]},
            "locked_visual_unit": {"shots": [{"segment_id": "shot-1", "prompt": "林岚走进机房。"}]},
        }
        output = shooting_adaptation_response(json.dumps(context, ensure_ascii=False))
        self.assertEqual(output["performance_facts"][0]["source_span_ids"], ["ch1:s1:f1"])
        context["locked_visual_unit"]["shots"][0]["prompt"] = "林岚观察星空。"
        output = shooting_adaptation_response(json.dumps(context, ensure_ascii=False))
        self.assertEqual(output["performance_facts"][0]["source_span_ids"], [])
        self.assertEqual(output["performance_facts"][0]["user_addition_ids"], ["director:shot-1"])

    def test_shooting_span_ids_requires_structured_source_spans(self) -> None:
        with self.assertRaisesRegex(ValueError, "shooting script prompt contains no source spans"):
            shooting_span_ids("旧文本中出现 ch1:s1:f1，但没有 source_spans 对象")

    def test_manga_item_uses_available_source_span_contract(self) -> None:
        body = {
            "messages": [
                {
                    "role": "user",
                    "content": json.dumps(
                        {
                            "task": {"prompt_name": "manga_item_extraction"},
                            "input": {
                                "available_source_span_ids": ["ch1:s26:f2"],
                                "shooting_script": {
                                    "events": [
                                        {
                                            "beat_ref": {
                                                "scene_index": 1,
                                                "shot_index": 36,
                                                "beat_index": 1,
                                            }
                                        }
                                    ]
                                },
                            },
                        },
                        ensure_ascii=False,
                    ),
                }
            ]
        }
        response = json.loads(response_for(body))
        self.assertEqual(response["items"][0]["source_span_ids"], ["ch1:s26:f2"])
        self.assertEqual(
            response["items"][0]["first_appearance_beat_ref"],
            {"scene_index": 1, "shot_index": 36, "beat_index": 1},
        )

    def test_galgame_branch_resolves_shared_context_reference(self) -> None:
        projection = {
            "scenes": [
                {
                    "index": 1,
                    "beats": [{"shot_index": 36, "beat_index": 1}],
                }
            ]
        }
        payload = {
            "task": {"prompt_name": "galgame_branch_design"},
            "input": {
                "previous_step": {
                    "shooting_script_projection": {
                        "$ref": "system.shared_context.shooting_script_projection"
                    }
                }
            },
        }
        body = {
            "messages": [
                {
                    "role": "system",
                    "content": json.dumps(
                        {"shared_context": {"shooting_script_projection": projection}}
                    ),
                },
                {
                    "role": "user",
                    "content": "执行阶段。\n\n输入：\n"
                    + json.dumps(payload, ensure_ascii=False),
                },
            ]
        }
        response = json.loads(response_for(body))
        self.assertEqual(
            response["choices"][0]["after_beat_ref"],
            {"scene_index": 1, "shot_index": 36, "beat_index": 1},
        )

    def test_chapter_material_fixtures_use_current_source_anchors(self) -> None:
        quotes = [
            "她在旧车站下车",
            "站厅深处传来金属碰撞声",
            "修表师周衡提着工具箱从封闭通道里走出",
            "他看见信封角落的齿轮暗记",
            "林岚没有把信交给他",
            "周衡身后的影子却没有跟着移动",
            "秒针逆着刻度走了整整一圈",
            "沿着封闭通道向地下走去",
        ]
        common = "\n".join(
            [json.dumps({"id": index, "quote": quote}, ensure_ascii=False) for index, quote in enumerate(quotes, 1)]
            + [
                '{"name":"林岚","profile_id":11}',
                '{"name":"周衡","profile_id":12}',
                '{"name":"旧怀表","object_profile_id":21}',
                '{"name":"齿轮暗记","object_profile_id":22}',
            ]
        )
        responses = [
            object_discovery_response(common),
            character_discovery_response(common),
            scene_profile_candidates_response(common),
            scene_occurrence_response(common),
            scene_transition_response(common),
            scene_entity_state_response(common),
        ]
        for profile_name in ("林岚", "周衡"):
            prompt = common + f'\n{{"profile_name":"{profile_name}"}}'
            responses.extend(
                [
                    character_sync_evidence_events_response(prompt),
                    character_sync_attribute_changes_response(prompt),
                    character_sync_chapter_states_response(prompt),
                    character_sync_behaviors_response(prompt),
                ]
            )
        encoded = json.dumps(responses, ensure_ascii=False)
        self.assertNotIn("补画在墙面", encoded)
        self.assertNotIn("进入地下机房", encoded)
        self.assertNotIn('"scene_type": "hall"', encoded)

    def test_implicit_classification_uses_current_prompt_name(self) -> None:
        body = {
            "messages": [
                {
                    "role": "user",
                    "content": json.dumps(
                        {
                            "profile_name": "林岚",
                            "task": {"name": "implicit_classification"},
                        },
                        ensure_ascii=False,
                        indent=2,
                    ),
                }
            ]
        }
        self.assertEqual(json.loads(response_for(body))["classifications"], {})

        body["messages"][0]["content"] = json.dumps(
            {
                "profile_name": "林岚",
                "current_implicit_attributes": {"谨慎": "遇事先观察"},
                "task": {"name": "implicit_classification"},
            },
            ensure_ascii=False,
        )
        self.assertEqual(
            json.loads(response_for(body))["classifications"], {"谨慎": "conform"}
        )

    def test_embeddings_are_two_by_eight_and_deterministic(self) -> None:
        request = {"model": "a4n-virtual-embedding", "input": ["alpha", "beta"]}
        status, first = self.request_json("/v1/embeddings", request)
        _, second = self.request_json("/v1/embeddings", request)
        self.assertEqual(status, 200)
        self.assertEqual([row["index"] for row in first["data"]], [0, 1])
        self.assertEqual([len(row["embedding"]) for row in first["data"]], [8, 8])
        self.assertEqual(first["data"], second["data"])
        self.assertNotEqual(first["data"][0]["embedding"], first["data"][1]["embedding"])

    def test_image_generation_and_edit_return_png(self) -> None:
        _, generated = self.request_json(
            "/v1/images/generations",
            {"model": "a4n-virtual-image", "prompt": "test"},
        )
        self.assert_png(base64.b64decode(generated["data"][0]["b64_json"], validate=True))

        boundary = "a4n-provider-test"
        image = base64.b64decode(PNG_1X1_BASE64)
        multipart = (
            f"--{boundary}\r\nContent-Disposition: form-data; name=\"model\"\r\n\r\na4n-virtual-image\r\n"
            f"--{boundary}\r\nContent-Disposition: form-data; name=\"prompt\"\r\n\r\ntest edit\r\n"
            f"--{boundary}\r\nContent-Disposition: form-data; name=\"image\"; filename=\"input.png\"\r\n"
            "Content-Type: image/png\r\n\r\n"
        ).encode("ascii") + image + f"\r\n--{boundary}--\r\n".encode("ascii")
        _, edited = self.request_json(
            "/v1/images/edits",
            multipart,
            f"multipart/form-data; boundary={boundary}",
        )
        self.assert_png(base64.b64decode(edited["data"][0]["b64_json"], validate=True))

    def test_video_generation_is_async_and_idempotent(self) -> None:
        request = {"model": "a4n-virtual-video", "prompt": "test"}
        key = "a" * 64
        _, created = self.request_json(
            "/v1/videos/generations",
            request,
            headers={"Idempotency-Key": key},
        )
        self.assertEqual(created["status"], "pending")
        job_id = created["id"]

        _, replayed = self.request_json(
            "/v1/videos/generations",
            request,
            headers={"Idempotency-Key": key},
        )
        self.assertEqual(replayed["id"], job_id)

        statuses = []
        for _ in range(3):
            _, payload = self.request_json(f"/v1/videos/generations/{job_id}")
            statuses.append(payload["status"])
        self.assertEqual(statuses, ["pending", "running", "completed"])

        video = base64.b64decode(payload["data"][0]["b64_json"], validate=True)
        self.assertEqual(payload["data"][0]["format"], "mp4")
        self.assertEqual(video[4:8], b"ftyp")
        self.assertIn(b"moov", video)
        self.assertIn(b"mdat", video)

    def test_video_fixture_has_a_frame_at_thumbnail_timestamp(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            input_path = root / "input.mp4"
            output_path = root / "thumbnail.jpg"
            input_path.write_bytes(base64.b64decode(MP4_1S_BASE64, validate=True))
            result = subprocess.run(
                [
                    "ffmpeg",
                    "-hide_banner",
                    "-loglevel",
                    "error",
                    "-y",
                    "-ss",
                    "0.8",
                    "-i",
                    str(input_path),
                    "-frames:v",
                    "1",
                    "-q:v",
                    "3",
                    "-update",
                    "1",
                    str(output_path),
                ],
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertGreater(output_path.stat().st_size, 0)

    def test_video_idempotency_key_rejects_conflicting_request(self) -> None:
        key = "b" * 64
        self.request_json(
            "/v1/videos/generations",
            {"model": "a4n-virtual-video", "prompt": "first"},
            headers={"Idempotency-Key": key},
        )
        with self.assertRaises(HTTPError) as caught:
            self.request_json(
                "/v1/videos/generations",
                {"model": "a4n-virtual-video", "prompt": "second"},
                headers={"Idempotency-Key": key},
            )
        error = caught.exception
        try:
            self.assertEqual(error.code, 409)
            payload = json.load(error)
        finally:
            error.close()
        self.assertIn("Idempotency-Key", payload["error"]["message"])

    def test_video_segment_plan_uses_current_shot_ids_and_roles(self) -> None:
        payload = {
            "constraints": {"selected_duration_secs": 1},
            "current_shot": {
                "source_shot_id": "shot_1",
                "title": "画面 1",
                "subject": "旧车站通道",
                "start_state": "通道静止",
                "blocking_movement": "雨水沿玻璃滑落",
                "end_state": "冷光稳定",
                "participants": [],
                "action_beats": [],
                "spatial_anchors": [],
            },
        }
        prompt = "前置提示\n\n输入：\n" + json.dumps(payload)
        response = video_segment_plan_response(prompt)
        segment = response["segments"][0]
        self.assertEqual(segment["source_shot_id"], "shot_1")
        self.assertEqual(segment["covered_beat_ids"], [])
        self.assertEqual(segment["emotion_action_goals"], [])
        self.assertEqual(segment["performance_plan"]["action_script"], [])
        self.assertEqual(segment["performance_plan"]["time_blocks"][0]["end_sec"], 1.0)

        payload["current_shot"] = {
            **payload["current_shot"],
            "source_shot_id": "shot_7",
            "participants": ["林岚"],
            "action_beats": [
                {
                    "beat_id": "shot_7_beat_2",
                    "actor": "林岚",
                    "visible_action": "抬头确认门牌",
                }
            ],
        }
        prompt = "前置提示\n\n输入：\n" + json.dumps(payload)
        segment = video_segment_plan_response(prompt)["segments"][0]
        self.assertEqual(segment["source_shot_id"], "shot_7")
        self.assertEqual(segment["covered_beat_ids"], ["shot_7_beat_2"])
        self.assertEqual(segment["emotion_action_goals"][0]["character"], "林岚")
        self.assertEqual(segment["performance_plan"]["action_script"][0]["character"], "林岚")


if __name__ == "__main__":
    unittest.main()
