#!/usr/bin/env python3
"""Verify authoring results, edits, conflicts, and downstream inputs through real CLI calls."""

import json
import re
import tempfile
import time
import traceback
import uuid
from pathlib import Path

from mock_openai.fixtures.reverse import embedded_json_objects, message_text
from run_cli_contract_e2e import checked, cli_e2e_parser, command_error, logged_in_cli, provider_log_entries, runtime_snapshot, secure_json


def main():
    parser = cli_e2e_parser(__doc__)
    parser.add_argument("--through-stage", choices=("inspiration", "blueprint", "outline", "chapter", "analysis"), default="analysis")
    args = parser.parse_args()
    args.report.parent.mkdir(parents=True, exist_ok=True)
    report = {"success": False, "checks": [], "stages": {name: {"status": "not_run"} for name in ("inspiration", "blueprint", "outline", "chapter", "analysis")}}
    try:
        with tempfile.TemporaryDirectory(prefix="authoring-", dir=args.report.parent) as directory:
            work = Path(directory)
            cli = logged_in_cli(args, work)
            def configure(model="a4n-virtual-text"):
                snapshot = runtime_snapshot(args.provider_url)
                snapshot["llm"]["model"] = model
                secure_json(work / "runtime.json", snapshot)
                checked(cli, ["config", "runtime", "set", "--input", "@" + str(work / "runtime.json")], "runtime")
            configure()
            described = set()

            def call(name, value, *, raw=False, reject: str | None = None, destructive=False):
                if name not in described:
                    checked(cli, ["capabilities", "describe", name], name + " contract")
                    described.add(name)
                command = ["call", name, "--args", "-"]
                if destructive:
                    command.append("--allow-destructive")
                payload = json.dumps(value if raw else {"input": value}, ensure_ascii=False)
                if reject is not None:
                    result = cli.run(command, payload)
                    error = command_error(result)
                    assert result["exit_code"] in (2, 6, 8), (name, result)
                    assert reject in str(error), (name, result)
                    return error
                return checked(cli, command, name, payload)[0]["data"]

            def complete(handle, expected="completed"):
                watch = ["task", "watch", handle["task_id"], "--attempt", str(handle["attempt"])]
                if expected == "completed":
                    checked(cli, watch, "task watch")
                else:
                    watched = cli.run(watch)
                    assert watched["exit_code"] == (9 if expected == "cancelled" else 8), watched
                result = checked(cli, ["task", "get", handle["task_id"], "--attempt", str(handle["attempt"])], "task reread")[0]["data"]
                assert result["handle"]["status"] == expected, result
                return result

            def meta():
                return {"start_request_id": str(uuid.uuid4())}

            def calls():
                return [row for row in provider_log_entries(args.provider_log) if isinstance(row.get("body", {}).get("messages"), list)]

            def wire_since(start):
                return "\n".join(message_text(message.get("content", "")) for row in calls()[start:] for message in row["body"]["messages"])

            def passed(stage, name):
                report["checks"].append({"stage": stage, "name": name})
                secure_json(args.report, report)

            def stage_done(name):
                report["stages"][name] = {"status": "passed"}
                secure_json(args.report, report)

            project = checked(cli, ["project", "create", "--title", "创作阶段验收", "--project-type", "novel_long", "--use-context"], "project")[0]["data"]
            project_id = project["id"]
            report["project_id"] = project_id
            first_message = "首轮不可丢失：林岚必须保留旧怀表，故事为六章都市时间悬疑。"
            request = {"project_id": project_id, "meta": meta(), "message": first_message}
            before = len(calls())
            first = call("start_inspiration_generation", request)
            repeated = call("start_inspiration_generation", request)
            assert (first["task_id"], first["attempt"]) == (repeated["task_id"], repeated["attempt"])
            complete(first)
            history = call("get_inspiration_history", {"project_id": project_id})
            assert [row["role"] for row in history] == ["user", "assistant"] and history[0]["content"] == first_message
            assert len(calls()) - before == 1
            passed("inspiration", "duplicate start makes one model call and persists one complete exchange")
            before = len(calls())
            call("start_inspiration_generation", {"project_id": project_id, "meta": meta(), "message": " "}, reject="不能为空")
            assert len(calls()) == before and call("get_inspiration_history", {"project_id": project_id}) == history
            passed("inspiration", "invalid input does not call a model or append a partial exchange")
            for turn in range(2, 42):
                complete(call("start_inspiration_generation", {"project_id": project_id, "meta": meta(), "message": f"第 {turn} 轮：保留首轮要求，继续完善动机。"}))
            assert first_message in wire_since(len(calls()) - 1)
            history = call("get_inspiration_history", {"project_id": project_id})
            assert len(history) == 80 and history[-2]["content"].startswith("第 41 轮")
            passed("inspiration", "long history remains in model input while the history view applies its documented page limit")
            configure("a4n-checkpoint-fail-inspiration")
            failed = call("start_inspiration_generation", {"project_id": project_id, "meta": meta(), "message": "完整回复必须包含题材"})
            complete(failed, "failed")
            assert call("get_inspiration_history", {"project_id": project_id}) == history
            configure("a4n-llm-lifecycle-slow")
            before = len(calls())
            cancelled = call("start_inspiration_generation", {"project_id": project_id, "meta": meta(), "message": "这一轮会被用户停止"})
            deadline = time.monotonic() + 10
            while len(calls()) == before and time.monotonic() < deadline:
                time.sleep(.1)
            assert len(calls()) > before, "cancellation must interrupt an actual model call"
            call("cancel_generation_task", {"task_id": cancelled["task_id"], "attempt": cancelled["attempt"]}, destructive=True)
            complete(cancelled, "cancelled")
            assert call("get_inspiration_history", {"project_id": project_id}) == history
            configure()
            passed("inspiration", "invalid model output and active cancellation do not persist a half exchange")
            no_key = runtime_snapshot(args.provider_url)
            no_key.pop("llm")
            no_key["llm_configs"] = []
            no_key["llm_default_source"] = "system"
            secure_json(work / "runtime.json", no_key)
            checked(cli, ["config", "runtime", "set", "--input", "@" + str(work / "runtime.json")], "missing personal key")
            before = len(calls())
            unavailable = call("start_inspiration_generation", {"project_id": project_id, "meta": meta(), "message": "没有密钥时也必须结束并显示失败"})
            failed_state = complete(unavailable, "failed")
            assert "个人 API Key" in json.dumps(failed_state, ensure_ascii=False)
            assert len(calls()) == before
            stream_file = work / "failed-events.txt"
            replay = cli.run(["request", "get", f"/generation-tasks/{unavailable['task_id']}/events?attempt={unavailable['attempt']}", "--output", str(stream_file)])
            assert replay["exit_code"] == 0 and stream_file.is_file(), replay
            events = [json.loads(line[5:].strip()) for line in stream_file.read_text().splitlines() if line.startswith("data:")]
            assert any(event["event"]["kind"] == "failed" for event in events)
            configure()
            passed("inspiration", "pre-stream configuration failure reaches observers and closes the event stream")
            stage_done("inspiration")
            if args.through_stage == "inspiration":
                report["success"] = True
                return 0

            before = len(calls())
            outcome = complete(call("start_blueprint_generation", {"project_id": project_id, "meta": meta(), "title": "逆行时钟", "genre": "都市时间悬疑", "target_length": 6}))
            blueprint = call("get_blueprint", {"projectId": project_id}, raw=True)
            assert blueprint["id"] == int(outcome["result_ref"]["id"])
            assert blueprint["current_version"] == outcome["result_ref"]["revision"]
            assert blueprint["full_synopsis"] and blueprint["world_setting"] and len(calls()) - before == 4
            versions = call("list_blueprint_versions", {"project_id": project_id})
            assert any(row["version"] == blueprint["current_version"] for row in versions)
            passed("blueprint", "four stages publish a readable version matching the task result")
            before = len(calls())
            complete(call("start_blueprint_refinement", {"project_id": project_id, "meta": meta(), "focus": "保留首轮设定，强化动机"}))
            refined = call("get_blueprint", {"projectId": project_id}, raw=True)
            assert refined["id"] == blueprint["id"] and refined["current_version"] != blueprint["current_version"]
            assert first_message in wire_since(before)
            assert refined["world_setting"] == blueprint["world_setting"]
            passed("blueprint", "refinement keeps full inspiration and untouched fields and creates a new version")
            author_marker = "作者蓝图约束：旧怀表不能离开林岚。"
            edit = {key: refined[key] for key in ("genre", "style", "tone", "target_audience", "one_sentence_summary", "full_synopsis")}
            edit.update(project_id=project_id, total_chapters=6, full_synopsis=refined["full_synopsis"] + author_marker)
            before = len(calls())
            call("save_blueprint", edit)
            saved = call("get_blueprint", {"projectId": project_id}, raw=True)
            assert saved["full_synopsis"] == edit["full_synopsis"] and saved["world_setting"] == refined["world_setting"]
            assert len(calls()) == before
            passed("blueprint", "manual editing persists exact text without a model call")
            call("restore_blueprint_version", {"project_id": project_id, "version": blueprint["current_version"]}, destructive=True)
            restored = call("get_blueprint", {"projectId": project_id}, raw=True)
            assert restored["full_synopsis"] == blueprint["full_synopsis"] and restored["world_setting"] == blueprint["world_setting"]
            call("save_blueprint", edit)
            assert call("get_blueprint", {"projectId": project_id}, raw=True)["full_synopsis"] == edit["full_synopsis"]
            passed("blueprint", "restoring a version restores content, and subsequent manual edits remain writable")
            stage_done("blueprint")
            if args.through_stage == "blueprint":
                report["success"] = True
                return 0

            complete(call("start_outline_generation", {"project_id": project_id, "meta": meta(), "operation": {"kind": "generate_parts", "part_count": 2, "chapters_per_part": 3}}))
            parts = call("list_parts", {"projectId": project_id}, raw=True)
            assert [(p["part_number"], p["start_chapter"], p["end_chapter"]) for p in parts] == [(1, 1, 3), (2, 4, 6)]
            assert author_marker in wire_since(before)
            part = parts[0]
            part_edit = {key: part[key] for key in ("part_number", "start_chapter", "end_chapter", "title", "summary", "theme")}
            part_edit.update(project_id=project_id, part_id=part["id"], expected_updated_at=part["updated_at"], summary=part["summary"] + "分部手工约束：林岚检查机房入口。")
            call("save_part_outline", part_edit)
            call("save_part_outline", part_edit, reject="大纲资料已经变化")
            reread = call("list_parts", {"projectId": project_id}, raw=True)
            assert reread[0]["summary"] == part_edit["summary"] and reread[1] == parts[1]
            passed("outline", "part generation uses edited blueprint; editing rejects stale versions and preserves other parts")
            complete(call("start_outline_generation", {"project_id": project_id, "meta": meta(), "operation": {"kind": "generate_part_chapter_outlines", "part_id": part["id"]}}))
            outlines = call("list_chapter_outlines", {"projectId": project_id}, raw=True)
            assert [row["chapter_number"] for row in outlines] == [1, 2, 3]
            outline = outlines[0]
            outline_marker = "章纲手工约束：林岚把旧怀表收进衣袋。"
            outline_edit = {"project_id": project_id, "outline_id": outline["id"], "expected_updated_at": outline["updated_at"], "summary": outline["summary"] + outline_marker}
            call("update_chapter_outline", outline_edit)
            call("update_chapter_outline", outline_edit, reject="章节大纲已经变化")
            reread = call("list_chapter_outlines", {"projectId": project_id}, raw=True)
            assert reread[0]["summary"] == outline_edit["summary"] and reread[1:] == outlines[1:]
            passed("outline", "chapter outline edits survive reread, reject stale writes, and preserve neighboring outlines")
            stage_done("outline")
            if args.through_stage == "outline":
                report["success"] = True
                return 0

            before = len(calls())
            complete(call("start_chapter_generation", {"project_id": project_id, "meta": meta(), "chapter_number": 1, "min_word_count": 800, "max_word_count": 1600}))
            chapters = call("list_chapters", {"projectId": project_id}, raw=True)
            assert len(chapters) == 1 and chapters[0]["content"] and outline_marker in wire_since(before)
            chapter = chapters[0]
            assert chapter["chapter_number"] == 1 and chapter["outline_id"] == outline["id"]
            workflow = chapter["derived_sync_workflow_run_id"]
            if workflow:
                checked(cli, ["workflow", "watch", str(workflow), "--interval-ms", "200", "--timeout-seconds", "60"], "derived material sync")
            passed("chapter", "generation consumes the edited outline and its persisted chapter points to that outline")
            fresh = call("list_chapters", {"projectId": project_id}, raw=True)[0]
            edited_content = fresh["content"] + "\n\n作者补充：林岚在门口停顿片刻。"
            payload = {"chapter_id": fresh["id"], "expected_revision": fresh["edit_revision"], "content": edited_content, "title": fresh["title"]}
            before = len(calls())
            call("update_chapter_content", payload)
            call("update_chapter_content", payload, reject="保存旧版本章节正文")
            saved = call("list_chapters", {"projectId": project_id}, raw=True)[0]
            assert saved["content"] == edited_content and saved["edit_revision"] != fresh["edit_revision"]
            assert len(calls()) == before
            passed("chapter", "saving exact body text invalidates the old revision without hidden model calls")
            stage_done("chapter")
            if args.through_stage == "chapter":
                report["success"] = True
                return 0
            before = len(calls())
            analysis = call("generate_chapter_analysis", {"chapter_id": chapter["id"]})
            assert analysis["warnings"] == [], analysis
            analyzed = call("list_chapters", {"projectId": project_id}, raw=True)[0]
            assert analyzed["analysis_data"], "analysis did not persist its structured result"
            sources = [re.sub(r"#\d+ ", "", value["content"])
                       for value in embedded_json_objects(wire_since(before)) if isinstance(value.get("content"), str)]
            assert edited_content in sources, "analysis source must equal the saved body after removing source markers"
            assert analyzed["content"] == edited_content, "analysis must not rewrite the saved body"
            profiles = call("list_profiles", {"project_id": project_id})
            assert any(row["name"] == "林岚" for row in profiles)
            passed("analysis", "analysis reads the saved revision, preserves body text, and publishes readable character materials")
            stage_done("analysis")
            report["success"] = True
    except Exception as error:
        report["error"] = f"{type(error).__name__}: {error}"
        report["traceback"] = traceback.format_exc()
        for name, state in report["stages"].items():
            if state["status"] == "not_run":
                report["stages"][name] = {"status": "failed", "error": report["error"]}
                break
    finally:
        secure_json(args.report, report)
        print(json.dumps({"success": report["success"], "checks": report["checks"], "report": str(args.report)}, ensure_ascii=False), flush=True)
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
