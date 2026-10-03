#!/usr/bin/env python3
"""Verify director points/segments through the current CLI, real Host and mock Provider."""

from __future__ import annotations

import base64
import copy
import hashlib
import io
import json
import os
import subprocess
import tempfile
import time
import traceback
import uuid
import zipfile
from pathlib import Path

from mock_openai.protocol import PNG_1X1_BASE64
from mock_openai.prompt_parsing import json_task_payload
from run_agent_cli_e2e import data
from run_cli_contract_e2e import RunnerError, checked, cli_e2e_parser, command_error, logged_in_cli, provider_log_entries, runtime_snapshot, secure_json


def main():
    parser = cli_e2e_parser(__doc__)
    args = parser.parse_args()
    args.report.parent.mkdir(parents=True, exist_ok=True)
    report = {"success": False, "checks": [], "commands": []}
    try:
        with tempfile.TemporaryDirectory(prefix="director-v3-", dir=args.report.parent) as directory:
            work = Path(directory)
            cli = logged_in_cli(args, work)

            def configure(model="a4n-virtual-text"):
                snapshot = runtime_snapshot(args.provider_url)
                snapshot["llm"]["model"] = model
                snapshot["image"]["supported_sizes"] = [{"width": 1024, "height": 576}]
                secure_json(work / "runtime.json", snapshot)
                checked(cli, ["config", "runtime", "set", "--input", "@" + str(work / "runtime.json")], "runtime")

            configure()
            described = set()

            def call(name, value, *, raw=False, destructive=False, failure=None):
                if name not in described:
                    checked(cli, ["capabilities", "describe", name], name + " contract")
                    described.add(name)
                payload = value if raw else {"input": value}
                parts = ["call", name, "--args", "-"]
                if destructive:
                    parts.append("--allow-destructive")
                encoded = json.dumps(payload, ensure_ascii=False)
                if failure:
                    result = cli.run(parts, encoded)
                    error = command_error(result)
                    message = str(error.get("message", ""))
                    assert result["exit_code"] not in (0, 7, 10, 11, 124) and failure in message, (name, result["exit_code"], message)
                    if name == "start_chapter_shooting_script_generation":
                        assert error["code"] == "generation_request_invalid" and error["retryable"] is False, error
                    report["commands"].append({"name": name, "expected_rejection": message})
                    return result
                result = checked(cli, parts, name, encoded)[0]["data"]
                report["commands"].append({"name": name, "status": "passed"})
                return result

            def summary(handle):
                return checked(cli, ["task", "get", handle["task_id"], "--attempt", str(handle["attempt"])], "task summary")[0]["data"]

            def complete(handle):
                try:
                    checked(cli, ["task", "watch", handle["task_id"], "--attempt", str(handle["attempt"])], "task watch")
                except RunnerError as error:
                    raise RunnerError(f"{error}: {json.dumps(summary(handle)['terminal_event'], ensure_ascii=False)}") from error
                result = summary(handle)
                assert result["handle"]["status"] == "completed", result
                return result

            def meta():
                return {"start_request_id": str(uuid.uuid4())}

            def shooting(source, mode="full"):
                task = complete(call("start_chapter_shooting_script_generation", {**source, "mode": mode, "meta": meta()}))
                reference = task["result_ref"]
                command = {
                    "chapter_shooting_script": "get_chapter_shooting_script",
                    "chapter_shooting_script_progress": "get_chapter_shooting_script_unit_progress",
                }[reference["kind"]]
                result = call(command, {"project_id": source["project_id"], "source_chapter_id": source["source_chapter_id"]})
                result_id = result["record"]["id"] if "record" in result else result["workflow_run_id"]
                assert str(result_id) == reference["id"]
                return result

            project = data(checked(cli, ["project", "create", "--title", "导演时间线 V3 隔离验证", "--project-type", "novel_long", "--use-context"], "project")[0], "project")
            chapter = data(checked(cli, ["chapter", "import", "--number", "1", "--title", "门与通道", "--content", "林岚停在旧车站通道入口。她观察墙上的钟，推开门，进入地下机房。周衡留在门外。"], "chapter")[0], "chapter")
            root_project_id, chapter_id = project["id"], chapter["id"]
            scenes = [call("create_scene_profile", {"project_id": root_project_id, "profile": {
                "name": name, "scene_type": "room", "importance": "primary", "base_description": name,
                "visual_description": "木门、挂钟与清晰的通道", "spatial_layout": "通道通向门口",
                "atmosphere": "安静", "first_seen_chapter": 1, "last_seen_chapter": 1, "image_prompt_seed": name,
            }})["profile"] for name in ("旧车站通道", "地下机房")]
            video_project = call("create_derived_project", {
                "root_project_id": root_project_id, "derived_kind": "video", "title": "导演时间线 V3 隔离验证 视频",
                "description": "director and video generation fixture", "start_chapter": 1, "end_chapter": 1,
            })
            project_id = video_project["id"]
            resource = {"project_id": project_id, "source_chapter_id": chapter_id}
            report.update(root_project_id=root_project_id, project_id=project_id, chapter_id=chapter_id)
            image = call("upload_image", {"project_id": project_id, "file_name": "director-reference.png", "mime_type": "image/png",
                "data_base64": PNG_1X1_BASE64, "image_type": "chapter_reference", "prompt": "门口的分镜参考",
                "chapter_id": chapter_id, "panel_id": None, "metadata_json": None})
            plan = call("get_chapter_visual_plan", resource)["starter_plan"]
            plan["points"] = [{"id": point_id, "at_ms": at_ms, "label": point_id, "image_id": image["id"] if at_ms == 0 else None,
                "description": "门口的静态画面", "reference_image_ids": [image["id"]] if point_id == "inside" else []}
                for point_id, at_ms in [("start", 0), ("inside", 1000), ("cut", 3000), ("arrive", 5000), ("end", 9000)]]
            bindings = [{"kind": "scene", "scene_profile_id": scenes[0]["id"]},
                {"kind": "transition", "from_scene_profile_id": scenes[0]["id"], "to_scene_profile_id": scenes[1]["id"]},
                {"kind": "scene", "scene_profile_id": scenes[1]["id"]}]
            plan["segments"] = [{"id": f"shot-{i+1}", "title": f"镜头 {i+1}", "start_point_id": start, "end_point_id": end,
                "prompt": prompt, "scene_binding": bindings[i], "use_end_point_image": False}
                for i, (start, end, prompt) in enumerate([("start", "cut", "林岚观察挂钟，缓缓走到门口。"),
                    ("cut", "arrive", "开门后从旧车站通道进入地下机房。"), ("arrive", "end", "林岚停在机房内，回头看向门外。")])]
            draft = call("save_chapter_visual_plan_draft", {**resource, "authoring_mode": "manual", "record_id": None, "expected_revision": None, "plan": plan})

            def record_input():
                return {**resource, "record_id": draft["record"]["id"], "expected_revision": draft["record"]["revision"]}

            def edit(operation):
                return call("edit_chapter_visual_plan_draft", {**record_input(), "operation": operation})["visual_plan"]

            unchanged = copy.deepcopy(draft)
            draft = edit({"kind": "insert_point", "point": {"id": "additional", "at_ms": 2000, "label": "段内图片点", "image_id": None, "description": "停在门前", "reference_image_ids": []}})
            assert len(draft["plan"]["segments"]) == 3 and len(draft["plan"]["points"]) == 6
            call("edit_chapter_visual_plan_draft", {**resource, "record_id": draft["record"]["id"], "expected_revision": unchanged["record"]["revision"], "operation": {"kind": "scale_timeline", "duration_ms": 18000}}, failure="topology")
            assert call("get_chapter_visual_plan", resource)["draft"]["record"]["revision"] == draft["record"]["revision"]
            report["checks"].append("inserting points preserves shots; stale writes fail")

            baseline = copy.deepcopy(draft)
            text_handle = call("start_visual_plan_prompt_generation", {**record_input(), "meta": meta(), "segment_id": "shot-1", "user_description": "镜头缓缓推近，保留停顿。"})
            result = complete(text_handle)["result_ref"]
            candidate = call("read_visual_plan_prompt_candidate", {"project_id": project_id, "generation_run_id": int(result["id"])})
            assert candidate["plan_sha256"] == baseline["record"]["plan_sha256"] and candidate["prompt"]
            assert hashlib.sha256(json.dumps(candidate, sort_keys=True, ensure_ascii=False, separators=(",", ":")).encode()).hexdigest() == result["revision"]
            current_draft = call("get_chapter_visual_plan", resource)["draft"]
            assert current_draft["plan"] == baseline["plan"], current_draft
            assert current_draft["record"]["revision"] == baseline["record"]["revision"], current_draft["record"]
            draft = edit({"kind": "update_segment", "segment": {**draft["plan"]["segments"][0], "prompt": candidate["prompt"]}})
            report["checks"].append("text generation publishes a readable candidate; only explicit adoption changes the plan")

            baseline = copy.deepcopy(draft)
            options = {"source_chapter_id": chapter_id, "record_id": draft["record"]["id"], "expected_revision": draft["record"]["revision"],
                "point_id": "inside", "user_description": "沿用参考画面，人物停在木门前。", "config_id": 900002}
            preview = call("preview_visual_plan_point_generation", {"project_id": project_id, "options": options})
            assert preview["reference_image_ids"] == [image["id"]] and preview["aspect_ratio"] == "16:9"
            before_images = len(provider_log_entries(args.provider_log))
            complete(call("start_image_generation", {"project_id": project_id, "meta": meta(), "operation": {"kind": "visual_plan_point", "options": options}}))
            generated = next(row for row in call("list_images_by_project", {"project_id": project_id})
                if (row.get("metadata_json") or {}).get("director_candidate", {}).get("point_id") == "inside")
            current_draft = call("get_chapter_visual_plan", resource)["draft"]
            assert current_draft["plan"] == baseline["plan"], current_draft
            assert current_draft["record"]["revision"] == baseline["record"]["revision"], current_draft["record"]
            requests = provider_log_entries(args.provider_log)[before_images:]
            assert any(row["path"].endswith("/images/edits") and row["body"].get("has_reference_image") for row in requests), requests
            point = next(point for point in draft["plan"]["points"] if point["id"] == "inside")
            draft = edit({"kind": "update_point", "point": {**point, "image_id": generated["id"]}})
            report["checks"].append("image generation sends actual reference bytes and adopts only on request")

            negative = "不要出现绿色圆形路牌"
            image_request = {"prompt": "旧车站走廊空景", "negative_prompt": negative,
                "image_type": "scene", "chapter_id": chapter_id, "aspect_ratio": "16:9", "config_id": 900002}
            before_batch = len(provider_log_entries(args.provider_log))
            call("preview_batch_images_request", {"project_id": project_id, "requests": [{**image_request, "negative_prompt": 7}]},
                 failure="negative_prompt 必须是字符串")
            assert len(provider_log_entries(args.provider_log)) == before_batch
            complete(call("start_image_generation", {"project_id": project_id, "meta": meta(),
                "operation": {"kind": "batch", "requests": [image_request]}}))
            wire = [row["body"] for row in provider_log_entries(args.provider_log)[before_batch:]
                    if row["path"].endswith("/images/generations")]
            assert len(wire) == 1 and wire[0]["prompt"].count(negative) == 1, wire
            report["checks"].append("batch negative constraints reach the image provider exactly once; invalid types fail before provider I/O")

            configure("a4n-llm-lifecycle-slow")
            before_cancel = len(provider_log_entries(args.provider_log))
            handle = call("start_visual_plan_prompt_generation", {**record_input(), "meta": meta(), "segment_id": "shot-2", "user_description": "取消验证"})
            deadline = time.monotonic() + 10
            while len(provider_log_entries(args.provider_log)) == before_cancel and time.monotonic() < deadline:
                time.sleep(0.05)
            assert len(provider_log_entries(args.provider_log)) > before_cancel
            call("cancel_generation_task", {"task_id": handle["task_id"], "attempt": handle["attempt"]}, destructive=True)
            deadline = time.monotonic() + 10
            while time.monotonic() < deadline:
                cancelled = summary(handle)
                if cancelled["handle"]["status"] == "cancelled":
                    break
                time.sleep(0.1)
            assert cancelled["handle"]["status"] == "cancelled" and cancelled["result_ref"] is None
            current_draft = call("get_chapter_visual_plan", resource)["draft"]
            assert current_draft["plan"] == draft["plan"], current_draft
            assert current_draft["record"]["revision"] == draft["record"]["revision"], current_draft["record"]
            configure()
            report["checks"].append("cancellation stops a running model call without writing a candidate or plan")

            draft = edit({"kind": "scale_timeline", "duration_ms": 18000})
            assert [point["at_ms"] for point in draft["plan"]["points"]] == [0, 2000, 4000, 6000, 10000, 18000]
            confirmed = call("confirm_chapter_visual_plan", record_input(), destructive=True)["visual_plan"]
            assert confirmed["record"]["status"] == "confirmed"
            call("delete_image", {"image_id": image["id"]}, destructive=True, failure="导演编排引用")
            call("delete_scene_profile", {"scene_profile_id": scenes[0]["id"], "downstream_strategy": "reject"}, destructive=True, failure="导演编排引用")
            video_options = {"workflow_mode": "shared_segment_plan", "style": "电影感", "prompt_output_language": "zh_cn", "prompt_batch_size": 1,
                "duration_secs": 6, "duration_secs_options": [6], "aspect_ratio": "16:9", "reference_image_ids": [], "video_library_image_ids": [], "regenerate_existing": False}
            partial = shooting({**resource, "visual_plan_id": confirmed["record"]["id"]}, "next_scene")
            assert partial["completed_unit_count"] == 1 and partial["total_unit_count"] == 3
            browser_fixture = {"chapter": chapter, "partial": {"before": copy.deepcopy(partial), "plan": copy.deepcopy(confirmed)}}
            partial_edit = copy.deepcopy(partial["script"])
            partial_edit["scenes"][0]["title"] = "作者保留的场景标题"
            partial_edit["scenes"][0]["shots"][0]["composition"] = "作者保存的构图：门框在左侧，人物在右侧。"
            partial_edit["directing_statement"]["audience_experience"] = "作者保存的观众体验"
            partial = call("edit_chapter_shooting_script_progress", {**resource, "workflow_run_id": partial["workflow_run_id"], "expected_script": partial["script"], "script": partial_edit})["progress"]
            assert call("get_chapter_shooting_script_unit_progress", resource)["script"] == partial_edit
            complete(call("start_video_generation", {"project_id": project_id, "meta": meta(), "operation": {"kind": "prompt_segment_plan", "chapter_id": chapter_id, "options": {**video_options, "target_narrative_unit_index": 1}}}))
            partial_plan = next(row for row in call("list_chapter_video_prompts", {"projectId": project_id, "chapterId": chapter_id}, raw=True) if row["workflow_mode"] == "shared_segment_plan")
            assert partial_plan["metadata_json"]["stale_source_shot_ids"] == []
            before_resume = len(provider_log_entries(args.provider_log))
            script = shooting({**resource, "visual_plan_id": confirmed["record"]["id"]})
            assert script["script"]["scenes"][0] == partial_edit["scenes"][0]
            assert script["script"]["directing_statement"] == partial_edit["directing_statement"]
            assert call("get_chapter_shooting_script", resource)["script"] == script["script"]
            assert not any("target_shot_id=shot-1" in json.dumps(row.get("body", {}), ensure_ascii=False)
                for row in provider_log_entries(args.provider_log)[before_resume:])
            browser_fixture["partial"]["after"] = copy.deepcopy(script)
            report["checks"].append("saved partial scene and directing edits survive full continuation and reread without regenerating completed shots")
            shots = [shot for scene in script["script"]["scenes"] for shot in scene["shots"]]
            assert [shot["shot_id"] for shot in shots] == ["shot-1", "shot-2", "shot-3"]
            assert [(shot["planned_end_ms"] - shot["planned_start_ms"]) / 1000 for shot in shots] == [6, 4, 8]
            assert shots[0]["visual_constraints"]["end_frame"] is None
            assert [point["at_ms"] for point in shots[0]["visual_constraints"]["keyframes"]] == [0, 2000, 4000]
            report["checks"].append("confirmation permits missing images; script preserves shot IDs, timing, internal points and explicit end-frame semantics")

            complete(call("start_video_generation", {"project_id": project_id, "meta": meta(), "operation": {"kind": "prompt_segment_plan", "chapter_id": chapter_id, "options": video_options}}))
            rows = call("list_chapter_video_prompts", {"projectId": project_id, "chapterId": chapter_id}, raw=True)
            shared = next(row for row in rows if row["workflow_mode"] == "shared_segment_plan")
            segments = shared["metadata_json"]["segments"]
            assert shared["id"] == partial_plan["id"]
            assert segments[0] == partial_plan["metadata_json"]["segments"][0]
            assert shared["metadata_json"]["stale_source_shot_ids"] == []
            report["checks"].append("video planning from a saved partial script remains current after later scenes are completed")
            assert [segment["duration_secs"] for segment in segments] == [6, 4, 8]
            assert [segment["source_shot_id"] for segment in segments] == ["shot-1", "shot-2", "shot-3"]
            recipe = shared["metadata_json"]["segment_recipes"][0]
            assert set(recipe["reference_image_ids"]) == {image["id"], generated["id"]}
            assert recipe["workflow_mode"] == "first_frame_chain" and recipe["guidance_mode"] == "keyframes"
            assert recipe["guidance_reference_image_ids"] == recipe["reference_image_ids"]
            video_options.update(segment_plan_prompt_id=shared["id"], target_segment={"source_shot_id": "shot-1", "segment_index_in_shot": 1}, workflow_mode=recipe["workflow_mode"])
            complete(call("start_video_generation", {"project_id": project_id, "meta": meta(), "operation": {"kind": "prompt_videos", "chapter_id": chapter_id, "options": video_options}}))
            report["checks"].append("video planning retains per-shot duration and bound images; keyframe guidance reuses the bound director images")
            rows = call("list_chapter_video_prompts", {"projectId": project_id, "chapterId": chapter_id}, raw=True)
            final_prompt = next(row for row in rows if row["workflow_mode"] == recipe["workflow_mode"] and row["prompt_type"] == "video_prompt")
            assert final_prompt["metadata_json"]["start_first_frame"] is None
            assert final_prompt["metadata_json"]["first_frame_chain"]["previous_video_shot_number"] is None
            assert final_prompt["metadata_json"]["segment"]["director_constraints"]["end_frame"] is None
            complete(call("start_video_generation", {"project_id": project_id, "meta": meta(), "operation": {
                "kind": "chapter", "chapter_id": chapter_id, "prompt_id": final_prompt["id"], "config_id": 900003,
            }}))
            videos = call("list_videos_by_chapter", {"projectId": project_id, "chapterId": chapter_id}, raw=True)
            video = videos[0]
            for _ in range(20):
                video = call("poll_video_job", {"videoId": video["id"]}, raw=True)
                if video["status"] == "completed":
                    break
                assert video["status"] not in ("failed", "cancelled"), video
                time.sleep(0.2)
            assert video["status"] == "completed" and video["file_path"] and video["duration_secs"] == 6
            report["video_id"] = video["id"]
            report["checks"].append("a director shot generates and persists a video through the real provider request and polling path")

            call("select_segment_video", {"projectId": project_id, "chapterId": chapter_id, "videoId": video["id"],
                "input": {"segment_plan_prompt_id": shared["id"], "target_segment": {"source_shot_id": "shot-2", "segment_index_in_shot": 1}}},
                raw=True, destructive=True, failure="不属于当前规划片段")
            call("select_segment_video", {"projectId": project_id, "chapterId": chapter_id, "videoId": video["id"],
                "input": {"segment_plan_prompt_id": shared["id"], "target_segment": {"source_shot_id": "shot-1", "segment_index_in_shot": 1}}},
                raw=True, destructive=True)
            continuity_recipe = copy.deepcopy(shared["metadata_json"]["segment_recipes"][1])
            continuity_recipe.update(workflow_mode="first_frame_chain", guidance_mode="continuity",
                guidance_reference_image_ids=[], continuity_source_video_id=video["id"])
            call("update_chapter_video_segment_recipe", {"promptId": shared["id"], "recipe": continuity_recipe}, raw=True)
            before_continuity = len(provider_log_entries(args.provider_log))
            complete(call("start_video_generation", {"project_id": project_id, "meta": meta(), "operation": {
                "kind": "prompt_videos", "chapter_id": chapter_id, "options": {**video_options,
                    "target_segment": {"source_shot_id": "shot-2", "segment_index_in_shot": 1}}}}))
            continuity_requests = [row["body"] for row in provider_log_entries(args.provider_log)[before_continuity:]
                                   if row["path"].endswith("/chat/completions")]
            assert len(continuity_requests) == 1, continuity_requests
            contents = [message["content"] for message in continuity_requests[0]["messages"]]
            combined = "\n".join(content if isinstance(content, str) else "\n".join(part.get("text", "") for part in content)
                                 for content in contents)
            payload = json_task_payload(combined)
            assert payload["previous_steps"]["previous_completed_shot"]["generation_prompt"] == video["prompt"]
            assert payload["constraints"]["input_frame_usage"]["opening_source"] == "previous_completed_shot"
            report["checks"].append("a single targeted continuity segment receives the selected completed video's immutable generation prompt")

            shared = next(row for row in call("list_chapter_video_prompts", {"projectId": project_id, "chapterId": chapter_id}, raw=True) if row["id"] == shared["id"])
            edited_segments = copy.deepcopy(shared["metadata_json"]["segments"])
            edited_segments[2]["scene_summary"] = "作者手工修改：人物在机房停下，仔细观察纸条。"
            preserved_recipes = copy.deepcopy(shared["metadata_json"]["segment_recipes"][:2])
            edited_plan = call("update_chapter_video_segment_plan", {"promptId": shared["id"], "input": {"expected_updated_at": shared["updated_at"], "segments": edited_segments}}, raw=True)
            assert edited_plan["metadata_json"]["segments"] == edited_segments
            assert edited_plan["metadata_json"]["segment_recipes"][:2] == preserved_recipes
            resume_options = {key: value for key, value in video_options.items() if key not in ("segment_plan_prompt_id", "target_segment")}
            resume_options["workflow_mode"] = "shared_segment_plan"
            complete(call("start_video_generation", {"project_id": project_id, "meta": meta(), "operation": {"kind": "prompt_segment_plan", "chapter_id": chapter_id, "options": resume_options}}))
            shared = next(row for row in call("list_chapter_video_prompts", {"projectId": project_id, "chapterId": chapter_id}, raw=True) if row["id"] == shared["id"])
            segments = shared["metadata_json"]["segments"]
            assert segments == edited_segments
            assert shared["metadata_json"]["segment_recipes"][:2] == preserved_recipes
            report["checks"].append("manual segment edits persist and resume from metadata while preserving unrelated recipes and videos")

            exported = call("export_project_family_data", {"root_project_id": root_project_id, "include_derived_projects": True})
            imported = call("import_project_family_upload", {"file_name": exported["file_name"], "data_base64": exported["data_base64"]})
            new_root = next(row for row in imported if row["project_kind"] == "text")
            new_video_project = next(row for row in imported if row["project_kind"] == "video")
            reexport = call("export_project_family_data", {"root_project_id": new_root["id"], "include_derived_projects": True})

            def manifest(package):
                with zipfile.ZipFile(io.BytesIO(base64.b64decode(package["data_base64"]))) as archive:
                    return json.loads(archive.read("a4n-project-family.json"))

            original, remapped = manifest(exported), manifest(reexport)
            old_plan, new_plan = original["chapter_visual_plans"][0], remapped["chapter_visual_plans"][0]
            assert old_plan["id"] != new_plan["id"] and old_plan["plan_sha256"] != new_plan["plan_sha256"]
            image_map = {old["id"]: new["id"] for old, new in zip(original["generated_images"], remapped["generated_images"])}
            scene_map = {old["id"]: new["id"] for old, new in zip(original["text_core"]["scene_profiles"], remapped["text_core"]["scene_profiles"])}
            for old, new in zip(old_plan["plan_json"]["points"], new_plan["plan_json"]["points"]):
                assert new["image_id"] == image_map.get(old["image_id"])
                assert new["reference_image_ids"] == [image_map[value] for value in old["reference_image_ids"]]
            for old, new in zip(old_plan["plan_json"]["segments"], new_plan["plan_json"]["segments"]):
                for field, value in old["scene_binding"].items():
                    assert new["scene_binding"][field] == (value if field == "kind" else scene_map[value])
            new_script = remapped["chapter_shooting_scripts"][0]
            assert new_script["visual_plan_sha256"] == new_plan["plan_sha256"]
            assert new_script["script_json"]["visual_plan_source"]["plan_sha256"] == new_plan["plan_sha256"]
            new_video_plan = next(row for row in remapped["chapter_video_prompts"] if row["workflow_mode"] == "shared_segment_plan")
            assert json.loads(new_video_plan["visual_prompt"])["segments"] == new_video_plan["metadata_json"]["segments"]
            new_director_images = [point["image_id"] for point in new_video_plan["metadata_json"]["segments"][0]["director_constraints"]["keyframes"]]
            assert new_director_images == [image_map[image["id"]], image_map[generated["id"]], None]
            report["checks"].append("real ZIP export/import remaps images, scene IDs and downstream plan/script hashes")
            report["imported_project_id"] = new_root["id"]
            imported_rows = call("list_chapter_video_prompts", {"projectId": new_video_project["id"], "chapterId": new_script["source_chapter_id"]}, raw=True)
            assert next(row for row in imported_rows if row["workflow_mode"] == "shared_segment_plan")["metadata_json"]["stale_source_shot_ids"] == []
            imported_video = next(row for row in remapped["generated_videos"] if row["status"] == "completed")
            call("select_segment_video", {
                "projectId": new_video_project["id"], "chapterId": new_script["source_chapter_id"], "videoId": video["id"],
                "input": {"segment_plan_prompt_id": new_video_plan["id"], "target_segment": {"source_shot_id": "shot-1", "segment_index_in_shot": 1}},
            }, raw=True, destructive=True, failure="不属于当前规划片段")
            report["checks"].append("video adoption rejects another segment and another project without weakening source validation")
            selected = call("select_segment_video", {
                "projectId": new_video_project["id"], "chapterId": new_script["source_chapter_id"], "videoId": imported_video["id"],
                "input": {"segment_plan_prompt_id": new_video_plan["id"], "target_segment": {"source_shot_id": "shot-1", "segment_index_in_shot": 1}},
            }, raw=True, destructive=True)
            assert selected["id"] == imported_video["id"]
            report["imported_video_project_id"] = new_video_project["id"]
            report["checks"].append("imported videos remain adoptable with remapped segment and recipe fingerprints")

            def revise_shot(prompt):
                nonlocal draft, confirmed
                revised = copy.deepcopy(confirmed["plan"])
                revised["segments"][2]["prompt"] = prompt
                draft = call("save_chapter_visual_plan_draft", {**resource, "authoring_mode": "manual", "record_id": None, "expected_revision": None, "plan": revised})
                outcome = call("confirm_chapter_visual_plan", record_input(), destructive=True)
                confirmed = outcome["visual_plan"]
                assert outcome["shooting_script_sync"]["stale_shot_ids"] == ["shot-3"], outcome
                return call("get_chapter_shooting_script", resource)

            baseline_script = copy.deepcopy(script)
            baseline_segments = copy.deepcopy(segments)
            if not os.environ.get("A4N_DATA_DIR"):
                raise RunnerError("Media-byte regression requires A4N_DATA_DIR for the local test Host")
            media_root = Path(os.environ["A4N_DATA_DIR"]).resolve()
            video_path = (media_root / video["file_path"]).resolve()
            assert video_path.is_relative_to(media_root), "test video must belong to the isolated data directory"
            baseline_video_bytes = hashlib.sha256(video_path.read_bytes()).hexdigest()
            script = revise_shot("林岚进入机房后俯身检查柜台上的纸条。")
            browser_fixture["manual"] = {"before": copy.deepcopy(script), "plan": copy.deepcopy(confirmed)}
            assert script["record"]["id"] == baseline_script["record"]["id"]
            assert script["script"]["scenes"][:2] == baseline_script["script"]["scenes"][:2]
            rows = call("list_chapter_video_prompts", {"projectId": project_id, "chapterId": chapter_id}, raw=True)
            assert next(row for row in rows if row["id"] == shared["id"])["metadata_json"]["stale_source_shot_ids"] == ["shot-3"]
            revised = copy.deepcopy(script["script"])
            revised["scenes"][2]["shots"][0]["composition"] = "作者手动调整：人物位于画面右侧，纸条位于左侧。"
            manual = call("edit_chapter_shooting_script", {**resource, "record_id": script["record"]["id"], "expected_revision": script["record"]["revision"], "script": revised, "rebind_shot_ids": ["shot-3"]})["shooting_script"]
            assert manual["plan_sync"]["stale_shot_ids"] == []
            browser_fixture["manual"]["after"] = copy.deepcopy(manual)
            assert manual["script"]["scenes"][:2] == baseline_script["script"]["scenes"][:2]
            call("edit_chapter_shooting_script", {**resource, "record_id": script["record"]["id"], "expected_revision": script["record"]["revision"], "script": revised, "rebind_shot_ids": ["shot-3"]}, failure="revision")
            report["checks"].append("director confirmation and manual rebinding invalidate only shot-3 and reject revision conflicts")

            script = revise_shot("林岚发现纸条后缓缓起身，回头看向通道。")
            browser_fixture["ai"] = {"before": copy.deepcopy(script), "plan": copy.deepcopy(confirmed)}
            before_local = len(provider_log_entries(args.provider_log))
            script = shooting({**resource, "visual_plan_id": confirmed["record"]["id"], "shot_ids": ["shot-3"]})
            assert script["plan_sync"]["stale_shot_ids"] == []
            browser_fixture["ai"]["after"] = copy.deepcopy(script)
            secure_json(args.report.parent / "browser-fixture.json", browser_fixture)
            assert script["script"]["scenes"][:2] == baseline_script["script"]["scenes"][:2]
            design_requests = [row for row in provider_log_entries(args.provider_log)[before_local:]
                if "只设计 narrative_unit_index=" in json.dumps(row.get("body", {}), ensure_ascii=False)]
            assert len(design_requests) == 1 and "target_shot_id=shot-3" in json.dumps(design_requests[0], ensure_ascii=False), design_requests
            call("start_chapter_shooting_script_generation", {**resource, "visual_plan_id": confirmed["record"]["id"], "shot_ids": ["unknown-shot"], "mode": "full", "meta": meta()}, failure="镜头 ID")
            call("start_chapter_shooting_script_generation", {**resource, "visual_plan_id": confirmed["record"]["id"], "shot_ids": ["shot-3"], "mode": "next_scene", "meta": meta()}, failure="局部镜头更新")
            planning_options = {key: value for key, value in video_options.items() if key not in ("segment_plan_prompt_id", "target_segment")}
            planning_options.update(workflow_mode="shared_segment_plan", target_narrative_unit_index=script["script"]["scenes"][2]["narrative_unit_index"], regenerate_existing=False)
            complete(call("start_video_generation", {"project_id": project_id, "meta": meta(), "operation": {"kind": "prompt_segment_plan", "chapter_id": chapter_id, "options": planning_options}}))
            rows = call("list_chapter_video_prompts", {"projectId": project_id, "chapterId": chapter_id}, raw=True)
            updated_plan = next(row for row in rows if row["workflow_mode"] == "shared_segment_plan")
            assert updated_plan["id"] == shared["id"]
            assert updated_plan["metadata_json"]["segments"][:2] == baseline_segments[:2]
            assert updated_plan["metadata_json"]["stale_source_shot_ids"] == []
            assert any(row["id"] == final_prompt["id"] for row in rows)
            assert any(row["id"] == video["id"] and row["status"] == "completed" for row in call("list_videos_by_chapter", {"projectId": project_id, "chapterId": chapter_id}, raw=True))
            assert hashlib.sha256(video_path.read_bytes()).hexdigest() == baseline_video_bytes
            report["checks"].append("AI updates one shooting shot and its segment plan; untouched plans, prompts and video bytes survive")

            deletion_chapter = data(checked(cli, ["chapter", "import", "--number", "2", "--title", "删除场景后的续写", "--content", chapter["content"]], "deletion chapter")[0], "deletion chapter")
            deletion_project = call("create_derived_project", {
                "root_project_id": root_project_id, "derived_kind": "video",
                "title": "导演时间线 V3 删除场景续写视频",
                "description": "Video project snapshot includes the deletion chapter.",
                "start_chapter": 1, "end_chapter": 2,
            })
            deletion_resource = {"project_id": deletion_project["id"], "source_chapter_id": deletion_chapter["id"]}
            deletion_plan = copy.deepcopy(confirmed["plan"])
            for point in deletion_plan["points"]:
                point["image_id"] = None
                point["reference_image_ids"] = []
            deletion_draft = call("save_chapter_visual_plan_draft", {**deletion_resource, "authoring_mode": "manual", "record_id": None, "expected_revision": None, "plan": deletion_plan})
            deletion_confirmed = call("confirm_chapter_visual_plan", {**deletion_resource, "record_id": deletion_draft["record"]["id"], "expected_revision": deletion_draft["record"]["revision"]}, destructive=True)["visual_plan"]
            deletion_input = {**deletion_resource, "visual_plan_id": deletion_confirmed["record"]["id"]}
            shooting(deletion_input, "next_scene")
            deletion_progress = shooting(deletion_input, "next_scene")
            assert deletion_progress["completed_unit_count"] == 2
            without_second = copy.deepcopy(deletion_progress["script"])
            without_second["scenes"].pop()
            without_second["scenes"][0]["title"] = "删除另一场景后仍保留的作者标题"
            without_second["scenes"][0]["transition_contracts"] = [transition for transition in without_second["scenes"][0]["transition_contracts"] if transition["to_narrative_unit_index"] != 2]
            call("edit_chapter_shooting_script_progress", {**deletion_resource, "workflow_run_id": deletion_progress["workflow_run_id"], "expected_script": deletion_progress["script"], "script": without_second})
            finished = shooting(deletion_input, "next_scene")
            assert finished["record"]["id"] > 0
            assert [scene["narrative_unit_index"] for scene in finished["script"]["scenes"]] == [1, 3]
            assert finished["script"]["scenes"][0] == without_second["scenes"][0]
            assert call("get_chapter_shooting_script", deletion_resource)["script"] == finished["script"]
            report["checks"].append("deleted completed scenes stay deleted on next-unit completion and the generation cursor remains accurate")
            report["success"] = True
    except (RunnerError, OSError, ValueError, TypeError, KeyError, AssertionError, StopIteration, subprocess.SubprocessError) as error:
        report["error"] = f"{type(error).__name__}: {error}"
        report["traceback"] = traceback.format_exc()
    finally:
        secure_json(args.report, report)
    print(json.dumps(report, ensure_ascii=False))
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
