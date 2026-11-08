#!/usr/bin/env python3
"""Reject malformed evidence IDs at each analysis stage, then resume via CLI."""

import json
import tempfile
from pathlib import Path

from mock_openai.fixtures.reverse import message_text
from mock_openai.fixtures.entities import character_sync_profile_name
from mock_openai.fixtures.story import chapter_body_response
from mock_openai.prompt_parsing import json_task_payload
from mock_openai.contract_guard import fixture_prompt_parts
from run_cli_contract_e2e import (
    RunnerError, checked, cli_e2e_parser, command_error, logged_in_cli,
    provider_log_entries, runtime_snapshot, secure_json,
)


def assert_single_correction_per_subject(details, prompt):
    attempts = {}
    for task, subject, correction in details:
        if task == prompt:
            attempts.setdefault(subject, []).append(correction)
    assert attempts, ("no failed task calls observed", prompt)
    for subject, corrections in attempts.items():
        assert corrections == [False, True], ("each logical call must get exactly one correction", prompt, subject, corrections)


def main():
    parser = cli_e2e_parser(__doc__)
    parser.add_argument("--llm-provider", choices=("openai", "openai-responses"), default="openai-responses")
    args = parser.parse_args()
    args.report.parent.mkdir(parents=True, exist_ok=True)
    report = {"success": False, "checks": [], "provider": args.llm_provider}
    try:
        with tempfile.TemporaryDirectory(prefix="analysis-anchor-", dir=args.report.parent) as directory:
            work = Path(directory)
            cli = logged_in_cli(args, work)
            described = set()

            def command(name, value):
                if name not in described:
                    checked(cli, ["capabilities", "describe", name], name + " contract")
                    described.add(name)
                return ["call", name, "--args", json.dumps({"input": value}, ensure_ascii=False)]

            def call(name, value):
                return checked(cli, command(name, value), name)[0]["data"]

            def configure(model):
                base_url = args.provider_url.rstrip("/").removesuffix("/v1") + "/v1"
                snapshot = runtime_snapshot(base_url, embedding_dimension=384, llm_provider=args.llm_provider)
                snapshot["llm"]["model"] = model
                secure_json(work / "runtime.json", snapshot)
                checked(cli, ["config", "runtime", "set", "--input", "@" + str(work / "runtime.json")], "runtime")

            calls = []

            def llm_calls(offset):
                found = []
                details = []
                for entry in provider_log_entries(args.provider_log)[offset:]:
                    if entry["path"].rstrip("/") not in {"/v1/responses", "/v1/chat/completions"}:
                        continue
                    body = entry.get("body", {})
                    messages = body.get("messages", body.get("input", []))
                    _,combined,_=fixture_prompt_parts(messages)
                    payload = json_task_payload(combined)
                    name = payload.get("task", {}).get("name")
                    if not name:
                        continue
                    contract = payload.get("output_contract", {})
                    for field, schema in contract.get("field_types", {}).items():
                        if "anchor_id" not in field:
                            continue
                        scalar = schema["items"] if schema["type"] == "array" else schema
                        assert scalar["type"] == "integer" and scalar["minimum"] == 1, (name, field, schema)
                        assert scalar["maximum"] == 9223372036854775807
                    if name in {"object_discovery", "character_discovery", "scene_profile_candidates"} or name.startswith(("scene_", "character_sync_")):
                        assert contract.get("field_types") or name == "character_sync_chapter_states", name
                        if args.llm_provider == "openai-responses":
                            assert entry["path"].endswith("/responses"), entry["path"]
                            native = body["text"]["format"]
                            assert native["type"] == "json_schema", native
                            root = contract["required_root_field"]
                            fields = native["schema"]["properties"][root]["items"]["properties"]
                            for key, schema in contract.get("field_types", {}).items():
                                assert fields[key] == schema, (name, key, fields)
                        found.append(name)
                        subject = character_sync_profile_name(combined) if name.startswith("character_sync_") else None
                        correction = any("model_output_correction" in message_text(item.get("content", "")) for item in messages)
                        details.append((name, subject, correction))
                calls.extend(found)
                return found, details

            configure("a4n-anchor-fail-objects")
            project = checked(cli, ["project", "create", "--title", "证据ID恢复回归", "--project-type", "novel_long", "--use-context"], "project")[0]["data"]
            chapter = checked(cli, ["chapter", "import", "--number", "1", "--title", "雨夜来信", "--content", chapter_body_response()], "chapter")[0]["data"]
            report.update(project_id=project["id"], chapter_id=chapter["id"])

            def workflows():
                runs = call("list_workflow_runs", {"project_id": project["id"], "chapter_id": None,
                    "workflow_kind": None, "status": None, "resumable_only": False, "limit": 100})
                return [call("get_workflow_run_detail", {"workflow_run_id": row["id"], "include_generation_runs": False}) for row in runs]

            failures = [
                ("objects", "evidence_anchor_ids", "object_discovery"),
                ("characters", "evidence_anchor_id", "character_discovery"),
                ("scenes", "evidence_anchor_ids", "scene_profile_candidates"),
                ("occurrences", "source_anchor_ids", "scene_occurrence_extraction"),
                ("transitions", "source_anchor_ids", "scene_transition_extraction"),
                ("entity_states", "source_anchor_ids", "scene_entity_state_extraction"),
                ("events", "evidence_anchor_id", "character_sync_evidence_events"),
                ("attribute_changes", "evidence_anchor_id", "character_sync_attribute_changes"),
                ("behaviors", "original_text_anchor_id", "character_sync_behavior_records"),
            ]
            completed = []
            for index, (root, field, prompt) in enumerate(failures):
                configure("a4n-anchor-fail-" + root)
                offset = len(provider_log_entries(args.provider_log))
                if index == 0:
                    result = call("generate_chapter_analysis", {"chapter_id": chapter["id"]})
                    message = "\n".join(result["warnings"])
                    assert result["chapter"]["analysis_sync_can_resume"]
                else:
                    result = cli.run(command("resume_chapter_analysis_sync", {"chapter_id": chapter["id"]}))
                    assert result["exit_code"] == 7, result
                    error = command_error(result)
                    assert error["code"] == "internal_error" and error["retryable"] is False, error
                    message = error["message"]
                assert f"{root}[0].{field}" in message and "JSON 整数数字" in message, message
                assert "LLM 调用失败" in message and "参数无效" not in message, message
                names, details = llm_calls(offset)
                assert prompt in names, names
                assert_single_correction_per_subject(details, prompt)
                for earlier in completed:
                    assert earlier not in names, ("completed work repeated", earlier, names)
                for _, _, later in failures[index + 1:]:
                    assert later not in names, ("invalid evidence reached downstream", prompt, later, names)
                details = workflows()
                analysis = max((row for row in details if row["run"]["workflow_kind"] == "chapter_analysis"), key=lambda row: row["run"]["id"])
                assert analysis["run"]["status"] == "failed" and analysis["run"]["can_resume"], analysis["run"]
                for detail in details:
                    for checkpoint in detail["checkpoints"]:
                        content = checkpoint.get("checkpoint_json", {})
                        if checkpoint["status"] == "completed" and isinstance(content, dict) and "response_content" in content:
                            value = json.loads(content["response_content"])
                            for item in value.get(root, []):
                                if field in item:
                                    ids = item[field] if isinstance(item[field], list) else [item[field]]
                                    assert all(type(anchor) is int and anchor > 0 for anchor in ids), checkpoint["step_key"]
                if root == "objects":
                    assert call("list_object_profiles", {"project_id": project["id"]}) == []
                if root in {"objects", "characters"}:
                    assert call("list_profiles", {"project_id": project["id"]}) == []
                if root in {"scenes", "occurrences", "transitions", "entity_states"}:
                    assert call("list_scene_profiles", {"project_id": project["id"]}) == []
                report["checks"].append(root + ": rejects string IDs before downstream calls, writes and completed checkpoints")
                completed = [name for name in calls if name != prompt and name not in {row[2] for row in failures[index + 1:]}]

            configure("a4n-virtual-text")
            offset = len(provider_log_entries(args.provider_log))
            call("resume_chapter_analysis_sync", {"chapter_id": chapter["id"]})
            resumed, _ = llm_calls(offset)
            assert "character_sync_behavior_records" in resumed
            assert not any(prompt in resumed for _, _, prompt in failures[:6]), resumed
            chapters = checked(cli, ["chapter", "list"], "chapters")[0]["data"]
            chapter_result = next(row for row in chapters if row["id"] == chapter["id"])
            assert chapter_result["content"] == chapter["content"], "analysis must not change the saved chapter"
            assert not chapter_result["analysis_sync_can_resume"], chapter_result
            assert len(call("list_object_profiles", {"project_id": project["id"]})) == 2
            assert len(call("list_profiles", {"project_id": project["id"]})) == 2
            assert len(call("list_scene_profiles", {"project_id": project["id"]})) == 2
            report["checks"].append("valid integer IDs finish from checkpoints, preserve text and avoid duplicate profiles")
            report.update(resume_calls=resumed, correction_cases=[])
            for fault in ("string_ids", "missing_anchor", "unknown_anchor"):
                configure("a4n-output-recover-" + fault)
                project = checked(cli, ["project", "create", "--title", "有限纠正-" + fault, "--project-type", "novel_long", "--use-context"], "correction project")[0]["data"]
                chapter = checked(cli, ["chapter", "import", "--number", "1", "--title", "雨夜来信", "--content", chapter_body_response()], "correction chapter")[0]["data"]
                offset = len(provider_log_entries(args.provider_log))
                result = call("generate_chapter_analysis", {"chapter_id": chapter["id"]})
                assert not result["warnings"], result["warnings"]
                assert not result["chapter"]["analysis_sync_can_resume"], result["chapter"]
                assert result["chapter"]["content"] == chapter["content"]
                entries = provider_log_entries(args.provider_log)[offset:]
                corrections = [entry for entry in entries if "model_output_correction" in json.dumps(entry.get("body", {}))]
                assert corrections, (fault, "no correction calls observed")
                corrected_tasks = set()
                for entry in corrections:
                    body = entry["body"]
                    messages = body.get("messages", body.get("input", []))
                    _,combined,_=fixture_prompt_parts(messages)
                    corrected_tasks.add(json_task_payload(combined)["task"]["name"])
                assert {row[2] for row in failures}.issubset(corrected_tasks), (fault, corrected_tasks)
                names, _ = llm_calls(offset)
                for _, _, prompt in failures:
                    assert prompt in names, (fault, prompt, names)
                assert len(call("list_object_profiles", {"project_id": project["id"]})) == 2
                assert len(call("list_profiles", {"project_id": project["id"]})) == 2
                assert len(call("list_scene_profiles", {"project_id": project["id"]})) == 2
                report["checks"].append(fault + ": one correction per failing call completes analysis without duplicate profiles or text changes")
                report["correction_cases"].append({"fault": fault, "project_id": project["id"], "chapter_id": chapter["id"], "correction_calls": len(corrections), "corrected_tasks": sorted(corrected_tasks)})
            report["success"] = True
    except (RunnerError, OSError, ValueError, TypeError, KeyError, AssertionError, StopIteration) as error:
        report["error"] = f"{type(error).__name__}: {error}"
    finally:
        secure_json(args.report, report)
    print(json.dumps(report, ensure_ascii=False))
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
