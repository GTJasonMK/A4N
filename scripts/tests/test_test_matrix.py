#!/usr/bin/env python3
"""Unit tests for the fail-closed test matrix runner."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from run_test_matrix import MatrixError, load_matrix, selected_suites


class TestMatrixTest(unittest.TestCase):
    def test_rejects_unknown_profile_and_suite(self) -> None:
        matrix = {"schema_version": 1, "suites": {"one": {"command": ["true"], "timeout_seconds": 1}}, "profiles": {"fast": ["one"]}}
        with self.assertRaisesRegex(MatrixError, "未知测试 profile"):
            selected_suites(matrix, "nightly", [])
        with self.assertRaisesRegex(MatrixError, "未知 suite"):
            selected_suites(matrix, "fast", ["missing"])

    def test_load_matrix_rejects_missing_profile_suite(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "matrix.json"
            path.write_text(json.dumps({"schema_version": 1, "suites": {}, "profiles": {"fast": ["missing"]}}))
            with self.assertRaisesRegex(MatrixError, "未知 suite"):
                load_matrix(path)

    def test_selected_suite_override_is_ordered_and_explicit(self) -> None:
        matrix = {"schema_version": 1, "suites": {"one": {"command": ["true"], "timeout_seconds": 1}, "two": {"command": ["true"], "timeout_seconds": 1}}, "profiles": {"fast": ["one", "two"]}}
        self.assertEqual(selected_suites(matrix, "fast", ["two"]), ["two"])

    def test_matrix_does_not_execute_commands_while_listing(self) -> None:
        from run_test_matrix import main

        with patch("sys.argv", ["run_test_matrix.py", "--list"]):
            self.assertEqual(main(), 0)


if __name__ == "__main__":
    unittest.main()
