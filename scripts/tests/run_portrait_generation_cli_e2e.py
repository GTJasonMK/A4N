#!/usr/bin/env python3
"""Exercise every portrait generation operation through the real CLI."""

import hashlib
import json
import tempfile
import uuid
from pathlib import Path

from run_cli_contract_e2e import (
    checked,
    cli_e2e_parser,
    logged_in_cli,
    provider_log_entries,
    runtime_snapshot,
    secure_json,
)


def main():
    parser = cli_e2e_parser(__doc__)
    args = parser.parse_args()
    args.report.parent.mkdir(parents=True, exist_ok=True)
    report = {"success": False, "checks": []}
    try:
        with tempfile.TemporaryDirectory(prefix="portrait-generation-", dir=args.report.parent) as directory:
            work = Path(directory)
            cli = logged_in_cli(args, work)
            for name in (
                "create_profile",
                "start_portrait_generation",
                "list_portraits_by_project",
            ):
                checked(cli, ["capabilities", "describe", name], name + " contract")

            secure_json(work / "runtime.json", runtime_snapshot(args.provider_url, embedding_dimension=384))
            checked(cli, ["config", "runtime", "set", "--input", "@" + str(work / "runtime.json")], "runtime")

            def call(name, value):
                return checked(
                    cli,
                    ["call", name, "--args", json.dumps({"input": value}, ensure_ascii=False)],
                    name,
                )[0]["data"]

            project = checked(
                cli,
                ["project", "create", "--title", "portrait generation regression", "--project-type", "novel_long"],
                "project",
            )[0]["data"]
            project_id = project["id"]
            report["project_id"] = project_id

            def create_profile(name):
                value = call(
                    "create_profile",
                    {
                        "project_id": project_id,
                        "name": name,
                        "base_attributes": {
                            "explicit_attributes": {},
                            "implicit_attributes": {},
                            "social_attributes": {},
                        },
                    },
                )
                profile_id = value.get("profile", {}).get("id")
                assert isinstance(profile_id, int) and profile_id > 0, value
                return profile_id

            primary_profile_id = create_profile("立绘生成主角")
            missing_profile_id = create_profile("缺失立绘角色")

            def run_operation(operation, result_kind, expects_llm):
                provider_start = len(provider_log_entries(args.provider_log))
                handle = call(
                    "start_portrait_generation",
                    {
                        "project_id": project_id,
                        "meta": {"start_request_id": str(uuid.uuid4())},
                        "operation": operation,
                    },
                )
                watched = cli.run(
                    ["task", "watch", handle["task_id"], "--attempt", str(handle["attempt"])]
                )
                events = watched["stdout_documents"]
                assert watched["exit_code"] == 0 and events, watched
                assert all(isinstance(event, dict) and event.get("ok") is True for event in events), events

                result = checked(
                    cli,
                    ["task", "get", handle["task_id"], "--attempt", str(handle["attempt"])],
                    "portrait task result",
                )[0]["data"]
                assert result["handle"]["status"] == "completed", result
                reference = result["result_ref"]
                assert reference["kind"] == result_kind and reference["revision"], result

                snapshot = checked(
                    cli,
                    ["task", "snapshot", handle["task_id"], "--attempt", str(handle["attempt"])],
                    "portrait task snapshot",
                )[0]["data"]
                terminal = snapshot["terminal_event"]
                assert terminal["event"]["kind"] == "completed", terminal
                assert terminal["event"]["data"]["result_ref"] == reference, terminal

                provider_entries = provider_log_entries(args.provider_log)[provider_start:]
                llm_calls = [entry for entry in provider_entries if entry["path"].endswith("/chat/completions")]
                image_calls = [
                    entry for entry in provider_entries
                    if entry["path"].endswith(("/images/generations", "/images/edits"))
                ]
                assert bool(llm_calls) is expects_llm, llm_calls
                assert len(image_calls) == 1, image_calls
                return reference

            def portraits():
                value = call("list_portraits_by_project", {"project_id": project_id})
                assert isinstance(value, list), value
                return value

            def assert_single_portrait(reference, profile_id, existing_ids):
                portrait_id = int(reference["id"])
                rows = portraits()
                portrait = next((row for row in rows if row["id"] == portrait_id), None)
                assert portrait is not None, rows
                assert portrait["project_id"] == project_id and portrait["profile_id"] == profile_id, portrait
                assert portrait["image_path"] and portrait["prompt"], portrait
                assert portrait_id not in existing_ids, portrait
                return portrait_id, rows

            provider_suite_start = len(provider_log_entries(args.provider_log))
            base_reference = run_operation(
                {
                    "kind": "generate",
                    "character_name": "立绘生成主角",
                    "style": "克制的水彩插画",
                    "expression": "平静",
                    "pose": "自然站立",
                    "reference_image_ids": [],
                    "width": 1024,
                    "height": 1024,
                },
                "portrait",
                True,
            )
            base_portrait_id, rows = assert_single_portrait(base_reference, primary_profile_id, set())
            report["checks"].append("portrait generation runs LLM prompt, image Provider, task, artifact and persisted-result paths")

            existing_ids = {row["id"] for row in rows}
            regenerated_reference = run_operation(
                {
                    "kind": "regenerate",
                    "portrait_id": base_portrait_id,
                    "reference_image_ids": [],
                    "width": 1024,
                    "height": 1024,
                },
                "portrait",
                False,
            )
            regenerated_id, rows = assert_single_portrait(regenerated_reference, primary_profile_id, existing_ids)
            assert base_portrait_id in {row["id"] for row in rows}, rows
            report["checks"].append("portrait regeneration reuses the saved prompt without an extra LLM call")

            existing_ids = {row["id"] for row in rows}
            age_reference = run_operation(
                {
                    "kind": "generate_age_variant",
                    "portrait_id": base_portrait_id,
                    "age": "年长十岁",
                    "reference_image_ids": [],
                    "width": 1024,
                    "height": 1024,
                },
                "portrait",
                False,
            )
            age_portrait_id, rows = assert_single_portrait(age_reference, primary_profile_id, existing_ids)
            assert base_portrait_id in {row["id"] for row in rows}, rows
            report["checks"].append("age variant persists as a separate portrait without an extra LLM call")

            before_missing = {row["id"] for row in rows}
            batch_reference = run_operation(
                {"kind": "generate_missing", "width": 1024, "height": 1024},
                "portrait_batch",
                False,
            )
            rows = portraits()
            batch_rows = [row for row in rows if row["profile_id"] == missing_profile_id]
            assert len(batch_rows) == 1 and batch_rows[0]["id"] not in before_missing, batch_rows
            assert batch_reference["id"] == str(project_id), batch_reference
            revision = hashlib.sha256(
                json.dumps([batch_rows[0]["id"]], separators=(",", ":")).encode()
            ).hexdigest()
            assert batch_reference["revision"] == revision, batch_reference
            report["checks"].append("missing portrait batch publishes the referenced portrait without an LLM call")
            report["provider_calls"] = len(provider_log_entries(args.provider_log)) - provider_suite_start
            report["results"] = {
                "generated": base_portrait_id,
                "regenerated": regenerated_id,
                "age_variant": age_portrait_id,
                "missing_batch": [batch_rows[0]["id"]],
            }
            report["success"] = True
    except Exception as error:
        report["error"] = str(error)
        raise
    finally:
        args.report.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
        print(json.dumps({"success": report["success"], "checks": len(report["checks"]), "report": str(args.report)}))


if __name__ == "__main__":
    main()
