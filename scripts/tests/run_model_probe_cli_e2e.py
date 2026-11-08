#!/usr/bin/env python3
"""Exercise actual configuration probes through CLI, Server and local adapters."""
import json
import tempfile
from pathlib import Path

from run_cli_contract_e2e import checked, cli_e2e_parser, logged_in_cli, provider_log_entries, runtime_snapshot, secure_json


def main():
    args = cli_e2e_parser(__doc__).parse_args()
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
        report["success"] = True
    except Exception as error:
        report["error"] = str(error)
    secure_json(args.report, report)
    print(json.dumps(report, ensure_ascii=False))
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
