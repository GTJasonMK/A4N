import tempfile
import unittest
from pathlib import Path

from check_model_call_coverage import inspect


class ModelCallCoverageTest(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / "src").mkdir()
        (self.root / "src/call.rs").write_text("fn run() { service.stream_complete_llm(request); }\n")
        (self.root / "scripts/tests").mkdir(parents=True)
        (self.root / "scripts/tests/flow.py").write_text("# actual suite path\n")
        self.ledger = {"schema_version":1,"scope":"registration only","roots":{"app":"src"},
                       "owners":{"app:call.rs":{"calls":{"stream_complete_llm":1},"kind":"json","recovery":"bounded_once","suites":["flow"]}}}

    def check(self):
        return inspect(self.root, self.ledger, {"flow":"flow.py"})

    def test_registered_inventory_passes_without_claiming_measured_runtime_coverage(self):
        result = self.check()
        self.assertEqual(result["static_calls"], 1)
        self.assertFalse(result["runtime_coverage_measured"])

    def test_added_file_and_additional_call_in_existing_file_require_registration(self):
        (self.root / "src/new.rs").write_text("fn run() { llm.chat_stream(request); }")
        with self.assertRaisesRegex(ValueError, "新增"):
            self.check()
        (self.root / "src/new.rs").unlink()
        (self.root / "src/call.rs").write_text("fn run() { service.stream_complete_llm(one); service.stream_complete_llm(two); }")
        with self.assertRaisesRegex(ValueError, "调用次数"):
            self.check()

    def test_removed_call_missing_suite_and_unknown_recovery_cannot_pass(self):
        source = self.root / "src/call.rs"
        previous = source.read_text()
        source.write_text("fn run() {}")
        with self.assertRaisesRegex(ValueError, "失效"):
            self.check()
        source.write_text(previous)
        self.ledger["owners"]["app:call.rs"]["suites"] = ["unregistered"]
        with self.assertRaisesRegex(ValueError, "未注册"):
            self.check()
        self.ledger["owners"]["app:call.rs"]["suites"] = ["flow"]
        self.ledger["owners"]["app:call.rs"]["recovery"] = "pretend_success"
        with self.assertRaisesRegex(ValueError, "恢复策略"):
            self.check()

    def test_test_modules_and_declarations_do_not_count_as_production_calls(self):
        (self.root / "src/call.rs").write_text('fn stream_complete_llm(x: Request) {}\nfn run() { service.stream_complete_llm(request); }\n#[cfg(test)]\nmod tests { fn case() { llm.chat_stream(request); } }')
        self.assertEqual(self.check()["static_calls"], 1)


if __name__ == "__main__":
    unittest.main()
