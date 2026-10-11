#!/usr/bin/env python3
"""Unit tests for the fail-closed test matrix runner."""

from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
from unittest.mock import patch

from run_test_matrix import MatrixError, load_matrix, run_suite, selected_suites


class TestMatrixTest(unittest.TestCase):
    def test_rejects_unknown_profile_and_suite(self) -> None:
        matrix = {"schema_version": 2, "suites": {"one": {"command": ["true"], "timeout_seconds": 1}}, "profiles": {"fast": {"suites": ["one"]}}}
        with self.assertRaisesRegex(MatrixError, "未知测试 profile"):
            selected_suites(matrix, "nightly", [])
        with self.assertRaisesRegex(MatrixError, "未知 suite"):
            selected_suites(matrix, "fast", ["missing"])

    def test_load_matrix_rejects_missing_profile_suite(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "matrix.json"
            path.write_text(json.dumps({"schema_version": 2, "suites": {}, "profiles": {"fast": {"suites": ["missing"]}}}))
            with self.assertRaisesRegex(MatrixError, "未知 suite"):
                load_matrix(path)

    def test_load_matrix_rejects_legacy_list_profiles(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "matrix.json"
            path.write_text(
                json.dumps(
                    {
                        "schema_version": 2,
                        "suites": {"one": {"command": ["true"], "timeout_seconds": 1}},
                        "profiles": {"fast": ["one"]},
                    }
                )
            )
            with self.assertRaisesRegex(MatrixError, "继承定义"):
                load_matrix(path)

    def test_load_matrix_rejects_invalid_test_center_group_ids(self) -> None:
        matrix = {
            "schema_version": 2,
            "suites": {"one": {"command": ["true"], "timeout_seconds": 1}},
            "profiles": {"fast": {"suites": ["one"]}},
            "test_center": {
                "functional_suites": ["one"],
                "groups": [{
                    "id": "Invalid_Group",
                    "category": "功能",
                    "title": "功能",
                    "description": "功能",
                    "suites": ["one"],
                }],
            },
        }
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "matrix.json"
            path.write_text(json.dumps(matrix))
            with self.assertRaisesRegex(MatrixError, "group id 无效"):
                load_matrix(path)

    def test_selected_suite_override_is_ordered_and_explicit(self) -> None:
        matrix = {"schema_version": 2, "suites": {"one": {"command": ["true"], "timeout_seconds": 1}, "two": {"command": ["true"], "timeout_seconds": 1}}, "profiles": {"fast": {"suites": ["one", "two"]}}}
        self.assertEqual(selected_suites(matrix, "fast", ["two"]), ["two"])

    def test_profiles_inherit_without_repeating_suite_lists(self) -> None:
        matrix = {
            "schema_version": 2,
            "suites": {name: {"command": ["true"], "timeout_seconds": 1} for name in ("one", "two", "three")},
            "profiles": {
                "fast": {"suites": ["one"]},
                "nightly": {"extends": "fast", "suites": ["two"]},
                "full": {"extends": "nightly", "suites": ["three", "one"], "exclude": ["two"]},
            },
        }
        self.assertEqual(selected_suites(matrix, "full", []), ["one", "three"])

    def test_requested_suites_are_deduplicated(self) -> None:
        matrix = load_matrix()
        self.assertEqual(
            selected_suites(matrix, "fast", ["python-tests", "python-tests"]),
            ["python-tests"],
        )

    def test_repository_profiles_only_run_additional_suites_once(self) -> None:
        matrix = load_matrix()
        fast = selected_suites(matrix, "fast", [])
        nightly = selected_suites(matrix, "nightly", [])
        nightly_quick = selected_suites(matrix, "nightly-quick", [])
        full = selected_suites(matrix, "full", [])
        self.assertIn("affected-rust-integration", fast)
        self.assertIn("rust-service-protocol", fast)
        self.assertNotIn("browser-fact-feedback", fast)
        self.assertNotIn("affected-rust-integration", nightly)
        self.assertNotIn("rust-service-protocol", nightly)
        self.assertIn("frontend-browser-helpers", fast)
        self.assertNotIn("browser-ui-functional", fast)
        self.assertIn("rust-workspace", nightly)
        self.assertIn("browser-ui-functional", nightly)
        self.assertIn("browser-ui-functional", nightly_quick)
        self.assertIn("browser-fact-feedback", nightly)
        self.assertIn("browser-fact-feedback", nightly_quick)
        self.assertIn("browser-canvas", nightly_quick)
        self.assertNotIn("rust-workspace", nightly_quick)
        self.assertNotIn("creation-e2e", nightly_quick)
        self.assertEqual(len(nightly), len(set(nightly)))
        self.assertTrue({"core-boundaries", "worker-isolation"}.issubset(full))
        self.assertEqual(len(full), len(set(full)))

    def test_feature_workflows_are_in_nightly_and_creation_e2e_runs_browser_flows(self) -> None:
        matrix = load_matrix()
        nightly = selected_suites(matrix, "nightly", [])
        quick = selected_suites(matrix, "nightly-quick", [])
        creation = matrix["suites"]["creation-e2e"]["command"]
        self.assertIn("browser-ui-functional", nightly)
        self.assertIn("browser-ui-functional", quick)
        self.assertNotIn("tauri-functional", nightly)
        self.assertEqual(selected_suites(matrix, "tauri-functional", []), ["tauri-functional"])
        self.assertIn("--browser-feature-flows", creation)

    def test_rejects_profile_inheritance_cycles(self) -> None:
        matrix = {
            "schema_version": 2,
            "suites": {"one": {"command": ["true"], "timeout_seconds": 1}},
            "profiles": {"first": {"extends": "second", "suites": ["one"]}, "second": {"extends": "first", "suites": ["one"]}},
        }
        with self.assertRaisesRegex(MatrixError, "形成循环"):
            selected_suites(matrix, "first", [])

    def test_matrix_does_not_execute_commands_while_listing(self) -> None:
        from run_test_matrix import main

        with patch("sys.argv", ["run_test_matrix.py", "--list"]):
            self.assertEqual(main(), 0)

    def test_matrix_reports_suite_start_before_completion(self) -> None:
        from run_test_matrix import main

        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "report"
            output = StringIO()
            with (
                patch("sys.argv", ["run_test_matrix.py", "--suite", "python-tests", "--report-dir", str(report)]),
                patch(
                    "run_test_matrix.run_suite",
                    return_value={"name": "python-tests", "status": "passed", "exit_code": 0},
                ),
                redirect_stdout(output),
            ):
                self.assertEqual(main(), 0)

            lines = [json.loads(line) for line in output.getvalue().splitlines()]
            suite_events = [line for line in lines if line.get("suite") == "python-tests"]
            self.assertEqual(
                [event["status"] for event in suite_events],
                ["running", "passed"],
            )

    def test_dry_run_does_not_create_report_directory(self) -> None:
        from run_test_matrix import main

        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "not-created"
            with patch("sys.argv", ["run_test_matrix.py", "--dry-run", "--report-dir", str(report)]):
                self.assertEqual(main(), 0)
            self.assertFalse(report.exists())

    def test_timeout_terminates_the_suite_process(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory)
            pid_file = report / "child.pid"
            code = "import os,sys,time; open(sys.argv[1], 'w').write(str(os.getpid())); time.sleep(30)"
            result = run_suite(
                "timeout",
                {"command": [sys.executable, "-c", code, str(pid_file)], "timeout_seconds": 1},
                report,
            )
            self.assertEqual(result["status"], "timeout")
            with self.assertRaises(ProcessLookupError):
                os.kill(int(pid_file.read_text()), 0)

    def test_suite_report_directory_placeholder_is_scoped_to_run(self) -> None:
        class CompletedProcess:
            pid = 1

            def wait(self, timeout=None):
                return 0

            def poll(self):
                return 0

        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "matrix-report"
            report.mkdir()
            with patch("run_test_matrix.subprocess.Popen", return_value=CompletedProcess()) as spawn:
                result = run_suite(
                    "creation-e2e",
                    {
                        "command": ["python3", "runner.py", "--report-dir", "{report_dir}/creation-e2e"],
                        "timeout_seconds": 1,
                    },
                    report,
                )
            self.assertEqual(result["status"], "passed")
            command = spawn.call_args.args[0]
            self.assertEqual(command[-1], str(report / "creation-e2e"))


if __name__ == "__main__":
    unittest.main()
