#!/usr/bin/env python3
"""Exercise image retries and changed files through an isolated real CLI/Server."""

import argparse
import base64
import json
import tempfile
import threading
import time
import uuid
from pathlib import Path

from mock_openai.protocol import MP4_1S_BASE64, PNG_1X1_BASE64
from mock_openai.transport import ProviderServer
from run_cli_contract_e2e import checked, logged_in_cli, provider_log_entries, runtime_snapshot, secure_json

FIRST = "checkpoint audit first image"
SECOND = "checkpoint audit second image"


class ImageFailureProvider(ProviderServer):
    fail_second = True

    def record(self, path, body):
        # ponytail: serial fixture; use per-request responses if parallel cases are added.
        if path.rstrip("/").endswith("/images/generations"):
            self.image_base64 = (
                base64.b64encode(b"invalid image bytes").decode("ascii")
                if self.fail_second and body["prompt"] == SECOND else PNG_1X1_BASE64
            )
        super().record(path, body)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("a4n", "server", "username"):
        parser.add_argument("--" + name, required=True)
    parser.add_argument("--data-dir", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--password-env", default="A4N_PASSWORD")
    args = parser.parse_args()
    args.report.parent.mkdir(parents=True, exist_ok=True)
    report = {"success": False, "checks": []}
    try:
        with tempfile.TemporaryDirectory(prefix="image-checkpoint-", dir=args.report.parent) as directory:
            work = Path(directory)
            log = work / "provider.jsonl"
            provider = ImageFailureProvider(("127.0.0.1", 0), log, PNG_1X1_BASE64, MP4_1S_BASE64, 384)
            thread = threading.Thread(target=provider.serve_forever, daemon=True)
            thread.start()
            try:
                cli = logged_in_cli(args, work)
                settings = runtime_snapshot(f"http://127.0.0.1:{provider.server_address[1]}/v1", embedding_dimension=384)
                settings["image"]["max_concurrent"] = 1
                for config in settings["image_configs"]:
                    config["max_concurrent"] = 1
                secure_json(work / "runtime.json", settings)
                checked(cli, ["config", "runtime", "set", "--input", "@" + str(work / "runtime.json")], "runtime")
                project = checked(cli, ["project", "create", "--title", "image checkpoint audit", "--project-type", "novel_long"], "project")[0]["data"]

                def call(name, value):
                    return checked(cli, ["call", name, "--args", json.dumps({"input": value})], name)[0]["data"]

                def generate(retry_of=None):
                    handle = call("start_image_generation", {
                        "project_id": project["id"],
                        "meta": {"start_request_id": str(uuid.uuid4()), "retry_of_task_id": retry_of},
                        "operation": {"kind": "batch", "requests": [
                            {"prompt": prompt, "image_type": "scene", "config_id": 900002, "aspect_ratio": "1:1"}
                            for prompt in (FIRST, SECOND)
                        ]},
                    })
                    deadline = time.monotonic() + 60
                    while time.monotonic() < deadline:
                        result = checked(cli, ["task", "get", handle["task_id"], "--attempt", str(handle["attempt"])], "task result")[0]["data"]
                        if result["handle"]["status"] in {"completed", "failed", "cancelled", "interrupted"}:
                            assert result["handle"]["status"] == "completed", result
                            snapshot = checked(cli, ["task", "snapshot", handle["task_id"], "--attempt", str(handle["attempt"])], "task snapshot")[0]["data"]
                            terminal = snapshot["terminal_event"]["event"]
                            assert terminal["kind"] == "completed", terminal
                            return handle, terminal["data"]["batch_summary"]
                        time.sleep(0.1)
                    raise AssertionError("image batch did not terminate")

                def requests():
                    return [entry["body"]["prompt"] for entry in provider_log_entries(log)
                            if entry["path"].rstrip("/").endswith("/images/generations")]

                first, summary = generate()
                assert (summary["succeeded"], summary["failed"]) == (1, 1), summary
                images = call("list_images_by_project", {"project_id": project["id"]})
                assert len(images) == 1 and images[0]["prompt"] == FIRST, images
                saved_id = images[0]["id"]
                workflows = call("list_workflow_runs", {
                    "project_id": project["id"], "workflow_kind": "image_batch",
                    "chapter_id": None, "status": None, "resumable_only": False, "limit": 10,
                })
                assert len(workflows) == 1 and workflows[0]["status"] == "failed" and workflows[0]["can_resume"], workflows
                report["checks"].append("partial best-effort result keeps successful calls resumable")

                root = args.data_dir.resolve(strict=True)
                path = (root / images[0]["file_path"]).resolve(strict=True)
                assert path.is_relative_to(root) and path.is_file(), path
                original = path.read_bytes()
                provider.fail_second = False
                try:
                    path.write_bytes(original + b"\nchanged checkpoint file")
                    second, summary = generate(first["task_id"])
                    assert (summary["succeeded"], summary["failed"]) == (1, 1), summary
                    assert summary["failed_item_keys"] == ["image-request-1"], summary
                    assert requests() == [FIRST, SECOND, SECOND], requests()
                    report["checks"].append("changed checkpoint bytes are rejected without repeating the successful provider call")
                finally:
                    path.write_bytes(original)

                _, summary = generate(second["task_id"])
                assert (summary["succeeded"], summary["failed"]) == (2, 0), summary
                assert requests() == [FIRST, SECOND, SECOND], requests()
                images = call("list_images_by_project", {"project_id": project["id"]})
                assert len(images) == 2 and saved_id in {image["id"] for image in images}, images
                report["checks"].append("repairing the file restores both checkpoints without any extra provider call")
                report.update(success=True, provider_calls=3, project_id=project["id"])
            finally:
                provider.shutdown()
                provider.server_close()
                thread.join(timeout=2)
    except Exception as error:
        report["error"] = str(error)
        raise
    finally:
        args.report.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
        print(json.dumps({"success": report["success"], "checks": len(report["checks"]), "report": str(args.report)}))


if __name__ == "__main__":
    main()
