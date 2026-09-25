#!/usr/bin/env python3
"""Run the smallest real Agent CLI workflow against an A4N Server.

The script deliberately uses the ordinary CLI for every step.  It only owns
the polling/assertion glue that a manifest ledger cannot express yet:
main-agent locate -> delegation -> child completion -> parent acceptance,
then a user followup sent through the same delegation.
"""

from __future__ import annotations

import argparse
import json
import os
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from run_cli_contract_e2e import (
    Cli,
    RunnerError,
    check_provider_contract,
    checked,
    provider_log_entries,
    runtime_snapshot,
    secure_json,
    start_provider,
)
from specialized_agent_fixture import GENERAL_MARKER, observation_data


TERMINAL_RUN_STATUSES = {
    "completed",
    "failed",
    "cancelled",
    "interrupted",
    "recovery_required",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--a4n",
        default=os.environ.get(
            "A4N_CLI", str(Path(__file__).parents[2] / "target/debug/a4n")
        ),
    )
    parser.add_argument("--server", required=True)
    parser.add_argument("--username", required=True)
    parser.add_argument("--password-env", default="A4N_PASSWORD")
    parser.add_argument("--report", type=Path)
    parser.add_argument("--provider-url")
    parser.add_argument(
        "--provider-script",
        type=Path,
        default=Path(__file__).with_name("mock_openai_provider.py"),
    )
    parser.add_argument("--provider-host", default="127.0.0.1")
    parser.add_argument("--provider-port", type=int, default=0)
    parser.add_argument("--provider-start-timeout", type=float, default=10)
    parser.add_argument("--command-timeout", type=float, default=30)
    parser.add_argument("--poll-interval", type=float, default=0.5)
    parser.add_argument("--poll-timeout", type=float, default=120)
    return parser.parse_args()


def data(envelope: dict[str, Any], label: str) -> dict[str, Any]:
    value = envelope.get("data")
    if not isinstance(value, dict):
        raise RunnerError(f"{label} 未返回 object data")
    return value


def required_text(value: Any, field: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise RunnerError(f"响应缺少有效 {field}")
    return value


def thread_snapshot(cli: Cli, thread_id: str) -> dict[str, Any]:
    envelope, _ = checked(cli, ["agent", "thread", "show", thread_id], "agent thread show")
    return data(envelope, "agent thread show")


def run_view(cli: Cli, run_id: str) -> dict[str, Any]:
    envelope, _ = checked(cli, ["agent", "run-show", run_id], "agent run show")
    return data(envelope, "agent run show")


def delegation_view(
    snapshot: dict[str, Any], parent_run_id: str
) -> dict[str, Any] | None:
    rows = snapshot.get("delegations")
    if not isinstance(rows, list):
        return None
    for row in rows:
        if isinstance(row, dict) and str(row.get("parent_run_id")) == parent_run_id:
            return row
    return None


def messages_with_content(
    snapshot: dict[str, Any], needle: str, run_id: str
) -> list[dict[str, Any]]:
    rows = snapshot.get("messages")
    if not isinstance(rows, list):
        return []
    return [
        row
        for row in rows
        if isinstance(row, dict)
        and str(row.get("run_id")) == run_id
        and needle in str(row.get("content", ""))
    ]


def wait_for_parent_and_child(
    cli: Cli,
    parent_run_id: str,
    parent_thread_id: str,
    timeout: float,
    interval: float,
) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any]]:
    deadline = time.monotonic() + timeout
    last_parent: dict[str, Any] = {}
    last_snapshot: dict[str, Any] = {}
    last_delegation: dict[str, Any] | None = None
    while time.monotonic() < deadline:
        last_parent = run_view(cli, parent_run_id)
        last_snapshot = thread_snapshot(cli, parent_thread_id)
        last_delegation = delegation_view(last_snapshot, parent_run_id)
        parent_status = last_parent.get("status")
        delegation_status = last_delegation.get("status") if last_delegation else None
        if parent_status in TERMINAL_RUN_STATUSES and delegation_status in {
            "accepted",
            "failed",
            "cancelled",
            "interrupted",
        }:
            break
        time.sleep(interval)
    if not last_delegation:
        raise RunnerError("主 Agent 在轮询期限内没有创建 delegation")
    if last_parent.get("status") != "completed":
        raise RunnerError(
            f"主 Agent 未完成: status={last_parent.get('status')!r}, run={parent_run_id}"
        )
    if last_delegation.get("status") != "accepted":
        raise RunnerError(
            f"子 Agent delegation 未被主 Agent 验收: status={last_delegation.get('status')!r}"
        )
    return last_parent, last_snapshot, last_delegation


def wait_for_run(cli: Cli, run_id: str, timeout: float, interval: float) -> dict[str, Any]:
    deadline = time.monotonic() + timeout
    latest: dict[str, Any] = {}
    while time.monotonic() < deadline:
        latest = run_view(cli, run_id)
        if latest.get("status") in TERMINAL_RUN_STATUSES:
            return latest
        time.sleep(interval)
    raise RunnerError(f"Agent run 轮询超时: {run_id}, status={latest.get('status')!r}")


def main() -> int:
    args = parse_args()
    run_id = f"{datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')}-{os.getpid()}"
    report_path = (
        args.report or Path("data/cli-contract-e2e/agent") / f"{run_id}.json"
    ).expanduser().resolve()
    report_path.parent.mkdir(parents=True, exist_ok=True)
    environment = os.environ.copy()
    password = environment.get(args.password_env)
    if not password:
        raise RunnerError(f"--username 已提供，但环境变量 {args.password_env} 为空")
    environment.pop("A4N_ACCESS_TOKEN", None)
    environment.pop(args.password_env, None)

    report: dict[str, Any] = {
        "checks": [],
        "schema_version": 1,
        "run_id": run_id,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "server": args.server,
        "success": False,
    }
    provider = None
    provider_log: Path | None = None
    scenario_start = 0
    try:
        with tempfile.TemporaryDirectory(prefix="a4n-agent-cli-e2e-") as temporary:
            work = Path(temporary)
            cli = Cli(
                args.a4n,
                args.server,
                work / "cli-config.json",
                work / "cli-context.json",
                args.command_timeout,
                environment,
            )
            checked(cli, ["health"], "health")
            checked(cli, ["capabilities", "check"], "capabilities check")

            provider, provider_log = start_provider(args, report_path.parent, run_id)
            report["provider_url"] = provider.base_url
            report["provider_contract"] = check_provider_contract(
                provider.base_url, args.command_timeout
            )
            runtime_path = work / "runtime-config.json"
            secure_json(runtime_path, runtime_snapshot(provider.base_url))
            checked(
                cli,
                ["config", "runtime", "set", "--input", f"@{runtime_path}"],
                "config runtime set",
            )
            checked(
                cli,
                ["auth", "login", "--username", args.username, "--password-stdin"],
                "auth login",
                f"{password}\n",
            )

            project, _ = checked(
                cli,
                [
                    "project",
                    "create",
                    "--title",
                    f"A4N Agent E2E {run_id}",
                    "--project-type",
                    "novel_long",
                    "--use-context",
                ],
                "project create",
            )
            project_value = data(project, "project create")
            project_id = project_value.get("id")
            if not isinstance(project_id, int) or project_id <= 0:
                raise RunnerError("project create 未返回有效 project id")
            chapter, _ = checked(
                cli,
                [
                    "chapter",
                    "import",
                    "--number",
                    "1",
                    "--title",
                    "雨夜来信",
                    "--content",
                    "暴雨夜里，旧车站的钟每天慢七分钟。林岚在候车室收到一封写给未来自己的信。",
                ],
                "chapter import",
            )
            chapter_value = data(chapter, "chapter import")
            chapter_id = chapter_value.get("id")
            if not isinstance(chapter_id, int) or chapter_id <= 0:
                raise RunnerError("chapter import 未返回有效 chapter id")
            report.update(project_id=project_id, chapter_id=chapter_id)

            accepted, _ = checked(
                cli,
                [
                    "agent",
                    "ask",
                    "请优化最近写作项目的最新一章正文，保持剧情事实与人物设定一致。",
                ],
                "agent ask",
            )
            accepted_value = data(accepted, "agent ask")
            parent_thread_id = required_text(accepted_value.get("thread_id"), "parent thread_id")
            parent_run_id = required_text(accepted_value.get("run_id"), "parent run_id")
            report.update(parent_thread_id=parent_thread_id, parent_run_id=parent_run_id)

            scenario_start = len(provider_log_entries(provider_log))
            parent_run, parent_snapshot, delegation = wait_for_parent_and_child(
                cli,
                parent_run_id,
                parent_thread_id,
                args.poll_timeout,
                args.poll_interval,
            )
            child_run_id = required_text(delegation.get("child_run_id"), "child_run_id")
            child_thread_id = required_text(delegation.get("child_thread_id"), "child_thread_id")
            delegation_id = required_text(delegation.get("id"), "delegation_id")
            child_snapshot = thread_snapshot(cli, child_thread_id)
            child_run = next(
                (
                    row
                    for row in child_snapshot.get("runs", [])
                    if isinstance(row, dict) and str(row.get("id")) == child_run_id
                ),
                {},
            )
            if child_run.get("status") != "completed":
                raise RunnerError(f"child Agent 未完成: {child_run.get('status')!r}")
            child_executions = [row for row in child_snapshot["executions"] if row["run_id"] == child_run_id]
            if not any(row["tool_name"] == "edit_article" and row["status"] == "succeeded"
                       and any(ref["kind"] == "optimization_patch_set" for ref in row["result_refs"])
                       for row in child_executions):
                raise RunnerError("child Agent 未生成真实 PatchSet")
            if child_run["model_calls_used"] != sum(row["kind"] == "model_call" for row in child_executions) or child_run["tool_calls_used"] != sum(row["kind"] != "model_call" for row in child_executions):
                raise RunnerError("专业执行的模型/工具计数与真实执行记录不一致")
            if not any(row["run_id"] == parent_run_id and row["tool_name"] == "review_agent_delegation"
                       and row["status"] == "succeeded" for row in parent_snapshot["executions"]):
                raise RunnerError("parent Agent 缺少显式验收执行")
            if parent_run["model_calls_used"] <= child_run["model_calls_used"] or parent_run["tokens_used"] <= child_run["tokens_used"]:
                raise RunnerError("父子成本未累计到共享用量")
            if not messages_with_content(child_snapshot, "正文尚未写回", child_run_id):
                raise RunnerError("child Agent 未区分提案与正式写回")
            current, _ = checked(cli, ["chapter", "get", str(chapter_id)], "verify unchanged chapter")
            if data(current, "chapter get")["content"] != chapter_value["content"]:
                raise RunnerError("待确认提案意外写回了章节正文")

            followup, _ = checked(
                cli,
                [
                    "agent",
                    "child",
                    "send",
                    delegation_id,
                    "请复核刚才的优化结果，并补充一句简短的交付说明。",
                ],
                "agent child send followup",
            )
            followup_value = data(followup, "agent child send followup")
            if followup_value.get("thread_id") != child_thread_id:
                raise RunnerError("child followup 没有回到同一个 child thread")
            followup_run_id = required_text(followup_value.get("run_id"), "followup run_id")
            followup_run = wait_for_run(cli, followup_run_id, args.poll_timeout, args.poll_interval)
            if followup_run.get("status") != "completed":
                raise RunnerError(f"child followup 未完成: {followup_run.get('status')!r}")
            followup_snapshot = thread_snapshot(cli, child_thread_id)
            if len(
                [
                    row
                    for row in followup_snapshot.get("runs", [])
                    if isinstance(row, dict) and str(row.get("id")) == followup_run_id
                ]
            ) != 1:
                raise RunnerError("child followup run 未持久化到同一 thread")

            general_thread, _ = checked(
                cli,
                [
                    "agent",
                    "thread",
                    "create",
                    "--kind",
                    "subagent",
                    "--capability",
                    "general",
                    "--project-id",
                    str(project_id),
                    "--resource-kind",
                    "project",
                    "--resource-id",
                    str(project_id),
                ],
                "general thread",
            )
            general_thread_value = data(general_thread, "general thread")
            general_run_value, _ = checked(
                cli,
                [
                    "agent",
                    "chat",
                    general_thread_value["id"],
                    GENERAL_MARKER + json.dumps({"project_id": project_id}, ensure_ascii=False),
                ],
                "general Agent chat",
            )
            general_run_id = required_text(data(general_run_value, "general Agent chat").get("run_id"), "general run_id")
            general_run = wait_for_run(cli, general_run_id, args.poll_timeout, args.poll_interval)
            general_snapshot = thread_snapshot(cli, general_thread_value["id"])
            general_executions = [row for row in general_snapshot["executions"] if row["run_id"] == general_run_id]
            assert general_run["status"] == "completed"
            assert [row["tool_name"] for row in general_executions if row["kind"] == "tool_call" and row["status"] == "succeeded"] == [
                "search_agent_tools",
                "describe_agent_tool",
                "get_project",
            ]
            report["checks"].append("general Agent discovers a project tool, loads its schema and reads a real project result")

            revision_value = data(checked(cli, [
                "agent", "chat", parent_thread_id,
                "A4N_REVIEW_REVISE 请优化最近写作项目的最新一章；首次交付后要求一次修订，再读取实际结果并验收。",
                "--budget", json.dumps({"max_steps": 64, "max_model_calls": 64}),
            ], "request revision workflow")[0], "revision workflow")
            revision_run_id = revision_value["run_id"]
            _, revision_parent, revised_delegation = wait_for_parent_and_child(
                cli, revision_run_id, parent_thread_id, args.poll_timeout, args.poll_interval,
            )
            reviews = [row for row in revision_parent["executions"]
                       if row["run_id"] == revision_run_id and row["tool_name"] == "review_agent_delegation"]
            review_inputs = [call["arguments"]["input"] for row in revision_parent["messages"]
                             if row["run_id"] == revision_run_id
                             for call in (row.get("metadata") or {}).get("tool_calls", [])
                             if call["name"] == "review_agent_delegation"]
            assert [value["decision"] for value in review_inputs] == ["request_revision", "accept"], review_inputs
            assert len(reviews) == 2 and all(row["status"] == "succeeded" for row in reviews), reviews
            child_ids = [value["expected_child_run_id"] for value in review_inputs]
            assert child_ids[0] != child_ids[1] == revised_delegation["child_run_id"], child_ids
            assert len([row for row in revision_parent["delegations"] if row["parent_run_id"] == revision_run_id]) == 1
            revised_child = thread_snapshot(cli, revised_delegation["child_thread_id"])
            assert all(any(row["id"] == child_id and row["status"] == "completed" for row in revised_child["runs"]) for child_id in child_ids)
            patches = [call["arguments"]["patches"][0]["new_text"] for row in revised_child["messages"]
                       if row["run_id"] in child_ids
                       for call in (row.get("metadata") or {}).get("tool_calls", []) if call["name"] == "edit_article"]
            assert len(patches) == 2 and patches[0] != patches[1] and "没有立即拆开" in patches[1], patches
            delivered_patch_ids = {ref["id"] for ref in revised_delegation["result_refs"] if ref["kind"] == "optimization_patch_set"}
            actual_patches = [patch for row in revision_parent["messages"] if row["run_id"] == revision_run_id
                              and (row.get("metadata") or {}).get("tool") == "list_optimize_patch_sets"
                              for patch in observation_data(row) if str(patch["id"]) in delivered_patch_ids]
            assert any(patch["patches"][0]["new_text"] == patches[1] for patch in actual_patches), actual_patches
            current = data(checked(cli, ["chapter", "get", str(chapter_id)], "verify revised proposal")[0], "chapter")
            assert current["content"] == chapter_value["content"]
            secure_json(report_path.parent / "revision-parent.json", revision_parent)
            secure_json(report_path.parent / "revision-child.json", revised_child)
            report["checks"].append("one delegation requests revision, preserves its child thread and parent Run, then accepts the reread revised patch")

            report["agent"] = {
                "parent": {"run_id": parent_run_id, "status": parent_run.get("status")},
                "delegation": {
                    "id": delegation_id,
                    "status": delegation.get("status"),
                    "child_thread_id": child_thread_id,
                    "child_run_id": child_run_id,
                },
                "child": {"run_id": child_run_id, "status": child_run.get("status")},
                "followup": {
                    "thread_id": child_thread_id,
                    "run_id": followup_run_id,
                    "status": followup_run.get("status"),
                },
                "general": {"thread_id": general_thread_value["id"], "run_id": general_run_id, "status": general_run["status"]},
            }
            report["success"] = True
    except (RunnerError, OSError, ValueError, TypeError, AssertionError, KeyError) as error:
        report["error"] = f"{type(error).__name__}: {error}"
    finally:
        if provider:
            provider.stop()
    if provider_log:
        entries = provider_log_entries(provider_log)
        report["provider_requests"] = {
            "total": len(entries),
            "scenario": max(0, len(entries) - scenario_start),
            "log": str(provider_log),
        }
    secure_json(report_path, report)
    print(
        json.dumps(
            {
                "success": report["success"],
                "report": str(report_path),
                "error": report.get("error"),
            },
            ensure_ascii=False,
        )
    )
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
