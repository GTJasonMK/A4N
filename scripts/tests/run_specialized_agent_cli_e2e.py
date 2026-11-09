#!/usr/bin/env python3
"""Exercise the Director pilot through the real CLI, Host, worker and database."""

from __future__ import annotations

import argparse
import copy
import json
import os
import re
import subprocess
import tempfile
import time
import traceback
import uuid
from pathlib import Path
from urllib.parse import unquote, urlparse

from mock_openai.protocol import PNG_1X1_BASE64
from run_agent_cli_e2e import data, thread_snapshot, wait_for_run
from run_cli_contract_e2e import RunnerError, checked, logged_in_cli, provider_log_entries, runtime_snapshot, secure_json, start_provider
from specialized_agent_fixture import MARKER, observation_data


def visual_plan():
    return {"schema_version": "chapter_visual_plan_v3", "frame": {"width": 1920, "height": 1080},
            "points": [{"id": f"point-{i}", "at_ms": i * 1000, "label": "关键帧", "image_id": None,
                        "description": "不得改动这一字段", "reference_image_ids": []} for i in range(3)],
            "segments": [{"id": f"shot-{i}", "title": f"镜头 {i}", "start_point_id": f"point-{i-1}",
                          "end_point_id": f"point-{i}", "prompt": f"原描述 {i}", "scene_binding": None,
                          "use_end_point_image": False} for i in (1, 2)]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--a4n", default=str(Path(__file__).parents[2] / "target/debug/a4n"))
    parser.add_argument("--server", required=True)
    parser.add_argument("--username", required=True)
    parser.add_argument("--password-env", default="A4N_PASSWORD")
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--provider-url")
    parser.add_argument("--provider-log", type=Path)
    parser.add_argument("--provider-script", type=Path, default=Path(__file__).with_name("mock_openai_provider.py"))
    parser.add_argument("--provider-host", default="127.0.0.1")
    parser.add_argument("--provider-port", type=int, default=0)
    parser.add_argument("--provider-start-timeout", type=float, default=10)
    parser.add_argument("--command-timeout", type=float, default=30)
    parser.add_argument("--fault-database-url-env", help="Opt in to persistence-gap injection in an a4n_agent_<hex> fixture database")
    parser.add_argument("--postgres-container", default="a4n-postgres")
    args = parser.parse_args()
    report = {"success": False, "checks": []}
    args.report.parent.mkdir(parents=True, exist_ok=True)
    provider = None
    try:
        with tempfile.TemporaryDirectory(prefix="a4n-director-e2e-", dir=args.report.parent) as tmp:
            work = Path(tmp)
            cli = logged_in_cli(args, work, timeout=args.command_timeout)
            provider, log_path = start_provider(
                args,
                args.report.parent,
                uuid.uuid4().hex,
                request_log_path=args.provider_log,
            )
            secure_json(work / "runtime.json", runtime_snapshot(provider.base_url))
            checked(cli, ["config", "runtime", "set", "--input", f"@{work / 'runtime.json'}"], "runtime")
            described = set()

            def call(name, value, failure=False, destructive=False):
                if name not in described:
                    checked(cli, ["capabilities", "describe", name], name + " contract")
                    described.add(name)
                parts = ["call", name, "--args", json.dumps({"input": value}, ensure_ascii=False)]
                if destructive:
                    parts.append("--allow-destructive")
                if failure:
                    result = cli.run(parts)
                    assert result["exit_code"] in (2, 4, 6, 8), (name, result)
                    return result
                envelope, _ = checked(cli, parts, name)
                return envelope["data"]

            project = data(checked(cli, ["project", "create", "--title", "特化 Agent 隔离验证", "--project-type", "novel_long", "--use-context"], "project")[0], "project")
            source_text = "SOURCE_NOT_REQUESTED_未请求的正文。" * 900
            chapter = data(checked(cli, ["chapter", "import", "--number", "1", "--title", "雨夜来信", "--content", source_text], "chapter")[0], "chapter")
            other_chapter = data(checked(cli, ["chapter", "import", "--number", "2", "--title", "另一章节", "--content", "独立作用域"], "second chapter")[0], "second chapter")
            compose_chapter = data(checked(cli, ["chapter", "import", "--number", "3", "--title", "门后的机房", "--content", "林岚停在旧车站通道入口，观察挂钟后推开门，冷光从地下机房涌出。" * 300], "compose chapter")[0], "compose chapter")
            root_project = project
            project = call("create_derived_project", {
                "root_project_id": root_project["id"], "derived_kind": "video",
                "title": "导演 Agent 视频验证", "description": "原生时间线编辑与来源快照隔离验证",
                "start_chapter": 1, "end_chapter": 3,
            })
            resource = {"project_id": project["id"], "source_chapter_id": chapter["id"]}
            draft = call("save_chapter_visual_plan_draft", {**resource, "authoring_mode": "collaborative", "record_id": None, "expected_revision": None, "plan": visual_plan()})
            create = ["agent", "thread", "create", "--kind", "subagent", "--capability", "director",
                      "--project-id", str(project["id"]), "--chapter-id", str(chapter["id"]),
                      "--resource-kind", "chapter", "--resource-id", str(chapter["id"])]
            restricted = data(checked(cli, create + ["--allowed-tool", "read_chapter_visual_plan"], "restricted thread")[0], "thread")
            reused = data(checked(cli, create, "reuse thread")[0], "thread")
            assert reused["scope"]["allowed_tools"] == ["read_chapter_visual_plan"]
            sibling = call("create_derived_project", {"root_project_id": root_project["id"], "derived_kind": "video", "title": "同源独立视频", "description": "同源章节绑定不共享导演会话", "start_chapter": 1, "end_chapter": 3})
            sibling_create = create.copy()
            sibling_create[sibling_create.index("--project-id") + 1] = str(sibling["id"])
            sibling_thread = data(checked(cli, sibling_create, "sibling director thread")[0], "thread")
            assert sibling_thread["id"] != restricted["id"]
            report["checks"].append("two videos sharing a source chapter keep distinct canonical Director threads")
            thread = data(checked(cli, ["agent", "thread", "update", restricted["id"], "--use-capability-tools", "--expected-revision", str(restricted["revision"])], "explicit catalog update")[0], "thread")
            thread_id = thread["id"]
            assert thread["scope"]["allowed_tools"] == []
            report.update(project_id=project["id"], chapter_id=chapter["id"], thread_id=thread_id)
            report["checks"].append("canonical reuse preserves restrictions; catalog upgrade is explicit")
            checked(cli, ["agent", "memory", "create", "thread", thread_id, "fixture", "MEMORY_NOT_REQUESTED_不得自动注入"], "memory")
            task = {"mode": "edit", "project_id": project["id"], "chapter_id": chapter["id"], "shot_id": "shot-1", "prompt": "门框遮挡，保留右侧视线空间。"}
            initial_task = MARKER + json.dumps(task, ensure_ascii=False)

            def send(value, budget=None, source=None, execution_mode=None):
                execution_mode = execution_mode or ("review_writes" if value.get("mode") == "edit" else "autonomous")
                parts = ["agent", "chat", thread_id, MARKER + json.dumps(value, ensure_ascii=False), "--execution-mode", execution_mode]
                if budget: parts += ["--budget", json.dumps(budget)]
                if source: parts += ["--editor-source", json.dumps(source)]
                return data(checked(cli, parts, "agent chat")[0], "agent chat")["run_id"]

            def wait_state(run_id, reason, target_thread_id=None):
                deadline = time.monotonic() + 60
                while time.monotonic() < deadline:
                    snapshot = thread_snapshot(cli, target_thread_id or thread_id)
                    run = next(row for row in snapshot["runs"] if row["id"] == run_id)
                    if run["status"] == "waiting" and run["waiting_reason"] == reason:
                        return snapshot
                    assert run["status"] not in {"failed", "completed", "recovery_required", "cancelled"}, run
                    time.sleep(0.1)
                raise RunnerError(f"waiting({reason}) timed out: {run_id}")

            def finish(run_id):
                result = wait_for_run(cli, run_id, 60, 0.1)
                assert result["status"] == "completed", result
                snapshot = thread_snapshot(cli, thread_id)
                secure_json(args.report.parent / f"run-{run_id}.json", snapshot)
                return snapshot

            def approve(snapshot, decision="approve"):
                approval = next(row for row in snapshot["approvals"] if row["status"] == "pending")
                checked(cli, ["agent", "approve", approval["id"], decision], "approval " + decision)
                return approval

            run_id = send(task)
            pending = wait_state(run_id, "approval")
            approval = approve(pending)
            completed = finish(run_id)
            same = next(row for row in completed["executions"] if row["id"] == approval["execution_id"])
            assert same["status"] == "succeeded" and same["result_refs"]
            assert len([row for row in completed["executions"] if row["run_id"] == run_id and row["tool_name"] == "edit_chapter_visual_plan_draft"]) == 1
            current = call("get_chapter_visual_plan", resource)["draft"]
            expected = copy.deepcopy(draft["plan"])
            expected["segments"][0]["prompt"] = task["prompt"]
            assert current["plan"] == expected
            assert current["record"]["revision"] == draft["record"]["revision"] + 1
            report["checks"].append("approval resumes the original execution once; only requested segment prompt changes")

            run_id = send(task)
            approve(wait_state(run_id, "approval"))
            finish(run_id)
            assert call("get_chapter_visual_plan", resource)["draft"]["record"]["revision"] == current["record"]["revision"]
            report["checks"].append("no-op edit preserves revision")
            autonomous = send({**task, "shot_id": "shot-2", "prompt": "自主执行草稿修改"}, execution_mode="autonomous")
            finish(autonomous)
            current = call("get_chapter_visual_plan", resource)["draft"]
            assert current["plan"]["segments"][1]["prompt"] == "自主执行草稿修改"
            advice = send({**task, "prompt": "建议模式不能写入"}, execution_mode="advice_only")
            advised = finish(advice)
            assert any(row["run_id"] == advice and row["status"] == "failed" and "advice_only" in (row["error"] or "") for row in advised["executions"])
            assert call("get_chapter_visual_plan", resource)["draft"] == current
            report["checks"].append("autonomous commits valid drafts without approval; advice-only does not write")
            batch_before = current
            batch_run = send({**task, "prompt": "原子批次描述", "batch_operations": [{"kind": "set_frame", "frame": {"width": 1280, "height": 720}}]})
            approve(wait_state(batch_run, "approval"))
            batched = finish(batch_run)
            current = call("get_chapter_visual_plan", resource)["draft"]
            assert current["record"]["revision"] == batch_before["record"]["revision"] + 1
            assert current["plan"]["frame"] == {"width": 1280, "height": 720}
            assert current["plan"]["segments"][0]["prompt"] == "原子批次描述"
            batch_executions = {row["id"] for row in batched["executions"] if row["run_id"] == batch_run}
            assert len([row for row in batched["approvals"] if row["execution_id"] in batch_executions]) == 1
            before_failed_batch = call("get_chapter_visual_plan", resource)
            failed_batch = send({**task, "prompt": "失败批次不能留下此修改", "batch_operations": [{"kind": "move_point", "point_id": "missing-point", "at_ms": 1}]})
            approve(wait_state(failed_batch, "approval"))
            failed_batch_result = finish(failed_batch)
            assert any(row["run_id"] == failed_batch and row["status"] == "failed" for row in failed_batch_result["executions"])
            assert call("get_chapter_visual_plan", resource)["draft"] == current
            failed_edits = call("read_chapter_visual_plan", {**resource, "view": {"kind": "edits", "after_sequence": before_failed_batch["edit_cursor"]}})
            assert failed_edits["projection"]["items"] == []
            report["checks"].append("one approval commits an atomic multi-operation batch; a failing operation leaves no partial plan or edit receipt")

            run_id = send({**task, "prompt": "应被拒绝"})
            approve(wait_state(run_id, "approval"), "reject")
            rejected = finish(run_id)
            assert any(row["run_id"] == run_id and row["status"] == "rejected" for row in rejected["executions"])
            assert call("get_chapter_visual_plan", resource)["draft"] == current
            report["checks"].append("rejection returns an observation without writing")

            run_id = send({**task, "prompt": "Agent 与人工修改不同字段"})
            pending = wait_state(run_id, "approval")
            change = {**resource, "record_id": current["record"]["id"], "expected_revision": current["record"]["revision"],
                      "operation": {"kind": "update_segment", "segment": {**current["plan"]["segments"][1], "title": "用户并发改名"}}}
            call("edit_chapter_visual_plan_draft", change)
            approve(pending)
            merged = finish(run_id)
            assert any(row["run_id"] == run_id and row["tool_name"] == "edit_chapter_visual_plan_draft" and row["status"] == "succeeded" for row in merged["executions"])
            current = call("get_chapter_visual_plan", resource)["draft"]
            assert current["plan"]["segments"][0]["prompt"] == "Agent 与人工修改不同字段"
            assert current["plan"]["segments"][1]["title"] == "用户并发改名"
            report["checks"].append("approved Agent edits automatically merge independent manual fields")
            run_id = send({**task, "prompt": "Agent 同字段修改"})
            pending = wait_state(run_id, "approval")
            change = {**resource, "record_id": current["record"]["id"], "expected_revision": current["record"]["revision"],
                      "operation": {"kind": "update_segment", "segment": {**current["plan"]["segments"][0], "prompt": "人工同字段修改"}}}
            manual = call("edit_chapter_visual_plan_draft", change)["visual_plan"]
            approve(pending)
            conflict = finish(run_id)
            assert any(row["run_id"] == run_id and row["status"] == "failed" and "冲突" in (row["error"] or "") for row in conflict["executions"])
            current = call("get_chapter_visual_plan", resource)["draft"]
            assert current["plan"] == manual["plan"] and current["record"]["revision"] == manual["record"]["revision"]
            task["prompt"] = current["plan"]["segments"][0]["prompt"]
            report["checks"].append("overlapping approved edits fail explicitly without overwriting manual content")

            discovery_run = send({**task, "mode": "discover"})
            discovered = finish(discovery_run)
            report["checks"].append("search is summary-only; describe activates one tool; large results are reread by handle")
            full_result = next(row for row in discovered["executions"] if row["run_id"] == discovery_run and row["tool_name"] == "get_chapter_visual_plan")
            result_task = {"mode": "read_result", "execution_id": full_result["id"]}
            history_run = send(result_task)
            history = finish(history_run)
            page = next(observation_data(row) for row in history["messages"] if row["run_id"] == history_run and row["role"] == "tool")
            assert page["text"] == source_text[2:10]
            model_result = next(row for row in discovered["executions"] if row["run_id"] == discovery_run and row["kind"] == "model_call")
            denied_run = send({**result_task, "execution_id": model_result["id"]})
            denied = finish(denied_run)
            assert any(row["run_id"] == denied_run and row["kind"] == "tool_call" and row["status"] == "failed" for row in denied["executions"])
            original_thread_id = thread_id
            other_thread = data(checked(cli, ["agent", "thread", "create", "--kind", "subagent", "--capability", "director", "--project-id", str(project["id"]), "--chapter-id", str(other_chapter["id"]), "--resource-kind", "chapter", "--resource-id", str(other_chapter["id"])], "second thread")[0], "second thread")
            thread_id = other_thread["id"]
            denied_run = send(result_task)
            denied = finish(denied_run)
            assert any(row["run_id"] == denied_run and row["status"] == "failed" and "Thread" in (row["error"] or "") for row in denied["executions"])
            thread_id = original_thread_id
            report["checks"].append("old-turn results remain readable in their thread; model executions and other threads are denied")
            run_id = send({**task, "mode": "batch"}, {"max_tool_calls": 1, "max_parallel_tools": 2})
            limited = wait_state(run_id, "budget")
            tools = [row for row in limited["executions"] if row["run_id"] == run_id and row["kind"] == "tool_call"]
            assert len(tools) == 1 and tools[0]["status"] == "succeeded"
            checked(cli, ["agent", "run", run_id, "cancel", "--thread-id", thread_id], "cancel partial batch")
            run_id = send(task, {"max_tokens": 30})
            wait_state(run_id, "budget")
            checked(cli, ["agent", "run", run_id, "pause", "--thread-id", thread_id], "pause exhausted budget")
            denied_resume = cli.run(["agent", "run", run_id, "resume", "--thread-id", thread_id])
            assert denied_resume["exit_code"] in (6, 8), denied_resume
            limited = thread_snapshot(cli, thread_id)
            assert not any(row["run_id"] == run_id and row["kind"] == "tool_call" for row in limited["executions"])
            checked(cli, ["agent", "run", run_id, "resume", "--thread-id", thread_id, "--budget", json.dumps({"max_tokens": 500000})], "extend exhausted budget")
            approve(wait_state(run_id, "approval"))
            resumed = finish(run_id)
            assert any(row["run_id"] == run_id and row["kind"] == "tool_call" for row in resumed["executions"])
            report["checks"].append("batch prefix respects budget; empty resume cannot bypass it; explicit extension continues the same Run")

            source = {"protocol_version": 2, "view_instance_id": "cli-original-view", "project_id": project["id"], "chapter_id": chapter["id"],
                      "editor_revision": "editor-v1", "scene_id": "scene-1", "shot_id": "shot-1", "keyframe_id": None, "entity_id": None}
            run_id = send({"mode": "inspect_only"}, source=source)
            waiting = wait_state(run_id, "external")
            event = next(row for row in reversed(waiting["events"]) if row.get("editor_request") and row["run_id"] == run_id)
            reply = {"run_id": run_id, "execution_id": event["editor_request"]["execution_id"], "view_instance_id": "other-view",
                     "response": {"kind": "state", "state": {"editor_revision": "editor-v1", "record_id": current["record"]["id"],
                         "record_revision": current["record"]["revision"], "dirty": False, "has_uncommitted_dialog": False}}}
            call("respond_agent_editor", reply, failure=True)
            reply["view_instance_id"] = source["view_instance_id"]
            assert call("respond_agent_editor", reply)["status"] == "succeeded"
            finish(run_id)
            call("respond_agent_editor", reply, failure=True)
            report["checks"].append("editor response is bound to the original view and execution; duplicates are rejected")
            run_id = send({"mode": "inspect_only"}, source=source)
            offline_run = wait_for_run(cli, run_id, 60, 0.1)
            offline = thread_snapshot(cli, thread_id)
            assert offline_run["status"] == "failed", offline_run
            assert any(row["run_id"] == run_id and row["status"] == "failed" and "原编辑器" in (row["error"] or "") for row in offline["executions"])
            report["checks"].append("offline editor fails explicitly and releases the worker")

            # Exercise the user-facing Director journey from a clean starter
            # plan. Every write is an Agent tool call and must pass the real
            # approval gate before the next revision is observed.
            compose_resource = {"project_id": project["id"], "source_chapter_id": compose_chapter["id"]}

            def create_scene(name):
                return call(
                    "create_scene_profile",
                    {
                        "project_id": project["id"],
                        "profile": {
                            "name": name,
                            "scene_type": "room",
                            "importance": "primary",
                            "base_description": name,
                            "visual_description": "木门、挂钟与通道纵深",
                            "spatial_layout": "通道通向门口",
                            "atmosphere": "冷静",
                            "first_seen_chapter": 2,
                            "last_seen_chapter": 2,
                            "image_prompt_seed": name,
                        },
                    },
                )["profile"]["id"]

            scene_a_id, scene_b_id = create_scene("旧车站通道"), create_scene("地下机房")
            compose_image = call(
                "upload_image",
                {
                    "project_id": project["id"],
                    "file_name": "compose-reference.png",
                    "mime_type": "image/png",
                    "data_base64": PNG_1X1_BASE64,
                    "image_type": "chapter_reference",
                    "prompt": "门把手与机房冷光参考",
                    "chapter_id": compose_chapter["id"],
                    "panel_id": None,
                    "metadata_json": None,
                },
            )
            compose_thread = data(
                checked(
                    cli,
                    [
                        "agent",
                        "thread",
                        "create",
                        "--kind",
                        "subagent",
                        "--capability",
                        "director",
                        "--project-id",
                        str(project["id"]),
                        "--chapter-id",
                        str(compose_chapter["id"]),
                        "--resource-kind",
                        "chapter",
                        "--resource-id",
                        str(compose_chapter["id"]),
                    ],
                    "compose thread",
                )[0],
                "compose thread",
            )
            compose_thread = data(
                checked(
                    cli,
                    [
                        "agent",
                        "thread",
                        "update",
                        compose_thread["id"],
                        "--use-capability-tools",
                        "--expected-revision",
                        str(compose_thread["revision"]),
                    ],
                    "compose thread catalog",
                )[0],
                "compose thread catalog",
            )
            report.update(compose_chapter_id=compose_chapter["id"], compose_thread_id=compose_thread["id"])
            compose_task = {
                "mode": "compose",
                "recover_preview_misuse": True,
                "project_id": project["id"],
                "chapter_id": compose_chapter["id"],
                "scene_a_id": scene_a_id,
                "scene_b_id": scene_b_id,
                "image_id": compose_image["id"],
            }
            compose_run = data(
                checked(
                    cli,
                    [
                        "agent",
                        "chat",
                        compose_thread["id"],
                        MARKER + json.dumps(compose_task, ensure_ascii=False),
                        "--execution-mode", "review_writes",
                        "--editor-source", json.dumps({
                            "protocol_version": 2, "view_instance_id": "offline-compose-origin",
                            "project_id": project["id"], "chapter_id": compose_chapter["id"],
                            "chapter_number": 3, "editor_revision": "initial-editor-revision",
                            "shot_id": "shot-1", "keyframe_id": "start",
                            "scene_id": None, "entity_id": None,
                        }),
                    ],
                    "compose Agent chat",
                )[0],
                "compose Agent chat",
            )["run_id"]

            def finish_with_approvals(run_id):
                deadline = time.monotonic() + 120
                while time.monotonic() < deadline:
                    snapshot = thread_snapshot(cli, compose_thread["id"])
                    run = next(row for row in snapshot["runs"] if row["id"] == run_id)
                    if run["status"] == "waiting" and run["waiting_reason"] == "approval":
                        approve(snapshot)
                        continue
                    if run["status"] == "completed":
                        return snapshot
                    assert run["status"] not in {"failed", "cancelled", "recovery_required"}, run
                    time.sleep(0.1)
                raise RunnerError("compose Agent 未在期限内完成审批链")

            composed = finish_with_approvals(compose_run)
            composed_executions = [row for row in composed["executions"] if row["run_id"] == compose_run]
            failed_previews = [row for row in composed_executions if row["tool_name"] == "preview_visual_plan_point_generation"]
            assert len(failed_previews) == 1 and failed_previews[0]["status"] == "failed"
            assert any(row["tool_name"] == "describe_agent_tool" and row["status"] == "succeeded" for row in composed_executions)
            compose_messages = [row for row in composed["messages"] if row["run_id"] == compose_run]
            result_reads = [call["arguments"]["pointer"] for row in compose_messages
                            for call in row.get("metadata", {}).get("tool_calls", [])
                            if call["name"] == "read_agent_execution_result"]
            assert result_reads[:3] == ["/draft", "/confirmed", "/starter_plan"], result_reads
            report["checks"].append("empty timeline with long source and an offline editor origin: preview misuse returns its contract, targeted reads recover starter_plan, then real draft writes succeed without an editor response")
            write_tools = [
                row
                for row in composed_executions
                if row["tool_name"] in {"save_chapter_visual_plan_draft", "edit_chapter_visual_plan_draft"}
                and row["status"] == "succeeded"
            ]
            assert len(write_tools) == 7, [row["tool_name"] for row in write_tools]
            composed_draft = call("get_chapter_visual_plan", compose_resource)["draft"]
            plan = composed_draft["plan"]
            assert composed_draft["record"]["revision"] == 7
            assert [point["id"] for point in plan["points"]] == ["start", "door", "enter", "end"]
            assert [point["at_ms"] for point in plan["points"]] == [0, 3000, 9000, 12000]
            door = next(point for point in plan["points"] if point["id"] == "door")
            assert door["image_id"] == compose_image["id"] and door["reference_image_ids"] == [compose_image["id"]]
            assert [segment["id"] for segment in plan["segments"]] == ["shot-1", "shot-2", "shot-3"]
            assert [(segment["start_point_id"], segment["end_point_id"]) for segment in plan["segments"]] == [
                ("start", "door"),
                ("door", "enter"),
                ("enter", "end"),
            ]
            assert plan["segments"][1]["scene_binding"] == {
                "kind": "transition",
                "from_scene_profile_id": scene_a_id,
                "to_scene_profile_id": scene_b_id,
            }
            broken_segment = copy.deepcopy(plan["segments"][1])
            broken_segment["scene_binding"] = {
                "kind": "transition", "from_scene_profile_id": scene_b_id, "to_scene_profile_id": scene_a_id,
            }
            broken = call("edit_chapter_visual_plan_draft", {
                **compose_resource,
                "record_id": composed_draft["record"]["id"],
                "expected_revision": composed_draft["record"]["revision"],
                "operation": {"kind": "update_segment", "segment": broken_segment},
            })["visual_plan"]
            confirm_run = data(checked(cli, [
                "agent", "chat", compose_thread["id"],
                MARKER + json.dumps({"mode": "confirm", "project_id": project["id"], "chapter_id": compose_chapter["id"]}),
                "--execution-mode", "autonomous",
            ], "confirm through Director Agent")[0], "confirm Agent")["run_id"]
            pending = wait_state(confirm_run, "approval", compose_thread["id"])
            pending_approval = next(row for row in pending["approvals"] if row["status"] == "pending")
            pending_execution = next(row for row in pending["executions"] if row["id"] == pending_approval["execution_id"])
            assert pending_execution["tool_name"] == "confirm_chapter_visual_plan"
            before_approval = call("get_chapter_visual_plan", compose_resource)
            assert before_approval["confirmed"] is None
            assert before_approval["draft"]["record"]["revision"] == broken["record"]["revision"]
            approve(pending)
            confirmation = finish_with_approvals(confirm_run)
            confirmation_executions = [row for row in confirmation["executions"] if row["run_id"] == confirm_run]
            confirms = [row for row in confirmation_executions if row["tool_name"] == "confirm_chapter_visual_plan"]
            assert [row["status"] for row in confirms] == ["failed", "succeeded"], confirms
            assert "segments[1].scene_binding" in confirms[0]["error"]
            assert "from_scene_profile_id" in confirms[0]["error"] and "to_scene_profile_id" in confirms[0]["error"]
            assert not confirms[0]["result_refs"]
            assert any(row["tool_name"] == "validate_chapter_visual_plan" and row["status"] == "succeeded" for row in confirmation_executions)
            saved_confirmation = call("get_chapter_visual_plan", compose_resource)
            assert saved_confirmation["draft"] is None
            confirmed = saved_confirmation["confirmed"]
            assert confirmed["record"]["status"] == "confirmed"
            assert confirmed["plan"] == plan, "repair must preserve every other authored field"
            assert confirmed["record"]["revision"] > broken["record"]["revision"]
            assert any(ref["kind"] == "chapter_visual_plan" and ref["id"] == str(confirmed["record"]["id"]) and ref["revision"] == str(confirmed["record"]["revision"]) for ref in confirms[1]["result_refs"])
            call("confirm_chapter_visual_plan", {**compose_resource, "record_id": confirmed["record"]["id"], "expected_revision": broken["record"]["revision"]}, failure=True, destructive=True)
            assert call("get_chapter_visual_plan", compose_resource)["confirmed"]["record"] == confirmed["record"]
            report["checks"].append("Director confirmation remains approval gated in autonomous mode; real transition errors drive an isolated repair, confirmed validation and successful confirmation with exact result refs; stale revisions do not change the confirmed plan")
            handle = call("start_chapter_shooting_script_generation", {
                **compose_resource,
                "visual_plan_id": confirmed["record"]["id"],
                "mode": "full",
                "meta": {"start_request_id": str(uuid.uuid4())},
            })
            checked(cli, ["task", "watch", handle["task_id"], "--attempt", str(handle["attempt"])], "shooting task")
            script = call("get_chapter_shooting_script", compose_resource)
            shots = [shot for scene in script["script"]["scenes"] for shot in scene["shots"]]
            assert [shot["shot_id"] for shot in shots] == ["shot-1", "shot-2", "shot-3"]
            assert [(shot["planned_end_ms"] - shot["planned_start_ms"]) / 1000 for shot in shots] == [3, 6, 3]
            report["checks"].append("Director Agent composes a starter into a persisted three-shot timeline; approvals, image binding, scene transition, confirmation and shooting-script projection are covered")

            generation_run = send({
                **task,
                "mode": "generate",
                "start_request_id": uuid.uuid4().hex,
                "prompt": "镜头缓慢推近，保留门把手停顿和冷光转场。",
            })
            generated_snapshot = finish(generation_run)
            generated_executions = [row for row in generated_snapshot["executions"] if row["run_id"] == generation_run]
            generated_tools = [
                row["tool_name"]
                for row in generated_executions
                if row["kind"] in {"tool_call", "wait"}
                and row["status"] == "succeeded"
                and row["tool_name"] != "read_agent_execution_result"
            ]
            assert generated_tools == [
                "read_chapter_visual_plan",
                "start_visual_plan_prompt_generation",
                "wait_generation_task",
                "read_visual_plan_prompt_candidate",
            ], generated_tools
            prompt_handle = next(
                observation_data(row)
                for row in generated_snapshot["messages"]
                if row["run_id"] == generation_run
                and row["role"] == "tool"
                and row["metadata"]["tool"] == "start_visual_plan_prompt_generation"
            )
            thread_id = other_thread["id"]
            cross_chapter_run = send({
                "mode": "wait_generation",
                "task_id": prompt_handle["task_id"],
                "attempt": prompt_handle["attempt"],
            })
            cross_chapter_snapshot = finish(cross_chapter_run)
            cross_chapter_execution = next(
                row
                for row in cross_chapter_snapshot["executions"]
                if row["run_id"] == cross_chapter_run
                and row["tool_name"] == "wait_generation_task"
            )
            assert cross_chapter_execution["status"] == "failed", cross_chapter_execution
            denial = next(observation_data(row) for row in cross_chapter_snapshot["messages"]
                          if (row.get("metadata") or {}).get("execution_id") == cross_chapter_execution["id"])
            assert denial["error"]["code"] == "FORBIDDEN", denial
            thread_id = original_thread_id
            report["checks"].append("a Director Agent cannot observe a completed generation task from another chapter in the same project")
            report["checks"].append("Director Agent starts an async prompt generation task, waits for its terminal result and reads a plan-bound candidate without auto-adoption")

            image_run = send({
                **task,
                "mode": "image_generate",
                "point_id": "point-1",
                "start_request_id": uuid.uuid4().hex,
                "prompt": "沿用当前构图，保留人物和冷光方向。",
            })
            image_snapshot = finish(image_run)
            image_executions = [row for row in image_snapshot["executions"] if row["run_id"] == image_run]
            image_tools = [
                row["tool_name"]
                for row in image_executions
                if row["kind"] in {"tool_call", "wait"}
                and row["status"] == "succeeded"
                and row["tool_name"] != "read_agent_execution_result"
            ]
            assert image_tools == [
                "read_chapter_visual_plan",
                "preview_visual_plan_point_generation",
                "start_image_generation",
                "wait_generation_task",
                "list_images_by_projects",
            ], image_tools
            report["checks"].append("Director Agent previews and runs image generation, waits for completion and reads the persisted point candidate without auto-binding it")

            if args.fault_database_url_env:
                database = urlparse(os.environ[args.fault_database_url_env])
                database_name = unquote(database.path.removeprefix("/"))
                assert re.fullmatch(r"a4n_agent_[0-9a-f]+", database_name), "fault injection requires the isolated fixture database"
                sql_env = os.environ.copy()
                sql_env["PGPASSWORD"] = unquote(database.password or "")

                def sql(query, **values):
                    parts = ["docker", "exec", "-i", "--env", "PGPASSWORD", args.postgres_container,
                             "psql", "-XqAt", "--set", "ON_ERROR_STOP=1", "--username", unquote(database.username), "--dbname", database_name]
                    for key, value in values.items():
                        parts += ["--set", f"{key}={value}"]
                    result = subprocess.run(parts, input=query, capture_output=True, text=True, env=sql_env, timeout=15)
                    assert result.returncode == 0, result.stderr
                    return result.stdout.strip()

                budget_run = send({"mode": "inspect_only"}, {"max_tokens": 1_000_000}, source)
                waiting_editor = wait_state(budget_run, "external")
                editor_event = next(row for row in reversed(waiting_editor["events"]) if row.get("editor_request"))
                model = next(row for row in waiting_editor["executions"] if row["run_id"] == budget_run and row["kind"] == "model_call")
                # Simulate a late usage receipt while the original view is open.
                sql("UPDATE agent_execution SET result_json = jsonb_set(result_json, '{usage}', "
                    "'{\"prompt_tokens\":1000000,\"completion_tokens\":1,\"total_tokens\":1000001}'::jsonb) "
                    "WHERE id = :'id'::bigint AND run_id = :'run_id'::bigint;", id=model["id"], run_id=budget_run)
                budget_reply = {**reply, "run_id": budget_run,
                                "execution_id": editor_event["editor_request"]["execution_id"],
                                "view_instance_id": source["view_instance_id"]}
                rejected = call("respond_agent_editor", budget_reply)
                assert rejected["status"] == "failed" and "预算" in rejected["error_message"], rejected
                wait_state(budget_run, "budget")
                checked(cli, ["agent", "run", budget_run, "cancel", "--thread-id", thread_id], "cancel exhausted editor")
                report["checks"].append("editor responses consult the durable budget ledger after a late usage receipt")

                def replay_faulted_run():
                    # Simulate a Worker reclaim after the injected persistence gap.
                    # User resume correctly rejects an unchanged exhausted budget.
                    replayed_id = sql("""
                        WITH resumed AS (
                            UPDATE agent_run SET status = 'queued', waiting_reason = NULL,
                              next_attempt_at = NOW(), updated_at = NOW()
                            WHERE id = :'run_id'::bigint AND status = 'paused'
                            RETURNING id, thread_id
                        )
                        UPDATE agent_thread SET status = 'active', updated_at = NOW()
                          FROM resumed WHERE agent_thread.id = resumed.thread_id
                          RETURNING resumed.id;
                    """, run_id=recovery_run)
                    assert replayed_id == recovery_run, replayed_id

                before_fault = call("get_chapter_visual_plan", resource)["draft"]
                recovery_run = send({**task, "prompt": "恢复验证：仅提交一次"}, {"max_model_calls": 2})
                approve(wait_state(recovery_run, "approval"))
                saved = wait_state(recovery_run, "budget")
                execution = next(row for row in saved["executions"] if row["run_id"] == recovery_run and row["tool_name"] == "edit_chapter_visual_plan_draft")
                assert execution["status"] == "succeeded"
                written = call("get_chapter_visual_plan", resource)["draft"]
                assert written["record"]["revision"] == before_fault["record"]["revision"] + 1
                observation = next(row for row in saved["messages"] if row["run_id"] == recovery_run and (row.get("metadata") or {}).get("execution_id") == execution["id"])
                call_id = observation["metadata"]["tool_call_id"]
                assistant = next(row for row in saved["messages"] if row["run_id"] == recovery_run and any(item["id"] == call_id for item in (row.get("metadata") or {}).get("tool_calls", [])))
                control = lambda action: checked(cli, ["agent", "run", recovery_run, action, "--thread-id", thread_id], "recovery " + action)
                control("pause")
                sql("""
                    BEGIN;
                    DELETE FROM agent_event WHERE run_id = :'run_id'::bigint AND message_id IN (:'assistant_id'::bigint, :'message_id'::bigint);
                    DELETE FROM agent_message WHERE run_id = :'run_id'::bigint AND id IN (:'assistant_id'::bigint, :'message_id'::bigint);
                    COMMIT;
                """, run_id=recovery_run, assistant_id=assistant["id"], message_id=observation["id"])
                replay_faulted_run()
                replayed = wait_state(recovery_run, "budget")
                replayed_message = next(row for row in replayed["messages"] if row["run_id"] == recovery_run and (row.get("metadata") or {}).get("tool_call_id") == call_id)
                assert replayed_message["metadata"] == observation["metadata"]
                assert call("get_chapter_visual_plan", resource)["draft"] == written
                assert len([row for row in replayed["executions"] if row["run_id"] == recovery_run]) == len([row for row in saved["executions"] if row["run_id"] == recovery_run])
                report["checks"].append("missing assistant and tool messages recover from original executions without another model call or write")

                control("pause")
                receipt = json.loads(sql("SELECT json_build_object('result', result_json, 'refs', result_refs_json) FROM agent_execution WHERE id = :'id'::bigint AND run_id = :'run_id'::bigint;", id=execution["id"], run_id=recovery_run))
                sql("""
                    BEGIN;
                    DELETE FROM agent_event WHERE run_id = :'run_id'::bigint AND message_id = :'message_id'::bigint;
                    DELETE FROM agent_message WHERE run_id = :'run_id'::bigint AND id = :'message_id'::bigint;
                    UPDATE agent_execution SET status = 'running', result_json = NULL, result_refs_json = NULL
                      WHERE id = :'id'::bigint AND run_id = :'run_id'::bigint;
                    COMMIT;
                """, run_id=recovery_run, message_id=replayed_message["id"], id=execution["id"])
                replay_faulted_run()
                recovered = wait_state(recovery_run, "budget")
                assert next(row for row in recovered["executions"] if row["id"] == execution["id"])["status"] == "succeeded"
                assert call("get_chapter_visual_plan", resource)["draft"] == written
                restored = json.loads(sql("SELECT json_build_object('result', result_json, 'refs', result_refs_json) FROM agent_execution WHERE id = :'id'::bigint AND run_id = :'run_id'::bigint;", id=execution["id"], run_id=recovery_run))
                assert restored == receipt
                report["checks"].append("a lost Agent execution receipt is recovered from the durable edit receipt without a second mutation")
                control("cancel")

            if log_path:
                requests = [entry["body"] for entry in provider_log_entries(log_path) if entry["body"].get("messages")]
                first = next(body for body in requests if any(message.get("content") == initial_task for message in body["messages"]))
                serialized = json.dumps(first, ensure_ascii=False)
                assert "SOURCE_NOT_REQUESTED" not in serialized and "MEMORY_NOT_REQUESTED" not in serialized
                assert "不得改动这一字段" not in serialized and "原描述 2" not in serialized
                report["checks"].append("initial model request contains no unrequested source, plan or memory")
            else:
                raise RunnerError("provider request log is required to verify on-demand context")
            report["success"] = True
    except (AssertionError, RunnerError, OSError, ValueError, KeyError, StopIteration, subprocess.SubprocessError) as error:
        report["error"] = str(error) or type(error).__name__
        report["traceback"] = traceback.format_exc()
    finally:
        if provider: provider.stop()
    secure_json(args.report, report)
    print(json.dumps(report, ensure_ascii=False))
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
