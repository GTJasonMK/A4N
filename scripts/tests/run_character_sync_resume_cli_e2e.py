#!/usr/bin/env python3
"""Cancel a character sync after two saved calls, then verify checkpoint inheritance."""

import argparse
import json
import os
import re
import tempfile
import time
import uuid
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from mock_openai_fixture_reverse import message_text
from mock_openai_fixture_story import chapter_body_response
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
    report = {"success": False, "checks": []}
    try:
        with tempfile.TemporaryDirectory(prefix="character-resume-", dir=args.report.parent) as directory:
            work = Path(directory)
            env = os.environ.copy()
            password = env.pop(args.password_env)
            env.pop("A4N_ACCESS_TOKEN", None)
            cli = Cli(args.a4n, args.server, work / "cli.json", work / "context.json", 60, env)
            checked(cli, ["health"], "health")
            checked(cli, ["capabilities", "check"], "CLI/Server compatibility")
            checked(cli, ["auth", "login", "--username", args.username, "--password-stdin"], "login", password + "\n")

            def parts(name, value, task_id=None):
                payload = {"input": value}
                if task_id is not None:
                    payload["taskId"] = task_id
                return ["call", name, "--args", json.dumps(payload, ensure_ascii=False)]

            def call(name, value, task_id=None):
                return checked(cli, parts(name, value, task_id), name)[0]["data"]

            def model(name):
                snapshot = runtime_snapshot(args.provider_url, embedding_dimension=384)
                snapshot["llm"]["model"] = name
                secure_json(work / "runtime.json", snapshot)
                checked(cli, ["config", "runtime", "set", "--input", "@" + str(work / "runtime.json")], "runtime")

            def stage_calls(offset):
                names = []
                for entry in provider_log_entries(args.provider_log)[offset:]:
                    combined = "\n".join(message_text(item.get("content", "")) for item in entry.get("body", {}).get("messages", []))
                    names.extend(re.findall(r'"name"\s*:\s*"(character_sync_[^"]+)"', combined))
                return names

            model("a4n-virtual-text")
            project = checked(cli, ["project", "create", "--title", "角色调用断点恢复回归", "--project-type", "novel_long", "--use-context"], "project")[0]["data"]
            chapter = checked(cli, ["chapter", "import", "--number", "1", "--title", "雨夜来信", "--content", chapter_body_response()], "chapter")[0]["data"]
            call("create_profile", {"project_id": project["id"], "name": "林岚", "base_attributes": {"explicit_attributes": {}, "implicit_attributes": {}, "social_attributes": {}}})
            profile = next(item for item in call("list_profiles", {"project_id": project["id"]}) if item["name"] == "林岚")
            report.update(project_id=project["id"], chapter_id=chapter["id"], profile_id=profile["id"])

            def runs():
                return call("list_workflow_runs", {"project_id": project["id"], "chapter_id": None,
                    "workflow_kind": "character_profile_sync", "status": None, "resumable_only": False, "limit": 20})

            def detail(run_id):
                return call("get_workflow_run_detail", {"workflow_run_id": run_id, "include_generation_runs": False})

            sync = {"project_id": project["id"], "name": "林岚", "chapter_number": 1}
            model("a4n-character-sync-cancel")
            before = len(provider_log_entries(args.provider_log))
            with ThreadPoolExecutor(max_workers=1) as pool:
                pending = pool.submit(cli.run, parts("sync_character_from_chapters", sync, str(uuid.uuid4())))
                deadline = time.monotonic() + 30
                original = None
                while time.monotonic() < deadline:
                    assert not pending.done(), pending.result() if pending.done() else None
                    candidates = runs()
                    if candidates:
                        original = max(candidates, key=lambda row: row["id"])
                        saved = detail(original["id"])["checkpoints"]
                        if len(saved) == 2 and "character_sync_chapter_states" in stage_calls(before):
                            break
                    time.sleep(0.1)
                else:
                    raise AssertionError("did not reach the third call after two completed checkpoints")
                assert all(item["status"] == "completed" for item in saved), saved
                cancel = parts("cancel_workflow", {"task_id": None, "workflow_run_id": original["id"]}) + ["--allow-destructive"]
                checked(cli, cancel, "cancel only the fixture workflow")
                assert pending.result(timeout=60)["exit_code"] != 0, "cancelled attempt must fail"
            cancelled = detail(original["id"])
            assert cancelled["run"]["status"] == "cancelled", cancelled
            assert len(cancelled["checkpoints"]) == 2, cancelled["checkpoints"]
            report["checks"].append("two calls persisted before cancellation; the late third response cannot write")

            model("a4n-virtual-text")
            before_retry = len(provider_log_entries(args.provider_log))
            result = call("sync_character_from_chapters", sync, str(uuid.uuid4()))
            assert result["status"] == "synced", result
            retried = stage_calls(before_retry)
            assert "character_sync_evidence_events" not in retried, retried
            assert "character_sync_attribute_changes" not in retried, retried
            assert "character_sync_chapter_states" in retried, retried
            latest = max(runs(), key=lambda row: row["id"])
            assert latest["id"] != original["id"] and latest["status"] == "completed", latest
            inherited = detail(latest["id"])["checkpoints"]
            for saved_call in saved:
                assert any(item["step_key"] == saved_call["step_key"] and item["status"] == "completed" for item in inherited), inherited
            assert detail(original["id"])["run"]["status"] == "cancelled"
            report.update(original_run_id=original["id"], retry_run_id=latest["id"], retry_stage_calls=retried)
            report["checks"].append("retry inherits completed calls, resumes at the unfinished third call, and completes")
            report["success"] = True
    except (RunnerError, OSError, ValueError, TypeError, KeyError, AssertionError, StopIteration, TimeoutError) as error:
        report["error"] = f"{type(error).__name__}: {error}"
    finally:
        secure_json(args.report, report)
    print(json.dumps(report, ensure_ascii=False))
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
