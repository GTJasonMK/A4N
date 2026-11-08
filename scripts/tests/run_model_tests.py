#!/usr/bin/env python3
"""Run offline model contracts, real loopback adapters and version-bound acceptance."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import signal
import shutil
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

from check_model_call_coverage import MANIFEST, inspect, scan
from model_contract_vectors import judge
from model_evidence import EvidenceError, digest_file, input_snapshot, seal_artifact, verify_report
from model_requirements import contract_cases, required_case_ids
from runner_support import private_file, safe_report_dir, terminate_process, raise_interrupted

ROOT=Path(__file__).resolve().parents[2]
SUMMARY=re.compile(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;")
EXCLUDED=("model_contract_matrix_production_recovery","model_contract_matrix_request_boundary","model_contract_matrix_business_schema_wire")
MANUAL_BENCHMARKS=("tests::performance_sse_batches", "anthropic::tests::performance_anthropic_batches")
SEALED={}


class ModelTestError(RuntimeError):pass


def write_json(path:Path,value)->None:
    with private_file(path) as stream:stream.write(json.dumps(value,ensure_ascii=False,indent=2).encode()+b"\n")


def runtime_command(command:list[str])->list[str]:
    return ["bash","-c",'set -euo pipefail; source "$1/scripts/lib/common.sh"; require_onnxruntime; shift; exec "$@"',"a4n-model-tests",str(ROOT),*command]


def run(command:list[str],log:Path,timeout:float,*,env:dict|None=None,stdin:Path|None=None)->str:
    process=None
    try:
        with private_file(log) as output, (stdin.open("rb") if stdin else open(os.devnull,"rb")) as input_stream:
            process=subprocess.Popen(command,cwd=ROOT,env=env,stdin=input_stream,stdout=output,stderr=subprocess.STDOUT,start_new_session=True)
            try:code=process.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                terminate_process(process)
                raise ModelTestError(f"命令超时（{timeout}s），不能记作通过：{log}")
            if code:raise ModelTestError(f"命令失败（exit={code}）：{log}")
        return log.read_text(errors="replace")
    except KeyboardInterrupt:
        if process is not None:terminate_process(process)
        raise


def rust_result(text:str)->int:
    found=SUMMARY.findall(text)
    if len(found)!=1:raise ModelTestError("Rust 测试缺少唯一完成记录")
    passed,failed,ignored=map(int,found[0])
    if not passed or failed or ignored:raise ModelTestError(f"Rust 测试未完整执行：passed={passed}, failed={failed}, ignored={ignored}")
    return passed


def observations(text:str,marker:str)->list[dict]:
    result=[]
    for line in text.splitlines():
        if marker in line:result.append(json.loads(line.split(marker,1)[1]))
    return result


def case_record(case_id:str,log:Path,report_dir:Path,assertions:int=1,**fields)->dict:
    key=(str(log.resolve()),log.stat().st_mtime_ns,log.stat().st_size)
    receipt=SEALED.get(key)
    if receipt is None:receipt=SEALED[key]=seal_artifact(log,report_dir)
    return {"case_id":case_id,"status":"passed","assertions":assertions,"evidence":[receipt],**fields}


def build(report_dir:Path,ledger:dict)->tuple[dict,str]:
    packages=sorted(ledger["semantic_rust_groups"])
    arguments=[str(ROOT/"scripts/cargo.sh"),"test","--locked","--lib","--no-run","--message-format=json"]
    for package in packages:arguments.extend(["-p",package])
    text=run(runtime_command(arguments),report_dir/"logs/build-tests.log",3600)
    binaries={}
    for line in text.splitlines():
        try:row=json.loads(line)
        except ValueError:continue
        if row.get("reason")=="compiler-artifact" and row.get("executable") and row.get("profile",{}).get("test"):
            binaries[row["target"]["name"].replace("_","-")]=str(Path(row["executable"]).resolve())
    if set(binaries)!=set(packages):raise ModelTestError(f"测试二进制缺失：{sorted(set(packages)-set(binaries))}")
    text=run(runtime_command([str(ROOT/"scripts/cargo.sh"),"build","--locked","-p","a4n-model-test-tools","--message-format=json"]),report_dir/"logs/build-tools.log",600)
    tool=None
    for line in text.splitlines():
        try:row=json.loads(line)
        except ValueError:continue
        if row.get("reason")=="compiler-artifact" and row.get("executable"):tool=row["executable"]
    if not tool:raise ModelTestError("AST/生产校验器工具没有编译产物")
    return binaries,str(Path(tool).resolve())


def exported_matrices(report_dir:Path,binaries:dict,report:dict)->dict:
    corpus={}
    for package in ("a4n-orchestrator","a4n-image-providers","a4n-video-providers","a4n-llm-http","a4n-embedding-providers","a4n-media-processing-core"):
        log=report_dir/"logs"/f"matrix-{package}.log"
        text=run(runtime_command([binaries[package],"model_contract_matrix","--nocapture","--test-threads=1",*sum((["--skip",name] for name in EXCLUDED),[])]),log,60)
        count=rust_result(text)
        report["suites"][f"matrix/{package}"]={"status":"passed","executed":count,"log":seal_artifact(log,report_dir)}
        for row in observations(text,"A4N_MODEL_SCHEMA:"):
            previous=corpus.get(row["call_id"])
            if previous is not None and previous!=row:raise ModelTestError("同一 call_id 导出了不一致的合同："+row["call_id"])
            corpus[row["call_id"]]=row
        for row in observations(text,"A4N_MODEL_CASE:"):
            if row.get("actual")!=row.get("expected") or row.get("status")!="passed":raise ModelTestError("适配器观察未通过断言")
            report["cases"].append(case_record(row["case_id"],log,report_dir,row["assertions"],call_id=row["call_id"],adapter=row["adapter"],expected=row["expected"],actual=row["actual"]))
    write_json(report_dir/"contracts.json",corpus)
    report["contracts"]=seal_artifact(report_dir/"contracts.json",report_dir)
    return corpus


def run_vectors(report_dir:Path,ledger:dict,corpus:dict,tool:str,binaries:dict,report:dict)->None:
    vectors=contract_cases(ledger,corpus)
    input_path=report_dir/"vectors.jsonl"
    with private_file(input_path) as output:
        for call_id,row in sorted(corpus.items()):output.write((json.dumps({"kind":"schema","schema_id":call_id,"schema":row["schema"]},ensure_ascii=False)+"\n").encode())
        for case in vectors:output.write((json.dumps({key:value for key,value in case.items() if key!="schema"}|{"schema_id":case["call_id"]},ensure_ascii=False)+"\n").encode())
    for engine,command in [("rust",[tool,"validate"]),("ajv",["node","scripts/contracts/check-model-vectors.mjs"])]:
        log=report_dir/"logs"/f"vectors-{engine}.jsonl"
        text=run(command,log,60,stdin=input_path)
        actual=[json.loads(line) for line in text.splitlines()]
        judge(vectors,actual)
        for case,row in zip(vectors,actual,strict=True):
            if case["case_id"]!=row["case_id"]:raise ModelTestError("校验器结果顺序改变")
            report["cases"].append(case_record(case["case_id"]+"/"+engine,log,report_dir,call_id=case["call_id"],adapter=engine,expected=case["expected"],actual=row["accepted"]))
        report["suites"]["vectors/"+engine]={"status":"passed","executed":len(actual),"log":seal_artifact(log,report_dir)}
    requests={row["call_id"]:row for row in ledger.get("request_contracts",[])}
    for call_id,row in sorted(corpus.items()):
        print(json.dumps({"contract":call_id,"stage":"production-boundary"}),flush=True)
        selected=[{key:value for key,value in case.items() if key!="schema"}|{"vector_key":case["case_id"].split("/contract/",1)[1]} for case in vectors if case["call_id"]==call_id]
        fixture=report_dir/"inputs"/(call_id+".json")
        write_json(fixture,{"call_id":call_id,"schema":row["schema"],"valid":selected[0]["value"],"cases":selected})
        env={**os.environ,"A4N_MODEL_CONTRACT_CASES":str(fixture)}
        package=requests[call_id]["crate"] if call_id in requests else "a4n-orchestrator"
        test=requests[call_id]["test"] if call_id in requests else "structured_llm::recovery::matrix::model_contract_matrix_production_recovery"
        log=report_dir/"logs"/("boundary-"+call_id+".log")
        text=run(runtime_command([binaries[package],test,"--exact","--ignored","--nocapture","--test-threads=1"]),log,60,env=env)
        if rust_result(text)!=1:raise ModelTestError("生产边界测试未按要求执行")
        records=observations(text,"A4N_MODEL_CASE:")
        if len(records)<len(selected):raise ModelTestError("生产边界没有执行完整字段变异")
        for record in records:
            report["cases"].append(case_record(record["case_id"],log,report_dir,record["assertions"],call_id=call_id,adapter=record["adapter"],expected=record["expected"],actual=record["actual"]))
        report["suites"]["boundary/"+call_id]={"status":"passed","executed":len(records),"log":seal_artifact(log,report_dir)}
        if call_id not in requests:
            wire_log=report_dir/"logs"/("wire-"+call_id+".log")
            wire_text=run(runtime_command([binaries["a4n-llm-http"],"contract_matrix::model_contract_matrix_business_schema_wire","--exact","--ignored","--nocapture","--test-threads=1"]),wire_log,60,env=env)
            if rust_result(wire_text)!=1:raise ModelTestError("业务合同实际传输测试未执行")
            wire_rows=observations(wire_text,"A4N_MODEL_CASE:")
            for record in wire_rows:
                report["cases"].append(case_record(record["case_id"],wire_log,report_dir,record["assertions"],call_id=call_id,adapter=record["adapter"],expected=record["expected"],actual=record["actual"]))
            report["suites"]["wire/"+call_id]={"status":"passed","executed":len(wire_rows),"log":seal_artifact(wire_log,report_dir)}


def list_tests(binary:str)->list[str]:
    result=subprocess.run(runtime_command([binary,"--list"]),cwd=ROOT,capture_output=True,text=True,check=True,timeout=60)
    return [line.removesuffix(": test") for line in result.stdout.splitlines() if line.endswith(": test")]


def partitions(prefix:str,names:list[str])->list[tuple[str,list[str],bool]]:
    if len(names)<=25:return [(prefix,names,False)]
    children={}
    for name in names:
        suffix=name[len(prefix):]
        child=prefix+suffix.split("::",1)[0]
        children.setdefault(child,[]).append(name)
    result=[]
    for child,selected in children.items():
        if len(selected)==1 and selected[0]==child:result.append((child,selected,True))
        else:result.extend(partitions(child+"::",selected))
    return result


def rust_phase(config:Path)->int:
    if not os.environ.get("A4N_TEST_DATABASE_URL"):raise ModelTestError("完整语义测试必须使用隔离数据库")
    value=json.loads(config.read_text());report_dir=Path(value["report_dir"])
    ledger=json.loads((ROOT/MANIFEST).read_text());cases=[];suites={};failures=[]
    for package,prefixes in ledger["semantic_rust_groups"].items():
        names=list_tests(value["binaries"][package])
        for prefix in prefixes:
            case_id=f"rust-tests/{package}/{prefix}"
            try:
                selected=[name for name in names if name.startswith(prefix) and name not in MANUAL_BENCHMARKS and not any(excluded in name for excluded in EXCLUDED)]
                if not selected:raise ModelTestError(f"生产语义测试前缀没有任何实际测试：{package}/{prefix}")
                logs=[];executed=0
                for index,(filter_name,tests,exact) in enumerate(partitions(prefix,selected)):
                    name=hashlib.sha256((package+prefix+str(index)).encode()).hexdigest()[:16]
                    log=report_dir/"logs"/("semantic-"+name+".log")
                    command=[value["binaries"][package],filter_name,"--nocapture","--test-threads=1",*sum((["--skip",excluded] for excluded in (*EXCLUDED,*MANUAL_BENCHMARKS)),[])]
                    command.extend(sum((["--skip",name] for name in names if filter_name in name and name not in tests),[]))
                    if exact:command.append("--exact")
                    count=rust_result(run(runtime_command(command),log,60))
                    if count!=len(tests):raise ModelTestError(f"语义测试未完整执行：{package}/{filter_name}")
                    logs.append(seal_artifact(log,report_dir));executed+=count
                cases.append({"case_id":case_id,"status":"passed","assertions":executed,"evidence":logs})
                suites[case_id]={"status":"passed","executed":executed}
                print(json.dumps({"suite":case_id,"executed":executed}),flush=True)
            except ModelTestError as error:
                failures.append({"suite":case_id,"error":str(error)})
                suites[case_id]={"status":"failed","error":str(error)}
                print(json.dumps(failures[-1],ensure_ascii=False),flush=True)
    write_json(report_dir/"semantic-results.json",{"cases":cases,"suites":suites,"failures":failures})
    return 1 if failures else 0


def main()->int:
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile",choices=("model-fast","model-full"),default="model-fast")
    parser.add_argument("--report-dir",type=Path)
    parser.add_argument("--verify-report",type=Path)
    parser.add_argument("--rust-phase",type=Path,help=argparse.SUPPRESS)
    args=parser.parse_args()
    if args.verify_report:
        print(json.dumps(verify_report(args.verify_report),ensure_ascii=False));return 0
    if args.rust_phase:return rust_phase(args.rust_phase)
    os.environ.setdefault("CARGO_BUILD_JOBS","2")
    report_dir=safe_report_dir(args.report_dir or ROOT/"data/model-tests"/datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ"))
    report={"schema_version":2,"profile":args.profile,"success":False,"started_at":datetime.now(timezone.utc).isoformat(),"cases":[],"suites":{},"failures":[],"inputs":input_snapshot(),"live_provider_tested":False,"excluded_manual_benchmarks":list(MANUAL_BENCHMARKS)}
    try:
        if args.profile=="model-full" and subprocess.check_output(["git","status","--porcelain"],cwd=ROOT).strip():raise ModelTestError("model-full 必须针对已提交且干净的最终工作区运行")
        ledger=json.loads((ROOT/MANIFEST).read_text())
        report["tested_commit"]=subprocess.check_output(["git","rev-parse","HEAD"],cwd=ROOT,text=True).strip()
        report["tested_tree"]=subprocess.check_output(["git","rev-parse","HEAD^{tree}"],cwd=ROOT,text=True).strip()
        print(json.dumps({"stage":"compile","profile":args.profile}),flush=True)
        binaries,tool=build(report_dir,ledger)
        report["binaries"]={path:digest_file(Path(path)) for path in [*binaries.values(),tool]}
        native=[Path(path) for name in ("ffmpeg","ffprobe") if (path:=shutil.which(name))]
        onnx=ROOT/"tools/onnxruntime/lib/libonnxruntime.so"
        if onnx.is_file():native.append(onnx.resolve())
        report["native_artifacts"]={str(path):digest_file(path) for path in native}
        report["build_features"]={"rust_tests":"default","creation":"a4n-server/embedded-capabilities" if args.profile=="model-full" else None}
        inventory=scan(binary=Path(tool));report["inventory_summary"]=inspect(ast=inventory)
        write_json(report_dir/"inventory.json",inventory);report["inventory"]=seal_artifact(report_dir/"inventory.json",report_dir)
        corpus=exported_matrices(report_dir,binaries,report)
        run_vectors(report_dir,ledger,corpus,tool,binaries,report)
        log=report_dir/"logs/python.log"
        text=run([sys.executable,"-m","unittest","discover","-s","scripts/tests","-p","test_model_*.py"],log,60)
        python_summary=re.search(r"Ran ([1-9][0-9]*) tests",text)
        if not python_summary:raise ModelTestError("Python 保证体系测试未执行")
        python_count=int(python_summary[1])
        report["cases"].append(case_record("python/model-assurance",log,report_dir,python_count));report["suites"]["python/model-assurance"]={"status":"passed","executed":python_count}
        if args.profile=="model-full":
            write_json(report_dir/"semantic-input.json",{"report_dir":str(report_dir),"binaries":binaries})
            run([sys.executable,"scripts/tests/run_with_test_database.py","--report-dir",str(report_dir/"database"),"--timeout","7200","--",sys.executable,"scripts/tests/run_model_tests.py","--rust-phase",str(report_dir/"semantic-input.json")],report_dir/"logs/semantic-phase.log",7500)
            semantic=json.loads((report_dir/"semantic-results.json").read_text());report["cases"].extend(semantic["cases"]);report["suites"].update(semantic["suites"])
            run([sys.executable,"scripts/tests/run_creation_e2e.py","--report-dir",str(report_dir/"creation"),"--stage","all","--browser-feature-flows"],report_dir/"logs/creation.log",14100)
            creation_path=report_dir/"creation/report.json";creation=json.loads(creation_path.read_text())
            if not creation.get("success") or not creation.get("source",{}).get("matches_current_worktree"):raise ModelTestError("完整创作验收没有在当前版本通过")
            report["creation"]=seal_artifact(creation_path,report_dir)
            report["cases"].append(case_record("creation/full-cli-browser",creation_path,report_dir,len(creation["stages"])));report["suites"]["creation/full-cli-browser"]={"status":"passed","executed":len(creation["stages"])}
        required=required_case_ids(ledger,inventory,corpus,args.profile)
        actual=[case["case_id"] for case in report["cases"]]
        if len(actual)!=len(set(actual)) or set(actual)!=required:raise ModelTestError(f"必测项执行不完整：missing={sorted(required-set(actual))[:20]}, extra={sorted(set(actual)-required)[:20]}")
        report["required_cases"]=sorted(required)
        report["finished_inputs_sha256"]=input_snapshot()["sha256"]
        if report["inputs"]["sha256"]!=report["finished_inputs_sha256"]:raise ModelTestError("执行期间测试输入发生变化，本次验收失效")
        report["success"]=True
    except (Exception,KeyboardInterrupt) as error:
        report["failures"].append(f"{type(error).__name__}: {error}")
    finally:
        report["finished_at"]=datetime.now(timezone.utc).isoformat()
        write_json(report_dir/"report.json",report)
    if report["success"]:
        try:verify_report(report_dir/"report.json",require_full=args.profile=="model-full")
        except Exception as error:
            report["success"]=False;report["failures"].append(str(error));write_json(report_dir/"report.json",report)
    print(json.dumps({"success":report["success"],"profile":args.profile,"cases":len(report["cases"]),"report":str(report_dir/"report.json"),"failures":report["failures"]},ensure_ascii=False),flush=True)
    return 0 if report["success"] else 1


if __name__=="__main__":
    signal.signal(signal.SIGTERM,raise_interrupted)
    try:raise SystemExit(main())
    except (EvidenceError,ModelTestError) as error:raise SystemExit(str(error))
