#!/usr/bin/env python3
"""Verify a full model acceptance receipt before updating master."""
import argparse
import json
import os
import re
import subprocess
from pathlib import Path
from model_evidence import ROOT, EvidenceError, verify_report


def main()->int:
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report",type=Path)
    parser.add_argument("--candidate")
    parser.add_argument("--candidate-tree")
    args=parser.parse_args()
    configured=subprocess.run(["git","config","--get","a4n.modelAcceptanceReport"],cwd=ROOT,capture_output=True,text=True).stdout.strip()
    selected=args.report or os.environ.get("A4N_MODEL_ACCEPTANCE_REPORT") or configured
    if not selected:raise EvidenceError("master 门禁缺少 model-full 报告；请运行 scripts/tests/run_test_matrix.py --profile model-full，见 docs/MODEL_TESTING.md")
    report_path=Path(selected).expanduser().resolve()
    result=verify_report(report_path)
    report=json.loads(report_path.read_text())
    candidate=args.candidate_tree
    if args.candidate:
        if not re.fullmatch(r"[0-9a-f]{40,64}",args.candidate):raise EvidenceError("候选提交 SHA 无效")
        candidate=subprocess.check_output(["git","rev-parse","--verify",args.candidate+"^{tree}"],cwd=ROOT,text=True).strip()
    if candidate and (not re.fullmatch(r"[0-9a-f]{40,64}",candidate) or candidate!=report.get("tested_tree")):
        raise EvidenceError("master 候选内容与实际通过 model-full 的版本不一致")
    print(json.dumps(result,ensure_ascii=False))
    return 0


if __name__=="__main__":
    try:raise SystemExit(main())
    except (OSError,ValueError,KeyError,TypeError,subprocess.CalledProcessError) as error:raise SystemExit("模型验收门禁失败："+str(error))
