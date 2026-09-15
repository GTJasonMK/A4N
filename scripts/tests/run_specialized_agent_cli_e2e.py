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
import uuid
from pathlib import Path
from urllib.parse import unquote, urlparse

from mock_openai_provider import PNG_1X1_BASE64
from run_agent_cli_e2e import data, thread_snapshot, wait_for_run
from run_cli_contract_e2e import Cli, RunnerError, checked, provider_log_entries, runtime_snapshot, secure_json, start_provider
from specialized_agent_fixture import MARKER


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
        with tempfile.TemporaryDirectory(prefix="a4n-director-e2e-") as tmp:
            work = Path(tmp)
            env = os.environ.copy()
            password = env.pop(args.password_env)
            env.pop("A4N_ACCESS_TOKEN", None)
            cli = Cli(args.a4n, args.server, work / "cli.json", work / "context.json", args.command_timeout, env)
            checked(cli, ["health"], "health")
            checked(cli, ["capabilities", "check"], "capabilities check")
            checked(cli, ["auth", "login", "--username", args.username, "--password-stdin"], "login", password + "\n")
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
                    assert result["exit_code"] != 0, name + " unexpectedly succeeded"
                    return result
                envelope, _ = checked(cli, parts, name)
                return envelope["data"]

            project = data(checked(cli, ["project", "create", "--title", "特化 Agent 隔离验证", "--project-type", "novel_long", "--use-context"], "project")[0], "project")
            source_text = "SOURCE_NOT_REQUESTED_未请求的正文。" * 900
            chapter = data(checked(cli, ["chapter", "import", "--number", "1", "--title", "雨夜来信", "--content", source_text], "chapter")[0], "chapter")
            resource = {"project_id": project["id"], "source_chapter_id": chapter["id"]}
            draft = call("save_chapter_visual_plan_draft", {**resource, "authoring_mode": "collaborative", "record_id": None, "expected_revision": None, "plan": visual_plan()})
            create = ["agent", "thread", "create", "--kind", "subagent", "--capability", "director",
                      "--project-id", str(project["id"]), "--chapter-id", str(chapter["id"]),
                      "--resource-kind", "chapter", "--resource-id", str(chapter["id"])]
            restricted = data(checked(cli, create + ["--allowed-tool", "read_chapter_visual_plan"], "restricted thread")[0], "thread")
            reused = data(checked(cli, create, "reuse thread")[0], "thread")
            assert reused["scope"]["allowed_tools"] == ["read_chapter_visual_plan"]
            thread = data(checked(cli, ["agent", "thread", "update", restricted["id"], "--use-capability-tools", "--expected-revision", str(restricted["revision"])], "explicit catalog update")[0], "thread")
            thread_id = thread["id"]
            assert thread["scope"]["allowed_tools"] == []
            report.update(project_id=project["id"], chapter_id=chapter["id"], thread_id=thread_id)
            report["checks"].append("canonical reuse preserves restrictions; catalog upgrade is explicit")
            checked(cli, ["agent", "memory", "create", "thread", thread_id, "fixture", "MEMORY_NOT_REQUESTED_不得自动注入"], "memory")
            task = {"mode": "edit", "project_id": project["id"], "chapter_id": chapter["id"], "shot_id": "shot-1", "prompt": "门框遮挡，保留右侧视线空间。"}

            def send(value, budget=None, source=None):
                parts = ["agent", "chat", thread_id, MARKER + json.dumps(value, ensure_ascii=False)]
                if budget: parts += ["--budget", json.dumps(budget)]
                if source: parts += ["--editor-source", json.dumps(source)]
                return data(checked(cli, parts, "agent chat")[0], "agent chat")["run_id"]

            def wait_state(run_id, reason):
                deadline = time.monotonic() + 60
                while time.monotonic() < deadline:
                    snapshot = thread_snapshot(cli, thread_id)
                    run = next(row for row in snapshot["runs"] if row["id"] == run_id)
                    if run["status"] == "waiting" and run["waiting_reason"] == reason:
                        return snapshot
                    assert run["status"] not in {"failed", "completed", "recovery_required", "cancelled"}, run
                    time.sleep(0.1)
                raise RunnerError(f"waiting({reason}) timed out: {run_id}")

            def finish(run_id):
                result = wait_for_run(cli, run_id, 60, 0.1)
                assert result["status"] == "completed", result
                return thread_snapshot(cli, thread_id)

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

            run_id = send({**task, "prompt": "应被拒绝"})
            approve(wait_state(run_id, "approval"), "reject")
            rejected = finish(run_id)
            assert any(row["run_id"] == run_id and row["status"] == "rejected" for row in rejected["executions"])
            assert call("get_chapter_visual_plan", resource)["draft"] == current
            report["checks"].append("rejection returns an observation without writing")

            run_id = send({**task, "prompt": "不得覆盖并发修改"})
            pending = wait_state(run_id, "approval")
            change = {**resource, "record_id": current["record"]["id"], "expected_revision": current["record"]["revision"],
                      "operation": {"kind": "update_segment", "segment": {**current["plan"]["segments"][1], "title": "用户并发改名"}}}
            call("edit_chapter_visual_plan_draft", change)
            approve(pending)
            conflict = finish(run_id)
            assert any(row["run_id"] == run_id and row["status"] == "failed" and "revision" in (row["error"] or "") for row in conflict["executions"])
            current = call("get_chapter_visual_plan", resource)["draft"]
            assert current["plan"]["segments"][0]["prompt"] == task["prompt"]
            report["checks"].append("approval never overwrites a newer draft revision")

            discovery_run = send({**task, "mode": "discover"})
            discovered = finish(discovery_run)
            report["checks"].append("search is summary-only; describe activates one tool; large results are reread by handle")
            full_result = next(row for row in discovered["executions"] if row["run_id"] == discovery_run and row["tool_name"] == "get_chapter_visual_plan")
            result_task = {"mode": "read_result", "execution_id": full_result["id"]}
            history_run = send(result_task)
            history = finish(history_run)
            page = next(row["metadata"]["result"] for row in history["messages"] if row["run_id"] == history_run and row["role"] == "tool")
            assert page["text"] == source_text[2:10]
            model_result = next(row for row in discovered["executions"] if row["run_id"] == discovery_run and row["kind"] == "model_call")
            denied_run = send({**result_task, "execution_id": model_result["id"]})
            denied = finish(denied_run)
            assert any(row["run_id"] == denied_run and row["kind"] == "tool_call" and row["status"] == "failed" for row in denied["executions"])
            other_chapter = data(checked(cli, ["chapter", "import", "--number", "2", "--title", "另一章节", "--content", "独立作用域"], "second chapter")[0], "second chapter")
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
            checked(cli, ["agent", "run", run_id, "resume", "--thread-id", thread_id], "resume exhausted budget")
            limited = wait_state(run_id, "budget")
            assert not any(row["run_id"] == run_id and row["kind"] == "tool_call" for row in limited["executions"])
            checked(cli, ["agent", "run", run_id, "cancel", "--thread-id", thread_id], "cancel exhausted run")
            report["checks"].append("batch prefix executes within tool budget; resume cannot bypass token budget")

            source = {"view_instance_id": "cli-original-view", "project_id": project["id"], "chapter_id": chapter["id"],
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
            offline = finish(run_id)
            assert any(row["run_id"] == run_id and row["status"] == "failed" and "原编辑器" in (row["error"] or "") for row in offline["executions"])
            report["checks"].append("offline editor fails explicitly and releases the worker")

            # Exercise the user-facing Director journey from a clean starter
            # plan. Every write is an Agent tool call and must pass the real
            # approval gate before the next revision is observed.
            compose_chapter = data(
                checked(
                    cli,
                    [
                        "chapter",
                        "import",
                        "--number",
                        "2",
                        "--title",
                        "门后的机房",
                        "--content",
                        "林岚停在旧车站通道入口，观察挂钟后推开门，冷光从地下机房涌出。",
                    ],
                    "compose chapter",
                )[0],
                "compose chapter",
            )
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
            confirmed = call("confirm_chapter_visual_plan", {
                **compose_resource,
                "record_id": composed_draft["record"]["id"],
                "expected_revision": composed_draft["record"]["revision"],
            }, destructive=True)["visual_plan"]
            assert confirmed["record"]["status"] == "confirmed"
            script = call("generate_chapter_shooting_script", {
                **compose_resource,
                "visual_plan_id": confirmed["record"]["id"],
            })
            shots = [shot for scene in script["script"]["scenes"] for shot in scene["shots"]]
            assert [shot["shot_id"] for shot in shots] == ["shot-1", "shot-2", "shot-3"]
            assert [shot["estimated_natural_seconds"] for shot in shots] == [3, 6, 3]
            report["checks"].append("Director Agent composes a starter into a persisted three-shot timeline; approvals, image binding, scene transition, confirmation and shooting-script projection are covered")

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

                before_fault = call("get_chapter_visual_plan", resource)["draft"]
                recovery_run = send({**task, "prompt": "恢复验证：仅提交一次"}, {"max_model_calls": 2})
                approve(wait_state(recovery_run, "approval"))
                saved = wait_state(recovery_run, "budget")
                execution = next(row for row in saved["executions"] if row["run_id"] == recovery_run and row["tool_name"] == "edit_chapter_visual_plan_draft")
                assert execution["status"] == "succeeded"
                written = call("get_chapter_visual_plan", resource)["draft"]
                assert written["record"]["revision"] == before_fault["record"]["revision"] + 1
                assistant = next(row for row in saved["messages"] if row["run_id"] == recovery_run and any(item["id"] == "edit" for item in (row.get("metadata") or {}).get("tool_calls", [])))
                observation = next(row for row in saved["messages"] if row["run_id"] == recovery_run and (row.get("metadata") or {}).get("tool_call_id") == "edit")
                control = lambda action: checked(cli, ["agent", "run", recovery_run, action, "--thread-id", thread_id], "recovery " + action)
                control("pause")
                sql("""
                    BEGIN;
                    DELETE FROM agent_event WHERE run_id = :'run_id'::bigint AND message_id IN (:'assistant_id'::bigint, :'message_id'::bigint);
                    DELETE FROM agent_message WHERE run_id = :'run_id'::bigint AND id IN (:'assistant_id'::bigint, :'message_id'::bigint);
                    COMMIT;
                """, run_id=recovery_run, assistant_id=assistant["id"], message_id=observation["id"])
                control("resume")
                replayed = wait_state(recovery_run, "budget")
                replayed_message = next(row for row in replayed["messages"] if row["run_id"] == recovery_run and (row.get("metadata") or {}).get("tool_call_id") == "edit")
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
                control("resume")
                uncertain = wait_state(recovery_run, "reconcile")
                assert next(row for row in uncertain["executions"] if row["id"] == execution["id"])["status"] == "outcome_unknown"
                assert call("get_chapter_visual_plan", resource)["draft"] == written
                report["checks"].append("a committed write with a lost receipt enters reconcile and is never blindly replayed")
                # Restore only the receipt deliberately removed above, then release this fixture Run.
                control("pause")
                sql("UPDATE agent_execution SET status = 'succeeded', result_json = :'result'::jsonb, result_refs_json = :'refs'::jsonb, error_message = NULL WHERE id = :'id'::bigint AND run_id = :'run_id'::bigint;",
                    result=json.dumps(receipt["result"]), refs=json.dumps(receipt["refs"]), id=execution["id"], run_id=recovery_run)
                control("resume")
                wait_state(recovery_run, "budget")
                control("cancel")

            if log_path:
                requests = [entry["body"] for entry in provider_log_entries(log_path) if entry["body"].get("messages")]
                initial_task = MARKER + json.dumps(task, ensure_ascii=False)
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
    finally:
        if provider: provider.stop()
    secure_json(args.report, report)
    print(json.dumps(report, ensure_ascii=False))
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
