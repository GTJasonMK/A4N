#!/usr/bin/env python3
"""Opt-in business evaluation through production CLI/Server validators and a bounded live-provider relay."""

import json
import math
import os
import tempfile
import time
import uuid
from pathlib import Path
from urllib.parse import urlsplit

from budgeted_llm_proxy import BudgetedLlmProxy
from mock_openai.fixtures.story import chapter_body_response
from run_cli_contract_e2e import RunnerError, checked, cli_e2e_parser, logged_in_cli, runtime_snapshot, secure_json


def evaluation_materials():
    original = chapter_body_response()
    observations = [
        f"第{index}份检修记录写明，林岚与周衡在清晨检查钟楼。林岚将旧怀表放在工具箱左侧，周衡只核对记录，没有改变指针。两人随后回到旧车站通道，约定保留原始记录作为证据。"
        for index in range(1, 49)
    ]
    return {"baseline": original, "extended": original + "\n\n" + "\n\n".join(observations[:6]),
            "long": original + "\n\n" + "\n\n".join(observations)}


def metrics(cases):
    total = len(cases)
    passed = [case for case in cases if case["success"]]
    return {"case_count": total, "passed": len(passed),
            "first_pass_rate": sum(not case["correction_calls"] for case in passed) / total if total else 0,
            "validated_success_rate": len(passed) / total if total else 0,
            "all_cases_passed": bool(total) and len(passed) == total}


def main():
    parser = cli_e2e_parser(__doc__)
    parser.add_argument("--llm-config", type=Path, required=True, help="LLM config object; credentials are supplied separately")
    parser.add_argument("--api-key-env", required=True)
    parser.add_argument("--samples", type=int, required=True)
    parser.add_argument("--material", choices=("baseline", "extended", "long"), action="append", help="default: all three synthetic material sizes")
    parser.add_argument("--max-calls", type=int, required=True)
    parser.add_argument("--max-output-tokens", type=int, required=True)
    parser.add_argument("--timeout-seconds", type=float, required=True)
    parser.add_argument("--isolated-server", action="store_true", help="confirm the supplied Server uses a disposable test database/data directory")
    parser.add_argument("--allow-local-http", action="store_true", help="only for validating this runner against a loopback fixture")
    args = parser.parse_args()
    if not args.isolated_server or args.samples <= 0 or args.max_calls <= 0 or args.max_output_tokens <= 0 or not math.isfinite(args.timeout_seconds) or args.timeout_seconds <= 0:
        parser.error("必须指定隔离 Server 并提供正数 samples；不可连接日常创作数据库")
    report = {"success": False, "mode": "live", "cases": [], "calls": [],
              "scope": "chapter analysis and character/object/scene synchronization; content quality is not graded",
              "budget": {"max_calls": args.max_calls, "max_output_tokens_per_call": args.max_output_tokens, "timeout_seconds": args.timeout_seconds}}
    args.report.parent.mkdir(parents=True, exist_ok=True)
    try:
        llm = json.loads(args.llm_config.read_text())
        if not isinstance(llm, dict) or any(not isinstance(llm.get(key), str) or not llm[key].strip() for key in ("provider", "base_url", "model")):
            raise RunnerError("llm-config 必须包含 provider、base_url 和 model")
        key = os.environ.get(args.api_key_env, "")
        if not key:
            raise RunnerError("指定的模型凭证环境变量为空；未执行真实模型验收")
        report.update(provider=llm["provider"], model=llm["model"], mode="fixture_runner_validation" if urlsplit(llm["base_url"]).scheme == "http" else "live")
        with tempfile.TemporaryDirectory(prefix="model-eval-", dir=args.report.parent) as directory:
            work = Path(directory)
            cli = logged_in_cli(args, work, timeout=min(args.timeout_seconds, 600))
            with BudgetedLlmProxy(llm["base_url"], key, max_calls=args.max_calls, max_output_tokens=args.max_output_tokens,
                                  timeout_seconds=args.timeout_seconds, allow_local_http=args.allow_local_http) as proxy:
                snapshot = runtime_snapshot(args.provider_url, embedding_dimension=384, llm_provider=llm["provider"])
                snapshot["llm"].update(llm, base_url=proxy.base_url, api_key_encrypted=proxy.token,
                                       max_output_tokens=args.max_output_tokens, max_concurrent=1, is_default=True)
                snapshot["llm_configs"] = [snapshot["llm"]]
                secure_json(work / "runtime.json", snapshot)
                checked(cli, ["config", "runtime", "set", "--input", "@" + str(work / "runtime.json")], "evaluation runtime")

                def call(name, value):
                    return checked(cli, ["call", name, "--args", json.dumps({"input": value}, ensure_ascii=False)], name)[0]["data"]

                materials = evaluation_materials()
                planned = [(name, repeat + 1) for name in (args.material or materials) for repeat in range(args.samples)]
                for material, sample in planned:
                    case = {"material": material, "sample": sample, "source_chars": len(materials[material]), "success": False, "correction_calls": 0}
                    report["cases"].append(case)
                    offset = len(proxy.calls)
                    started = time.monotonic()
                    try:
                        if time.monotonic() >= proxy.deadline or proxy.forwarded_calls >= proxy.max_calls:
                            raise RunnerError("评测预算已用尽，剩余样本未执行")
                        project = checked(cli, ["project", "create", "--title", "模型验收-" + str(uuid.uuid4()), "--project-type", "novel_long", "--use-context"], "eval project")[0]["data"]
                        chapter = checked(cli, ["chapter", "import", "--number", "1", "--title", "雨夜来信", "--content", materials[material]], "eval chapter")[0]["data"]
                        case.update(project_id=project["id"], chapter_id=chapter["id"])
                        result = call("generate_chapter_analysis", {"chapter_id": chapter["id"]})
                        if result["warnings"] or result["chapter"]["analysis_sync_can_resume"]:
                            raise RunnerError("分析或资料同步未全部完成，保留隔离项目供排查")
                        if result["chapter"]["content"] != chapter["content"]:
                            raise RunnerError("分析改变了已保存正文")
                        for command in ("list_profiles", "list_object_profiles", "list_scene_profiles"):
                            if not call(command, {"project_id": project["id"]}):
                                raise RunnerError(command + " 未产生来源明确的实体，不能以空结果通过验收")
                        case["success"] = True
                    except (RunnerError, OSError, ValueError, TypeError, KeyError) as error:
                        case["error"] = type(error).__name__ + ": " + str(error)
                    finally:
                        case["duration_ms"] = round((time.monotonic() - started) * 1000)
                        case["correction_calls"] = sum(row["correction"] for row in proxy.calls[offset:])
                        case["provider_calls"] = len(proxy.calls) - offset
                report["calls"] = proxy.calls
                report.update(metrics(report["cases"]))
                report["success"] = report["all_cases_passed"] and proxy.forwarded_calls > 0 and not any(row.get("error") or row["status"] != 200 or not row["stream_completed"] for row in proxy.calls)
    except (RunnerError, OSError, ValueError, TypeError, KeyError) as error:
        report["error"] = type(error).__name__ + ": " + str(error)
    finally:
        secure_json(args.report, report)
    print(json.dumps({"success": report["success"], "mode": report["mode"], "report": str(args.report)}, ensure_ascii=False))
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
