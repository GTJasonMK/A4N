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
    UnhandledPromptError,
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


def optimization_messages(observations, user_message="请提出局部补丁"):
    task = {"task": {"name": "content_optimization_chat_agent_loop"}, "user_message": user_message}
    base = {"kind": "base_task", "base_task": "任务\n\n输入：\n" + json.dumps(task, ensure_ascii=False)}
    return [
        {"role": "user", "content": json.dumps(base, ensure_ascii=False)},
        *[{"role": "user", "content": json.dumps({"loop_observation": {"sequence": index, "value": value}}, ensure_ascii=False)}
          for index, value in enumerate(observations, 1)],
        {"role": "user", "content": json.dumps({"loop_state": {"observations_ref": "previous messages", "progress": {"observation_count": len(observations)}}})},
    ]


class MockOpenAIProviderTest(unittest.TestCase):
    def test_anchor_faults_do_not_mutate_other_contracts_with_the_same_root_name(self):
        from mock_openai.fixtures.story import chapter_body_response
        content = json.dumps({"content":"#0 " + chapter_body_response()}, ensure_ascii=False)
        content += "\n\n输入：\n" + json.dumps({"task":{"name":"chapter_fact_extraction"}})
        body = {"model":"a4n-virtual-text","messages":[{"role":"user","content":content}]}
        expected = response_for(body)
        for model in ("a4n-output-recover-string_ids", "a4n-output-recover-missing_anchor", "a4n-output-recover-unknown_anchor", "a4n-anchor-fail-characters"):
            self.assertEqual(response_for({**body,"model":model}), expected)
        body["messages"][0]["content"] = "A4N_AGENT_CANVAS_INPUT"
        self.assertEqual(response_for({**body,"model":"a4n-output-recover-string_ids"}), response_for(body))

    def test_output_correction_fixtures_require_feedback_and_keep_task_payload_strict(self):
        from mock_openai.prompt_parsing import json_task_payload

        common = '\n'.join([
            json.dumps({"id":1,"quote":"林岚没有把信交给他"},ensure_ascii=False),
            json.dumps({"id":2,"quote":"修表师周衡提着工具箱从封闭通道里走出"},ensure_ascii=False),
            "\n\n输入：\n" + json.dumps({"task":{"name":"character_discovery"}}),
        ])
        messages = [{"role":"user","content":common}]
        for fault in ("string_ids", "missing_anchor", "unknown_anchor"):
            body = {"model":"a4n-output-recover-" + fault,"messages":messages}
            invalid = json.loads(response_for(body))["characters"][0]
            self.assertTrue("evidence_anchor_id" not in invalid or invalid["evidence_anchor_id"] != 1)
            feedback = {"kind":"model_output_correction","validation_error":"invalid id"}
            corrected = {**body,"messages":[*messages,{"role":"user","content":json.dumps(feedback)}]}
            self.assertEqual(json.loads(response_for(corrected))["characters"][0]["evidence_anchor_id"], 1)
            self.assertEqual(json_task_payload(common + '\n' + json.dumps(feedback))["task"]["name"], "character_discovery")
        with self.assertRaises(ValueError):
            json_task_payload(common + '\n{}')
        with self.assertRaisesRegex(ValueError, "unknown recovery fixture"):
            response_for({"model":"a4n-output-recover-typo","messages":messages})

    def test_fact_feedback_reads_current_annotated_and_structured_sources(self) -> None:
        from mock_openai.fixtures.fact_feedback import fact_feedback_response
        from mock_openai.fixtures.story import chapter_body_response

        annotated = json.dumps({"content": "#0 记忆城每年有七个季节。"}, ensure_ascii=False)
        result = fact_feedback_response("chapter_fact_extraction\n" + annotated)
        self.assertEqual(result["world_facts"][0]["source_span_ids"], ["#0"])
        structured = json.dumps({"id": "ch2:s1:f1", "text": "#1 记忆城每年有六个季节。"}, ensure_ascii=False)
        result = fact_feedback_response("chapter_fact_extraction\n" + structured)
        self.assertEqual(result["world_facts"][0]["source_span_ids"], ["#1"])
        self.assertGreater(len(chapter_body_response()), 800)

    def test_creation_journey_exercises_all_inspiration_controls(self) -> None:
        messages = [{"role": "system", "content": "小说概念大师·文思"}]
        for round_number, kind in enumerate(("inspired_options", "text_input", "info_display"), 1):
            messages.append({"role": "user", "content": "继续"})
            response = response_for({"model": "a4n-creation-journey", "messages": messages})
            value = json.loads(response)
            self.assertEqual(value["ui_control"]["type"], kind)
            self.assertEqual(value["is_complete"], round_number == 3)
            messages.append({"role": "assistant", "content": response})

    def test_slow_lifecycle_fixture_echoes_tauri_flow_marker(self) -> None:
        prompt = "A4N_TAURI_GENERATION_INPUT:123"
        marker = "A4N_TAURI_GENERATION_REPLY:123"
        _, response = self.request_json(
            "/v1/chat/completions",
            {
                "model": "a4n-llm-lifecycle-slow",
                "messages": [
                    {"role": "system", "content": "小说概念大师·文思"},
                    {"role": "user", "content": prompt},
                ],
            },
        )
        inspiration = json.loads(response["choices"][0]["message"]["content"])
        self.assertTrue(inspiration["ai_message"].startswith(marker))
        self.assertEqual(inspiration["ui_control"]["type"], "inspired_options")

        _, response = self.request_json(
            "/v1/chat/completions",
            {
                "model": "a4n-llm-lifecycle-slow",
                "messages": [{"role": "user", "content": prompt}],
            },
        )
        self.assertTrue(response["choices"][0]["message"]["content"].startswith(marker))

    def test_blueprint_refinement_is_not_routed_by_inspiration_history(self) -> None:
        from mock_openai.fixtures.story import inspiration_response

        result = json.loads(response_for({"messages": [
            {"role": "system", "content": "蓝图优化，只输出 full_synopsis 等允许字段。"},
            {"role": "user", "content": json.dumps({"inspiration_conversation": inspiration_response()})},
        ]}))
        self.assertIn("full_synopsis", result)
        self.assertNotIn("conversation_state", result)

    def test_task_routing_does_not_depend_on_json_whitespace(self) -> None:
        from mock_openai.prompt_parsing import current_task_prompt_name

        for field in ("name", "prompt_name"):
            for separators in ((",", ":"), (", ", ": ")):
                payload = json.dumps({"task": {field: "chapter_scene_generation"}}, separators=separators)
                self.assertEqual(current_task_prompt_name(payload), "chapter_scene_generation")
                result = response_for({"messages": [{"role": "user", "content": payload}]})
                self.assertIn("林岚", result)

    def test_source_markers_are_read_from_actual_context_content(self) -> None:
        from mock_openai.fixtures.entities import source_span_pairs, text_anchor_id

        source = json.dumps({"content": "“#0 别走！”她说。\n#1 门关上了。"}, ensure_ascii=False)
        self.assertEqual([identifier for identifier, _ in source_span_pairs(source)], ["#0", "#1"])
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
        request = {"messages": optimization_messages([observation])}
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
        with error.exception as response:
            self.assertEqual(response.code, 415)

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

    def test_director_bootstrap_recovers_contract_and_reads_large_plan_by_pointer(self) -> None:
        from specialized_agent_fixture import MARKER, director_fixture

        task = {"mode": "compose", "recover_preview_misuse": True, "project_id": 18, "chapter_id": 9}
        messages = [{"role": "user", "content": MARKER + json.dumps(task)}]
        body = {"tools": [{"function": {"name": name}} for name in
                          ("get_chapter_visual_plan", "save_chapter_visual_plan_draft")]}

        def observe(decision, observation):
            call = decision["tool_call"]
            messages.extend([
                {"role": "assistant", "content": decision["content"], "tool_calls": [call]},
                {"role": "tool", "tool_call_id": call["id"], "content": json.dumps(observation)},
            ])

        bad = director_fixture(body, messages)
        self.assertEqual(bad["tool_call"]["name"], "preview_visual_plan_point_generation")
        error = agent_observation(None)
        error.update(status="error", error={
            "code": "INVALID_ARGUMENT", "next_action": "describe_tool_then_fix_arguments",
            "details": {"contract_tool": {"name": "describe_agent_tool", "arguments": {"name": bad["tool_call"]["name"]}}},
        })
        observe(bad, error)
        contract = director_fixture(body, messages)
        self.assertEqual(contract["tool_call"]["name"], "describe_agent_tool")
        observe(contract, agent_observation({"name": "preview_visual_plan_point_generation"}))
        plan = director_fixture(body, messages)
        self.assertEqual(plan["tool_call"]["name"], "get_chapter_visual_plan")
        handle = {"complete": False, "execution_id": "38", "read_tool": "read_agent_execution_result",
                  "available_pointers": ["/draft", "/confirmed", "/starter_plan"]}
        observation = agent_observation(handle)
        observation["status"] = "partial"
        observe(plan, observation)
        starter = {"points": [{"id": "start"}, {"id": "end"}], "segments": [{"id": "shot-1"}]}
        for field, value in (("draft", None), ("confirmed", None), ("starter_plan", starter)):
            decision = director_fixture(body, messages)
            self.assertEqual(decision["tool_call"]["name"], "read_agent_execution_result")
            self.assertEqual(decision["tool_call"]["arguments"]["pointer"], "/" + field)
            observe(decision, agent_observation({"text": json.dumps(value), "complete": True}))
        save = director_fixture(body, messages)["tool_call"]
        self.assertEqual(save["name"], "save_chapter_visual_plan_draft")
        self.assertEqual(save["arguments"]["input"]["plan"], starter)
        self.assertIsNone(save["arguments"]["input"]["record_id"])
        self.assertIsNone(save["arguments"]["input"]["expected_revision"])

    def test_director_confirmation_repairs_observed_errors_before_confirming(self) -> None:
        from specialized_agent_fixture import MARKER, director_fixture

        messages = [{"role": "user", "content": MARKER + json.dumps({"mode": "confirm", "project_id": 18, "chapter_id": 9})}]
        body = {"tools": []}

        def observe(decision, observation):
            call = decision["tool_call"]
            messages.extend([
                {"role": "assistant", "content": decision["content"], "tool_calls": [call]},
                {"role": "tool", "tool_call_id": call["id"], "content": json.dumps(observation)},
            ])

        outline = director_fixture(body, messages)
        self.assertEqual(outline["tool_call"]["name"], "read_chapter_visual_plan")
        observe(outline, agent_observation({"resource": {"id": "12", "revision": "3"}, "projection": {"items": [
            {"segment_id": "before", "scene_binding": {"kind": "scene", "scene_profile_id": 11}},
            {"segment_id": "transition", "scene_binding": {"kind": "transition", "from_scene_profile_id": 22, "to_scene_profile_id": 11}},
            {"segment_id": "after", "scene_binding": {"kind": "scene", "scene_profile_id": 22}},
        ]}}))
        confirm = director_fixture(body, messages)
        self.assertEqual(confirm["tool_call"]["name"], "confirm_chapter_visual_plan")
        error = agent_observation(None)
        error.update(status="error", error={"code": "INVALID_ARGUMENT", "retryable": False, "next_action": "describe_tool_then_fix_arguments", "details": {
            "message": "segments[1].scene_binding transition: from_scene_profile_id 应为 11; to_scene_profile_id 应为 22",
        }})
        observe(confirm, error)
        read = director_fixture(body, messages)
        self.assertEqual(read["tool_call"]["arguments"]["input"]["view"]["segment_id"], "transition")
        original = {"id": "transition", "prompt": "保留原有镜头", "scene_binding": {"kind": "transition", "from_scene_profile_id": 22, "to_scene_profile_id": 11}}
        observe(read, agent_observation({"projection": {"segment": original}}))
        repair = director_fixture(body, messages)
        self.assertEqual(repair["tool_call"]["name"], "edit_chapter_visual_plan_draft")
        repaired_segment = repair["tool_call"]["arguments"]["input"]["operation"]["segment"]
        self.assertEqual(repaired_segment, {**original, "scene_binding": {"kind": "transition", "from_scene_profile_id": 11, "to_scene_profile_id": 22}})
        observe(repair, agent_observation({"resource": {"id": "12", "revision": "4"}}))
        validate = director_fixture(body, messages)
        self.assertEqual(validate["tool_call"]["name"], "validate_chapter_visual_plan")
        self.assertEqual(validate["tool_call"]["arguments"]["input"]["mode"], "confirmed")
        observe(validate, agent_observation({"validation": {"valid": False}}))
        with self.assertRaisesRegex(ValueError, "still fails"):
            director_fixture(body, messages)
        messages[-1]["content"] = json.dumps(agent_observation({"validation": {"valid": True}}))
        final_confirm = director_fixture(body, messages)
        self.assertEqual(final_confirm["tool_call"]["arguments"]["input"]["expected_revision"], 4)
        observe(final_confirm, agent_observation({"visual_plan": {"record": {"status": "draft"}}}))
        with self.assertRaisesRegex(ValueError, "confirmed record"):
            director_fixture(body, messages)
        messages[-1]["content"] = json.dumps(agent_observation({"visual_plan": {"record": {"status": "confirmed"}}}))
        self.assertNotIn("tool_call", director_fixture(body, messages))

    def test_director_summary_keeps_saved_plan_and_revision_without_duplicated_receipts(self) -> None:
        from specialized_agent_fixture import fixture_call, summary_fixture, SUMMARY_STATE_MARKER

        call = fixture_call("edit", "edit_chapter_visual_plan_draft", {})
        plan = {"points": [{"id": "door", "at_ms": 3000}], "segments": [{"id": "shot-1"}]}
        resource = {"id": "12", "revision": "7", "kind": "chapter_visual_plan"}
        changes = [{"before": "旧描述" * 1000, "after": "新描述" * 1000}]
        saved = {"resource": resource, "changed": True, "changes": changes,
                 "visual_plan": {"plan": plan, "edit_receipt": {"changes": changes}}}
        summary = json.loads(summary_fixture({
            "previous_summary": None, "current_request": [{"message_id": 10, "content": "编排本章"}],
            "covered_through_sequence": 12,
            "messages": [
                {"message_id": 11, "message": {"role": "assistant", "content": call["content"], "tool_calls": [call["tool_call"]]}},
                {"message_id": 12, "message": {"role": "tool", "tool_call_id": "edit", "content": json.dumps(agent_observation(saved))}},
            ],
        })["content"])
        self.assertLess(len(summary["tool_state"][0]), 4000)
        state = json.loads(summary["tool_state"][0][len(SUMMARY_STATE_MARKER):])["data"]
        self.assertEqual(state, {"resource": resource, "changed": True, "visual_plan": {"plan": plan}})

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
            {"messages": [system, historical_user, historical_delegate, historical_result, historical_inbox, current_user, locate, observation,
                          {"role": "user", "content": "当前执行阶段：context_read。当前目标：开始新的正文优化任务。"}]}
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
        read = agent_chat_fixture({"messages": [*messages,
            {"role": "user", "content": "当前执行阶段：context_read。当前目标：优化当前章节。"},
        ]})["tool_call"]
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
        response = json.loads(response_for({"messages": optimization_messages([])}))
        self.assertEqual(response["action"], "run_tool")
        self.assertEqual(response["arguments"]["tool"], "get_project_information")
        plain = optimization_messages([], "A4N_OPTIMIZATION_PLAIN 只回答当前问题")
        self.assertEqual(json.loads(response_for({"messages": plain}))["action"], "final")
        plain[-1]["content"] = json.dumps({"loop_state": {"observations": []}})
        with self.assertRaisesRegex(ValueError, "observation references"):
            response_for({"messages": plain})
        missing = optimization_messages([])
        missing[-1]["content"] = json.dumps({"loop_state": {"observations_ref": "previous messages", "progress": {"observation_count": 1}}})
        with self.assertRaisesRegex(ValueError, "lost full observations"):
            response_for({"messages": missing})

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
            output_contract = {"name": prompt_name, "schema": {"type": "object", "required": sorted(expected_keys)}}
            _, payload = self.request_json(
                "/v1/chat/completions",
                {
                    "model": "a4n-virtual-text",
                    "messages": [
                        {"role": "system", "content": "[A4N_OUTPUT_CONTRACT]\n" + json.dumps(output_contract) + "\n[/A4N_OUTPUT_CONTRACT]"},
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
            "source": "#0 走进机房\n#1 观察钟表\n#2 走进机房",
            "unit": {"narrative_unit_index": 1, "participants": ["林岚"],
                     "source_span_ids": ["#0", "#1", "#99"]},
            "locked_visual_unit": {"shots": [{"segment_id": "shot-1", "prompt": "林岚走进机房。"}]},
        }
        output = shooting_adaptation_response(json.dumps(context, ensure_ascii=False))
        self.assertEqual(output["performance_facts"][0]["source_span_ids"], ["#0"])
        context["locked_visual_unit"]["shots"][0]["prompt"] = "林岚观察星空。"
        output = shooting_adaptation_response(json.dumps(context, ensure_ascii=False))
        self.assertEqual(output["performance_facts"][0]["source_span_ids"], [])
        self.assertEqual(output["performance_facts"][0]["user_addition_ids"], ["director:shot-1"])

    def test_shooting_span_ids_requires_structured_source_spans(self) -> None:
        with self.assertRaisesRegex(ValueError, "shooting script prompt contains no source spans"):
            shooting_span_ids("旧文本中出现 ch1:s1:f1，但没有 source_spans 对象")

    def test_manga_preprocessing_uses_current_contract_and_source_ids(self) -> None:
        response = {}
        for step, fields in {
            "facts": {"source_events", "characters"}, "speech": {"speech"},
            "visuals": {"visual_anchors", "items"}, "synthesis": {"comic_synthesis"},
        }.items():
            body = {"messages": [{"role": "user", "content": json.dumps({
                "task": {"prompt_name": "manga_preprocessing_" + step},
                "input": {"source": "#0 林岚在旧车站检查怀表。"},
            }, ensure_ascii=False)}]}
            partial = json.loads(response_for(body))
            self.assertEqual(set(partial), fields)
            response.update(partial)
        self.assertEqual(
            response["source_events"]["events"][0]["source_span_ids"],
            ["#0"],
        )
        self.assertEqual(
            response["visual_anchors"]["scenes"][0]["source_event_ids"],
            ["event-1"],
        )

    def test_preprocessing_faults_are_scoped_to_speech_and_recovery_attempt(self) -> None:
        messages = [{"role": "user", "content": json.dumps({
            "task": {"prompt_name": "manga_preprocessing_speech"},
            "input": {"source": "#0 林岚检查怀表。"},
        }, ensure_ascii=False)}]
        normal = json.loads(response_for({"messages": messages}))
        correction = {"role": "user", "content": json.dumps({"kind": "model_output_correction"})}
        for model, recovers in (("a4n-preprocessing-fail-speech", False), ("a4n-preprocessing-recover-speech", True)):
            initial = json.loads(response_for({"model": model, "messages": messages}))
            self.assertNotEqual(initial, normal)
            corrected = json.loads(response_for({"model": model, "messages": [*messages, correction]}))
            self.assertEqual(corrected, normal if recovers else initial)
            facts_messages = [{"role": "user", "content": messages[0]["content"].replace("manga_preprocessing_speech", "manga_preprocessing_facts")}]
            self.assertEqual(response_for({"model": model, "messages": facts_messages}), response_for({"messages": facts_messages}))

    def test_manga_reference_discovery_only_selects_available_candidates(self) -> None:
        def request(page_number: int | None, assets: list[dict[str, object]] | None) -> str:
            stable_input: dict[str, object] = {}
            if assets is not None:
                stable_input["available_assets"] = assets
            current_task: dict[str, object] = {
                "task": {"prompt_name": "manga_page_reference_discovery"},
                "input": {"current_page": {"page_number": page_number}},
            }
            if page_number is not None:
                current_task["output_contract"] = {"page_number": page_number}
            return (
                "参考素材发现请求\n\n稳定上下文：\n"
                + json.dumps({"input": stable_input}, ensure_ascii=False)
                + "\n\n输入：\n"
                + json.dumps(current_task, ensure_ascii=False)
            )

        body = {"messages": [{
            "role": "user",
            "content": request(3, [{
                "asset_kind": "generated_image",
                "asset_id": 9,
                "description": "旧怀表的黄铜外壳和磨损刻度。",
            }]),
        }]}
        response = json.loads(response_for(body))
        self.assertEqual(response["page_number"], 3)
        self.assertEqual(response["requirements"], [])
        self.assertEqual(response["references"][0]["asset_id"], 9)

        body["messages"][0]["content"] = request(3, [])
        empty_response = json.loads(response_for(body))
        self.assertEqual(empty_response["references"], [])

        for page_number, assets in ((None, []), (3, None)):
            body["messages"][0]["content"] = request(page_number, assets)
            with self.subTest(page_number=page_number, assets=assets), self.assertRaises(UnhandledPromptError):
                response_for(body)

    def test_manga_page_pipeline_responses_follow_each_page_contract(self) -> None:
        beat_ref = {"unit_id": "station", "beat_id": "evidence"}
        slots_request = {
            "messages": [{
                "role": "user",
                "content": json.dumps({
                    "task": {"prompt_name": "manga_page_slots"},
                    "input": {
                        "source": "#0 林岚在通道检查怀表。",
                        "adaptation_plan": {
                            "page_count": 2,
                            "units": [{"id": beat_ref["unit_id"], "beats": [{"id": beat_ref["beat_id"]}]}],
                        },
                    },
                }, ensure_ascii=False) + "\n示例引用：\n" + json.dumps({
                    "unit_id": "unit-1", "beat_id": "beat-1",
                }),
            }],
        }
        slots = json.loads(response_for(slots_request))
        self.assertEqual(slots["schema_version"], "manga_page_slots_v1")
        self.assertEqual(len(slots["pages"]), 2)
        self.assertEqual(slots["pages"][0]["content"]["source_span_ids"], ["#0"])
        self.assertEqual(slots["pages"][1]["content"]["source_span_ids"], ["#0"])
        self.assertEqual(slots["pages"][0]["content"]["beat_refs"], [beat_ref])
        self.assertEqual(
            slots["pages"][1]["content"]["entry_state"],
            slots["pages"][0]["content"]["exit_state"],
        )

        content = slots["pages"][1]["content"]
        adaptation = json.loads(response_for({"messages": [{
            "role": "user",
            "content": json.dumps({
                "task": {"prompt_name": "manga_page_adaptation_plan"},
                "input": {"page_number": 2, "page_content": content},
            }, ensure_ascii=False),
        }]}))
        self.assertEqual(adaptation["beat_refs"], content["beat_refs"])

        storyboard = json.loads(response_for({"messages": [{
            "role": "user",
            "content": json.dumps({
                "task": {"prompt_name": "manga_page_storyboard"},
                "input": {"page_number": 2, "page_content": content, "page_adaptation": adaptation,
                          "source_context": {"source": "#0 林岚停下脚步。", "speech": {"dialogues": [], "narrations": []}}},
            }, ensure_ascii=False),
        }]}))
        self.assertEqual(storyboard["page_number"], 2)
        self.assertEqual(storyboard["beat_refs"], adaptation["beat_refs"])
        self.assertEqual(storyboard["entry_state"], content["entry_state"])

    def test_galgame_branch_resolves_shared_context_reference(self) -> None:
        projection = {
            "scenes": [
                {
                    "index": 1,
                    "source_chapter_id": 11, "unit_id": "station",
                    "beats": [{"id": "evidence"}],
                },
                {
                    "index": 2,
                    "source_chapter_id": 11, "unit_id": "machine-room",
                    "beats": [{"id": "machine-room-reveal"}],
                },
            ]
        }
        payload = {
            "task": {"prompt_name": "galgame_branch_design"},
            "input": {
                "previous_step": {
                    "adaptation_projection": {
                        "$ref": "system.shared_context.adaptation_projection"
                    }
                }
            },
        }
        body = {
            "messages": [
                {
                    "role": "system",
                    "content": json.dumps(
                        {"shared_context": {"adaptation_projection": projection}}
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
            {"source_chapter_id": 11, "unit_id": "station", "beat_id": "evidence"},
        )
        for option in response["choices"][0]["options"]:
            self.assertEqual(option["returns_to"], {"source_chapter_id": 11, "unit_id": "machine-room"})
        projection["scenes"] = projection["scenes"][:1]
        body["messages"][0]["content"] = json.dumps({"shared_context": {"adaptation_projection": projection}})
        self.assertEqual(json.loads(response_for(body)), {"choices": []})

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
        for prompt, root, field in [
            ("object_discovery", "objects", "evidence_anchor_ids"),
            ("character_discovery", "characters", "evidence_anchor_id"),
            ("scene_profile_candidates", "scenes", "evidence_anchor_ids"),
            ("scene_occurrence_extraction", "occurrences", "source_anchor_ids"),
            ("scene_transition_extraction", "transitions", "source_anchor_ids"),
            ("scene_entity_state_extraction", "entity_states", "source_anchor_ids"),
        ]:
            body = {"model": "a4n-virtual-text", "messages": [{"role": "user", "content": common + json.dumps({"task": {"name": prompt}})}]}
            valid = json.loads(response_for(body))[root][0][field]
            invalid = json.loads(response_for({**body, "model": "a4n-anchor-fail-" + root}))[root][0][field]
            self.assertEqual(invalid, [str(anchor) for anchor in valid] if isinstance(valid, list) else str(valid))
            self.assertEqual(json.loads(response_for({**body, "model": "a4n-anchor-fail-events"}))[root][0][field], valid)
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
        scene_body = {"messages": [{"role": "user", "content": common + json.dumps({"task": {"name": "scene_occurrence_extraction"}})}]}
        feedback = {"role": "user", "content": json.dumps({"kind": "model_output_correction"})}
        for model, field in (("missing-entry", "entry_state"), ("missing-exit", "exit_state"), ("recover-exit", "exit_state")):
            body = {**scene_body, "model": "a4n-scene-sync-" + model}
            self.assertNotIn(field, json.loads(response_for(body))["occurrences"][0])
            corrected = json.loads(response_for({**body, "messages": [*body["messages"], feedback]}))["occurrences"][0]
            self.assertEqual(field in corrected, model == "recover-exit")
            if field in corrected:
                self.assertEqual(corrected[field], {})
        with self.assertRaisesRegex(ValueError, "unknown anchor failure fixture"):
            response_for({"model": "a4n-anchor-fail-typo", "messages": [{"role": "user", "content": common + json.dumps({"task": {"name": "character_discovery"}})}]})
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
