import copy
import json
import subprocess
import tempfile
import unittest
import shutil
from pathlib import Path
from model_evidence import EvidenceError, input_snapshot, seal_artifact, verify_report


class ModelEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)
        (self.root / "prompts").mkdir()
        (self.root / "prompts/task.md").write_text("contract")
        self.artifact = self.root / "observations.json"
        self.artifact.write_text('{"actually_executed":true}')
        self.binary = self.root / "test-binary"
        shutil.copy2(shutil.which("true"),self.binary)
        from model_requirements import required_case_ids
        source=self.root/"crates/host/a4n-orchestrator/src/run.rs"
        source.parent.mkdir(parents=True);source.write_text("fn run() {}")
        self.ledger={"schema_version":2,"scope":"unit fixture","structured":[{"calls":["task"],"owners":["run.rs"],"validators":["run.rs"],"branches":["run"],"contract_owner":str(source.relative_to(self.root))}],"shared_scenarios":{"text":["normal"],"image":["actual-request-parameters"],"video":["actual-request-parameters"],"embedding":[]},"semantic_rust_groups":{}}
        self.inventory={"schema_version":2,"scanner":"syn-workspace","calls":[{"file":str(source.relative_to(self.root)),"function":"run","callee":"chat_stream"}],"implementations":[],"catalogs":{"text":[{"id":"text"}],"image":[{"id":"image","references":False}],"video":[{"id":"video","asynchronous":False,"last_frame":False,"max_reference_images":0,"reference_video_audio":False,"first_frame_with_references":False}]}}
        self.corpus={"task":{"schema":{"type":"object","required":["state"],"additionalProperties":False,"properties":{"state":{"type":"object"}}}}}
        manifest=self.root/"scripts/tests/model_calls.json";manifest.parent.mkdir(parents=True);manifest.write_text(json.dumps(self.ledger))
        (manifest.parent/"model_call_sites.json").write_text(json.dumps(self.inventory))
        inventory=self.root/"inventory.json";inventory.write_text(json.dumps(self.inventory))
        corpus=self.root/"contracts.json";corpus.write_text(json.dumps(self.corpus))
        from model_evidence import digest_file
        inputs = input_snapshot(self.root)
        required=required_case_ids(self.ledger,self.inventory,self.corpus,"model-fast")
        self.report = {"schema_version":2,"profile":"model-fast","success":True,
            "inputs":inputs,"finished_inputs_sha256":inputs["sha256"],"binaries":{str(self.binary):digest_file(self.binary)},
            "contracts":seal_artifact(corpus,self.root),"inventory":seal_artifact(inventory,self.root),
            "required_cases":sorted(required),"cases":[{"case_id":case_id,"status":"passed","assertions":2,"evidence":[seal_artifact(self.artifact,self.root)]} for case_id in sorted(required)],
            "suites":{"production-validator":{"status":"passed","executed":1}}}
        self.path = self.root / "report.json"

    def verify(self, full=False):
        self.path.write_text(json.dumps(self.report))
        return verify_report(self.path,root=self.root,require_full=full)

    def test_requires_real_cases_and_rejects_empty_skipped_failed_and_forged_coverage(self):
        self.assertGreater(self.verify()["cases"],1)
        for field,value in [("cases",[]),("required_cases",[]),("suites",{}),("binaries",{})]:
            original = self.report[field]
            self.report[field] = value
            with self.assertRaises(EvidenceError): self.verify()
            self.report[field] = original
        self.report["cases"][0]["status"] = "skipped"
        with self.assertRaises(EvidenceError): self.verify()

    def test_prompt_test_fixture_lock_and_binary_changes_invalidate_evidence(self):
        original = (self.root / "prompts/task.md").read_text()
        (self.root / "prompts/task.md").write_text("changed")
        with self.assertRaisesRegex(EvidenceError,"旧报告失效"): self.verify()
        (self.root / "prompts/task.md").write_text(original)
        for name in ["scripts/tests/case.py","scripts/tests/fixtures/sample.png","Cargo.lock"]:
            target = self.root / name
            target.parent.mkdir(parents=True,exist_ok=True)
            target.write_text("new test input")
            with self.assertRaisesRegex(EvidenceError,"旧报告失效"): self.verify()
            target.unlink()
        self.binary.write_text("different binary")
        with self.assertRaisesRegex(EvidenceError,"二进制"): self.verify()

    def test_missing_tampered_and_outside_artifacts_cannot_pass(self):
        self.artifact.write_text("tampered")
        with self.assertRaisesRegex(EvidenceError,"内容已改变"): self.verify()
        self.artifact.unlink()
        with self.assertRaisesRegex(EvidenceError,"缺失"): self.verify()
        self.report["cases"][0]["evidence"][0]["path"] = "../outside"
        with self.assertRaisesRegex(EvidenceError,"无效证据路径"): self.verify()

    def test_full_report_requires_same_version_creation_and_rejects_fast_receipt(self):
        with self.assertRaisesRegex(EvidenceError,"model-full"): self.verify(True)
        self.report["profile"] = "model-full"
        from model_requirements import required_case_ids
        required=required_case_ids(self.ledger,self.inventory,self.corpus,"model-full")
        self.report["required_cases"]=sorted(required)
        self.report["cases"].append({"case_id":"creation/full-cli-browser","status":"passed","assertions":1,"evidence":[seal_artifact(self.artifact,self.root)]})
        with self.assertRaisesRegex(EvidenceError,"完整 CLI"): self.verify(True)

    def test_removing_both_required_and_observed_cases_does_not_forge_completeness(self):
        removed=self.report["cases"].pop()
        self.report["required_cases"].remove(removed["case_id"])
        with self.assertRaisesRegex(EvidenceError,"必测项"):self.verify()

    def test_pass_label_cannot_hide_a_failed_assertion(self):
        self.report["cases"][0].update(expected=True,actual=False)
        with self.assertRaisesRegex(EvidenceError,"实际结果"):self.verify()


if __name__ == "__main__": unittest.main()
