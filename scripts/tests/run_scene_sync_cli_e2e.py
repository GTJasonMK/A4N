#!/usr/bin/env python3
"""Verify scene output validation, checkpoints and recovery through the real CLI/Host."""

import argparse
import json
import os
import tempfile
from pathlib import Path

from mock_openai_fixture_story import chapter_body_response
from mock_openai_prompt_parsing import json_task_payload
from mock_openai_fixture_reverse import message_text
from run_cli_contract_e2e import (
    Cli, RunnerError, checked, command_error, provider_log_entries, runtime_snapshot, secure_json,
)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--a4n", required=True)
    parser.add_argument("--server", required=True)
    parser.add_argument("--username", required=True)
    parser.add_argument("--password-env", default="A4N_PASSWORD")
    parser.add_argument("--provider-url", required=True)
    parser.add_argument("--provider-log", type=Path, required=True)
    parser.add_argument("--embedding-dimension", type=int, default=384)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    args.report.parent.mkdir(parents=True, exist_ok=True)
    report = {"success": False, "checks": []}
    try:
        with tempfile.TemporaryDirectory(prefix="scene-sync-", dir=args.report.parent) as directory:
            work = Path(directory)
            env = os.environ.copy()
            password = env.pop(args.password_env)
            env.pop("A4N_ACCESS_TOKEN", None)
            cli = Cli(args.a4n, args.server, work / "cli.json", work / "context.json", 60, env)
            checked(cli, ["auth", "login", "--username", args.username, "--password-stdin"], "login", password + "\n")

            def call(name, value):
                parts = ["call", name, "--args", json.dumps({"input": value}, ensure_ascii=False)]
                return checked(cli, parts, name)[0]["data"]

            def model(kind):
                snapshot = runtime_snapshot(args.provider_url, embedding_dimension=args.embedding_dimension)
                snapshot["llm"]["model"] = "a4n-scene-sync-" + kind
                secure_json(work / "runtime.json", snapshot)
                checked(cli, ["config", "runtime", "set", "--input", "@" + str(work / "runtime.json")], "runtime")

            def scene_calls(offset):
                names = []
                for entry in provider_log_entries(args.provider_log)[offset:]:
                    messages = entry.get("body", {}).get("messages", [])
                    combined = "\n".join(message_text(item.get("content", "")) for item in messages)
                    if "[阶段指令]" not in combined:
                        continue
                    name = json_task_payload(combined)["task"]["name"]
                    if name and name.startswith("scene_"):
                        names.append(name)
                        if name == "scene_occurrence_extraction":
                            assert "occurrences[].timeline_position 只能是 string|null" in combined
                            assert entry["body"].get("response_format") == {"type": "json_object"}
                return names

            project = checked(cli, ["project", "create", "--title", "场景合同回归", "--project-type", "novel_long", "--use-context"], "project")[0]["data"]
            chapter = checked(cli, ["chapter", "import", "--number", "1", "--title", "雨夜来信", "--content", chapter_body_response()], "chapter")[0]["data"]
            report.update(project_id=project["id"], chapter_id=chapter["id"])

            def workflow_details():
                runs = call("list_workflow_runs", {
                    "project_id": project["id"], "chapter_id": None, "workflow_kind": None,
                    "status": None, "resumable_only": False, "limit": 20,
                })
                return [call("get_workflow_run_detail", {
                    "workflow_run_id": run["id"], "include_generation_runs": False,
                }) for run in runs]

            for kind, command in [("number", "generate_chapter_analysis"), ("object", "resume_chapter_analysis_sync")]:
                model(kind)
                before = len(provider_log_entries(args.provider_log))
                if command == "generate_chapter_analysis":
                    mutation = call(command, {"chapter_id": chapter["id"]})
                    assert mutation["chapter"]["analysis_sync_has_saved_analysis"], mutation
                    assert mutation["chapter"]["analysis_sync_can_resume"], mutation
                    message = "\n".join(mutation["warnings"])
                else:
                    result = cli.run(["call", command, "--args", json.dumps({"input": {"chapter_id": chapter["id"]}})])
                    assert result["exit_code"] != 0, "invalid model output unexpectedly succeeded"
                    error = command_error(result)
                    assert error.get("code") == "internal_error", error
                    message = error.get("message", "")
                assert "occurrences[0].timeline_position" in message, message
                assert "LLM 调用失败" in message, message
                assert "状态无效" not in message and "参数无效" not in message, message
                names = scene_calls(before)
                assert "scene_occurrence_extraction" in names, names
                assert "scene_transition_extraction" not in names, names
                assert "scene_entity_state_extraction" not in names, names
                details = workflow_details()
                assert any(detail["run"]["workflow_kind"] == "chapter_analysis" and detail["run"]["status"] == "failed" for detail in details), details
                checkpoints = [item for detail in details for item in detail["checkpoints"]]
                assert any(item["step_key"].endswith(".scenes") and item["status"] == "completed" for item in checkpoints), checkpoints
                assert not any(item["step_key"].endswith(".occurrences") and item["status"] == "completed" for item in checkpoints), checkpoints
                assert call("list_scene_profiles", {"project_id": project["id"]}) == []
                report["checks"].append(kind + " rejected before downstream calls, completed checkpoint or scene writes")

            model("null")
            before = len(provider_log_entries(args.provider_log))
            call("resume_chapter_analysis_sync", {"chapter_id": chapter["id"]})
            assert "scene_occurrence_extraction" in scene_calls(before), "resume reused a rejected response"
            profiles = call("list_scene_profiles", {"project_id": project["id"]})
            assert len(profiles) == 2 and all(item["occurrence_count"] > 0 for item in profiles), profiles
            checkpoints = [item for detail in workflow_details() for item in detail["checkpoints"]]
            assert any(item["step_key"].endswith(".occurrences") and item["status"] == "completed" for item in checkpoints), checkpoints
            report["checks"].append("null timeline resumes successfully and persists validated scene occurrences")

            model("text")
            call("sync_scene_profiles", {"project_id": project["id"], "chapter_number": 1})
            profiles = call("list_scene_profiles", {"project_id": project["id"]})
            assert profiles and all(item["occurrence_count"] > 0 for item in profiles), profiles
            report["checks"].append("text timeline succeeds through standalone scene synchronization")
            report["success"] = True
    except (RunnerError, OSError, ValueError, TypeError, KeyError, AssertionError) as error:
        report["error"] = f"{type(error).__name__}: {error}"
    finally:
        secure_json(args.report, report)
    print(json.dumps(report, ensure_ascii=False))
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
