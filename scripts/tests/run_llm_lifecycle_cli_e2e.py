#!/usr/bin/env python3
"""Verify LLM lifecycle failures through CLI, real Host, queue and fixture Provider."""

import argparse
import json
import os
import tempfile
import time
from pathlib import Path

from run_agent_cli_e2e import data, thread_snapshot, wait_for_run
from run_cli_contract_e2e import Cli, RunnerError, checked, provider_log_entries, runtime_snapshot, secure_json


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--a4n", required=True)
    parser.add_argument("--server", required=True)
    parser.add_argument("--username", required=True)
    parser.add_argument("--password-env", default="A4N_PASSWORD")
    parser.add_argument("--provider-url", required=True)
    parser.add_argument("--provider-log", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    args.report.parent.mkdir(parents=True, exist_ok=True)
    report = {"success": False, "checks": []}
    try:
        with tempfile.TemporaryDirectory(prefix="a4n-llm-lifecycle-") as directory:
            work = Path(directory)
            env = os.environ.copy()
            password = env.pop(args.password_env)
            env.pop("A4N_ACCESS_TOKEN", None)
            cli = Cli(args.a4n, args.server, work / "cli.json", work / "context.json", 60, env)
            bare = Cli(args.a4n, args.server, work / "bare.json", work / "bare-context.json", 60, env)
            for client in (cli, bare):
                checked(client, ["auth", "login", "--username", args.username, "--password-stdin"], "login", password + "\n")
            checked(cli, ["capabilities", "check"], "capabilities")

            def call(client, name, value):
                parts = ["call", name, "--args", json.dumps({"input": value}, ensure_ascii=False)]
                if name == "cancel_workflow":
                    parts.append("--allow-destructive")
                return checked(client, parts, name)[0]["data"]

            def model(kind):
                snapshot = runtime_snapshot(args.provider_url)
                snapshot["llm"]["model"] = "a4n-llm-lifecycle-" + kind
                snapshot["llm"]["max_concurrent"] = 1
                secure_json(work / "runtime.json", snapshot)
                checked(cli, ["config", "runtime", "set", "--input", "@" + str(work / "runtime.json")], "runtime")

            def canvas(client, label):
                project = call(client, "create_freeform_video_project", {"title": label, "description": "isolated LLM lifecycle fixture"})
                nodes = []
                for client_id, kind, config in [
                    ("source", "TextInputNode", {"content": "雨夜里，角色推开仓库门。"}),
                    ("target", "LlmTextNode", {"instruction": "改写这句话", "output_kind": "canvas_prompt", "max_tokens": 4096}),
                ]:
                    nodes.append({"client_id": client_id, "node_kind": kind, "ref_type": None, "ref_id": None,
                                  "config_json": config, "position_json": {"x": len(nodes) * 200, "y": 0}, "state": "idle"})
                graph = call(client, "save_canvas_graph", {"project_id": project["id"], "expected_version": None,
                    "graph": {"id": None, "name": label, "graph_kind": "video_studio", "viewport_json": None,
                              "nodes": nodes, "edges": [{"source_client_id": "source", "target_client_id": "target",
                                  "source_port": "text", "target_port": "input", "role": "context", "config_json": {}}]}})
                return {"project_id": project["id"], "graph_id": graph["graph"]["id"],
                        "node_id": next(node["id"] for node in graph["nodes"] if node["node_kind"] == "LlmTextNode")}

            def workflow(client, workflow_id):
                deadline = time.monotonic() + 60
                while time.monotonic() < deadline:
                    run = call(client, "get_workflow_run_detail", {"workflow_run_id": workflow_id, "include_generation_runs": False})["run"]
                    if run["status"] in {"completed", "failed", "cancelled", "interrupted"}:
                        return run
                    time.sleep(0.1)
                raise RunnerError("workflow did not reach a terminal state")

            model("ok")
            resource = canvas(cli, "LLM successful Canvas")
            run = call(cli, "run_canvas_node", resource)
            assert workflow(cli, run["workflow_run_id"])["status"] == "completed"
            report["checks"].append("Canvas completes through the configured model with max_concurrent=1")

            resource = canvas(bare, "LLM missing personal configuration")
            before = len(provider_log_entries(args.provider_log))
            run = call(bare, "run_canvas_node", resource)
            result = workflow(bare, run["workflow_run_id"])
            assert result["status"] == "failed", result
            assert "个人 API Key" in result["error_message"], result
            assert len(provider_log_entries(args.provider_log)) == before, "server credentials reached the Provider"
            report["checks"].append("Canvas without a personal snapshot rejects before any Provider request")

            for kind, expected in [("length", "截断"), ("missing", "finish_reason")]:
                model(kind)
                resource = canvas(cli, "LLM " + kind)
                run = call(cli, "run_canvas_node", resource)
                result = workflow(cli, run["workflow_run_id"])
                assert result["status"] == "failed" and expected in result["error_message"], result
                report["checks"].append("Canvas rejects " + kind + " instead of committing a successful output")

            model("slow")
            resource = canvas(cli, "LLM cancellation")
            before = len(provider_log_entries(args.provider_log))
            run = call(cli, "run_canvas_node", resource)
            deadline = time.monotonic() + 10
            while len(provider_log_entries(args.provider_log)) == before and time.monotonic() < deadline:
                time.sleep(0.05)
            assert len(provider_log_entries(args.provider_log)) > before, "model did not start"
            call(cli, "cancel_workflow", {"task_id": None, "workflow_run_id": run["workflow_run_id"]})
            assert workflow(cli, run["workflow_run_id"])["status"] == "cancelled"
            report["checks"].append("workflow cancellation reaches a running LLM call")

            thread = data(checked(cli, ["agent", "thread", "create", "--kind", "subagent", "--capability", "canvas",
                "--project-id", str(resource["project_id"]), "--resource-kind", "project", "--resource-id", str(resource["project_id"])], "thread")[0], "thread")
            model("length")
            before = len(provider_log_entries(args.provider_log))
            started = data(checked(cli, ["agent", "chat", thread["id"], "测试截断回复。"], "agent chat")[0], "send")
            result = wait_for_run(cli, started["run_id"], 60, 0.1)
            assert result["status"] == "failed", result
            assert any(row["body"]["model"] == "a4n-llm-lifecycle-length" for row in provider_log_entries(args.provider_log)[before:]), "Agent did not reach the Provider"
            report["checks"].append("Agent rejects a nonempty length-limited final answer")

            thread = data(checked(cli, ["agent", "thread", "create", "--kind", "subagent", "--capability", "canvas",
                "--project-id", str(resource["project_id"]), "--resource-kind", "project", "--resource-id", str(resource["project_id"])], "thread")[0], "thread")
            model("slow")
            before = len(provider_log_entries(args.provider_log))
            started = data(checked(cli, ["agent", "chat", thread["id"], "测试持续输出时的时长预算。", "--budget", '{"max_duration_seconds":1}'], "agent budget")[0], "send")
            deadline = time.monotonic() + 10
            while time.monotonic() < deadline:
                snapshot = thread_snapshot(cli, thread["id"])
                result = next(row for row in snapshot["runs"] if row["id"] == started["run_id"])
                if result["status"] == "waiting" and result["waiting_reason"] == "budget":
                    break
                assert result["status"] not in {"completed", "failed", "cancelled"}, result
                time.sleep(0.1)
            assert result["status"] == "waiting" and result["waiting_reason"] == "budget", result
            assert any(row["body"]["model"] == "a4n-llm-lifecycle-slow" for row in provider_log_entries(args.provider_log)[before:]), "duration fixture did not start streaming"
            checked(cli, ["agent", "run", started["run_id"], "cancel", "--thread-id", thread["id"]], "cancel budget fixture")
            report["checks"].append("Agent stops a continuously streaming response at its duration budget")

            before = len(provider_log_entries(args.provider_log))
            started = data(checked(cli, ["agent", "chat", thread["id"], "测试无法表示的时长预算。",
                "--budget", '{"max_duration_seconds":9223372036854775807}'], "agent clock limit")[0], "send")
            result = wait_for_run(cli, started["run_id"], 60, 0.1)
            assert result["status"] == "failed", result
            assert len(provider_log_entries(args.provider_log)) == before, "invalid duration reached the Provider"
            report["checks"].append("Agent clock overflow persists failure instead of leaving the run running")
            report["success"] = True
    except (RunnerError, OSError, ValueError, TypeError, KeyError, AssertionError, StopIteration) as error:
        report["error"] = f"{type(error).__name__}: {error}"
    finally:
        secure_json(args.report, report)
    print(json.dumps(report, ensure_ascii=False))
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
