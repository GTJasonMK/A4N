#!/usr/bin/env python3
"""Fail when chat model call boundaries drift from their declared validation/recovery test inventory."""

import json
import re
from collections import Counter
from pathlib import Path

from run_creation_e2e import FULL_GENERATION_SUITES, SUITES

ROOT = Path(__file__).resolve().parents[2]
CALLS = re.compile(r"(?<!fn )\b(execute_llm_generation|execute_validated_llm_generation|execute_outline_llm_call|execute_portrait_llm_call|execute_task_llm_chat_response|stream_complete_llm|stream_complete_llm_with_sink|chat_json_stream_result_with_sink|run_workflow_llm_once|render_and_chat_trace_with_context|chat_stream)\s*\(")


def inspect(root=ROOT, ledger=None, suites=None):
    ledger = ledger if ledger is not None else json.loads((root / "scripts/tests/model_call_coverage.json").read_text())
    if ledger.get("schema_version") != 1 or not ledger.get("roots") or not ledger.get("owners"):
        raise ValueError("模型调用清单版本、扫描根或 owner 不能为空")
    suite_scripts = {name: row[0] for name, row in SUITES.items()} | FULL_GENERATION_SUITES if suites is None else suites
    observed = {}
    for alias, directory in ledger["roots"].items():
        parent = (root / directory).resolve()
        if not parent.is_dir() or not parent.is_relative_to(root.resolve()):
            raise ValueError("无效模型调用扫描根：" + directory)
        for source in sorted(parent.rglob("*.rs")):
            if source.name == "tests.rs" or "tests" in source.relative_to(parent).parts:
                continue
            text = re.split(r"#\[cfg\(test\)\]\s*mod tests\s*\{", source.read_text())[0]
            counts = dict(Counter(CALLS.findall(text)))
            if counts:
                observed[alias + ":" + str(source.relative_to(parent))] = counts
    declared = ledger["owners"]
    if set(observed) != set(declared):
        raise ValueError(f"模型调用登记不完整；新增：{sorted(set(observed)-set(declared))}；失效：{sorted(set(declared)-set(observed))}")
    for name, counts in observed.items():
        owner = declared[name]
        if owner.get("calls") != counts:
            raise ValueError(f"{name} 的调用次数或执行器已改变：{counts}")
        if owner.get("kind") not in {"text", "json", "agent", "driver"} or owner.get("recovery") not in {"bounded_once", "cancel_resume", "agent_budget", "terminal_error", "delegated"}:
            raise ValueError(name + " 缺少明确的输出及恢复策略")
        if not owner.get("suites"):
            raise ValueError(name + " 未登记功能验证套件")
        for suite in owner["suites"]:
            if suite not in suite_scripts or not (root / "scripts/tests" / suite_scripts[suite]).is_file():
                raise ValueError(name + " 引用了未注册或不存在的套件：" + suite)
    return {"owners": len(observed), "static_calls": sum(sum(row.values()) for row in observed.values()),
            "scope": ledger["scope"], "runtime_coverage_measured": False}


if __name__ == "__main__":
    try:
        print(json.dumps(inspect(), ensure_ascii=False, indent=2))
    except (OSError, ValueError, TypeError, KeyError) as error:
        raise SystemExit("模型调用登记检查失败：" + str(error))
