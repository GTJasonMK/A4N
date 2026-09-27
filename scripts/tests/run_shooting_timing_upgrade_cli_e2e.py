#!/usr/bin/env python3
"""Prepare with a real v1 Server, then verify the same data with the current Server.

Run --phase prepare against an isolated legacy stack whose provider fails shooting
shot-2 and video frozen-2, after saving earlier calls in the same scene.
Stop it, run the current migration, and restart with the same configuration and a
healthy provider before --phase verify. State contains private model evidence.
"""

import base64
import copy
import io
import json
import uuid
import zipfile
from pathlib import Path

from mock_openai.protocol import PNG_1X1_BASE64
from run_cli_contract_e2e import checked, cli_e2e_parser, command_error, logged_in_cli, provider_log_entries, runtime_snapshot, secure_json


def manifest(data):
    with zipfile.ZipFile(io.BytesIO(base64.b64decode(data["data_base64"]))) as archive:
        return json.loads(archive.read("a4n-project-bundle.json"))


def without_estimates(script):
    value = copy.deepcopy(script)
    value["schema_version"] = "chapter_shooting_script_v2"
    for scene in value["scenes"]:
        for shot in scene["shots"]:
            del shot["estimated_natural_seconds"]
            for beat in shot["performance_beats"]:
                del beat["estimated_seconds"]
    return value


def main():
    parser = cli_e2e_parser(__doc__)
    parser.add_argument("--phase", choices=("prepare", "verify"), required=True)
    parser.add_argument("--state-dir", type=Path, required=True)
    args = parser.parse_args()
    args.state_dir.mkdir(parents=True, exist_ok=True)
    report = {"success": False, "checks": []}
    try:
        cli = logged_in_cli(args, args.state_dir)
        secure_json(args.state_dir / "runtime.json", runtime_snapshot(args.provider_url))
        checked(cli, ["config", "runtime", "set", "--input", "@" + str(args.state_dir / "runtime.json")], "runtime")
        described = set()

        def call(name, value, *, raw=False, destructive=False, failure=None):
            if name not in described:
                checked(cli, ["capabilities", "describe", name], name + " contract")
                described.add(name)
            command = ["call", name, "--args", "-"]
            if destructive:
                command.append("--allow-destructive")
            encoded = json.dumps(value if raw else {"input": value}, ensure_ascii=False)
            if failure:
                result = cli.run(command, encoded)
                message = str(command_error(result).get("message", ""))
                assert result["exit_code"] not in (0, 7, 10, 11, 124) and failure in message, (name, result["exit_code"], message)
                return result
            return checked(cli, command, name, encoded)[0]["data"]

        def terminal(handle, status="completed"):
            watched = cli.run(["task", "watch", handle["task_id"], "--attempt", str(handle["attempt"])])
            task = checked(cli, ["task", "get", handle["task_id"], "--attempt", str(handle["attempt"])], "task")[0]["data"]
            assert watched["exit_code"] == {"completed": 0, "failed": 8}[status], task
            assert task["handle"]["status"] == status, task
            return task

        def export(project_id):
            return call("export_project_bundle_data", {"project_id": project_id, "include_derived_projects": True})

        def plan_video(project_id, chapter_id, status="completed"):
            return terminal(call("start_video_generation", {"project_id": project_id, "meta": {"start_request_id": str(uuid.uuid4())}, "operation": {
                "kind": "prompt_segment_plan", "chapter_id": chapter_id, "options": {
                    "workflow_mode": "shared_segment_plan", "style": "电影感", "prompt_output_language": "zh_cn",
                    "prompt_batch_size": 1, "duration_secs": 6, "duration_secs_options": [6], "aspect_ratio": "16:9",
                    "reference_image_ids": [], "video_library_image_ids": [], "regenerate_existing": False,
                    "target_narrative_unit_index": 1,
                },
            }}), status)

        state_path = args.state_dir / "legacy-state.json"
        if args.phase == "prepare":
            assert not state_path.exists(), "Use a fresh isolated state directory"
            project = checked(cli, ["project", "create", "--title", "估时删除：旧版本真实断点", "--project-type", "novel_long", "--use-context"], "project")[0]["data"]
            chapter = checked(cli, ["chapter", "import", "--number", "1", "--title", "门与机房", "--content", "林岚停在旧车站通道入口。她观察墙上的钟，推开门，进入地下机房。周衡留在门外。"], "chapter")[0]["data"]
            resource = {"project_id": project["id"], "source_chapter_id": chapter["id"]}
            scenes = [call("create_scene_profile", {"project_id": project["id"], "profile": {
                "name": name, "scene_type": "room", "importance": "primary", "base_description": name,
                "visual_description": "门框与挂钟", "spatial_layout": "通道通向门口", "atmosphere": "安静",
                "first_seen_chapter": 1, "last_seen_chapter": 1, "image_prompt_seed": name,
            }})["profile"] for name in ("旧车站通道", "地下机房")]
            image = call("upload_image", {"project_id": project["id"], "file_name": "kept.png", "mime_type": "image/png",
                "data_base64": PNG_1X1_BASE64, "image_type": "chapter_reference", "prompt": "保留的参考图",
                "chapter_id": chapter["id"], "panel_id": None, "metadata_json": None})
            plan = call("get_chapter_visual_plan", resource)["starter_plan"]
            plan["points"] = [{"id": f"p-{i}", "at_ms": i * 6000, "label": f"p-{i}", "image_id": image["id"] if i == 0 else None,
                "description": "当前场景", "reference_image_ids": []} for i in range(5)]
            plan["segments"] = [{"id": shot, "title": shot, "start_point_id": f"p-{i}", "end_point_id": f"p-{i+1}",
                "prompt": prompt, "scene_binding": {"kind": "scene", "scene_profile_id": scenes[0 if i < 2 else 1]["id"]}, "use_end_point_image": False}
                for i, (shot, prompt) in enumerate((("frozen-1", "林岚观察墙上的挂钟。"), ("frozen-2", "林岚走到门口。"), ("shot-middle", "林岚站在地下机房内。"), ("shot-2", "林岚回头看向门外的周衡。")))]
            draft = call("save_chapter_visual_plan_draft", {**resource, "record_id": None, "expected_revision": None, "authoring_mode": "manual", "plan": plan})
            confirmed = call("confirm_chapter_visual_plan", {**resource, "record_id": draft["record"]["id"], "expected_revision": draft["record"]["revision"]}, destructive=True)["visual_plan"]
            source = {**resource, "visual_plan_id": confirmed["record"]["id"], "shot_ids": []}
            terminal(call("start_chapter_shooting_script_generation", {**source, "mode": "full", "meta": {"start_request_id": str(uuid.uuid4())}}), "failed")
            partial = call("get_chapter_shooting_script_unit_progress", resource)
            assert partial["completed_unit_count"] == 1 and partial["total_unit_count"] == 2, partial
            assert partial["script"]["schema_version"] == "chapter_shooting_script_v1"
            edited = copy.deepcopy(partial["script"])
            edited["scenes"][0]["title"] = "作者已修改的第一场，升级后保留"
            partial = call("edit_chapter_shooting_script_progress", {**resource, "workflow_run_id": partial["workflow_run_id"], "expected_script": partial["script"], "script": edited})["progress"]
            video = call("create_derived_project", {"root_project_id": project["id"], "derived_kind": "video", "title": "冻结首场",
                "description": "估时迁移回归", "start_chapter": 1, "end_chapter": 1})
            frozen = call("get_chapter_shooting_script", {**resource, "project_id": video["id"]})
            plan_video(video["id"], chapter["id"], "failed")
            package = export(project["id"])
            family_package = call("export_project_family_data", {"root_project_id": project["id"], "include_derived_projects": True})
            old = manifest(package)
            completed = [row for row in old["history"]["workflow_checkpoints"] if row["status"] == "completed"]
            assert any(row["step_key"] == "shooting_design_shot:2:shot-middle" for row in completed)
            assert any(row["step_key"].startswith("video_segment_plan.shot.") for row in completed)
            secure_json(state_path, {"source": source, "video_id": video["id"], "partial": partial, "frozen": frozen, "package": package, "family_package": family_package})
            report["checks"].append("legacy Server saved author edits, frozen script, media and completed shooting/video calls before failing the next shot in each workflow")
        else:
            state = json.loads(state_path.read_text())
            source = state["source"]
            resource = {key: source[key] for key in ("project_id", "source_chapter_id")}
            partial = call("get_chapter_shooting_script_unit_progress", resource)
            assert partial["script"] == without_estimates(state["partial"]["script"])
            frozen = call("get_chapter_shooting_script", {**resource, "project_id": state["video_id"]})
            assert frozen["script"] == without_estimates(state["frozen"]["script"])
            assert frozen["record"]["id"] == state["frozen"]["record"]["id"] and frozen["record"]["revision"] == state["frozen"]["record"]["revision"]
            current_package = export(source["project_id"])
            upgraded = manifest(current_package)
            old = manifest(state["package"])
            assert upgraded["family"]["version"] == "project_family_v7"
            assert upgraded["family"]["generated_images"] == old["family"]["generated_images"]
            for before in old["history"]["workflow_checkpoints"]:
                if before["status"] != "completed" or not before["step_key"].startswith("shooting_design_shot:"):
                    continue
                after = next(row for row in upgraded["history"]["workflow_checkpoints"] if row["id"] == before["id"])
                receipt = after["artifacts_json"]["shooting_timing_v2"]
                assert receipt["original_checkpoint"] == before["checkpoint_json"]
                assert receipt["original_input_fingerprint_json"] == before["input_fingerprint_json"]
            report["checks"].append("migration preserves script edits, frozen revisions, media and original call evidence")
            before = len(provider_log_entries(args.provider_log))
            terminal(call("start_chapter_shooting_script_generation", {**source, "mode": "full", "meta": {"start_request_id": str(uuid.uuid4())}}))
            requests = provider_log_entries(args.provider_log)[before:]
            chats = [row for row in requests if row["path"].endswith("/chat/completions")]
            assert len(chats) == 1 and "target_shot_id=shot-2" in json.dumps(chats[0]["body"], ensure_ascii=False), [row["body"].get("model") for row in chats]
            final = call("get_chapter_shooting_script", resource)
            assert final["script"]["scenes"][0] == partial["script"]["scenes"][0]
            before = len(provider_log_entries(args.provider_log))
            plan_video(state["video_id"], source["source_chapter_id"])
            chats = [row for row in provider_log_entries(args.provider_log)[before:] if row["path"].endswith("/chat/completions")]
            assert len(chats) == 1 and "frozen-2" in json.dumps(chats[0]["body"], ensure_ascii=False), len(chats)
            before = len(provider_log_entries(args.provider_log))
            plan_video(state["video_id"], source["source_chapter_id"])
            assert len(provider_log_entries(args.provider_log)) == before, "unchanged frozen segment planning repeated provider calls"
            report["checks"].append("current CLI resumes only the missing shooting/video shots; repeating the completed segment plan makes no provider calls")
            imported = call("import_project_bundle_upload", {key: state["package"][key] for key in ("file_name", "data_base64")})
            root = next(row for row in imported["project_summaries"] if row["project_kind"] == "text")
            remapped = manifest(export(root["id"]))
            assert remapped["family"]["version"] == "project_family_v7"
            assert len(remapped["family"]["generated_images"]) == len(old["family"]["generated_images"])
            for row in remapped["history"]["workflow_checkpoints"]:
                receipt = (row.get("artifacts_json") or {}).get("shooting_timing_v2")
                if receipt:
                    original = next(candidate for candidate in old["history"]["workflow_checkpoints"] if candidate["step_key"] == row["step_key"])
                    assert receipt["original_checkpoint"] == original["checkpoint_json"]
                    assert receipt["original_input_fingerprint_json"] == original["input_fingerprint_json"]
            # Cross-project runtime ownership follows the existing bundle contract.
            # This upgrade preserves its history; it does not take over an old task.
            assert all(row["active_task_id"] is None for row in remapped["history"]["workflow_runs"])
            report["checks"].append("a real legacy bundle imports as v7, preserves original calls and media, and clears old runtime task ownership")
            projects_before = checked(cli, ["project", "list"], "projects before invalid imports")[0]["data"]
            for field, expected_error in (("input", "原始输入指纹不一致"), ("receipt", "升级后的结构化输出不一致")):
                broken = copy.deepcopy(upgraded)
                checkpoint = next(row for row in broken["history"]["workflow_checkpoints"]
                    if (row.get("artifacts_json") or {}).get("shooting_timing_v2"))
                if field == "input":
                    checkpoint["input_fingerprint_sha256"] = "0" * 64
                else:
                    checkpoint["artifacts_json"]["shooting_timing_v2"]["upgraded_parsed_output_sha256"] = "0" * 64
                output = io.BytesIO()
                with zipfile.ZipFile(io.BytesIO(base64.b64decode(current_package["data_base64"]))) as original, zipfile.ZipFile(output, "w") as archive:
                    for entry in original.infolist():
                        data = json.dumps(broken, ensure_ascii=False).encode() if entry.filename == "a4n-project-bundle.json" else original.read(entry)
                        archive.writestr(entry, data)
                call("import_project_bundle_upload", {"file_name": current_package["file_name"], "data_base64": base64.b64encode(output.getvalue()).decode()}, failure=expected_error)
                assert checked(cli, ["project", "list"], "projects after invalid import")[0]["data"] == projects_before
            report["checks"].append("corrupt checkpoint input hashes and upgrade receipts are rejected without leaving imported projects")
            for version in ("project_family_v6", "project_family_v5"):
                raw = io.BytesIO(base64.b64decode(state["family_package"]["data_base64"]))
                output = io.BytesIO()
                with zipfile.ZipFile(raw) as original, zipfile.ZipFile(output, "w") as archive:
                    for name in original.namelist():
                        data = original.read(name)
                        if name == "a4n-project-family.json":
                            family = json.loads(data)
                            assert family["version"] == "project_family_v6"
                            family["version"] = version
                            if version == "project_family_v5":
                                family["text_core"].pop("fact_feedback", None)
                            data = json.dumps(family, ensure_ascii=False).encode()
                        archive.writestr(name, data)
                projects = call("import_project_family_upload", {"file_name": state["family_package"]["file_name"], "data_base64": base64.b64encode(output.getvalue()).decode()})
                family = manifest(export(next(row["id"] for row in projects if row["project_kind"] == "text")))["family"]
                assert family["version"] == "project_family_v7" and family["chapter_shooting_scripts"]
                assert all(row["script_json"]["schema_version"] == "chapter_shooting_script_v2" for row in family["chapter_shooting_scripts"])
            report["checks"].append("the separate family import endpoint upgrades v6 and the v5 format without losing frozen scripts")
        report["success"] = True
    except Exception as error:
        report["error"] = str(error)
        raise
    finally:
        secure_json(args.report, report)
        print(json.dumps({"success": report["success"], "checks": report["checks"]}, ensure_ascii=False))


if __name__ == "__main__":
    main()
