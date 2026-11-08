import tempfile
import json
import unittest
from pathlib import Path

from run_model_tests import ModelTestError, creation_binary_hashes, observations, partitions, run, rust_result


class ModelRunnerTests(unittest.TestCase):
    def test_only_complete_nonempty_rust_execution_passes(self):
        self.assertEqual(rust_result("test result: ok. 2 passed; 0 failed; 0 ignored;"), 2)
        for text in ("", "test result: ok. 0 passed; 0 failed; 0 ignored;",
                     "test result: ok. 1 passed; 0 failed; 1 ignored;",
                     "test result: FAILED. 1 passed; 1 failed; 0 ignored;",
                     "test result: ok. 1 passed; 0 failed; 0 ignored;\n" * 2):
            with self.subTest(text=text), self.assertRaises(ModelTestError):
                rust_result(text)

    def test_partition_keeps_every_test_once_and_bounds_group_size(self):
        names = [f"owner::group{i // 10}::case{i}" for i in range(81)]
        groups = partitions("owner::", names)
        self.assertEqual(sorted(name for _, rows, _ in groups for name in rows), sorted(names))
        self.assertTrue(all(0 < len(rows) <= 25 for _, rows, _ in groups))
        for prefix, rows, exact in groups:
            self.assertEqual([name for name in names if name == prefix] if exact else [name for name in names if name.startswith(prefix)], rows)

    def test_failure_timeout_and_malformed_observation_cannot_be_success(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "failed.log"
            with self.assertRaises(ModelTestError):
                run(["python3", "-c", "raise SystemExit(2)"], path, 2)
            with self.assertRaises(ModelTestError):
                run(["python3", "-c", "import time; time.sleep(2)"], path, 0.05)
        with self.assertRaises(ValueError):
            observations("test: A4N_MODEL_CASE:{broken}", "A4N_MODEL_CASE:")

    def test_creation_executables_must_match_the_source_and_build_receipt(self):
        from run_creation_e2e import BINARIES
        from model_evidence import digest_file
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            for name in BINARIES:(root/name).write_bytes(b"actual test executable")
            receipt={"source_sha256":"source","binaries":{name:digest_file(root/name) for name in BINARIES}}
            path=root/"source.json";path.write_text(json.dumps(receipt))
            creation={"binaries":str(root),"source":{"sha256":"source"}}
            self.assertEqual(len(creation_binary_hashes(creation)),len(BINARIES))
            creation["source"]["sha256"]="changed"
            with self.assertRaisesRegex(ModelTestError,"源码"):creation_binary_hashes(creation)
            creation["source"]["sha256"]="source"
            (root/BINARIES[0]).write_bytes(b"different executable")
            with self.assertRaisesRegex(ModelTestError,"运行后改变"):creation_binary_hashes(creation)


if __name__ == "__main__":
    unittest.main()
