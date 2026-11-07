#!/usr/bin/env python3
"""Verify chapter facts, consolidation cadence, source invalidation and recovery through CLI/Host."""

import base64
import io
import json
import tempfile
from pathlib import Path
import zipfile

from mock_openai.fixtures.reverse import message_text
from mock_openai.fixtures.story import chapter_body_response
from run_cli_contract_e2e import (
    RunnerError, checked, cli_e2e_parser, logged_in_cli, provider_log_entries, runtime_snapshot, secure_json,
)


def main():
    parser = cli_e2e_parser(__doc__)
    parser.add_argument("--embedding-dimension", type=int, default=384)
    args = parser.parse_args()
    args.report.parent.mkdir(parents=True, exist_ok=True)
    report = {"success": False, "checks": []}
    try:
        with tempfile.TemporaryDirectory(prefix="fact-feedback-", dir=args.report.parent) as directory:
            work = Path(directory)
            cli = logged_in_cli(args, work)

            def call(name, payload):
                secure_json(work / "call.json", payload)
                return checked(cli, ["call", name, "--args", "@" + str(work / "call.json")], name)[0]["data"]

            def typed(name, value):
                return call(name, {"input": value})

            def model(name="a4n-fact-feedback"):
                snapshot = runtime_snapshot(args.provider_url, embedding_dimension=args.embedding_dimension)
                snapshot["llm"]["model"] = name
                secure_json(work / "runtime.json", snapshot)
                checked(cli, ["config", "runtime", "set", "--input", "@" + str(work / "runtime.json")], "runtime")

            def consolidation_calls():
                return [entry for entry in provider_log_entries(args.provider_log)
                        if entry.get("body", {}).get("model", "").startswith("a4n-fact-feedback")
                        and "project_fact_consolidation" in "\n".join(
                            message_text(item.get("content", ""))
                            for item in entry.get("body", {}).get("messages", []))]

            model()
            project = checked(cli, ["project", "create", "--title", "正文事实反馈回归", "--project-type", "novel_long", "--use-context"], "project")[0]["data"]
            project_id = project["id"]
            report["project_id"] = project_id

            def feedback():
                return call("get_project_fact_feedback", {"projectId": project_id})

            def blueprint():
                return call("get_blueprint", {"projectId": project_id})

            def versions():
                return typed("list_blueprint_versions", {"project_id": project_id})

            def analyze(chapter_id):
                return typed("generate_chapter_analysis", {"chapter_id": chapter_id})

            def import_chapter(number, calendar="七"):
                return checked(cli, ["chapter", "import", "--number", str(number), "--title", f"历法 {number}", "--content", f"记忆城每年有{calendar}个季节。"], "chapter")[0]["data"]

            assert feedback()["interval_chapters"] == 5
            planned_synopsis = "作者计划最终让城市恢复记忆，结局尚未发生。"
            typed("save_blueprint", {
                "project_id": project_id, "genre": "奇幻", "style": None, "tone": None,
                "target_audience": None, "one_sentence_summary": None,
                "full_synopsis": planned_synopsis, "total_chapters": 30,
            })
            typed("update_world_setting", {
                "project_id": project_id,
                "expected_current_version": blueprint()["current_version"],
                "world_setting": {
                    "culture_and_society": {"calendar": "尚未确认"},
                    "factions": ["作者保留的议会"],
                },
            })
            call("update_project", {"projectId": project_id, "input": {"title": "作者修改后的项目名", "description": None}})
            original_versions = len(versions())
            baseline_calls = len(consolidation_calls())
            chapters = []
            for number in range(1, 6):
                chapter = import_chapter(number)
                chapters.append(chapter)
                assert number in feedback()["legacy_chapters"], "unanalyzed chapters must not claim coverage"
                result = analyze(chapter["id"])
                assert result["warnings"] == [], result
                state = feedback()
                if number < 5:
                    assert state["pending_chapters"] == list(range(1, number + 1)), state
                    assert len(consolidation_calls()) == baseline_calls
                    assert len(versions()) == original_versions
                else:
                    assert state["consolidated_chapters"] == list(range(1, 6)), state
                    assert state["pending_chapters"] == []
                    assert len(consolidation_calls()) == baseline_calls + 1
            world = blueprint()["world_setting"]
            assert world["culture_and_society"]["calendar"] == "记忆城每年有七个季节", world
            assert world["factions"] == ["作者保留的议会"], world
            assert blueprint()["full_synopsis"] == planned_synopsis
            assert call("get_project", {"projectId": project_id})["title"] == "作者修改后的项目名"
            report["checks"].append("chapters 1–4 update facts; chapter 5 consolidates once without overwriting unrelated material")

            version_count = len(versions())
            result = analyze(chapters[-1]["id"])
            assert result["warnings"] == [], result
            assert feedback()["pending_chapters"] == []
            assert len(consolidation_calls()) == baseline_calls + 1
            assert len(versions()) == version_count
            preview_calls = len(consolidation_calls())
            preview = typed("preview_project_fact_feedback_request", {"project_id": project_id, "mode": "consolidate", "workflow_run_id": None})
            assert preview and len(consolidation_calls()) == preview_calls
            typed("run_project_fact_feedback", {"project_id": project_id, "mode": "consolidate", "workflow_run_id": None})
            assert len(versions()) == version_count
            report["checks"].append("repeated analysis does not count twice; preview makes no provider call; unchanged consolidation adds no version")

            call("update_project", {"projectId": project_id, "input": {"title": None, "description": None, "fact_feedback_interval_chapters": 3}})
            assert feedback()["interval_chapters"] == 3
            calls_before = len(consolidation_calls())
            for number in range(6, 9):
                chapter = import_chapter(number)
                chapters.append(chapter)
                result = analyze(chapter["id"])
                assert result["warnings"] == [], result
                assert len(consolidation_calls()) == calls_before + (number == 8)
            assert feedback()["consolidated_chapters"] == list(range(1, 9))
            assert len(versions()) == version_count
            report["checks"].append("project interval changes to 3; unchanged facts do not generate material versions")

            model("a4n-fact-feedback-invalid")
            chapter = import_chapter(9, "六")
            chapters.append(chapter)
            result = analyze(chapter["id"])
            assert result["warnings"] and "#999999" in "\n".join(result["warnings"]), result
            failed = feedback()
            assert failed["workflow_status"] == "failed" and failed["can_resume"], failed
            assert failed["pending_chapters"] == [9], failed
            assert blueprint()["world_setting"]["culture_and_society"]["calendar"] == "记忆城每年有七个季节"
            detail = typed("get_workflow_run_detail", {"workflow_run_id": failed["workflow_run_id"], "include_generation_runs": False})
            assert not any(checkpoint["status"] == "completed" and "response_content" in (checkpoint.get("checkpoint_json") or {})
                           for checkpoint in detail["checkpoints"]), detail
            model()
            typed("run_project_fact_feedback", {"project_id": project_id, "mode": "consolidate", "workflow_run_id": failed["workflow_run_id"]})
            assert feedback()["workflow_status"] == "completed"
            assert blueprint()["world_setting"]["culture_and_society"]["calendar"] == "记忆城每年有六个季节"
            report["checks"].append("a changed world fact triggers early review; fabricated evidence fails visibly and recovery discards the invalid response")

            latest = next(item for item in call("list_chapters", {"projectId": project_id}) if item["id"] == chapter["id"])
            calls_before = len(consolidation_calls())
            chat_calls_before = sum(entry.get("path", "").endswith("chat/completions") for entry in provider_log_entries(args.provider_log))
            typed("update_chapter_content", {
                "chapter_id": chapter["id"], "expected_revision": latest["edit_revision"],
                "content": "记忆城每年有七个季节。", "title": latest["title"],
            })
            assert len(consolidation_calls()) == calls_before
            assert sum(entry.get("path", "").endswith("chat/completions") for entry in provider_log_entries(args.provider_log)) == chat_calls_before
            assert 9 in feedback()["legacy_chapters"]
            assert blueprint()["world_setting"]["culture_and_society"]["calendar"] == "记忆城每年有七个季节"
            rebuilt = typed("run_project_fact_feedback", {"project_id": project_id, "mode": "rebuild", "workflow_run_id": None})
            assert rebuilt["legacy_chapters"] == [] and rebuilt["pending_chapters"] == [], rebuilt
            assert rebuilt["consolidated_chapters"] == list(range(1, 10)), rebuilt
            assert blueprint()["full_synopsis"] == planned_synopsis
            report["checks"].append("plain saving calls no model, retracts invalid facts, and explicit full-book rebuild flushes the final partial batch")

            exported = typed("export_project_family_data", {"root_project_id": project_id, "include_derived_projects": False})
            with zipfile.ZipFile(io.BytesIO(base64.b64decode(exported["data_base64"]))) as archive:
                manifest = json.loads(archive.read("a4n-project-family.json"))
            assert manifest["version"] == "project_family_v8", manifest["version"]
            assert manifest["text_core"]["fact_feedback"]["changes"]
            calls_before = len(consolidation_calls())
            imported = typed("import_project_family_upload", exported)[0]
            imported_feedback = call("get_project_fact_feedback", {"projectId": imported["id"]})
            assert imported_feedback["interval_chapters"] == 3, imported_feedback
            assert imported_feedback["consolidated_chapters"] == list(range(1, 10)), imported_feedback
            assert imported_feedback["legacy_chapters"] == [] and imported_feedback["pending_chapters"] == [], imported_feedback
            assert imported_feedback["changes"], imported_feedback
            copied = typed("duplicate_project", {"project_id": project_id, "new_title": "事实反馈副本"})
            copied_feedback = call("get_project_fact_feedback", {"projectId": copied["id"]})
            assert copied_feedback["interval_chapters"] == 3, copied_feedback
            assert copied_feedback["consolidated_chapters"] == [] and copied_feedback["legacy_chapters"] == list(range(1, 10)), copied_feedback
            assert len(consolidation_calls()) == calls_before
            report["checks"].append("family ZIP round-trip preserves source coverage and history; ordinary copy keeps interval and requests fresh analysis")

            for command in ["list_profiles", "list_foreshadowing", "add_profile_attribute", "attribute_history"]:
                checked(cli, ["capabilities", "describe", command], "describe " + command)
            model("a4n-fact-feedback-identity")
            identity_project = checked(cli, ["project", "create", "--title", "事实撤回身份回归", "--project-type", "novel_long", "--use-context"], "identity project")[0]["data"]
            identity_id = identity_project["id"]
            report["identity_project_id"] = identity_id
            identity_chapter = checked(cli, ["chapter", "import", "--number", "1", "--title", "雨夜来信", "--content", chapter_body_response()], "identity chapter")[0]["data"]
            result = analyze(identity_chapter["id"])
            assert result["warnings"] == [], result
            profiles = typed("list_profiles", {"project_id": identity_id})
            profile_ids = {profile["id"] for profile in profiles}
            lin = next(profile for profile in profiles if profile["name"] == "林岚")
            assert lin["base_attributes"]["explicit_attributes"]["位置"]
            typed("add_profile_attribute", {
                "profile_id": lin["id"], "chapter_number": 1, "category": "explicit", "key": "author_note",
                "value": "作者保留的设定", "cause": "作者设定", "evidence": "作者设定",
            })
            planted = typed("list_foreshadowing", {"project_id": identity_id})
            assert len(planted) == 1, planted
            result = analyze(identity_chapter["id"])
            assert result["warnings"] == [], result
            repeated = typed("list_foreshadowing", {"project_id": identity_id})
            assert len(repeated) == 1 and repeated[0]["id"] == planted[0]["id"], repeated
            assert repeated[0]["description"] != planted[0]["description"]
            assert {profile["id"] for profile in typed("list_profiles", {"project_id": identity_id})} == profile_ids
            report["checks"].append("changed reanalysis references the withdrawn foreshadowing ID and preserves character identities")

            latest = call("list_chapters", {"projectId": identity_id})[0]
            chat_calls_before = sum(entry.get("path", "").endswith("chat/completions") for entry in provider_log_entries(args.provider_log))
            typed("update_chapter_content", {
                "chapter_id": latest["id"], "expected_revision": latest["edit_revision"],
                "content": "记忆城每年有七个季节。", "title": latest["title"],
            })
            assert sum(entry.get("path", "").endswith("chat/completions") for entry in provider_log_entries(args.provider_log)) == chat_calls_before
            restored = typed("list_profiles", {"project_id": identity_id})
            assert {profile["id"] for profile in restored} == profile_ids
            for profile in restored:
                expected = {"author_note": "作者保留的设定"} if profile["id"] == lin["id"] else {}
                assert profile["current_state"] == {"explicit_attributes": expected, "implicit_attributes": {}, "social_attributes": {}}, profile
                assert profile["last_synced_chapter"] == 0
            assert typed("list_foreshadowing", {"project_id": identity_id}) == []
            history = typed("attribute_history", {
                "profile_id": lin["id"], "chapter_from": None, "chapter_to": None,
                "category": None, "key": None, "operation": None, "limit": 100,
            })
            assert len(history) == 1 and history[0]["source_kind"] == "user", history
            report["checks"].append("plain rewrite clears discovered baseline and chapter observations while preserving the manual event and stable profile IDs")
            report["success"] = True
    except (RunnerError, OSError, ValueError, TypeError, KeyError, AssertionError) as error:
        report["error"] = f"{type(error).__name__}: {error}"
    finally:
        secure_json(args.report, report)
    print(json.dumps(report, ensure_ascii=False))
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
