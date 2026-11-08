#!/usr/bin/env python3
"""Check Cargo-wide AST discovery and stable business contract registration.

This is inventory validation. Executed case evidence is verified independently;
file existence or membership of a suite is never reported as runtime coverage.
"""
from __future__ import annotations
import argparse
import json
import os
import subprocess
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
MANIFEST="scripts/tests/model_calls.json"
BASELINE="scripts/tests/model_call_sites.json"
ORCHESTRATOR="crates/host/a4n-orchestrator/src/"


def scan(root:Path=ROOT,binary:Path|None=None)->dict:
    selected=binary or os.environ.get("A4N_MODEL_TEST_TOOLS_BINARY")
    command=[str(selected),str(root)] if selected else ["cargo","run","--locked","-q","-p","a4n-model-test-tools","--",str(root)]
    result=subprocess.run(command,cwd=root,capture_output=True,text=True,timeout=300)
    if result.returncode: raise ValueError("Rust AST 扫描失败："+result.stderr[-2000:])
    return json.loads(result.stdout)


def structured_ids(ledger:dict)->set[str]:
    result=set()
    for group in ledger.get("structured",[]):
        for call_id in group.get("calls",[]):
            if call_id in result: raise ValueError("重复业务 call_id："+call_id)
            result.add(call_id)
    return result


def inspect(root:Path=ROOT,ledger:dict|None=None,ast:dict|None=None,baseline:dict|None=None)->dict:
    ledger=ledger if ledger is not None else json.loads((root/MANIFEST).read_text())
    ast=ast if ast is not None else scan(root)
    baseline=baseline if baseline is not None else json.loads((root/BASELINE).read_text())
    if ledger.get("schema_version")!=2 or not ledger.get("structured") or not ledger.get("shared_scenarios"):
        raise ValueError("模型业务清单必须是非空版本 2 合同/场景登记")
    if ast.get("schema_version")!=2 or ast.get("scanner")!="syn-workspace" or not ast.get("calls"):
        raise ValueError("缺少 Cargo 工作区 AST 扫描结果")
    for field in ("calls","implementations","catalogs"):
        if ast.get(field)!=baseline.get(field):
            if field=="calls":
                before={json.dumps(row,sort_keys=True) for row in baseline.get(field,[])}
                after={json.dumps(row,sort_keys=True) for row in ast[field]}
                raise ValueError(f"模型调用登记不完整；新增调用：{sorted(after-before)}；失效调用：{sorted(before-after)}")
            raise ValueError(f"模型 {field} 已变化，必须审查适用能力和执行证据清单")
    observed_files={row["file"] for row in ast["calls"]}
    ids=structured_ids(ledger)
    if not ids or any(not isinstance(call_id,str) or not call_id.strip() for call_id in ids):
        raise ValueError("业务 call_id 无效")
    for group in ledger["structured"]:
        if "suites" in group or not group.get("owners") or not group.get("validators") or not group.get("branches"):
            raise ValueError("业务调用必须登记 owner、生产 validator 和分支，不能用 suite 名称替代执行证据")
        for owner in group["owners"]:
            if ORCHESTRATOR+owner not in observed_files:
                raise ValueError("业务 owner 未发现实际模型调用："+owner)
        for source in [group.get("contract_owner",""),*(ORCHESTRATOR+name for name in group["validators"])]:
            path=(root/source).resolve()
            if not source or not path.is_relative_to(root.resolve()) or not path.exists():
                raise ValueError("生产合同或 validator 不存在："+source)
    for row in ledger.get("free_text",[]):
        if row.get("call_id") in ids: raise ValueError("业务 call_id 重复")
        if ORCHESTRATOR+row.get("owner","") not in observed_files or not row.get("validator") or not row.get("branches"):
            raise ValueError("自由文本业务调用未完整登记")
    for capability in ("text","image","video"):
        catalog=ast["catalogs"].get(capability)
        if not catalog or len({row["id"] for row in catalog})!=len(catalog):
            raise ValueError("生产能力目录为空或重复："+capability)
        if not ledger["shared_scenarios"].get(capability):
            raise ValueError("生产能力缺少必测异常场景："+capability)
    return {"business_json_calls":len(ids),"free_text_calls":len(ledger.get("free_text",[])),"ast_call_sites":len(ast["calls"]),"provider_implementations":len(ast["implementations"]),"scope":ledger["scope"],"runtime_coverage_measured":False}


if __name__=="__main__":
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ast",type=Path)
    parser.add_argument("--binary",type=Path)
    args=parser.parse_args()
    try:
        ast=json.loads(args.ast.read_text()) if args.ast else scan(binary=args.binary)
        print(json.dumps(inspect(ast=ast),ensure_ascii=False,indent=2))
    except (OSError,ValueError,TypeError,KeyError,subprocess.TimeoutExpired) as error:
        raise SystemExit("模型调用登记检查失败："+str(error))
