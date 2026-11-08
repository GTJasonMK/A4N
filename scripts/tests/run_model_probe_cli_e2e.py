#!/usr/bin/env python3
"""Exercise actual configuration probes through CLI, Server and local adapters."""
import json
import csv
import hashlib
import io
import os
import subprocess
import tempfile
from datetime import datetime,timezone
from urllib.parse import urlparse
from pathlib import Path

from run_cli_contract_e2e import checked, cli_e2e_parser, logged_in_cli, provider_log_entries, runtime_snapshot, secure_json


def main():
    parser=cli_e2e_parser(__doc__)
    parser.add_argument("--postgres-container",required=True)
    args=parser.parse_args()
    args.report.parent.mkdir(parents=True, exist_ok=True)
    report = {"success": False, "checks": []}
    try:
        with tempfile.TemporaryDirectory(prefix="model-probes-", dir=args.report.parent) as directory:
            work = Path(directory)
            cli = logged_in_cli(args, work)

            def configure(protocol):
                secure_json(work / "runtime.json", runtime_snapshot(args.provider_url, protocol, embedding_dimension=384))
                checked(cli, ["config", "runtime", "set", "--input", "@" + str(work / "runtime.json")], "runtime")

            for protocol, endpoint in [("openai", "/chat/completions"), ("openai-responses", "/responses"), ("anthropic", "/messages"), ("gemini", ":streamGenerateContent")]:
                configure(protocol)
                before = len(provider_log_entries(args.provider_log))
                checked(cli, ["call", "test_llm_config", "--args", json.dumps({"configId": 900001})], "probe " + protocol)
                requests = provider_log_entries(args.provider_log)[before:]
                assert len(requests) == 1 and endpoint in requests[0]["path"], requests
                report["checks"].append("actual config probe: " + protocol)

            configure("openai")
            for command, config_id, endpoint in [("test_image_config", 900002, "/images/generations"), ("test_embedding_config", 900004, "/embeddings")]:
                before = len(provider_log_entries(args.provider_log))
                checked(cli, ["call", command, "--args", json.dumps({"configId": config_id})], command)
                requests = provider_log_entries(args.provider_log)[before:]
                assert len(requests) == 1 and requests[0]["path"].endswith(endpoint), requests
                report["checks"].append("actual config probe: " + command)

            for command in ["test_llm_config", "test_image_config", "test_embedding_config"]:
                before = len(provider_log_entries(args.provider_log))
                result = cli.run(["call", command, "--args", json.dumps({"configId": 123456})])
                assert result["exit_code"] != 0, result
                assert len(provider_log_entries(args.provider_log)) == before, "mismatched config must fail before a provider call"
                report["checks"].append(command + " rejects mismatched snapshot identity before HTTP")

            snapshot=runtime_snapshot(args.provider_url,embedding_dimension=384)
            snapshot["llm"]["agent_protocol"]="json_compat"
            secure_json(work/"runtime.json",snapshot)
            checked(cli,["config","runtime","set","--input","@"+str(work/"runtime.json")],"JSON Agent runtime")
            project=checked(cli,["project","create","--title","JSON Agent probe","--project-type","novel_long","--use-context"],"project")[0]["data"]
            draft=checked(cli,["call","generate_canvas_text_asset_draft","--args",json.dumps({"input":{"project_id":project["id"],"llm_config_id":None,"instruction":"写出雨夜镜头描述","source_asset_id":None,"source_version":None}})],"text asset draft")[0]["data"]
            assert draft["content"].strip(),draft
            report["checks"].append("Canvas text asset draft executes the real model request and validates plain text output")
            chapter=checked(cli,["chapter","import","--number","1","--title","原文引用","--content","第一段。\n第二段。\n\n第三段。"],"chapter")[0]["data"]
            thread=checked(cli,["agent","thread","create","--kind","subagent","--capability","content_optimization","--project-id",str(project["id"]),"--chapter-id",str(chapter["id"]),"--resource-kind","chapter","--resource-id",str(chapter["id"])],"JSON Agent thread")[0]["data"]
            run_id=checked(cli,["agent","chat",thread["id"],"A4N_OPTIMIZATION_CONTEXT 搜索第二段，按搜索路径读取，再只读取第二段及其引用。"],"JSON Agent send")[0]["data"]["run_id"]
            from run_agent_cli_e2e import wait_for_run,thread_snapshot
            result=wait_for_run(cli,run_id,60,0.1)
            assert result["status"]=="completed",result
            executions=thread_snapshot(cli,thread["id"])["executions"]
            calls=[row["tool_name"] for row in executions if row["run_id"]==run_id and row["kind"]=="tool_call" and row["status"]=="succeeded"]
            assert calls==["search_project_context","read_project_context","read_project_context"],calls
            report["checks"].append("JSON Agent performs three validated real tools and completes through CLI/Server")

            database=urlparse(os.environ["A4N_DATABASE_URL"])
            assert args.postgres_container.startswith("a4n-creation-e2e-") and database.path=="/a4n_e2e", "history fixtures require the disposable creation database"

            def summary_run(failing):
                snapshot=runtime_snapshot(args.provider_url,embedding_dimension=384)
                snapshot["llm"].update(max_context_tokens=24000,max_output_tokens=1024)
                if failing:snapshot["llm"]["model"]="a4n-summary-fail-fields"
                secure_json(work/"runtime.json",snapshot)
                checked(cli,["config","runtime","set","--input","@"+str(work/"runtime.json")],"summary runtime")
                thread=checked(cli,["agent","thread","create","--kind","main","--capability","main"],"summary thread")[0]["data"]
                texts=["长期约束：地点名是白塔。最终按最新指令作答，保留这个原始事实。"]+[f"Archived evidence {index}: "+"z"*7600 for index in range(6)]
                records=io.StringIO();writer=csv.writer(records,lineterminator="\n")
                timestamp=datetime.now(timezone.utc).replace(tzinfo=None).isoformat()
                for index,text in enumerate(texts):writer.writerow([int(thread["id"]),"user" if index==0 else "assistant","user" if index==0 else "agent",hashlib.sha256(text.encode()).hexdigest(),text,"complete",timestamp,timestamp])
                sql="COPY agent_message(thread_id,role,sender_kind,content_sha256,content,status,created_at,updated_at) FROM STDIN WITH (FORMAT csv);\n"+records.getvalue()+"\\.\n"
                seeded=subprocess.run(["docker","exec","-i",args.postgres_container,"psql","-v","ON_ERROR_STOP=1","-U",database.username,"-d",database.path[1:]],input=sql,text=True,capture_output=True,timeout=20)
                assert seeded.returncode==0,seeded.stderr
                marker='A4N_RELIABILITY {"mode":"summary_final","instruction":"仅输出历史约定的地点名"}'
                run_id=checked(cli,["agent","chat",thread["id"],marker,"--budget",json.dumps({"max_steps":32,"max_model_calls":16})],"summary send")[0]["data"]["run_id"]
                result=wait_for_run(cli,run_id,60,0.1)
                saved=thread_snapshot(cli,thread["id"])
                summaries=[row for row in saved["executions"] if row["run_id"]==run_id and row["kind"]=="model_call" and row.get("result_summary")=="Agent context summary"]
                assert summaries,saved
                if failing:
                    assert result["status"]=="failed" and any(row["status"]=="failed" for row in summaries),result
                    assert all(any(message["content"]==text for message in saved["messages"]) for text in texts), "failed summary lost original history"
                else:
                    assert result["status"]=="completed" and all(row["status"]=="succeeded" for row in summaries),result
                    assert any(message["content"]=="白塔" for message in saved["messages"] if message["role"]=="assistant"),saved
                return len(summaries)

            report["checks"].append(f"Agent performs {summary_run(False)} actual durable summaries and preserves the original constraint")
            report["checks"].append(f"Agent rejects {summary_run(True)} invalid model summaries and retains every original history message")
        report["success"] = True
    except Exception as error:
        report["error"] = str(error)
    secure_json(args.report, report)
    print(json.dumps(report, ensure_ascii=False))
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
