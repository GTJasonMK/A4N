#!/usr/bin/env python3
"""Verify LLM lifecycle failures through CLI, real Host, queue and fixture Provider."""

import json
import tempfile
import time
from pathlib import Path

from run_agent_cli_e2e import data, thread_snapshot, wait_for_run
from run_cli_contract_e2e import RunnerError, checked, cli_e2e_parser, command_error, logged_in_cli, provider_log_entries, runtime_snapshot, secure_json


def main():
    parser = cli_e2e_parser(__doc__)
    args = parser.parse_args()
    args.report.parent.mkdir(parents=True, exist_ok=True)
    report = {"success": False, "checks": []}
    try:
        with tempfile.TemporaryDirectory(prefix="a4n-llm-lifecycle-", dir=args.report.parent) as directory:
            work = Path(directory)
            cli = logged_in_cli(args, work)
            bare = logged_in_cli(args, work / "bare")

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
                project = call(client, "create_canvas_project", {"title": label, "description": "isolated LLM lifecycle fixture"})
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

            def canvas_generation_chain(label):
                project = call(cli, "create_canvas_project", {"title": label, "description": "full Canvas generation fixture"})
                nodes = [
                    ("source", "TextInputNode", {"content": "雨夜里，角色推开仓库门。"}),
                    ("llm", "LlmTextNode", {"instruction": "改写为可执行的画面提示词", "output_kind": "canvas_prompt", "max_tokens": 4096}),
                    ("image", "GenerateImageNode", {"image_config_id": 900002, "width": 1024, "height": 1024}),
                    ("video", "GenerateVideoNode", {"video_config_id": 900003, "input_mode": "first_frame", "width": 1280, "height": 720, "duration_secs": 1, "fps": 24}),
                ]
                edges = [
                    ("source", "llm", "text", "input", "context"),
                    ("llm", "image", "text", "text", "context"),
                    ("llm", "video", "text", "text", "context"),
                    ("image", "video", "image", "first_frame", "first_frame"),
                ]
                saved = call(cli, "save_canvas_graph", {
                    "project_id": project["id"], "expected_version": None,
                    "graph": {
                        "id": None, "name": label, "graph_kind": "video_studio", "viewport_json": None,
                        "nodes": [
                            {"client_id": client_id, "node_kind": kind, "ref_type": None, "ref_id": None,
                             "config_json": config, "position_json": {"x": index * 200, "y": 0}, "state": "idle"}
                            for index, (client_id, kind, config) in enumerate(nodes)
                        ],
                        "edges": [
                            {"source_client_id": source, "target_client_id": target, "source_port": source_port,
                             "target_port": target_port, "role": role, "config_json": {}}
                            for source, target, source_port, target_port, role in edges
                        ],
                    },
                })
                return {
                    "project_id": project["id"],
                    "graph_id": saved["graph"]["id"],
                    "node_ids": {node["node_kind"].removesuffix("Node").lower(): node["id"] for node in saved["nodes"]},
                }

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

            chain = canvas_generation_chain("Canvas LLM to image to video")
            provider_start = len(provider_log_entries(args.provider_log))
            run = call(cli, "run_canvas_graph", {"project_id": chain["project_id"], "graph_id": chain["graph_id"], "target_node_id": None})
            final_run = workflow(cli, run["workflow_run_id"])
            assert final_run["status"] == "completed", final_run
            document = call(cli, "get_canvas_graph", {"project_id": chain["project_id"], "graph_id": chain["graph_id"]})
            nodes = {node["node_kind"].removesuffix("Node").lower(): node for node in document["nodes"]}
            llm_node, image_node, video_node = nodes["llmtext"], nodes["generateimage"], nodes["generatevideo"]
            llm_text = llm_node["config_json"].get("text")
            assert llm_node["state"] == image_node["state"] == video_node["state"] == "completed", nodes
            assert llm_node["config_json"].get("canvas_runtime_result") is True and isinstance(llm_text, str) and llm_text.strip(), llm_node
            assert llm_node["config_json"].get("prompt_artifact_id") and llm_node["ref_type"] == "prompt_artifact", llm_node
            image_config, video_config = image_node["config_json"], video_node["config_json"]
            image_id, video_id = image_config.get("generated_image_id"), video_config.get("generated_video_id")
            assert isinstance(image_id, int) and image_config.get("file_path"), image_node
            assert isinstance(video_id, int) and video_config.get("file_path"), video_node
            images = call(cli, "list_images_by_project", {"project_id": chain["project_id"]})
            image = next(row for row in images if row["id"] == image_id)
            assert image["prompt"] == llm_text and image["file_path"] == image_config["file_path"], image

            detail = call(cli, "get_workflow_run_detail", {"workflow_run_id": run["workflow_run_id"], "include_generation_runs": False})
            assert detail["run"]["status"] == "completed", detail
            checkpoints = {row["step_key"]: row for row in detail["checkpoints"]}
            llm_checkpoint = checkpoints[f"canvas_node:{chain['node_ids']['llmtext']}"]
            image_checkpoint = checkpoints[f"canvas_node:{chain['node_ids']['generateimage']}"]
            video_checkpoint = checkpoints[f"canvas_node:{chain['node_ids']['generatevideo']}"]
            assert all(row["status"] == "completed" for row in (llm_checkpoint, image_checkpoint, video_checkpoint)), checkpoints
            assert llm_checkpoint["artifacts_json"]["prompt_artifact_id"] == llm_node["config_json"]["prompt_artifact_id"]
            assert image_checkpoint["artifacts_json"]["generated_image_id"] == image_id
            assert video_checkpoint["input_fingerprint_json"]["first_frame_image_id"] == image_id
            assert video_checkpoint["input_fingerprint_json"]["prompt"] == llm_text
            assert video_checkpoint["artifacts_json"]["generated_video_id"] == video_id
            assert video_checkpoint["artifacts_json"]["file_path"] == video_config["file_path"]
            requests = provider_log_entries(args.provider_log)[provider_start:]
            llm_requests = [row for row in requests if row.get("body", {}).get("model") == "a4n-llm-lifecycle-ok" and isinstance(row.get("body", {}).get("messages"), list)]
            image_requests = [row for row in requests if row.get("path", "").endswith(("/images/generations", "/images/edits"))]
            video_requests = [row for row in requests if row.get("path", "").endswith(("/videos", "/videos/generations"))]
            assert len(llm_requests) == len(image_requests) == len(video_requests) == 1, requests
            assert "雨夜里，角色推开仓库门。" in json.dumps(llm_requests[0]["body"]["messages"], ensure_ascii=False)
            assert image_requests[0]["body"].get("prompt") == llm_text, image_requests
            assert video_requests[0]["body"].get("prompt") == llm_text, video_requests
            report["checks"].append("Canvas graph runs LLM, generated image and first-frame video in dependency order; all Provider calls, generated records and checkpoints read back")

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
            executions = [row for row in thread_snapshot(cli, thread["id"])["executions"]
                          if row["run_id"] == started["run_id"] and row["kind"] == "model_call"]
            assert len(executions) == 1 and executions[0]["status"] == "failed", executions
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
            prior = thread_snapshot(cli, thread["id"])
            rejected = cli.run(["agent", "chat", thread["id"], "测试无法表示的时长预算。",
                "--budget", '{"max_duration_seconds":9223372036854775807}'])
            error = command_error(rejected)
            assert rejected["exit_code"] in (2, 8) and "max_duration_seconds" in error["message"], rejected
            assert len(provider_log_entries(args.provider_log)) == before, "invalid duration reached the Provider"
            after = thread_snapshot(cli, thread["id"])
            assert [row["id"] for row in after["runs"]] == [row["id"] for row in prior["runs"]], "invalid budget created a run"
            assert [row["id"] for row in after["messages"]] == [row["id"] for row in prior["messages"]], "invalid budget created a message"
            assert {row["id"]: row["content"] for row in after["messages"] if row["role"] == "user"} == {
                row["id"]: row["content"] for row in prior["messages"] if row["role"] == "user"
            }, "invalid budget changed the user's history"
            report["checks"].append("Agent clock overflow rejects before creating a run, message or Provider request")
            report["success"] = True
    except (RunnerError, OSError, ValueError, TypeError, KeyError, AssertionError, StopIteration) as error:
        report["error"] = f"{type(error).__name__}: {error}"
    finally:
        secure_json(args.report, report)
    print(json.dumps(report, ensure_ascii=False))
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
