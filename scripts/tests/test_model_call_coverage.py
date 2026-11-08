import copy
import tempfile
import unittest
from pathlib import Path
from check_model_call_coverage import inspect,structured_ids


class ModelCallCoverageTest(unittest.TestCase):
    def setUp(self):
        self.temporary=tempfile.TemporaryDirectory();self.addCleanup(self.temporary.cleanup)
        self.root=Path(self.temporary.name)
        source=self.root/"crates/host/a4n-orchestrator/src/run.rs"
        source.parent.mkdir(parents=True);source.write_text("fn run() {}")
        self.ast={"schema_version":2,"scanner":"syn-workspace","calls":[{"file":"crates/host/a4n-orchestrator/src/run.rs","function":"run","callee":"chat_stream"}],"implementations":[{"trait":"LlmProvider"}],"catalogs":{key:[{"id":key}] for key in ("text","image","video")}}
        self.baseline=copy.deepcopy(self.ast)
        self.ledger={"schema_version":2,"scope":"registration only","structured":[{"calls":["task"],"owners":["run.rs"],"validators":["run.rs"],"contract_owner":"crates/host/a4n-orchestrator/src/run.rs","branches":["run"]}],"shared_scenarios":{key:["normal","invalid"] for key in ("text","image","video")}}

    def check(self):return inspect(self.root,self.ledger,self.ast,self.baseline)

    def test_static_inventory_does_not_claim_runtime_execution(self):
        self.assertEqual(self.check()["business_json_calls"],1)
        self.assertFalse(self.check()["runtime_coverage_measured"])

    def test_new_call_and_provider_outside_old_scan_roots_cannot_pass(self):
        self.ast["calls"].append({"file":"crates/new-provider/src/run.rs","function":"run","callee":"embed"})
        with self.assertRaisesRegex(ValueError,"新增调用"):self.check()
        self.ast["calls"].pop()
        self.ast["implementations"].append({"trait":"ImageProvider"})
        with self.assertRaisesRegex(ValueError,"implementations"):self.check()

    def test_suite_assignment_cannot_replace_business_contracts(self):
        self.ledger["structured"][0]["suites"]=["authoring"]
        with self.assertRaisesRegex(ValueError,"执行证据"):self.check()
        del self.ledger["structured"][0]["suites"]
        self.ledger["structured"][0]["validators"]=["missing.rs"]
        with self.assertRaisesRegex(ValueError,"不存在"):self.check()
        self.ledger["structured"][0]["validators"]=[""]
        with self.assertRaisesRegex(ValueError,"不存在"):self.check()

    def test_free_text_identifiers_and_search_execution_scenarios_cannot_disappear(self):
        row={"call_id":"text","owner":"run.rs","validator":"validate_text","branches":["run"]}
        self.ledger["free_text"]=[row,row.copy()]
        with self.assertRaisesRegex(ValueError,"重复"):self.check()
        self.ledger["free_text"]=[row]
        self.ast["catalogs"]["search"]=[{"id":"anthropic"}]
        self.baseline["catalogs"]["search"]=[{"id":"anthropic"}]
        with self.assertRaisesRegex(ValueError,"必测异常"):self.check()

    def test_missing_owner_duplicate_id_and_catalog_drift_fail(self):
        self.ledger["structured"][0]["owners"]=["other.rs"]
        with self.assertRaisesRegex(ValueError,"实际模型调用"):self.check()
        self.ledger["structured"][0]["owners"]=["run.rs"]
        self.ledger["structured"][0]["calls"].append("task")
        with self.assertRaisesRegex(ValueError,"重复业务"):structured_ids(self.ledger)
        self.ledger["structured"][0]["calls"].pop()
        self.ast["catalogs"]["image"].append({"id":"new-backend"})
        with self.assertRaisesRegex(ValueError,"catalogs"):self.check()


if __name__=="__main__":unittest.main()
