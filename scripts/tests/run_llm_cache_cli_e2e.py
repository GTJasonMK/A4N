#!/usr/bin/env python3
"""Verify cache wire metadata and accounting through CLI/Host/sidecar, not cache performance."""

import argparse
import json
import os
import tempfile
import time
from pathlib import Path

from run_agent_cli_e2e import data, wait_for_run
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
    report = {"success": False, "checks": [], "usage_is_synthetic": True}
    try:
        with tempfile.TemporaryDirectory(prefix="cache-cli-", dir=args.report.parent) as directory:
            work = Path(directory)
            env = os.environ.copy()
            password = env.pop(args.password_env)
            env.pop("A4N_ACCESS_TOKEN", None)
            cli = Cli(args.a4n, args.server, work / "cli.json", work / "context.json", 60, env)
            checked(cli, ["auth", "login", "--username", args.username, "--password-stdin"], "login", password + "\n")

            def call(name, value, destructive=False):
                command = ["call", name, "--args", json.dumps({"input": value}, ensure_ascii=False)]
                if destructive:
                    command.append("--allow-destructive")
                return checked(cli, command, name)[0]["data"]

            for package in ["ai-workflow", "video", "canvas"]:
                operation = call("start_package_lifecycle", {"package_id": package, "action": "enable", "operation_id": f"cache-e2e-{package}-{time.time_ns()}"}, True)
                assert operation["state"] == "completed", operation
            invalid = runtime_snapshot(args.provider_url, "gemini")["llm"]
            invalid = {key: value for key, value in invalid.items() if key not in {"id", "is_default"}}
            invalid["name"] = "incompatible cache options"
            invalid["extra_body_json"] = {"prompt_cache_options": {"mode": "explicit"}}
            try:
                call("create_llm_config", invalid)
            except RunnerError as error:
                assert "prompt_cache_options" in str(error) and "仅适用于 OpenAI" in str(error), str(error)
            else:
                raise AssertionError("Gemini configuration accepted OpenAI cache options")
            report["checks"].append({"check": "foreign_cache_options_rejected_before_save"})
            for protocol in ["openai", "openai-responses", "anthropic", "gemini"]:
                snapshot = runtime_snapshot(args.provider_url, protocol)
                snapshot["llm"]["extra_body_json"] = {"a4n_test_cache_usage": True}
                if protocol in {"openai", "openai-responses"}:
                    snapshot["llm"]["model"] = "gpt-5.6-luna"
                secure_json(work / "runtime.json", snapshot)
                checked(cli, ["config", "runtime", "set", "--input", "@" + str(work / "runtime.json")], "runtime")
                before = len(provider_log_entries(args.provider_log))
                prior = call("get_generation_call_analytics", {"days": 1, "recent_limit": 200})
                previous_id = max((row["id"] for row in prior["recent_llm_calls"]), default=0)
                project = call("create_freeform_video_project", {"title": "cache " + protocol, "description": "isolated cache regression"})
                graph = call("save_canvas_graph", {"project_id": project["id"], "expected_version": None, "graph": {
                    "id": None, "name": "cache " + protocol, "graph_kind": "video_studio", "viewport_json": None,
                    "nodes": [
                        {"client_id": "source", "node_kind": "TextInputNode", "ref_type": None, "ref_id": None,
                         "config_json": {"content": "雨夜里，角色推开仓库门。"}, "position_json": {"x": 0, "y": 0}, "state": "idle"},
                        {"client_id": "target", "node_kind": "LlmTextNode", "ref_type": None, "ref_id": None,
                         "config_json": {"instruction": "保留原意，改写这句话", "output_kind": "canvas_prompt", "max_tokens": 4096},
                         "position_json": {"x": 200, "y": 0}, "state": "idle"},
                    ],
                    "edges": [{"source_client_id": "source", "target_client_id": "target", "source_port": "text", "target_port": "input", "role": "context", "config_json": {}}],
                }})
                node_id = next(node["id"] for node in graph["nodes"] if node["node_kind"] == "LlmTextNode")
                started = call("run_canvas_node", {"project_id": project["id"], "graph_id": graph["graph"]["id"], "node_id": node_id})
                deadline = time.monotonic() + 60
                while time.monotonic() < deadline:
                    run = call("get_workflow_run_detail", {"workflow_run_id": started["workflow_run_id"], "include_generation_runs": False})["run"]
                    if run["status"] in {"completed", "failed", "cancelled", "interrupted"}:
                        break
                    time.sleep(0.1)
                assert run["status"] == "completed", run
                requests = provider_log_entries(args.provider_log)[before:]
                request = next(row["body"] for row in reversed(requests) if row.get("body", {}).get("a4n_test_cache_usage") is True)
                if protocol in {"openai", "openai-responses"}:
                    assert request["prompt_cache_options"]["mode"] == "explicit", request
                    messages = request["messages" if protocol == "openai" else "input"]
                    marked_roles = {item["role"] for item in messages if isinstance(item.get("content"), list)
                                    and any(part.get("prompt_cache_breakpoint") for part in item["content"])}
                    assert {"system", "user"} <= marked_roles, marked_roles
                elif protocol == "anthropic":
                    assert any(block.get("cache_control") for block in request["system"])
                    assert any(block.get("cache_control") for item in request["messages"] for block in item["content"])
                rows = call("get_generation_call_analytics", {"days": 1, "recent_limit": 200})["recent_llm_calls"]
                row = next(row for row in rows if row["id"] > previous_id and row["model"] == snapshot["llm"]["model"])
                assert row["prompt_tokens"] == 10000, row
                assert row["prompt_cache_hit_tokens"] == 9000, row
                assert row["prompt_cache_miss_tokens"] == 1000, row
                assert row["prompt_cache_hit_ratio"] == 0.9, row
                assert row["prompt_cache_write_tokens"] == (None if protocol == "gemini" else 400), row
                report["checks"].append({"protocol": protocol, "workflow_run_id": started["workflow_run_id"], "trace_id": row["id"], "cache_hit_ratio": row["prompt_cache_hit_ratio"]})
                if protocol in {"openai", "openai-responses", "anthropic"}:
                    thread = data(checked(cli, ["agent", "thread", "create", "--kind", "subagent", "--capability", "canvas",
                        "--project-id", str(project["id"]), "--resource-kind", "project", "--resource-id", str(project["id"])], "cache thread")[0], "thread")
                    before = len(provider_log_entries(args.provider_log))
                    started = data(checked(cli, ["agent", "chat", thread["id"], "仅回复缓存回归完成，不需要调用工具。"], "initial cache request")[0], "send")
                    run = wait_for_run(cli, started["run_id"], 60, 0.1)
                    assert run["status"] == "completed", run
                    request = next(row["body"] for row in provider_log_entries(args.provider_log)[before:]
                                   if row.get("body", {}).get("a4n_test_cache_usage") is True)
                    if protocol == "anthropic":
                        assert any(block.get("cache_control") for item in request["messages"] if item["role"] == "user" for block in item["content"])
                    else:
                        assert request["prompt_cache_options"]["mode"] == "implicit", request
                    report["checks"].append({"protocol": protocol, "check": "initial_agent_task_cache", "run_id": started["run_id"]})
            report["success"] = True
    except (RunnerError, OSError, ValueError, TypeError, KeyError, AssertionError, StopIteration) as error:
        report["error"] = f"{type(error).__name__}: {error}"
    finally:
        secure_json(args.report, report)
    print(json.dumps(report, ensure_ascii=False))
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
