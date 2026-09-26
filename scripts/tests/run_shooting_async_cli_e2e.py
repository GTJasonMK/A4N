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

            def fixture(title):
                project = checked(cli, ["project", "create", "--title", title, "--project-type", "novel_long", "--use-context"], "project")[0]["data"]
                chapter = checked(cli, ["chapter", "import", "--number", "1", "--title", "门与机房", "--content", "林岚停在旧车站通道入口。她观察墙上的钟，推开门，进入地下机房。周衡留在门外。"], "chapter")[0]["data"]
                resource = {"project_id": project["id"], "source_chapter_id": chapter["id"]}
                scenes = [call("create_scene_profile", {"project_id": project["id"], "profile": {
                    "name": name, "scene_type": "room", "importance": "primary", "base_description": name,
                    "visual_description": "门框与挂钟", "spatial_layout": "通道通向门口", "atmosphere": "安静",
                    "first_seen_chapter": 1, "last_seen_chapter": 1, "image_prompt_seed": name,
                }})["profile"] for name in ["旧车站通道", "地下机房"]]
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
                return resource, source

            configure("a4n-shooting-background-slow")
            resource, source = fixture("异步剧本：断线与停止")
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
            partial = call("edit_chapter_shooting_script_progress", {**resource, "workflow_run_id": retained["workflow_run_id"],
                         "expected_script": retained["script"], "script": edited})["progress"]
            configure()
            planning = call("start_video_generation", {"project_id": resource["project_id"], "meta": meta(), "operation": {
                "kind": "prompt_segment_plan", "chapter_id": resource["source_chapter_id"], "options": {
                    "workflow_mode": "shared_segment_plan", "style": "电影感", "prompt_output_language": "zh_cn",
                    "prompt_batch_size": 1, "duration_secs": 6, "duration_secs_options": [6], "aspect_ratio": "16:9",
                    "reference_image_ids": [], "video_library_image_ids": [], "regenerate_existing": False,
                    "target_narrative_unit_index": 1,
                },
            }})
            terminal(planning)
            plans = call("list_chapter_video_prompts", {"projectId": resource["project_id"], "chapterId": resource["source_chapter_id"]}, raw=True)
            plan = next(item for item in plans if item["workflow_mode"] == "shared_segment_plan")
            assert plan["metadata_json"]["segments"] and all(segment["source_shot_id"] == "shot-1" for segment in plan["metadata_json"]["segments"])
            report["checks"].append("the saved first scene remains editable and usable by real segment planning after cancellation")

            before = len(provider_log_entries(args.provider_log))
            resumed = call("start_chapter_shooting_script_generation", {"meta": meta(), **source, "mode": "next_scene"})
            result = terminal(resumed)
            view = call("get_chapter_shooting_script", resource)
            assert len(view["script"]["scenes"]) == 2 and view["script"]["scenes"][0] == edited["scenes"][0]
            assert result["result_ref"] == {"kind": "chapter_shooting_script", "id": str(view["record"]["id"]), "revision": str(view["record"]["revision"])}
            assert not any("target_shot_id=shot-1" in json.dumps(row.get("body", {}), ensure_ascii=False)
                           for row in provider_log_entries(args.provider_log)[before:])
            report["checks"].append("a new observer resumes the remaining scene without regenerating the completed shot or losing author edits, and reads the final revision by result reference")

            configure("a4n-checkpoint-fail-shooting-design-second-shot")
            resource, source = fixture("异步剧本：失败恢复")
            failed = call("start_chapter_shooting_script_generation", {"meta": meta(), **source, "mode": "full"})
            terminal(failed, "failed")
            partial = call("get_chapter_shooting_script_unit_progress", resource)
            assert partial and partial["completed_unit_count"] == 1, partial
            configure()
            resumed = call("start_chapter_shooting_script_generation", {"meta": meta(), **source, "mode": "full"})
            terminal(resumed)
            assert len(call("get_chapter_shooting_script", resource)["script"]["scenes"]) == 2
            report["checks"].append("invalid output in the second scene fails visibly; the first scene survives and a later task completes the remaining work")
            report["success"] = True
    except (RunnerError, OSError, ValueError, TypeError, KeyError, AssertionError, subprocess.TimeoutExpired) as error:
        report["error"] = f"{type(error).__name__}: {error}"
    finally:
        secure_json(args.report, report)
    print(json.dumps(report, ensure_ascii=False))
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
