#!/usr/bin/env python3
"""Verify professional execution, task ownership and local Canvas edits via CLI."""

import argparse
import copy
import json
import os
import tempfile
import time
import uuid
from pathlib import Path

from run_agent_cli_e2e import data, run_view, thread_snapshot, wait_for_run
from run_cli_contract_e2e import Cli, RunnerError, checked, provider_log_entries, runtime_snapshot, secure_json, start_provider
from specialized_agent_fixture import MARKER


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--a4n", default=str(Path(__file__).parents[2] / "target/debug/a4n"))
    parser.add_argument("--server", required=True)
    parser.add_argument("--username", required=True)
    parser.add_argument("--password-env", default="A4N_PASSWORD")
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--provider-url")
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
        with tempfile.TemporaryDirectory(prefix="a4n-agent-domains-") as temporary:
            work = Path(temporary)
            env = os.environ.copy()
            password = env.pop(args.password_env)
            env.pop("A4N_ACCESS_TOKEN", None)
            cli = Cli(args.a4n, args.server, work / "cli.json", work / "context.json", args.command_timeout, env)
            checked(cli, ["health"], "health")
            checked(cli, ["capabilities", "check"], "capabilities")
            checked(cli, ["auth", "login", "--username", args.username, "--password-stdin"], "login", password + "\n")
            args.embedding_dimension = 384
            provider, log_path = start_provider(args, args.report.parent, uuid.uuid4().hex)
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
                    assert result["exit_code"] != 0, name + " unexpectedly succeeded"
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

            def finish_with_approvals(thread_id, run_id):
                deadline = time.monotonic() + 60
                while time.monotonic() < deadline:
                    value = snapshot(thread_id, run_id)
                    run = next(row for row in value["runs"] if row["id"] == run_id)
                    if run["status"] == "completed":
                        return run, value
                    if run["status"] == "waiting" and run["waiting_reason"] == "approval":
                        pending = next(row for row in value["approvals"] if row["status"] == "pending")
                        checked(cli, ["agent", "approve", pending["id"], "approve"], "approval")
                    else:
                        assert run["status"] not in {"failed", "cancelled", "recovery_required"}, run
                    time.sleep(0.1)
                raise RunnerError(f"Run {run_id} did not complete through approvals")

            def cancel(thread_id, run_id):
                checked(cli, ["agent", "run", run_id, "cancel", "--thread-id", thread_id], "cancel")

            def task_summary(handle):
                return data(checked(cli, ["task", "get", handle["task_id"], "--attempt", str(handle["attempt"])], "task get")[0], "task get")

            project = data(checked(cli, ["project", "create", "--title", "Agent C/D 隔离验证", "--project-type", "novel_long", "--use-context"], "project")[0], "project")
            original = "林岚在雨夜收到一封信。" + "未请求的后文哨兵_。" * 500
            chapter = data(checked(cli, ["chapter", "import", "--number", "1", "--title", "局部优化", "--content", original], "chapter")[0], "chapter")
            thread = data(checked(cli, ["agent", "thread", "create", "--kind", "subagent", "--capability", "content_optimization", "--project-id", str(project["id"]), "--chapter-id", str(chapter["id"]), "--resource-kind", "chapter", "--resource-id", str(chapter["id"])], "optimization thread")[0], "thread")
            thread_id = thread["id"]
            report.update(project_id=project["id"], chapter_id=chapter["id"], optimization_thread_id=thread_id)
            run_id = send(thread_id, "A4N_OPTIMIZATION_WAIT 请只润色开头一句，提出补丁，保留其他正文。")
            pending = wait_state(thread_id, run_id, "external")
            waiting_run = next(row for row in pending["runs"] if row["id"] == run_id)
            assert waiting_run["lease_owner"] is None and waiting_run["lease_expires_at"] is None
            handle = next(row["metadata"]["result"] for row in pending["messages"]
                          if row["run_id"] == run_id and row["role"] == "tool" and row["metadata"]["tool"] == "send_content_optimization_message")
            session = next(row["metadata"]["result"] for row in pending["messages"]
                           if row["run_id"] == run_id and row["role"] == "tool" and row["metadata"]["tool"] == "ensure_content_optimization_session")
            observer = data(checked(cli, ["agent", "thread", "create", "--kind", "main", "--title", "任务观察者", "--project-id", str(project["id"])], "observer thread")[0], "thread")
            observer_run = send(observer["id"], MARKER + json.dumps({"mode": "wait_generation", "task_id": handle["task_id"], "attempt": handle["attempt"]}))
            wait_state(observer["id"], observer_run, "external")
            cancel(observer["id"], observer_run)
            summary = task_summary(handle)
            assert summary["handle"]["status"] in {"running", "completed"}, summary
            run, completed = finish(thread_id, run_id)
            executions = [row for row in completed["executions"] if row["run_id"] == run_id]
            assert run["status"] == "completed"
            assert [row["tool_name"] for row in executions if row["status"] == "succeeded"] == [
                "ensure_content_optimization_session",
                "send_content_optimization_message",
                "wait_generation_task",
            ]
            assert data(checked(cli, ["chapter", "get", str(chapter["id"])], "unchanged chapter")[0], "chapter")["content"] == original
            report["checks"].append("professional content-optimization execution reserves a real Generation task, waits for completion, and leaves writeback pending")
            report["checks"].append("durable wait releases its lease and reuses one execution; cancelling an observer preserves the owned task")

            plain_id = send(thread_id, "A4N_OPTIMIZATION_PLAIN 仅回答：补丁何时会写回正文？")
            plain, plain_snapshot = finish(thread_id, plain_id)
            assert plain["status"] == "completed" and not plain.get("error"), plain
            assert not any(row["status"] == "failed" for row in plain_snapshot["executions"])
            report["checks"].append("a follow-up content-optimization request completes through the same durable workflow")

            cancel_id = send(thread_id, "A4N_OPTIMIZATION_CANCEL_WAIT 请检查并润色开头句。")
            cancellation = wait_state(thread_id, cancel_id, "external")
            cancelled_handle = next(row["metadata"]["result"] for row in cancellation["messages"]
                                    if row["run_id"] == cancel_id and row["role"] == "tool" and row["metadata"]["tool"] == "send_content_optimization_message")
            cancel(thread_id, cancel_id)
            deadline = time.monotonic() + 60
            while time.monotonic() < deadline:
                task = task_summary(cancelled_handle)
                if task["handle"]["status"] in {"cancelled", "interrupted"}:
                    break
                time.sleep(0.1)
            assert task["handle"]["status"] in {"cancelled", "completed"}, task
            report["checks"].append(
                "cancelling the owner drives its actual Generation task to a terminal state: "
                + task["handle"]["status"]
            )

            patch_sets = call("list_optimize_patch_sets", {"session_id": session["id"]})
            pending_patch = next(
                patch_set
                for patch_set in patch_sets
                if patch_set["status"] == "pending" and patch_set["patches"]
            )
            call("apply_optimization_patches", {
                "session_id": session["id"],
                "base_content_hash": "stale-content-hash",
                "patches": pending_patch["patches"],
            }, failure=True, destructive=True)
            assert data(checked(cli, ["chapter", "get", str(chapter["id"])], "stale apply chapter")[0], "chapter")["content"] == original
            report["checks"].append("stale optimization PatchSet hash is rejected without changing chapter content")
            applied = call("apply_optimization_patches", {
                "session_id": session["id"],
                "base_content_hash": pending_patch["base_content_hash"],
                "patches": pending_patch["patches"],
            }, destructive=True)
            assert applied["chapter_id"] == chapter["id"]
            assert applied["applied_count"] == len(pending_patch["patches"])
            rewritten = data(
                checked(cli, ["chapter", "get", str(chapter["id"])], "applied chapter")[0],
                "chapter",
            )["content"]
            assert rewritten != original and "未请求的后文哨兵_" in rewritten
            report["checks"].append("user approval applies the pending optimization PatchSet and persists the rewritten chapter")

            video = call("create_freeform_video_project", {"title": "Canvas 局部验证", "description": "isolated fixture"})
            checked(cli, ["context", "set", "--project-id", str(video["id"])], "canvas context")
            nodes = [{"client_id": str(i), "node_kind": "LlmTextNode", "ref_type": None, "ref_id": None,
                      "config_json": {"prompt": "原提示词"}, "position_json": {"x": i * 200, "y": 0}, "state": "idle"} for i in range(3)]
            edges = [{"source_client_id": "0", "target_client_id": "1", "source_port": "text", "target_port": "input", "role": "context", "config_json": {}}]
            graph = call("save_canvas_graph", {"project_id": video["id"], "expected_version": None,
                         "graph": {"id": None, "name": "局部测试图", "graph_kind": "video_studio", "viewport_json": None, "nodes": nodes, "edges": edges}})
            resource = {"project_id": video["id"], "graph_id": graph["graph"]["id"]}
            index = call("read_canvas_graph", {**resource, "limit": 2})
            assert index["projection"]["total"] == 3 and index["projection"]["next_offset"] == 2
            canvas_thread = data(checked(cli, ["agent", "thread", "create", "--kind", "subagent", "--capability", "canvas", "--project-id", str(video["id"]), "--resource-kind", "project", "--resource-id", str(video["id"])], "canvas thread")[0], "thread")
            canvas_task = {"mode": "canvas_edit", **resource, "node_id": graph["nodes"][2]["id"], "config": {"prompt": "仅修改这一节点"}, "run": True}
            canvas_run = send(canvas_thread["id"], MARKER + json.dumps(canvas_task, ensure_ascii=False))
            approval = wait_state(canvas_thread["id"], canvas_run, "approval")
            pending_approval = next(row for row in approval["approvals"] if row["status"] == "pending")
            checked(cli, ["agent", "approve", pending_approval["id"], "approve"], "canvas approval")
            _, canvas_completed = finish_with_approvals(canvas_thread["id"], canvas_run)
            canvas_executions = [row for row in canvas_completed["executions"] if row["run_id"] == canvas_run]
            run_execution = next(row for row in canvas_executions if row["tool_name"] == "run_canvas_node")
            wait_execution = next(row for row in canvas_executions if row["tool_name"] == "wait_workflow_run")
            assert run_execution["status"] == "succeeded" and any(ref["kind"] == "workflow_run" for ref in run_execution["result_refs"])
            assert wait_execution["status"] == "succeeded"
            wait_result = next(
                row["metadata"]["result"]
                for row in canvas_completed["messages"]
                if row["run_id"] == canvas_run
                and row["role"] == "tool"
                and row["metadata"]["tool"] == "wait_workflow_run"
            )
            assert wait_result["run"]["status"] in {"completed", "failed", "cancelled", "stopped"}
            changed = call("get_canvas_graph", resource)
            assert changed["nodes"][:2] == graph["nodes"][:2] and changed["edges"] == graph["edges"]
            assert [row["id"] for row in changed["nodes"]] == [row["id"] for row in graph["nodes"]]
            assert changed["nodes"][2]["config_json"] == canvas_task["config"]
            rejected_task = {**canvas_task, "config": {"prompt": "用户拒绝的执行"}}
            rejected_run = send(canvas_thread["id"], MARKER + json.dumps(rejected_task, ensure_ascii=False))
            rejected_approval = wait_state(canvas_thread["id"], rejected_run, "approval")
            rejected_pending = next(row for row in rejected_approval["approvals"] if row["status"] == "pending")
            checked(cli, ["agent", "approve", rejected_pending["id"], "reject"], "canvas rejection")
            _, rejected_snapshot = finish(canvas_thread["id"], rejected_run)
            rejected_execution = next(row for row in rejected_snapshot["executions"] if row["run_id"] == rejected_run and row["tool_name"] == "edit_canvas_graph")
            assert rejected_execution["status"] == "rejected"
            assert call("get_canvas_graph", resource) == changed
            report["checks"].append("rejecting a Canvas write leaves graph state and workflow untouched")
            version = changed["graph"]["version"]
            edit = {**resource, "expected_version": version, "operations": [{"kind": "set_node_config", "node_id": canvas_task["node_id"], "config_json": canvas_task["config"]}]}
            noop = call("edit_canvas_graph", edit)
            assert noop["changed"] is False and int(noop["resource"]["revision"]) == version
            call("edit_canvas_graph", {**edit, "expected_version": version - 1}, failure=True)
            invalid = copy.deepcopy(edit)
            invalid["operations"][0]["config_json"] = {"prompt": "不得保存的半成品"}
            invalid["operations"].append({"kind": "connect", "edge": {**edges[0], "source_client_id": str(graph["nodes"][0]["id"]), "target_client_id": str(graph["nodes"][2]["id"]), "source_port": "invalid"}})
            call("edit_canvas_graph", invalid, failure=True)
            assert call("get_canvas_graph", resource) == changed
            report["checks"].append("Canvas index, approval, stable node/edge identities, node execution, workflow wait, noop, CAS and atomic rejection of invalid ports")
            report["provider_requests"] = {"count": len(provider_log_entries(log_path)), "log": str(log_path)}
            report["success"] = True
    except (RunnerError, OSError, ValueError, TypeError, KeyError, AssertionError, StopIteration) as error:
        report["error"] = f"{type(error).__name__}: {error}"
    finally:
        if provider:
            provider.stop()
        secure_json(args.report, report)
    print(json.dumps({"success": report["success"], "report": str(args.report), "error": report.get("error")}, ensure_ascii=False))
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
