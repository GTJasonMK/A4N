#!/usr/bin/env python3
"""Check call-level validation and reuse through a real isolated CLI/Server/Provider."""

import json
import tempfile
import time
import uuid
from pathlib import Path

from mock_openai.fixtures.story import chapter_body_response
from mock_openai.protocol import PNG_1X1_BASE64
from run_cli_contract_e2e import RunnerError, checked, cli_e2e_parser, command_error, logged_in_cli, provider_log_entries, runtime_snapshot, secure_json


def main():
    parser = cli_e2e_parser(__doc__)
    parser.add_argument("--embedding-dimension", type=int, default=384)
    args = parser.parse_args()
    args.report.parent.mkdir(parents=True, exist_ok=True)
    report = {"success": False, "checks": [], "projects": []}
    try:
        with tempfile.TemporaryDirectory(prefix="call-checkpoints-", dir=args.report.parent) as directory:
            work = Path(directory)
            cli = logged_in_cli(args, work)
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
                snapshot = runtime_snapshot(args.provider_url, embedding_dimension=args.embedding_dimension)
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

            def shooting(source, expected=None):
                state = task(call("start_chapter_shooting_script_generation", {
                    **source, "mode": "full", "meta": {"start_request_id": str(uuid.uuid4())},
                }), expected)
                if expected:
                    return state["terminal_event"]
                result = call("get_chapter_shooting_script", {
                    "project_id": source["project_id"], "source_chapter_id": source["source_chapter_id"],
                })
                assert state["result_ref"]["id"] == str(result["record"]["id"]), state
                return result

            def new_project(title):
                project = checked(cli, ["project", "create", "--title", title, "--project-type", "novel_long", "--use-context"], "project")[0]["data"]
                report["projects"].append(project["id"])
                return project["id"]

            def shooting_source(title, unit_count, shots_per_unit=1):
                root_project_id = new_project(title)
                shot_prompt = "林岚留下齿轮暗记，进入地下机房。"
                chapter = checked(cli, ["chapter", "import", "--number", "1", "--title", "雨夜来信", "--content", chapter_body_response() + "\n\n" + shot_prompt], "chapter")[0]["data"]
                chapter_id = chapter["id"]
                profiles = call("list_profiles", {"project_id": root_project_id})
                if not isinstance(profiles, list):
                    raise RunnerError("checkpoint manga fixture did not receive character profiles")
                for character_name in ("林岚", "周衡"):
                    profile = next(
                        (
                            row
                            for row in profiles
                            if isinstance(row, dict)
                            and str(row.get("name", "")).strip() == character_name
                        ),
                        None,
                    )
                    if profile is None:
                        profile = call("create_profile", {
                            "project_id": root_project_id,
                            "name": character_name,
                            "base_attributes": {
                                "explicit_attributes": {},
                                "implicit_attributes": {},
                                "social_attributes": {},
                            },
                        })
                        profile = profile.get("profile", profile)
                    profile_id = profile.get("id")
                    if not isinstance(profile_id, int) or profile_id <= 0:
                        raise RunnerError(f"checkpoint manga fixture profile ID invalid: {character_name}")
                    call("upload_portrait", {
                        "project_id": root_project_id,
                        "profile_id": profile_id,
                        "file_name": f"checkpoint-{profile_id}.png",
                        "mime_type": "image/png",
                        "data_base64": PNG_1X1_BASE64,
                        "portrait_type": "face",
                        "prompt": f"{character_name} 的漫画参考立绘",
                        "set_primary": True,
                    })
                scenes = [call("create_scene_profile", {"project_id": root_project_id, "profile": {
                    "name": name, "scene_type": "room", "importance": "primary", "base_description": name,
                    "visual_description": "门与挂钟形成清晰关系", "spatial_layout": "通道通向机房", "atmosphere": "安静",
                    "first_seen_chapter": 1, "last_seen_chapter": 1, "image_prompt_seed": name,
                }})["profile"] for name in ("旧车站通道", "地下机房")[:unit_count]]
                image = call("upload_image", {"project_id": root_project_id, "file_name": "checkpoint.png", "mime_type": "image/png",
                    "data_base64": PNG_1X1_BASE64, "image_type": "chapter_reference", "prompt": "测试参考图",
                    "chapter_id": chapter_id, "panel_id": None, "metadata_json": None})
                video_project = call("create_derived_project", {
                    "root_project_id": root_project_id, "derived_kind": "video", "title": title + " 视频",
                    "description": "chapter generation fixture", "start_chapter": 1, "end_chapter": 1,
                })
                report["projects"].append(video_project["id"])
                source = {"project_id": video_project["id"], "source_chapter_id": chapter_id}
                plan = call("get_chapter_visual_plan", source)["starter_plan"]
                shot_count = unit_count * shots_per_unit
                plan["points"] = [{"id": f"point-{index}", "at_ms": index * 5000, "label": "边界", "description": "门口画面",
                    "image_id": image["id"] if index == 0 else None, "reference_image_ids": []} for index in range(shot_count + 1)]
                plan["segments"] = [{"id": f"shot-{index + 1}", "title": "进入机房", "start_point_id": f"point-{index}", "end_point_id": f"point-{index + 1}",
                    "prompt": shot_prompt, "scene_binding": {"kind": "scene", "scene_profile_id": scenes[index // shots_per_unit]["id"]},
                    "use_end_point_image": False} for index in range(shot_count)]
                draft = call("save_chapter_visual_plan_draft", {**source, "authoring_mode": "manual", "record_id": None, "expected_revision": None, "plan": plan})
                confirmed = call("confirm_chapter_visual_plan", {**source, "record_id": draft["record"]["id"], "expected_revision": draft["record"]["revision"]}, True)
                return {**source, "visual_plan_id": confirmed["visual_plan"]["record"]["id"]}, chapter_id, root_project_id

            def detail(project_id, kind):
                runs = call("list_workflow_runs", {"project_id": project_id, "workflow_kind": kind, "chapter_id": None, "status": None, "resumable_only": False, "limit": 20})
                latest = max(runs, key=lambda run: run["id"])
                return call("get_workflow_run_detail", {"workflow_run_id": latest["id"], "include_generation_runs": False})

            model()
            for fault, expected, prefix in [("shooting-layout", "slugline", 0), ("shooting-adaptation", "actor", 1), ("shooting-design", "lens_intent", 2)]:
                source, _, _ = shooting_source(fault, 2)
                model(fault)
                before = len(calls())
                error = shooting(source, expected)
                assert len(calls()) - before == prefix + 1, (fault, len(calls()) - before)
                failed = detail(source["project_id"], "chapter_shooting_script")
                assert sum(item["status"] == "completed" for item in failed["checkpoints"]) == prefix, failed
                model()
                before = len(calls())
                result = shooting(source)
                assert len(result["script"]["scenes"]) == 2, result
                assert len(calls()) - before == 6 - prefix, (fault, len(calls()) - before)
                report["checks"].append({"fault": fault, "error": error, "reused_calls": prefix, "retry_calls": 6 - prefix})

            source, _, _ = shooting_source("shooting second-shot checkpoint", 1, shots_per_unit=3)
            model("shooting-design-second-shot")
            before = len(calls())
            shooting(source, "lens_intent")
            assert len(calls()) - before == 4, len(calls()) - before
            failed = detail(source["project_id"], "chapter_shooting_script")
            assert {item["step_key"] for item in failed["checkpoints"] if item["status"] == "completed"} == {
                "screenplay_layout_unit:1", "screenplay_unit_adaptation:1", "shooting_design_shot:1:shot-1",
            }, failed
            model()
            before = len(calls())
            result = shooting(source)
            assert len(result["script"]["scenes"]) == 1 and len(result["script"]["scenes"][0]["shots"]) == 3, result
            assert len(calls()) - before == 2, len(calls()) - before
            report["checks"].append({"fault": "shooting-design-second-shot", "reused_calls": 3, "retry_calls": 2})

            video_source, video_chapter_id, _ = shooting_source("video strict JSON checkpoint", 2)
            shooting(video_source)
            video_request = {"project_id": video_source["project_id"], "operation": {
                "kind": "prompt_segment_plan", "chapter_id": video_chapter_id, "options": {
                    "workflow_mode": "shared_segment_plan", "style": "电影感", "prompt_output_language": "zh_cn",
                    "prompt_batch_size": 1, "duration_secs": 6, "duration_secs_options": [6], "aspect_ratio": "16:9",
                    "reference_image_ids": [], "video_library_image_ids": [], "regenerate_existing": False,
                }}}
            model("video-json")
            before = len(calls())
            first = call("start_video_generation", {**video_request, "meta": {"start_request_id": str(uuid.uuid4())}})
            task(first, "JSON")
            assert len(calls()) - before == 2, len(calls()) - before
            model()
            before = len(calls())
            task(call("start_video_generation", {**video_request, "meta": {
                "start_request_id": str(uuid.uuid4()), "retry_of_task_id": first["task_id"]}}))
            assert len(calls()) - before == 1, len(calls()) - before
            report["checks"].append({"fault": "video-json", "reused_calls": 1, "retry_calls": 1})

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

            source, chapter_id, root_project_id = shooting_source("derived media checkpoint boundary", 1)
            shooting(source)

            def derived(kind, title, parent_project_id):
                project = call("create_derived_project", {"root_project_id": parent_project_id, "derived_kind": kind, "title": title,
                    "description": "isolated checkpoint fixture", "start_chapter": 1, "end_chapter": 1})
                report["projects"].append(project["id"])
                return project["id"]

            for fault, expected, prefix in [("galgame-branch", "after_beat_ref", 0), ("galgame-script", "未知改编节拍", 1), ("galgame-staging", "原剧本行", 2), ("galgame-visual", "背景引用的改编场景不存在", 3)]:
                project_id = derived("galgame", fault, root_project_id)
                value = {"project_id": project_id, "task_id": str(uuid.uuid4()), "resume_from_checkpoint": True, "start_from_stage": None}
                model(fault)
                before = len(calls())
                error = reject("start_galgame_pipeline", value, expected)
                assert len(calls()) - before == prefix + 2, (fault, len(calls()) - before)
                model()
                before = len(calls())
                call("start_galgame_pipeline", {**value, "task_id": str(uuid.uuid4())})
                assert len(calls()) - before == 4 - prefix, (fault, len(calls()) - before)
                report["checks"].append({"fault": fault, "error": error, "reused_calls": prefix, "retry_calls": 4 - prefix})

            def manga_result(project_id, expected=None, previous=None, expected_task=None):
                deadline = time.monotonic() + 60
                last_binding_error = None
                last_binding_task_status = None
                while time.monotonic() < deadline:
                    try:
                        result = call("get_manga_pipeline_status", {
                            "project_id": project_id,
                            "chapter_id": chapter_id,
                        })
                    except RunnerError as error:
                        if (
                            expected_task is not None
                            and any(
                                message in str(error)
                                for message in (
                                    "缺少活动 Generation Task",
                                    "generation task checkpoint 不存在",
                                )
                            )
                        ):
                            task_result = checked(
                                cli,
                                [
                                    "task",
                                    "get",
                                    expected_task["task_id"],
                                    "--attempt",
                                    str(expected_task["attempt"]),
                                ],
                                "Manga task while workflow binding",
                            )[0]["data"]
                            task_handle = task_result.get("handle")
                            if not isinstance(task_handle, dict):
                                raise RunnerError(
                                    "Manga task status 缺少 handle: "
                                    f"{task_result}"
                                ) from error
                            task_status = task_handle.get("status")
                            if task_status not in {
                                "completed", "failed", "cancelled", "interrupted"
                            }:
                                last_binding_error = str(error)
                                last_binding_task_status = task_status
                                time.sleep(0.1)
                                continue
                        try:
                            runs = call("list_workflow_runs", {
                                "project_id": project_id,
                                "chapter_id": chapter_id,
                                "workflow_kind": "manga_pipeline",
                                "status": None,
                                "resumable_only": False,
                                "limit": 20,
                            })
                            workflows = [
                                {
                                    "id": run.get("id"),
                                    "status": run.get("status"),
                                    "active_task_id": run.get("active_task_id"),
                                }
                                for run in runs
                            ]
                        except RunnerError as diagnostic_error:
                            workflows = f"diagnostic failed: {diagnostic_error}"
                        expected_id = expected_task.get("task_id") if expected_task else None
                        raise RunnerError(
                            f"{error}; expected_generation_task={expected_id}; "
                            f"manga_workflows={workflows}"
                        ) from error
                    # Wait for the resumed task to publish a new terminal summary.
                    if result == previous:
                        time.sleep(0.1)
                        continue
                    previous = None
                    task_handle = result.get("task_handle")
                    if (
                        result.get("pipeline_status") == "running"
                        and expected_task is not None
                        and isinstance(task_handle, dict)
                        and task_handle.get("task_id") != expected_task.get("task_id")
                    ):
                        raise RunnerError(
                            "fixture Manga pipeline task identity mismatch: "
                            f"started={expected_task.get('task_id')}, "
                            f"status={task_handle.get('task_id')}"
                        )
                    if result["pipeline_status"] in {"completed", "failed", "cancelled"}:
                        assert result["pipeline_status"] == ("failed" if expected else "completed"), result
                        if expected:
                            assert expected in json.dumps(result, ensure_ascii=False), result
                        return result
                    time.sleep(0.1)
                raise RunnerError(
                    "manga fixture did not terminate; "
                    f"last_binding_task_status={last_binding_task_status}; "
                    f"last_binding_error={last_binding_error}"
                )

            early_faults = [
                ("manga-slots", 0, "不存在的改编节拍"),
                # These prefixes count completed provider calls before each
                # injected page fault in the current pipeline contract.
                ("manga-speech", 5, "beat_refs 必须来自页面内容节拍"),
                ("manga-visual", 4, "page.panels[0].beat_refs 无效"),
                ("manga-storyboard", 0, "manga_page_storyboard page_number 必须等于"),
                ("manga-items", 7, "引用了不可用素材"),
            ]
            page_faults = [
                ("manga-drawing", "image_prompt 无效"),
            ]
            for fault, prefix, expected in [*early_faults, *((name, 8, message) for name, message in page_faults)]:
                manga_source, chapter_id, root_project_id = shooting_source(fault, 1)
                shooting(manga_source)
                manga_project = call("create_derived_project", {
                    "root_project_id": root_project_id, "derived_kind": "comic", "title": fault + " 漫画",
                    "description": "isolated checkpoint fixture", "start_chapter": 1, "end_chapter": 1,
                })
                report["projects"].append(manga_project["id"])
                project_id = manga_project["id"]
                value = {
                    "project_id": project_id,
                    "chapter_number": 1,
                    "image_size": {"width": 1024, "height": 1024},
                }
                preprocessing = call("start_manga_pipeline", {
                    **value,
                    "run_stage": "preprocessing",
                    "meta": {"start_request_id": str(uuid.uuid4())},
                })
                manga_result(project_id, expected_task=preprocessing)
                model()
                adaptation = call("generate_media_adaptation_plan", {
                    "project_id": project_id,
                    "chapter_id": chapter_id,
                    "task_id": str(uuid.uuid4()),
                    "instruction": "保留原作因果，以漫画页节奏呈现改编内容",
                    "video_reference": None,
                    "clear_video_reference": False,
                    "record_id": None,
                    "expected_revision": None,
                    "resume_from_checkpoint": True,
                })
                call("adopt_media_authoring_document", {
                    "project_id": adaptation["project_id"],
                    "record_id": adaptation["id"],
                    "expected_revision": adaptation["revision"],
                })
                model(fault)
                before = len(calls())
                started = call("start_manga_pipeline", {
                    **value, "meta": {"start_request_id": str(uuid.uuid4())},
                })
                failed = manga_result(project_id, expected, expected_task=started)
                start_calls = len(calls()) - before
                if fault == "manga-slots":
                    assert start_calls == prefix + 1, (fault, start_calls)
                else:
                    # Page work is scheduled independently; whichever page
                    # reaches the injected invalid response first is valid.
                    assert start_calls > 0, (fault, start_calls)
                pages_before_resume = call("list_manga_page_prompts", {"project_id": project_id, "chapter_id": chapter_id})
                if fault == "manga-slots":
                    assert failed["can_resume"] is False, failed
                    model()
                    before = len(calls())
                    restarted = call("start_manga_pipeline", {
                        **value, "meta": {"start_request_id": str(uuid.uuid4())},
                    })
                    manga_result(project_id, previous=failed, expected_task=restarted)
                    pages = call("list_manga_page_prompts", {"project_id": project_id, "chapter_id": chapter_id})
                    restart_calls = len(calls()) - before
                    assert pages and restart_calls == 1 + 4 * len(pages), (restart_calls, len(pages))
                    assert all(page["page_adaptation"] and page["page_plan"] and page["drawing_prompt"] for page in pages)
                    report["checks"].append({
                        "fault": fault,
                        "resume_available": False,
                        "restart_calls": restart_calls,
                    })
                    continue
                assert failed["can_resume"] is True, failed
                saved_content = [artifact for artifact in call("list_manga_pipeline_artifacts", {
                    "project_id": project_id, "chapter_id": chapter_id,
                }) if artifact["stage"] == "page_content"]
                expected_retry_calls = sum(
                    page.get("page_adaptation") is None
                    for page in pages_before_resume
                ) + sum(
                    page.get("page_plan") is None
                    for page in pages_before_resume
                ) + sum(
                    page.get("reference_discovery") is None
                    for page in pages_before_resume
                ) + sum(
                    page.get("drawing_prompt") is None
                    for page in pages_before_resume
                )
                model()
                before = len(calls())
                resumed = call("resume_manga_pipeline", {
                    **value, "meta": {"start_request_id": str(uuid.uuid4())},
                })
                manga_result(project_id, previous=failed, expected_task=resumed)
                resumed_content = [artifact for artifact in call("list_manga_pipeline_artifacts", {
                    "project_id": project_id, "chapter_id": chapter_id,
                }) if artifact["stage"] == "page_content"]
                assert resumed_content == saved_content, "Resume must preserve saved page content and its revisions"
                pages = call("list_manga_page_prompts", {"project_id": project_id, "chapter_id": chapter_id})
                assert pages and len(calls()) - before == expected_retry_calls, (fault, len(calls()) - before, expected_retry_calls, len(pages))
                assert all(page["page_adaptation"] and page["page_plan"] and page["drawing_prompt"] for page in pages)
                report["checks"].append({"fault": fault, "reused_calls": start_calls, "retry_calls": len(calls()) - before})

            page_before_optimization = pages[0]
            generated_image_id = page_before_optimization.get("generated_image_id")
            if not isinstance(generated_image_id, int) or generated_image_id <= 0:
                raise RunnerError("checkpoint Manga optimization fixture has no generated image")
            value = {
                "prompt_id": page_before_optimization["id"],
                "instruction": "增强画面构图",
                "expected_updated_at": page_before_optimization["updated_at"],
            }
            model("manga-optimization")
            reject("optimize_manga_prompt", value, "image_prompt")
            model()
            optimized_page = call("optimize_manga_prompt", value)
            assert optimized_page["generated_image_id"] == generated_image_id, optimized_page
            optimized = detail(project_id, "manga_prompt_optimization")
            assert optimized["run"]["status"] == "completed", optimized
            assert len(optimized["checkpoints"]) == 1 and optimized["checkpoints"][0]["status"] == "completed", optimized
            report["checks"].append({"fault": "manga-optimization", "saved_calls": 1})

            deletion_input = {"project_id": project_id, "chapter_id": chapter_id, "stage": "storyboard", "page_number": 1}
            deletion = call("preview_delete_manga_pipeline_artifact", deletion_input)
            call("delete_manga_pipeline_artifact", {
                **deletion_input,
                "expected_artifact_ids": [entry["artifact_id"] for entry in [deletion["target"], *deletion["downstream"]]],
            }, destructive=True)
            staged_request = {
                "project_id": project_id, "chapter_number": 1, "page_number": 1,
                "run_stage": "storyboard", "stop_after": "reference_discovery",
                "image_size": {"width": 1024, "height": 1024},
            }
            model("manga-items")
            started = call("start_manga_pipeline", {
                **staged_request, "meta": {"start_request_id": str(uuid.uuid4())},
            })
            failed = manga_result(project_id, "引用了不可用素材", expected_task=started)
            assert failed["checkpoint_valid"] is True and failed["can_resume"] is True, failed
            model()
            before = len(calls())
            resumed = call("resume_manga_pipeline", {
                **staged_request, "meta": {"start_request_id": str(uuid.uuid4())},
            })
            manga_result(project_id, previous=failed, expected_task=resumed)
            assert len(calls()) - before == 1, "The stage resume must reuse its completed storyboard"
            report["checks"].append({"fault": "manga-staged-reference-discovery", "retry_calls": 1, "checkpoint_valid": True})

            permissions = {"read_project_detail": True, "read_blueprint": True, "read_outlines": True,
                "read_character_profiles": True, "read_foreshadowing": True, "automatic_writeback": False}
            session = call("create_optimization_analysis_session", {"project_id": root_project_id, "chapter_number": 1,
                "name": "checkpoint validation", "mode": "quick", "permissions": permissions})
            model("optimization-final")
            before = len(calls())
            first = call("start_optimize_analysis_generation", {"session_id": session["id"], "meta": {"start_request_id": str(uuid.uuid4())}})
            task(first, "missing-span")
            failed_calls = len(calls()) - before
            failed = detail(root_project_id, None)
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
            image_result = task(call("start_image_generation", {"project_id": root_project_id,
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
