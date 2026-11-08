#!/usr/bin/env python3
"""Verify independent image calls and regeneration after deletion through the real CLI."""

import json
import tempfile
import time
import uuid
from pathlib import Path

from run_cli_contract_e2e import checked, cli_e2e_parser, logged_in_cli, provider_log_entries, runtime_snapshot, secure_json


def main():
    parser = cli_e2e_parser(__doc__)
    args = parser.parse_args()
    args.report.parent.mkdir(parents=True, exist_ok=True)
    report = {"success": False, "checks": []}
    try:
        with tempfile.TemporaryDirectory(prefix="image-publication-", dir=args.report.parent) as directory:
            work = Path(directory)
            cli = logged_in_cli(args, work)
            for name in ("start_image_generation", "list_images_by_project", "delete_image"):
                checked(cli, ["capabilities", "describe", name], name + " contract")
            secure_json(work / "runtime.json", runtime_snapshot(args.provider_url, embedding_dimension=384))
            checked(cli, ["config", "runtime", "set", "--input", "@" + str(work / "runtime.json")], "runtime")

            def call(name, value, destructive=False):
                command = ["call", name, "--args", json.dumps({"input": value}, ensure_ascii=False)]
                if destructive:
                    command.append("--allow-destructive")
                return checked(cli, command, name)[0]["data"]

            project = checked(cli, [
                "project", "create", "--title", "image publication regression",
                "--project-type", "novel_long",
            ], "project")[0]["data"]
            report["project_id"] = project["id"]
            item = {"prompt": "雨夜中的仓库入口，电影感构图", "image_type": "scene", "config_id": 900002, "aspect_ratio": "1:1"}

            def generate(requests=None, expected_error=None):
                handle = call("start_image_generation", {"project_id": project["id"],
                    "meta": {"start_request_id": str(uuid.uuid4())},
                    "operation": {"kind": "batch", "requests": requests if requests is not None else [item, item]}})
                deadline = time.monotonic() + 60
                while time.monotonic() < deadline:
                    result = checked(cli, ["task", "get", handle["task_id"], "--attempt", str(handle["attempt"])], "image result")[0]["data"]
                    status = result["handle"]["status"]
                    if status in {"completed", "failed", "cancelled", "interrupted"}:
                        if expected_error:
                            assert status == "failed" and expected_error in json.dumps(result, ensure_ascii=False), result
                            return
                        assert status == "completed", result
                        return call("list_images_by_project", {"project_id": project["id"]})
                    time.sleep(0.1)
                raise AssertionError("image generation did not terminate")

            before = len(provider_log_entries(args.provider_log))
            generate([{**item, "metadata_json": ["invalid"]}], "metadata_json")
            early_calls = [entry for entry in provider_log_entries(args.provider_log)[before:]
                           if entry["path"].endswith(("/images/generations", "/images/edits"))]
            assert not early_calls, early_calls
            report["checks"].append("invalid metadata fails before any image Provider request")
            first = generate()
            assert len(first) == 2 and len({image["file_path"] for image in first}) == 2, first
            selected = call("list_images_by_project", {
                "project_id": project["id"],
                "image_ids": [first[0]["id"]],
            })
            assert selected == [first[0]], selected
            report["checks"].append("project image list filters requested IDs and preserves summary order")
            report["checks"].append("identical batch items publish distinct images")
            inspection_start=len(provider_log_entries(args.provider_log))
            inspection=call("read_image",{"projectId":project["id"],"imageIds":[first[0]["id"]],"chapterId":None,"instruction":"检查图片构图"})
            assert inspection["image_ids"]==[first[0]["id"]] and inspection["analysis"].strip(),inspection
            inspection_calls=provider_log_entries(args.provider_log)[inspection_start:]
            assert len(inspection_calls)==1 and "data:image/png;base64," in json.dumps(inspection_calls[0]["body"]),inspection_calls
            report["checks"].append("read_image sends real persisted image bytes to the vision model and returns validated analysis")
            second = generate()
            assert len(second) == 4 and len({image["file_path"] for image in second}) == 4, second
            report["checks"].append("a new run with the same requests creates new publications")
            old_ids = {image["id"] for image in second}
            old_paths = {image["file_path"] for image in second}
            for image in second:
                call("delete_image", {"image_id": image["id"]}, True)
            assert not call("list_images_by_project", {"project_id": project["id"]})
            regenerated = generate()
            assert len(regenerated) == 2, regenerated
            assert not old_ids.intersection(image["id"] for image in regenerated)
            assert not old_paths.intersection(image["file_path"] for image in regenerated)
            report["checks"].append("deleted publications never block or replace new generations")
            requests = [entry for entry in provider_log_entries(args.provider_log)[before:]
                        if entry["path"].endswith(("/images/generations", "/images/edits"))]
            assert len(requests) == 6, len(requests)
            report["provider_calls"] = len(requests)
            report["success"] = True
    except Exception as error:
        report["error"] = str(error)
        raise
    finally:
        args.report.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
        print(json.dumps({"success": report["success"], "checks": len(report["checks"]), "report": str(args.report)}))


if __name__ == "__main__":
    main()
