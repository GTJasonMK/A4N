#!/usr/bin/env python3
"""Check call-level validation and reuse through a real isolated CLI/Server/Provider."""

import argparse
import json
import os
import tempfile
import time
import uuid
from pathlib import Path

from mock_openai_fixture_story import chapter_body_response
from mock_openai_provider_protocol import PNG_1X1_BASE64
from run_cli_contract_e2e import Cli, RunnerError, checked, command_error, provider_log_entries, runtime_snapshot, secure_json


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for option in ("a4n", "server", "username", "provider-url"):
        parser.add_argument("--" + option, required=True)
    parser.add_argument("--password-env", default="A4N_PASSWORD")
    parser.add_argument("--provider-log", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    args.report.parent.mkdir(parents=True, exist_ok=True)
    report = {"success": False, "checks": [], "projects": []}
    try:
        with tempfile.TemporaryDirectory(prefix="call-checkpoints-", dir=args.report.parent) as directory:
            work = Path(directory)
            env = os.environ.copy()
            password = env.pop(args.password_env)
            env.pop("A4N_ACCESS_TOKEN", None)
            cli = Cli(args.a4n, args.server, work / "cli.json", work / "context.json", 60, env)
            checked(cli, ["health"], "health")
            checked(cli, ["capabilities", "check"], "CLI/Server compatibility")
            checked(cli, ["auth", "login", "--username", args.username, "--password-stdin"], "login", password + "\n")
            described = set()

            def parts(name, value, destructive=False):
                if name not in described:
                    checked(cli, ["capabilities", "describe", name], name + " contract")
                    described.add(name)
                result = ["call", name, "--args", json.dumps({"input": value}, ensure_ascii=False)]
                if destructive:
                    result.append("--allow-destructive")
                return result

            def call(name, value, destructive=False):
                return checked(cli, parts(name, value, destructive), name)[0]["data"]

            def reject(name, value, expected):
                result = cli.run(parts(name, value))
                message = str(command_error(result).get("message", ""))
                assert result["exit_code"] in (2, 6, 8) and expected in message, (name, result["exit_code"], message)
                return message

            def model(fault=None):
                snapshot = runtime_snapshot(args.provider_url, embedding_dimension=384)
                snapshot["llm"]["model"] = "a4n-checkpoint-fail-" + fault if fault else "a4n-checkpoint-valid"
                secure_json(work / "runtime.json", snapshot)
                checked(cli, ["config", "runtime", "set", "--input", "@" + str(work / "runtime.json")], "runtime")

            def calls():
                return [entry for entry in provider_log_entries(args.provider_log) if isinstance(entry.get("body", {}).get("messages"), list)]

            def task(handle, expected=None):
                deadline = time.monotonic() + 60
                while time.monotonic() < deadline:
                    state = checked(cli, ["task", "get", handle["task_id"], "--attempt", str(handle["attempt"])], "task result")[0]["data"]
                    status = state["handle"]["status"]
                    if status in {"completed", "failed", "cancelled", "interrupted"}:
                        if expected:
                            assert status == "failed" and expected in json.dumps(state, ensure_ascii=False), state
                        else:
                            assert status == "completed", state
                        return state
                    time.sleep(0.1)
                raise RunnerError("fixture task did not terminate")

            def new_project(title):
                project = checked(cli, ["project", "create", "--title", title, "--project-type", "novel_long", "--use-context"], "project")[0]["data"]
                report["projects"].append(project["id"])
                return project["id"]

            def shooting_source(title, unit_count):
                project_id = new_project(title)
                chapter = checked(cli, ["chapter", "import", "--number", "1", "--title", "雨夜来信", "--content", chapter_body_response()], "chapter")[0]["data"]
                chapter_id = chapter["id"]
                scenes = [call("create_scene_profile", {"project_id": project_id, "profile": {
                    "name": name, "scene_type": "room", "importance": "primary", "base_description": name,
                    "visual_description": "门与挂钟形成清晰关系", "spatial_layout": "通道通向机房", "atmosphere": "安静",
                    "first_seen_chapter": 1, "last_seen_chapter": 1, "image_prompt_seed": name,
                }})["profile"] for name in ("旧车站通道", "地下机房")[:unit_count]]
                image = call("upload_image", {"project_id": project_id, "file_name": "checkpoint.png", "mime_type": "image/png",
                    "data_base64": PNG_1X1_BASE64, "image_type": "chapter_reference", "prompt": "测试参考图",
                    "chapter_id": chapter_id, "panel_id": None, "metadata_json": None})
                source = {"project_id": project_id, "source_chapter_id": chapter_id}
                plan = call("get_chapter_visual_plan", source)["starter_plan"]
                plan["points"] = [{"id": f"point-{index}", "at_ms": index * 5000, "label": "边界", "description": "门口画面",
                    "image_id": image["id"] if index == 0 else None, "reference_image_ids": []} for index in range(unit_count + 1)]
                plan["segments"] = [{"id": f"shot-{index + 1}", "title": "进入机房", "start_point_id": f"point-{index}", "end_point_id": f"point-{index + 1}",
                    "prompt": "林岚留下齿轮暗记，进入地下机房。", "scene_binding": {"kind": "scene", "scene_profile_id": scenes[index]["id"]},
                    "use_end_point_image": False} for index in range(unit_count)]
                draft = call("save_chapter_visual_plan_draft", {**source, "authoring_mode": "manual", "record_id": None, "expected_revision": None, "plan": plan})
                confirmed = call("confirm_chapter_visual_plan", {**source, "record_id": draft["record"]["id"], "expected_revision": draft["record"]["revision"]}, True)
                return {**source, "visual_plan_id": confirmed["visual_plan"]["record"]["id"]}, chapter_id

            def detail(project_id, kind):
                runs = call("list_workflow_runs", {"project_id": project_id, "workflow_kind": kind, "chapter_id": None, "status": None, "resumable_only": False, "limit": 20})
                latest = max(runs, key=lambda run: run["id"])
                return call("get_workflow_run_detail", {"workflow_run_id": latest["id"], "include_generation_runs": False})

            model()
            for fault, expected, prefix in [("shooting-layout", "slugline", 0), ("shooting-adaptation", "actor", 1), ("shooting-design", "lens_intent", 2)]:
                source, _ = shooting_source(fault, 2)
                model(fault)
                before = len(calls())
                error = reject("generate_chapter_shooting_script", source, expected)
                assert len(calls()) - before == prefix + 1, (fault, len(calls()) - before)
                failed = detail(source["project_id"], "chapter_shooting_script")
                assert sum(item["status"] == "completed" for item in failed["checkpoints"]) == prefix, failed
                model()
                before = len(calls())
                result = call("generate_chapter_shooting_script", source)
                assert len(result["script"]["scenes"]) == 2, result
                assert len(calls()) - before == 6 - prefix, (fault, len(calls()) - before)
                report["checks"].append({"fault": fault, "error": error, "reused_calls": prefix, "retry_calls": 6 - prefix})

            project_id = new_project("blueprint checkpoint boundary")
            task(call("start_inspiration_generation", {"meta": {"start_request_id": str(uuid.uuid4())}, "project_id": project_id,
                "message": "写一部六章都市时间悬疑小说：暴雨夜、未来来信、旧车站与每天消失的七分钟。"}))
            blueprint = {"project_id": project_id, "title": "逆行时钟", "genre": "都市时间悬疑", "target_length": 6}
            model("blueprint-characters")
            before = len(calls())
            first = call("start_blueprint_generation", {**blueprint, "meta": {"start_request_id": str(uuid.uuid4())}})
            task(first, "角色重复")
            assert len(calls()) - before == 3, len(calls()) - before
            model()
            before = len(calls())
            task(call("start_blueprint_generation", {**blueprint, "meta": {"start_request_id": str(uuid.uuid4()), "retry_of_task_id": first["task_id"]}}))
            assert len(calls()) - before == 2, len(calls()) - before
            report["checks"].append({"fault": "blueprint-characters", "reused_calls": 2, "retry_calls": 2})

            source, chapter_id = shooting_source("derived media checkpoint boundary", 1)
            call("generate_chapter_shooting_script", source)
            for name in ("林岚", "周衡"):
                call("create_profile", {"project_id": source["project_id"], "name": name,
                    "base_attributes": {"explicit_attributes": {}, "implicit_attributes": {}, "social_attributes": {}}})

            def derived(kind, title):
                project = call("create_derived_project", {"root_project_id": source["project_id"], "derived_kind": kind, "title": title,
                    "description": "isolated checkpoint fixture", "start_chapter": 1, "end_chapter": 1})
                report["projects"].append(project["id"])
                return project["id"]

            for fault, expected, prefix in [("galgame-branch", "after_beat_ref", 0), ("galgame-script", "拍摄节拍", 1), ("galgame-staging", "原剧本行", 2), ("galgame-visual", "scene_index", 3)]:
                project_id = derived("galgame", fault)
                value = {"project_id": project_id, "task_id": str(uuid.uuid4()), "resume_from_checkpoint": True, "start_from_stage": None}
                model(fault)
                before = len(calls())
                error = reject("start_galgame_pipeline", value, expected)
                assert len(calls()) - before == prefix + 1, (fault, len(calls()) - before)
                model()
                before = len(calls())
                call("start_galgame_pipeline", {**value, "task_id": str(uuid.uuid4())})
                assert len(calls()) - before == 4 - prefix, (fault, len(calls()) - before)
                report["checks"].append({"fault": fault, "error": error, "reused_calls": prefix, "retry_calls": 4 - prefix})

            def manga_result(project_id, expected=None):
                deadline = time.monotonic() + 60
                while time.monotonic() < deadline:
                    result = call("get_manga_pipeline_status", {"project_id": project_id, "chapter_id": chapter_id})
                    if result["pipeline_status"] in {"completed", "failed", "cancelled"}:
                        assert result["pipeline_status"] == ("failed" if expected else "completed"), result
                        if expected:
                            assert expected in json.dumps(result, ensure_ascii=False), result
                        return result
                    time.sleep(0.1)
                raise RunnerError("manga fixture did not terminate")

            for fault, prefix in [("manga-speech", 1), ("manga-visual", 2), ("manga-items", 3)]:
                manga_source, chapter_id = shooting_source(fault, 1)
                call("generate_chapter_shooting_script", manga_source)
                project_id = manga_source["project_id"]
                value = {"project_id": project_id, "chapter_number": 1}
                model(fault)
                before = len(calls())
                call("start_manga_pipeline", value)
                manga_result(project_id, "不存在的拍摄节拍")
                assert len(calls()) - before == prefix + 1, (fault, len(calls()) - before)
                model()
                before = len(calls())
                call("resume_manga_pipeline", value)
                manga_result(project_id)
                pages = call("list_manga_page_prompts", {"project_id": project_id, "chapter_id": chapter_id})
                assert pages and len(calls()) - before == 6 + len(pages) - prefix, (fault, len(calls()) - before, len(pages))
                report["checks"].append({"fault": fault, "reused_calls": prefix, "retry_calls": len(calls()) - before})

            value = {"prompt_id": pages[0]["id"], "instruction": "增强画面构图", "downstream_strategy": "reject"}
            model("manga-optimization")
            reject("optimize_manga_prompt", value, "image_prompt")
            model()
            call("optimize_manga_prompt", value)
            optimized = detail(project_id, "manga_prompt_optimization")
            assert optimized["run"]["status"] == "completed", optimized
            assert len(optimized["checkpoints"]) == 1 and optimized["checkpoints"][0]["status"] == "completed", optimized
            report["checks"].append({"fault": "manga-optimization", "saved_calls": 1})

            permissions = {"read_project_detail": True, "read_blueprint": True, "read_outlines": True,
                "read_character_profiles": True, "read_foreshadowing": True, "automatic_writeback": False}
            session = call("create_optimization_analysis_session", {"project_id": project_id, "chapter_number": 1,
                "name": "checkpoint validation", "mode": "quick", "permissions": permissions})
            model("optimization-final")
            before = len(calls())
            first = call("start_optimize_analysis_generation", {"session_id": session["id"], "meta": {"start_request_id": str(uuid.uuid4())}})
            task(first, "missing-span")
            failed_calls = len(calls()) - before
            failed = detail(project_id, None)
            assert sum(item["status"] == "completed" for item in failed["checkpoints"]) == failed_calls - 1, failed
            model()
            before = len(calls())
            task(call("start_optimize_analysis_generation", {"session_id": session["id"], "meta": {
                "start_request_id": str(uuid.uuid4()), "retry_of_task_id": first["task_id"]}}))
            assert len(calls()) - before == 1, len(calls()) - before
            report["checks"].append({"fault": "optimization-final", "reused_calls": failed_calls - 1, "retry_calls": 1})
            before = len(calls())
            task(call("start_optimize_analysis_generation", {"session_id": session["id"], "meta": {"start_request_id": str(uuid.uuid4())}}), "waiting_user")
            assert len(calls()) == before, "a fresh start must still require prepared analysis input"
            model("optimization-patch")
            first = call("start_optimize_patch_generation", {"request": {"session_id": session["id"]}, "meta": {"start_request_id": str(uuid.uuid4())}})
            task(first, "old_text")
            model()
            before = len(calls())
            task(call("start_optimize_patch_generation", {"request": {"session_id": session["id"]}, "meta": {
                "start_request_id": str(uuid.uuid4()), "retry_of_task_id": first["task_id"]}}))
            assert len(calls()) - before == 1, len(calls()) - before
            report["checks"].append({"fault": "optimization-patch", "retry_calls": 1})
            before_images = len(provider_log_entries(args.provider_log))
            image_result = task(call("start_image_generation", {"project_id": project_id,
                "meta": {"start_request_id": str(uuid.uuid4())}, "operation": {
                    "kind": "scene", "prompt": "安静的室内场景，清晰的门口与通道", "config_id": 900002,
                    "chapter_id": chapter_id, "width": 1024, "height": 1024}}))
            image_calls = [entry for entry in provider_log_entries(args.provider_log)[before_images:]
                           if entry["path"].endswith(("/images/generations", "/images/edits"))]
            assert len(image_calls) == 1 and image_result["result_ref"] is not None, image_result
            report["checks"].append({"case": "image-call", "provider_calls": 1})
            report["success"] = True
    except Exception as error:
        report["error"] = str(error)
        raise
    finally:
        args.report.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
        print(json.dumps({"success": report["success"], "checks": len(report["checks"]), "report": str(args.report)}), flush=True)


if __name__ == "__main__":
    main()
