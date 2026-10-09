#!/usr/bin/env python3
"""Unit tests for the fail-closed test matrix runner."""

from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest
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


if __name__ == "__main__":
    unittest.main()
