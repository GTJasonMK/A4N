#!/usr/bin/env python3
"""Check a populated, isolated project family's import and rollback through the CLI."""

import argparse
import base64
import hashlib
import io
import json
import os
from pathlib import Path
import tempfile
import zipfile

from run_cli_contract_e2e import Cli, RunnerError, checked


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--a4n", type=Path, required=True)
    parser.add_argument("--server", required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--project-id", type=int, required=True)
    parser.add_argument("--canvas-project-id", type=int, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--execute-writes", action="store_true", required=True)
    args = parser.parse_args()
    if min(args.project_id, args.canvas_project_id) <= 0:
        parser.error("project IDs must be positive")
    args.report.parent.mkdir(parents=True, exist_ok=True)
    report = {"passed": False, "imports": []}
    try:
        with tempfile.TemporaryDirectory(dir=args.report.parent) as temporary:
            work = Path(temporary)
            cli = Cli(str(args.a4n.resolve()), args.server, args.config.resolve(),
                      work / "context.json", 60, os.environ.copy())

            def call(command, payload):
                envelope, _ = checked(cli, ["call", command, "--args", "-", "--allow-destructive"],
                                      command, json.dumps(payload))
                return envelope["data"]

            def graph(project_id):
                return call("get_canvas_graph", {"input": {"project_id": project_id, "graph_id": None}})

            def project_ids():
                ids, page = set(), 1
                while True:
                    result = call("list_projects", {"page": page, "pageSize": 100})
                    ids.update(row["id"] for row in result["items"])
                    if len(ids) == result["total"]:
                        return ids
                    if not result["items"]:
                        raise RunnerError("project pagination stopped before total")
                    page += 1

            def download_hash(path, label):
                destination = work / label
                checked(cli, ["media", "download", path, "--output", str(destination)], label)
                return hashlib.sha256(destination.read_bytes()).hexdigest()

            source = graph(args.canvas_project_id)
            assert source["nodes"] and source["edges"], "fixture must contain connected canvas nodes"
            source_images = [n for n in source["nodes"] if n["ref_type"] == "generated_image"]
            assert source_images, "fixture must contain a completed generated image"
            source_hash = download_hash(source_images[0]["config_json"]["file_path"], "source.png")
            exported = call("export_project_bundle_data", {"input": {
                "project_id": args.project_id, "include_derived_projects": True,
            }})
            archive = base64.b64decode(exported["data_base64"], validate=True)
            with zipfile.ZipFile(io.BytesIO(archive)) as package:
                manifest = json.loads(package.read("a4n-project-bundle.json"))
                assert manifest["history"]["model_usage_records"], "fixture must contain model usage history"
                canvases = [c for c in manifest["canvas_projects"] if c["project"]["id"] == args.canvas_project_id]
                assert len(canvases) == 1
                canvas = canvases[0]
                assert len(canvas["canvas_nodes"]) == len(source["nodes"])
                assert len(canvas["canvas_edges"]) == len(source["edges"])
                assert not canvas["generated_images"], "family media must have one owner"
                title = canvas["project"]["title"]
                bad = json.loads(json.dumps(manifest))
                bad["history"]["model_usage_records"][0]["project_id"] = 9_000_000_000_000
                buffer = io.BytesIO()
                with zipfile.ZipFile(buffer, "w", zipfile.ZIP_DEFLATED) as broken:
                    for name in package.namelist():
                        data = json.dumps(bad).encode() if name == "a4n-project-bundle.json" else package.read(name)
                        broken.writestr(name, data)
            payload = {"input": {"file_name": exported["file_name"], "data_base64": exported["data_base64"]}}
            for index in range(3):
                if index == 2:
                    roots = [p for p in imported["project_summaries"] if p["project_kind"] == "text" and p["root_project_id"] is None]
                    assert len(roots) == 1
                    repacked = call("export_project_bundle_data", {"input": {
                        "project_id": roots[0]["id"], "include_derived_projects": True,
                    }})
                    payload = {"input": {"file_name": repacked["file_name"], "data_base64": repacked["data_base64"]}}
                imported = call("import_project_bundle_upload", payload)
                matches = [p for p in imported["project_summaries"] if p["title"] == title and p["project_kind"] == "video"]
                assert len(matches) == 1
                project_id = matches[0]["id"]
                restored = graph(project_id)
                assert len(restored["nodes"]) == len(source["nodes"])
                assert len(restored["edges"]) == len(source["edges"])
                assert restored["graph"]["id"] != source["graph"]["id"]
                images = [n for n in restored["nodes"] if n["ref_type"] == "generated_image"]
                assert len(images) == len(source_images)
                assert len({n["ref_id"] for n in images}) == len({n["ref_id"] for n in source_images})
                for image in images:
                    assert image["ref_id"] not in {n["ref_id"] for n in source_images}
                    assert image["config_json"]["generated_image_id"] == image["ref_id"]
                assert download_hash(images[0]["config_json"]["file_path"], f"restored-{index}.png") == source_hash
                text = [n for n in restored["nodes"] if n["ref_type"] == "canvas_text_asset"]
                original_text = [n for n in source["nodes"] if n["ref_type"] == "canvas_text_asset"]
                assert [n["config_json"]["content"] for n in text] == [n["config_json"]["content"] for n in original_text]
                assert {n["ref_id"] for n in text}.isdisjoint(n["ref_id"] for n in original_text)
                report["imports"].append({"project_id": project_id, "graph": restored,
                                          "workflow_runs": imported["workflow_runs_imported"]})
            before = project_ids()
            failed = cli.run(["call", "import_project_bundle_upload", "--args", "-", "--allow-destructive"],
                             json.dumps({"input": {"file_name": exported["file_name"],
                                                    "data_base64": base64.b64encode(buffer.getvalue()).decode()}}))
            assert failed["exit_code"] != 0
            errors = failed["stderr_documents"]
            assert errors and "缺少旧 ID" in errors[-1]["error"]["message"]
            assert "回滚失败" not in errors[-1]["error"]["message"]
            assert project_ids() == before, "failed import left partial projects behind"
            report.update(passed=True, rollback_error=errors[-1]["error"], media_sha256=source_hash)
    except Exception as error:
        report["error"] = str(error)
        raise
    finally:
        args.report.write_text(json.dumps(report, ensure_ascii=False, indent=2))
        args.report.chmod(0o600)


if __name__ == "__main__":
    main()
