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
            if not messages_with_content(
                child_snapshot, "正文优化子 Agent 已完成", child_run_id
            ):
                raise RunnerError("child Agent 缺少真实完成消息")
            if not messages_with_content(
                parent_snapshot, "已验收正文优化子 Agent 的回传结果", parent_run_id
            ):
                raise RunnerError("parent Agent 缺少验收后的最终消息")

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
            }
            report["success"] = True
    except (RunnerError, OSError, ValueError, TypeError) as error:
        report["error"] = str(error)
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
