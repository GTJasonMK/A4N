#!/usr/bin/env python3
"""Verify director points/segments through the current CLI, real Host and mock Provider."""

from __future__ import annotations

import argparse
import base64
import copy
import hashlib
import io
import json
import os
import tempfile
import time
import traceback
import uuid
import zipfile
from pathlib import Path

from mock_openai.protocol import PNG_1X1_BASE64
from run_agent_cli_e2e import data
from run_cli_contract_e2e import Cli, RunnerError, checked, command_error, provider_log_entries, runtime_snapshot, secure_json


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
    report = {"success": False, "checks": [], "commands": []}
    try:
        with tempfile.TemporaryDirectory(prefix="director-v3-") as directory:
            work = Path(directory)
            env = os.environ.copy()
            password = env.pop(args.password_env)
            env.pop("A4N_ACCESS_TOKEN", None)
            cli = Cli(args.a4n, args.server, work / "cli.json", work / "context.json", 60, env)
            checked(cli, ["health"], "health")
            checked(cli, ["capabilities", "check"], "capabilities")
            checked(cli, ["auth", "login", "--username", args.username, "--password-stdin"], "login", password + "\n")

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
                    message = str(command_error(result).get("message", ""))
                    assert result["exit_code"] not in (0, 7, 10, 11, 124) and failure in message, (name, result["exit_code"], message)
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

            project = data(checked(cli, ["project", "create", "--title", "导演时间线 V3 隔离验证", "--project-type", "novel_long", "--use-context"], "project")[0], "project")
            chapter = data(checked(cli, ["chapter", "import", "--number", "1", "--title", "门与通道", "--content", "林岚停在旧车站通道入口。她观察墙上的钟，推开门，进入地下机房。周衡留在门外。"], "chapter")[0], "chapter")
            project_id, chapter_id = project["id"], chapter["id"]
            resource = {"project_id": project_id, "source_chapter_id": chapter_id}
            report.update(project_id=project_id, chapter_id=chapter_id)
            scenes = [call("create_scene_profile", {"project_id": project_id, "profile": {
                "name": name, "scene_type": "room", "importance": "primary", "base_description": name,
                "visual_description": "木门、挂钟与清晰的通道", "spatial_layout": "通道通向门口",
                "atmosphere": "安静", "first_seen_chapter": 1, "last_seen_chapter": 1, "image_prompt_seed": name,
            }})["profile"] for name in ("旧车站通道", "地下机房")]
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
            call("edit_chapter_visual_plan_draft", {**resource, "record_id": draft["record"]["id"], "expected_revision": unchanged["record"]["revision"], "operation": {"kind": "scale_timeline", "duration_ms": 18000}}, failure="revision")
            report["checks"].append("inserting points preserves shots; stale writes fail")

            baseline = copy.deepcopy(draft)
            text_handle = call("start_visual_plan_prompt_generation", {**record_input(), "meta": meta(), "segment_id": "shot-1", "user_description": "镜头缓缓推近，保留停顿。"})
            result = complete(text_handle)["result_ref"]
            candidate = call("read_visual_plan_prompt_candidate", {"project_id": project_id, "generation_run_id": int(result["id"])})
            assert candidate["plan_sha256"] == baseline["record"]["plan_sha256"] and candidate["prompt"]
            assert hashlib.sha256(json.dumps(candidate, sort_keys=True, ensure_ascii=False, separators=(",", ":")).encode()).hexdigest() == result["revision"]
            assert call("get_chapter_visual_plan", resource)["draft"] == baseline
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
            assert call("get_chapter_visual_plan", resource)["draft"] == baseline
            requests = provider_log_entries(args.provider_log)[before_images:]
            assert any(row["path"].endswith("/images/edits") and row["body"].get("has_reference_image") for row in requests), requests
            point = next(point for point in draft["plan"]["points"] if point["id"] == "inside")
            draft = edit({"kind": "update_point", "point": {**point, "image_id": generated["id"]}})
            report["checks"].append("image generation sends actual reference bytes and adopts only on request")

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
            assert call("get_chapter_visual_plan", resource)["draft"] == draft
            configure()
            report["checks"].append("cancellation stops a running model call without writing a candidate or plan")

            draft = edit({"kind": "scale_timeline", "duration_ms": 18000})
            assert [point["at_ms"] for point in draft["plan"]["points"]] == [0, 2000, 4000, 6000, 10000, 18000]
            confirmed = call("confirm_chapter_visual_plan", record_input(), destructive=True)["visual_plan"]
            assert confirmed["record"]["status"] == "confirmed"
            call("delete_image", {"image_id": image["id"]}, destructive=True, failure="导演编排引用")
            call("delete_scene_profile", {"scene_profile_id": scenes[0]["id"], "downstream_strategy": "reject"}, destructive=True, failure="导演编排引用")
            script = call("generate_chapter_shooting_script", {**resource, "visual_plan_id": confirmed["record"]["id"]})
            shots = [shot for scene in script["script"]["scenes"] for shot in scene["shots"]]
            assert [shot["shot_id"] for shot in shots] == ["shot-1", "shot-2", "shot-3"]
            assert [shot["estimated_natural_seconds"] for shot in shots] == [6, 4, 8]
            assert shots[0]["visual_constraints"]["end_frame"] is None
            assert [point["at_ms"] for point in shots[0]["visual_constraints"]["keyframes"]] == [0, 2000, 4000]
            report["checks"].append("confirmation permits missing images; script preserves shot IDs, timing, internal points and explicit end-frame semantics")

            video_options = {"workflow_mode": "shared_segment_plan", "style": "电影感", "prompt_output_language": "zh_cn", "prompt_batch_size": 1,
                "duration_secs": 6, "duration_secs_options": [6], "aspect_ratio": "16:9", "reference_image_ids": [], "video_library_image_ids": [], "regenerate_existing": False}
            complete(call("start_video_generation", {"project_id": project_id, "meta": meta(), "operation": {"kind": "prompt_segment_plan", "chapter_id": chapter_id, "options": video_options}}))
            rows = call("list_chapter_video_prompts", {"projectId": project_id, "chapterId": chapter_id}, raw=True)
            shared = next(row for row in rows if row["workflow_mode"] == "shared_segment_plan")
            segments = shared["metadata_json"]["segments"]
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

            exported = call("export_project_family_data", {"root_project_id": project_id, "include_derived_projects": True})
            imported = call("import_project_family_upload", {"file_name": exported["file_name"], "data_base64": exported["data_base64"]})
            new_root = next(row for row in imported if row["project_kind"] == "text")
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
            report["success"] = True
    except (RunnerError, OSError, ValueError, TypeError, KeyError, AssertionError, StopIteration) as error:
        report["error"] = f"{type(error).__name__}: {error}"
        report["traceback"] = traceback.format_exc()
    finally:
        secure_json(args.report, report)
    print(json.dumps(report, ensure_ascii=False))
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
