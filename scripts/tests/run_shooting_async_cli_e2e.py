#!/usr/bin/env python3
"""Verify background shooting generation, disconnected observers and durable completed scenes."""

import argparse
import copy
import json
import os
import selectors
import subprocess
import tempfile
import time
import uuid
from pathlib import Path

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
    parser.add_argument("--prepare-browser", action="store_true")
    args = parser.parse_args()
    args.report.parent.mkdir(parents=True, exist_ok=True)
    report = {"success": False, "checks": [], "fixtures": []}
    try:
        with tempfile.TemporaryDirectory(prefix="shooting-async-", dir=args.report.parent) as directory:
            work = Path(directory)
            env = os.environ.copy()
            password = env.pop(args.password_env)
            env.pop("A4N_ACCESS_TOKEN", None)
            cli = Cli(args.a4n, args.server, work / "cli.json", work / "context.json", 60, env)
            checked(cli, ["health"], "health")
            checked(cli, ["capabilities", "check"], "capabilities")
            checked(cli, ["auth", "login", "--username", args.username, "--password-stdin"], "login", password + "\n")
            described = set()

            def call(name, value, *, raw=False, destructive=False):
                if name not in described:
                    checked(cli, ["capabilities", "describe", name], name + " contract")
                    described.add(name)
                parts = ["call", name, "--args", "-"]
                if destructive:
                    parts.append("--allow-destructive")
                return checked(cli, parts, name, json.dumps(value if raw else {"input": value}, ensure_ascii=False))[0]["data"]

            def configure(model="a4n-virtual-text"):
                snapshot = runtime_snapshot(args.provider_url)
                snapshot["llm"]["model"] = model
                secure_json(work / "runtime.json", snapshot)
                checked(cli, ["config", "runtime", "set", "--input", "@" + str(work / "runtime.json")], "runtime")

            def meta():
                return {"start_request_id": str(uuid.uuid4()), "retry_of_task_id": None}

            def summary(handle):
                return checked(cli, ["task", "get", handle["task_id"], "--attempt", str(handle["attempt"])], "task summary")[0]["data"]

            def terminal(handle, status="completed"):
                watched = cli.run(["task", "watch", handle["task_id"], "--attempt", str(handle["attempt"])])
                actual = summary(handle)
                assert watched["exit_code"] == {"completed": 0, "failed": 8, "cancelled": 9}[status], (watched, actual)
                assert actual["handle"]["status"] == status, actual
                return actual

            def fixture(title, *, video_before_plan=False):
                project = checked(cli, ["project", "create", "--title", title, "--project-type", "novel_long", "--use-context"], "project")[0]["data"]
                chapter = checked(cli, ["chapter", "import", "--number", "1", "--title", "门与机房", "--content", "林岚停在旧车站通道入口。她观察墙上的钟，推开门，进入地下机房。周衡留在门外。"], "chapter")[0]["data"]
                resource = {"project_id": project["id"], "source_chapter_id": chapter["id"]}
                scenes = [call("create_scene_profile", {"project_id": project["id"], "profile": {
                    "name": name, "scene_type": "room", "importance": "primary", "base_description": name,
                    "visual_description": "门框与挂钟", "spatial_layout": "通道通向门口", "atmosphere": "安静",
                    "first_seen_chapter": 1, "last_seen_chapter": 1, "image_prompt_seed": name,
                }})["profile"] for name in ["旧车站通道", "地下机房"]]
                video = call("create_derived_project", {
                    "root_project_id": project["id"], "derived_kind": "video", "title": title + "：视频",
                    "description": "来源同步回归", "start_chapter": 1, "end_chapter": 1,
                }) if video_before_plan else None
                plan = call("get_chapter_visual_plan", resource)["starter_plan"]
                plan["points"] = [{"id": name, "at_ms": at, "label": name, "image_id": None,
                                   "description": "当前场景画面", "reference_image_ids": []}
                                  for name, at in [("start", 0), ("cut", 6000), ("end", 12000)]]
                plan["segments"] = [{"id": f"shot-{i+1}", "title": f"镜头 {i+1}", "start_point_id": start, "end_point_id": end,
                                     "prompt": prompt, "scene_binding": {"kind": "scene", "scene_profile_id": scenes[i]["id"]},
                                     "use_end_point_image": False}
                                    for i, (start, end, prompt) in enumerate([
                                        ("start", "cut", "林岚观察挂钟并走到门口。"), ("cut", "end", "林岚站在机房内回头看门外。"),
                                    ])]
                draft = call("save_chapter_visual_plan_draft", {**resource, "record_id": None, "expected_revision": None,
                                                               "authoring_mode": "manual", "plan": plan})
                confirmed = call("confirm_chapter_visual_plan", {**resource, "record_id": draft["record"]["id"],
                                                                  "expected_revision": draft["record"]["revision"]}, destructive=True)["visual_plan"]
                source = {**resource, "visual_plan_id": confirmed["record"]["id"], "shot_ids": []}
                report["fixtures"].append(source)
                return resource, source, video

            def sync_preview(video, changes):
                preview = call("preview_source_sync", {"derived_project_id": video["id"]})
                assert preview["run"]["summary_json"]["chapter_changes"] == changes, preview
                return preview

            def sync_apply(preview):
                applied = call("apply_source_sync_decisions", {"sync_run_id": preview["run"]["id"], "decisions": []}, destructive=True)
                assert applied["run"]["status"] == "completed", applied
                return applied

            def video_state(video):
                return call("get_project", {"projectId": video["id"]}, raw=True)

            configure("a4n-shooting-background-slow")
            resource, source, video = fixture("异步剧本：断线与停止", video_before_plan=True)
            video_resource = {**resource, "project_id": video["id"]}
            assert video_state(video)["source_sync_status"] == "root_updated"
            sync_apply(sync_preview(video, 1))
            assert video_state(video)["source_sync_status"] == "current"
            assert call("get_chapter_shooting_script", video_resource) is None
            report["checks"].append("confirming a director plan after video creation marks its source updated and appears in the sync preview even without body changes")
            request = {"meta": meta(), **source, "mode": "full"}
            started = time.monotonic()
            handle = call("start_chapter_shooting_script_generation", request)
            assert time.monotonic() - started < 3, "start waited for the six-second provider call"
            repeated = call("start_chapter_shooting_script_generation", request)
            assert repeated["task_id"] == handle["task_id"] and repeated["attempt"] == handle["attempt"]

            watcher = subprocess.Popen([args.a4n, "--output", "jsonl", "--no-input", "--server", args.server,
                                        "--config", str(cli.config), "--context", str(cli.context), "task", "watch",
                                        handle["task_id"], "--attempt", str(handle["attempt"])],
                                       stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)
            try:
                with selectors.DefaultSelector() as selector:
                    selector.register(watcher.stdout, selectors.EVENT_READ)
                    assert selector.select(3), "observer did not attach"
                    assert watcher.stdout.readline(), "observer closed without an event"
            finally:
                watcher.terminate()
                watcher.wait(timeout=5)
            assert summary(handle)["handle"]["status"] in ["reserved", "running"]
            report["checks"].append("start responds before the slow model, duplicate start is idempotent, and detaching the CLI observer does not stop generation")

            deadline = time.monotonic() + 30
            partial = None
            while time.monotonic() < deadline:
                partial = call("get_chapter_shooting_script_unit_progress", resource)
                if partial is not None:
                    break
                current = summary(handle)
                assert current["handle"]["status"] in ["reserved", "running"], current
                time.sleep(0.15)
            assert partial and partial["completed_unit_count"] == 1 and partial["total_unit_count"] == 2, partial
            assert summary(handle)["handle"]["status"] == "running"
            call("cancel_generation_task", {"task_id": handle["task_id"], "attempt": handle["attempt"]}, destructive=True)
            terminal(handle, "cancelled")
            retained = call("get_chapter_shooting_script_unit_progress", resource)
            assert retained["script"] == partial["script"]
            report["checks"].append("full generation publishes its first scene while the second is still running; stopping preserves the completed scene as resumable progress")

            edited = copy.deepcopy(retained["script"])
            edited["scenes"][0]["title"] = "作者保留的第一场"
            edited["scenes"][0]["transition_contracts"] = [{
                "to_narrative_unit_index": 2, "transition_type": "hard_cut",
                "preserved_action": "推开门", "preserved_sound": None,
                "preserved_knowledge": [], "required_cut_point": None,
            }]
            partial = call("edit_chapter_shooting_script_progress", {**resource, "workflow_run_id": retained["workflow_run_id"],
                         "expected_script": retained["script"], "script": edited})["progress"]
            assert call("get_chapter_shooting_script_unit_progress", video_resource) is None
            assert video_state(video)["source_sync_status"] == "root_updated"
            sync_apply(sync_preview(video, 1))
            partial_frozen = call("get_chapter_shooting_script", video_resource)
            assert len(partial_frozen["script"]["scenes"]) == 1
            assert partial_frozen["script"]["scenes"][0]["title"] == edited["scenes"][0]["title"]
            assert partial_frozen["script"]["scenes"][0]["transition_contracts"] == []
            assert call("get_chapter_shooting_script_unit_progress", resource)["script"] == edited
            sync_preview(video, 0)
            planning_video = call("create_derived_project", {
                "root_project_id": resource["project_id"], "derived_kind": "video", "title": "冻结部分场景的片段规划",
                "description": "已完成场景可独立规划", "start_chapter": 1, "end_chapter": 1,
            })
            configure()
            planning = call("start_video_generation", {"project_id": planning_video["id"], "meta": meta(), "operation": {
                "kind": "prompt_segment_plan", "chapter_id": resource["source_chapter_id"], "options": {
                    "workflow_mode": "shared_segment_plan", "style": "电影感", "prompt_output_language": "zh_cn",
                    "prompt_batch_size": 1, "duration_secs": 6, "duration_secs_options": [6], "aspect_ratio": "16:9",
                    "reference_image_ids": [], "video_library_image_ids": [], "regenerate_existing": False,
                    "target_narrative_unit_index": 1,
                },
            }})
            terminal(planning)
            plans = call("list_chapter_video_prompts", {"projectId": planning_video["id"], "chapterId": resource["source_chapter_id"]}, raw=True)
            plan = next(item for item in plans if item["workflow_mode"] == "shared_segment_plan")
            assert plan["metadata_json"]["segments"] and all(segment["source_shot_id"] == "shot-1" for segment in plan["metadata_json"]["segments"])
            report["checks"].append("explicit source sync freezes the completed first scene, preserves the root continuation, and supports real segment planning in a derived video project")

            before = len(provider_log_entries(args.provider_log))
            resumed = call("start_chapter_shooting_script_generation", {"meta": meta(), **source, "mode": "next_scene"})
            result = terminal(resumed)
            view = call("get_chapter_shooting_script", resource)
            assert len(view["script"]["scenes"]) == 2 and view["script"]["scenes"][0] == edited["scenes"][0]
            assert result["result_ref"] == {"kind": "chapter_shooting_script", "id": str(view["record"]["id"]), "revision": str(view["record"]["revision"])}
            assert not any("target_shot_id=shot-1" in json.dumps(row.get("body", {}), ensure_ascii=False)
                           for row in provider_log_entries(args.provider_log)[before:])
            report["checks"].append("a new observer resumes the remaining scene without regenerating the completed shot or losing author edits, and reads the final revision by result reference")

            assert video_state(video)["source_sync_status"] == "root_updated"
            assert call("get_chapter_shooting_script", video_resource) == partial_frozen
            report["checks"].append("frozen partial scenes remain visible after the root workflow completes; they change only on the next source sync")
            sync_apply(sync_preview(video, 1))
            frozen = call("get_chapter_shooting_script", video_resource)
            assert frozen["script"]["scenes"] == view["script"]["scenes"]
            assert frozen["script"]["visual_plan_source"]["plan_id"] != view["script"]["visual_plan_source"]["plan_id"]
            sync_apply(sync_preview(video, 0))
            assert video_state(video)["source_sync_status"] == "current"
            report["checks"].append("a script generated after video creation is detected and becomes readable after explicit sync; copying plan identities does not create false changes")

            unchanged_preview = sync_preview(video, 0)
            modified = copy.deepcopy(view["script"])
            modified["scenes"][0]["title"] = "同步后的作者修订"
            call("edit_chapter_shooting_script", {
                **resource, "record_id": view["record"]["id"], "expected_revision": view["record"]["revision"],
                "script": modified,
            })
            assert video_state(video)["source_sync_status"] == "root_updated"
            stale = cli.run(["call", "apply_source_sync_decisions", "--args", "-", "--allow-destructive"], stdin=json.dumps({
                "input": {"sync_run_id": unchanged_preview["run"]["id"], "decisions": []},
            }))
            assert stale["exit_code"] != 0 and "root_changed_after_dry_run" in json.dumps(stale), stale
            sync_apply(sync_preview(video, 1))
            assert call("get_chapter_shooting_script", video_resource)["script"]["scenes"][0]["title"] == modified["scenes"][0]["title"]
            report["checks"].append("script edits invalidate stale sync previews and a new preview safely imports the edited scenes")
            report["synced_video_project_id"] = video["id"]

            configure("a4n-checkpoint-fail-shooting-design-second-shot")
            resource, source, _ = fixture("异步剧本：失败恢复")
            failed = call("start_chapter_shooting_script_generation", {"meta": meta(), **source, "mode": "full"})
            terminal(failed, "failed")
            partial = call("get_chapter_shooting_script_unit_progress", resource)
            assert partial and partial["completed_unit_count"] == 1, partial
            configure()
            resumed = call("start_chapter_shooting_script_generation", {"meta": meta(), **source, "mode": "full"})
            terminal(resumed)
            assert len(call("get_chapter_shooting_script", resource)["script"]["scenes"]) == 2
            report["checks"].append("invalid output in the second scene fails visibly; the first scene survives and a later task completes the remaining work")

            configure("a4n-shooting-background-slow")
            resource, source, _ = fixture("首场完成前停止后续写")
            early = call("start_chapter_shooting_script_generation", {"meta": meta(), **source, "mode": "full"})
            deadline = time.monotonic() + 30
            run_id = None
            while time.monotonic() < deadline:
                runs = call("list_workflow_runs", {"project_id": resource["project_id"], "workflow_kind": "chapter_shooting_script",
                    "chapter_id": resource["source_chapter_id"], "status": None, "resumable_only": True, "limit": 10})
                if runs:
                    run_id = max(run["id"] for run in runs)
                    detail = call("get_workflow_run_detail", {"workflow_run_id": run_id, "include_generation_runs": False})
                    if len([row for row in detail["checkpoints"] if row["status"] == "completed"]) >= 2:
                        break
                time.sleep(0.1)
            assert run_id and call("get_chapter_shooting_script_unit_progress", resource) is None
            call("cancel_generation_task", {"task_id": early["task_id"], "attempt": early["attempt"]}, destructive=True)
            terminal(early, "cancelled")
            detail = call("get_workflow_run_detail", {"workflow_run_id": run_id, "include_generation_runs": False})
            assert detail["run"]["status"] == "interrupted" and detail["run"]["can_resume"]
            resumed = call("start_chapter_shooting_script_generation", {"meta": meta(), **source, "mode": "next_scene"})
            terminal(resumed)
            partial = call("get_chapter_shooting_script_unit_progress", resource)
            assert partial["workflow_run_id"] == run_id and partial["completed_unit_count"] == 1
            report["checks"].append("stopping before the first scene preserves completed stages and resumes the same workflow")

            configure("a4n-checkpoint-fail-shooting-design-second-shot")
            failed = call("start_chapter_shooting_script_generation", {"meta": meta(), **source, "mode": "next_scene"})
            terminal(failed, "failed")
            partial = call("get_chapter_shooting_script_unit_progress", resource)
            discarded_video = call("create_derived_project", {
                "root_project_id": resource["project_id"], "derived_kind": "video", "title": "保留来源删除前的场景",
                "description": "快照稳定性", "start_chapter": 1, "end_chapter": 1,
            })
            call("cancel_workflow", {"task_id": None, "workflow_run_id": partial["workflow_run_id"]}, destructive=True)
            assert call("get_chapter_shooting_script_unit_progress", resource) is None
            assert video_state(discarded_video)["source_sync_status"] == "root_updated"
            kept = call("get_chapter_shooting_script", {**resource, "project_id": discarded_video["id"]})
            assert len(kept["script"]["scenes"]) == 1
            report["checks"].append("discarding failed resumable work removes live progress, marks derived sources updated and preserves their frozen scenes")
            if args.prepare_browser:
                _, report["browser_source"], browser_video = fixture("浏览器改编恢复验证", video_before_plan=True)
                report["browser_video_project_id"] = browser_video["id"]
            report["success"] = True
    except (RunnerError, OSError, ValueError, TypeError, KeyError, AssertionError, subprocess.TimeoutExpired) as error:
        report["error"] = f"{type(error).__name__}: {error}"
    finally:
        secure_json(args.report, report)
    print(json.dumps(report, ensure_ascii=False))
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
