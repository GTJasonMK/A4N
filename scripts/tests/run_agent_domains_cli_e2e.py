#!/usr/bin/env python3
"""Verify professional execution, task ownership and local Canvas edits via CLI."""

import argparse
import copy
import json
import tempfile
import time
import traceback
import uuid
from pathlib import Path

from run_agent_cli_e2e import data, thread_snapshot, wait_for_run
from mock_openai.fixtures.reverse import embedded_json_objects, message_text
from mock_openai.prompt_parsing import json_task_payload
from run_cli_contract_e2e import RunnerError, checked, logged_in_cli, provider_log_entries, runtime_snapshot, secure_json, start_provider
from specialized_agent_fixture import MARKER, observation_data


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
    args = parser.parse_args()
    args.report.parent.mkdir(parents=True, exist_ok=True)
    report = {"success": False, "checks": []}
    provider = None
    try:
        with tempfile.TemporaryDirectory(prefix="a4n-agent-domains-", dir=args.report.parent) as temporary:
            work = Path(temporary)
            cli = logged_in_cli(args, work, timeout=args.command_timeout)
            args.embedding_dimension = 384
            provider, log_path = start_provider(args, args.report.parent, uuid.uuid4().hex,
                                                request_log_path=args.provider_log)
            secure_json(work / "runtime.json", runtime_snapshot(provider.base_url, embedding_dimension=384))
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
                    assert result["exit_code"] in (2, 6, 8), (name, result)
                    return result
                return checked(cli, parts, name)[0]["data"]

            def send(thread_id, text, budget=None):
                parts = ["agent", "chat", thread_id, text]
                if budget is not None:
                    parts += ["--budget", json.dumps(budget)]
                return data(checked(cli, parts, "send")[0], "send")["run_id"]

            def snapshot(thread_id, run_id):
                value = thread_snapshot(cli, thread_id)
                secure_json(args.report.parent / (run_id + ".json"), value)
                return value

            def wait_state(thread_id, run_id, reason):
                deadline = time.monotonic() + 60
                while time.monotonic() < deadline:
                    value = snapshot(thread_id, run_id)
                    run = next(row for row in value["runs"] if row["id"] == run_id)
                    if run["status"] == "waiting" and run["waiting_reason"] == reason:
                        return value
                    assert run["status"] not in {"completed", "failed", "cancelled", "recovery_required"}, run
                    time.sleep(0.1)
                raise RunnerError(f"Run {run_id} did not reach waiting({reason})")

            def finish(thread_id, run_id):
                run = wait_for_run(cli, run_id, 60, 0.1)
                assert run["status"] == "completed", run
                return run, snapshot(thread_id, run_id)

            def approval_for_run(value, run_id):
                executions = {row["id"] for row in value["executions"] if row["run_id"] == run_id}
                return next(row for row in value["approvals"]
                            if row["status"] == "pending" and row["execution_id"] in executions)

            def finish_with_approvals(thread_id, run_id):
                deadline = time.monotonic() + 60
                while time.monotonic() < deadline:
                    value = snapshot(thread_id, run_id)
                    run = next(row for row in value["runs"] if row["id"] == run_id)
                    if run["status"] == "completed":
                        return run, value
                    if run["status"] == "waiting" and run["waiting_reason"] == "approval":
                        pending = approval_for_run(value, run_id)
                        checked(cli, ["agent", "approve", pending["id"], "approve"], "approval")
                    else:
                        assert run["status"] not in {"failed", "cancelled", "interrupted", "recovery_required"}, run
                        assert run["waiting_reason"] not in {"budget", "user", "reconcile"}, run
                    time.sleep(0.1)
                raise RunnerError(f"Run {run_id} did not complete through approvals")

            project = data(checked(cli, ["project", "create", "--title", "Agent C/D 隔离验证", "--project-type", "novel_long", "--use-context"], "project")[0], "project")
            original = "林岚在雨夜收到一封信。" + "未请求的后文哨兵_。" * 500
            chapter = data(checked(cli, ["chapter", "import", "--number", "1", "--title", "局部优化", "--content", original], "chapter")[0], "chapter")
            thread = data(checked(cli, ["agent", "thread", "create", "--kind", "subagent", "--capability", "content_optimization", "--project-id", str(project["id"]), "--chapter-id", str(chapter["id"]), "--resource-kind", "chapter", "--resource-id", str(chapter["id"])], "optimization thread")[0], "thread")
            thread_id = thread["id"]
            report.update(project_id=project["id"], chapter_id=chapter["id"], optimization_thread_id=thread_id)
            run_id = send(thread_id, "A4N_OPTIMIZATION_PROPOSE 请只润色开头一句，提出补丁，保留其他正文。")
            run, completed = finish(thread_id, run_id)
            executions = [row for row in completed["executions"] if row["run_id"] == run_id]
            assert run["status"] == "completed"
            assert [row["tool_name"] for row in executions if row["kind"] == "tool_call" and row["status"] == "succeeded"] == [
                "inspect_project_context",
                "read_project_context",
                "read_agent_execution_result",
                "edit_article",
            ]
            assert data(checked(cli, ["chapter", "get", str(chapter["id"])], "unchanged chapter")[0], "chapter")["content"] == original
            report["checks"].append("content-optimization Agent reads bounded chapter evidence and leaves a reviewable article patch without writeback")

            plain_id = send(thread_id, "A4N_OPTIMIZATION_PLAIN 仅回答：补丁何时会写回正文？")
            plain, plain_snapshot = finish(thread_id, plain_id)
            assert plain["status"] == "completed" and not plain.get("error"), plain
            assert not any(row["status"] == "failed" for row in plain_snapshot["executions"])
            report["checks"].append("a follow-up content-optimization request completes through the same durable workflow")

            context_chapter = data(checked(cli, ["chapter", "import", "--number", "2", "--title", "原文引用", "--content", "\r\n第一段。\r\n第二段。\r\n\r\n第三段。"], "context chapter")[0], "chapter")
            context_thread = data(checked(cli, ["agent", "thread", "create", "--kind", "subagent", "--capability", "content_optimization", "--project-id", str(project["id"]), "--chapter-id", str(context_chapter["id"]), "--resource-kind", "chapter", "--resource-id", str(context_chapter["id"])], "context thread")[0], "thread")
            context_run = send(context_thread["id"], "A4N_OPTIMIZATION_CONTEXT 搜索第二段，按搜索路径读取，再只读取第二段及其引用。")
            _, context_result = finish(context_thread["id"], context_run)
            context_tools = [row["tool_name"] for row in context_result["executions"] if row["run_id"] == context_run and row["kind"] == "tool_call" and row["status"] == "succeeded"]
            assert context_tools == ["search_project_context", "read_project_context", "read_project_context"], context_tools
            report["checks"].append("native context search paths roundtrip through reads; selected paragraph and evidence agree across CRLF and blank lines")

            permissions = {name: False for name in ("read_project_detail", "read_blueprint", "read_outlines", "read_character_profiles", "read_foreshadowing", "automatic_writeback")}
            chat = call("create_optimization_chat_session", {"project_id": project["id"], "chapter_number": 2, "name": "完整长历史", "permissions": permissions})
            first_message = "A4N_OPTIMIZATION_PLAIN A4N_OPTIMIZATION_LONG_REPLY 首轮约束" + "甲" * 6500 + "末尾约束"
            before = len(provider_log_entries(log_path))
            for turn in range(14):
                message = first_message if turn == 0 else f"A4N_OPTIMIZATION_PLAIN 第 {turn + 1} 轮，只回答当前问题，保留首轮约束。"
                handle = call("start_optimize_chat_generation", {"meta": {"start_request_id": str(uuid.uuid4()), "retry_of_task_id": None}, "request": {"session_id": chat["id"], "message": message, "permissions": permissions}})
                checked(cli, ["task", "watch", handle["task_id"], "--attempt", str(handle["attempt"])], "optimization history turn")
            requests = provider_log_entries(log_path)[before:]
            assert len(requests) == 14, "a plain answer must not trigger extra model or tool calls"
            tasks = []
            for entry in requests:
                objects = [value for message in entry["body"]["messages"]
                           for value in embedded_json_objects(message_text(message.get("content", "")))]
                bases = [json_task_payload(value["base_task"]) for value in objects
                         if value.get("kind") == "base_task" and isinstance(value.get("base_task"), str)]
                assert len(bases) == 1, bases
                tasks.append(bases[0])
            assert [len(task["conversation_history"]) for task in tasks] == list(range(14))
            history = tasks[-1]["conversation_history"]
            assert history[0]["user"] == first_message
            assert history[0]["assistant"].endswith("乙" * 6500 + "完整答复末尾")
            assert history[:12] == tasks[-2]["conversation_history"][:12], "unchanged history must retain its cacheable prefix"
            turns = call("list_optimize_turns", {"session_id": chat["id"]})
            assert len(turns) == 14 and all(turn["status"] == "completed" for turn in turns), turns
            report["checks"].append("fourteen optimization turns retain complete instructions and replies beyond 6000 characters with one model call per plain answer")

            video = call("create_freeform_video_project", {"title": "Canvas 局部验证", "description": "isolated fixture"})
            checked(cli, ["context", "set", "--project-id", str(video["id"])], "canvas context")
            nodes = [{"client_id": str(i), "node_kind": "LlmTextNode", "ref_type": None, "ref_id": None,
                      "config_json": {"instruction": "原指令", "output_kind": "shot_script"}, "position_json": {"x": i * 200, "y": 0}, "state": "idle"} for i in range(3)]
            nodes[0]["node_kind"] = "TextInputNode"
            nodes[0]["config_json"] = {"content": "A4N_AGENT_CANVAS_INPUT 雨夜的钟楼。"}
            edges = [{"source_client_id": "0", "target_client_id": "1", "source_port": "text", "target_port": "input", "role": "context", "config_json": {}}]
            edges.append({**edges[0], "target_client_id": "2"})
            graph = call("save_canvas_graph", {"project_id": video["id"], "expected_version": None,
                         "graph": {"id": None, "name": "局部测试图", "graph_kind": "video_studio", "viewport_json": None, "nodes": nodes, "edges": edges}})
            resource = {"project_id": video["id"], "graph_id": graph["graph"]["id"]}
            index = call("read_canvas_graph", {**resource, "limit": 2})
            assert index["projection"]["total"] == 3 and index["projection"]["next_offset"] == 2
            canvas_thread = data(checked(cli, ["agent", "thread", "create", "--kind", "subagent", "--capability", "canvas", "--project-id", str(video["id"]), "--resource-kind", "project", "--resource-id", str(video["id"])], "canvas thread")[0], "thread")
            canvas_task = {"mode": "canvas_edit", **resource, "node_id": graph["nodes"][2]["id"], "config": {"instruction": "转为镜头描述", "output_kind": "shot_script"}, "run": True}
            canvas_run = send(canvas_thread["id"], MARKER + json.dumps(canvas_task, ensure_ascii=False))
            approval = wait_state(canvas_thread["id"], canvas_run, "approval")
            pending_approval = approval_for_run(approval, canvas_run)
            checked(cli, ["agent", "approve", pending_approval["id"], "approve"], "canvas approval")
            _, canvas_completed = finish_with_approvals(canvas_thread["id"], canvas_run)
            canvas_executions = [row for row in canvas_completed["executions"] if row["run_id"] == canvas_run]
            run_execution = next(row for row in canvas_executions if row["tool_name"] == "run_canvas_node")
            wait_execution = next(row for row in canvas_executions if row["tool_name"] == "wait_workflow_run")
            assert run_execution["status"] == "succeeded" and any(ref["kind"] == "workflow_run" for ref in run_execution["result_refs"])
            assert wait_execution["status"] == "succeeded"
            wait_result = next(
                observation_data(row)
                for row in canvas_completed["messages"]
                if row["run_id"] == canvas_run
                and row["role"] == "tool"
                and row["metadata"]["tool"] == "wait_workflow_run"
            )
            assert wait_result["run"]["status"] == "completed"
            changed = call("get_canvas_graph", resource)
            secure_json(args.report.parent / "canvas-before-after.json", {"before": graph, "after": changed})
            assert changed["nodes"][:2] == graph["nodes"][:2], {"before": graph["nodes"][:2], "after": changed["nodes"][:2]}
            assert changed["edges"] == graph["edges"], {"before": graph["edges"], "after": changed["edges"]}
            assert [row["id"] for row in changed["nodes"]] == [row["id"] for row in graph["nodes"]]
            target = changed["nodes"][2]
            artifact = next(row["artifacts_json"] for row in wait_result["checkpoints"]
                            if row["step_key"] == f"canvas_node:{canvas_task['node_id']}")
            assert target["config_json"] == {**canvas_task["config"], **artifact}, target
            assert target["state"] == "completed" and target["ref_type"] == "prompt_artifact", target
            assert target["ref_id"] == artifact["prompt_artifact_id"] and target["position_json"] == graph["nodes"][2]["position_json"], target
            assert artifact["text"] == "镜头：雨水沿钟楼的窗棂滑落。", artifact
            rejected_task = {**canvas_task, "config": {"instruction": "用户拒绝的执行"}}
            rejected_run = send(canvas_thread["id"], MARKER + json.dumps(rejected_task, ensure_ascii=False))
            rejected_approval = wait_state(canvas_thread["id"], rejected_run, "approval")
            rejected_pending = approval_for_run(rejected_approval, rejected_run)
            checked(cli, ["agent", "approve", rejected_pending["id"], "reject"], "canvas rejection")
            _, rejected_snapshot = finish(canvas_thread["id"], rejected_run)
            rejected_execution = next(row for row in rejected_snapshot["executions"] if row["run_id"] == rejected_run and row["tool_name"] == "edit_canvas_graph")
            assert rejected_execution["status"] == "rejected"
            assert call("get_canvas_graph", resource) == changed
            report["checks"].append("rejecting a Canvas write leaves graph state and workflow untouched")
            version = changed["graph"]["version"]
            edit = {**resource, "expected_version": version, "operations": [{"kind": "set_node_config", "node_id": canvas_task["node_id"], "config_json": target["config_json"]}]}
            noop = call("edit_canvas_graph", edit)
            assert noop["changed"] is False and int(noop["resource"]["revision"]) == version
            call("edit_canvas_graph", {**edit, "expected_version": version - 1}, failure=True)
            invalid = copy.deepcopy(edit)
            invalid["operations"][0]["config_json"] = {"instruction": "不得保存的半成品"}
            invalid["operations"].append({"kind": "connect", "edge": {**edges[0], "source_client_id": str(graph["nodes"][0]["id"]), "target_client_id": str(graph["nodes"][2]["id"]), "source_port": "invalid"}})
            call("edit_canvas_graph", invalid, failure=True)
            assert call("get_canvas_graph", resource) == changed
            report["checks"].append("Canvas index, approval, stable node/edge identities, node execution, workflow wait, noop, CAS and atomic rejection of invalid ports")
            report["provider_requests"] = {"count": len(provider_log_entries(log_path)), "log": str(log_path)}
            report["success"] = True
    except (RunnerError, OSError, ValueError, TypeError, KeyError, AssertionError, StopIteration) as error:
        report["error"] = f"{type(error).__name__}: {error}"
        report["traceback"] = traceback.format_exc()
    finally:
        if provider:
            provider.stop()
        secure_json(args.report, report)
    print(json.dumps({"success": report["success"], "report": str(args.report), "error": report.get("error")}, ensure_ascii=False))
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
