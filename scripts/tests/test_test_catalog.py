#!/usr/bin/env python3
"""Tests for the functional catalog maintenance gate."""

from __future__ import annotations

import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

from check_test_catalog import (
    CatalogError,
    build_report,
    resolve_profile,
    scenario_catalog,
)


class TestCatalog(unittest.TestCase):
    def test_repository_catalog_has_handlers_and_matrix_profiles(self) -> None:
        report = build_report()
        self.assertEqual(report["scenario_count"], 11)
        self.assertIn("fast", report["matrix"]["profiles"])
        self.assertEqual(report["matrix"]["functional_suite_count"], 16)
        self.assertGreaterEqual(report["matrix"]["test_center_group_count"], 5)

    def test_profile_cycles_are_rejected(self) -> None:
        with self.assertRaisesRegex(CatalogError, "循环"):
            resolve_profile(
                {
                    "first": {"extends": "second", "suites": ["one"]},
                    "second": {"extends": "first", "suites": ["one"]},
                },
                {"one": {"command": ["true"]}},
                "first",
            )

    def test_unregistered_handler_is_rejected(self) -> None:
        with TemporaryDirectory() as directory:
            source = Path(directory) / "test_console.rs"
            source.write_text(
                'scenario("declared");\nasync fn run_scenario() {\n'
                '"declared" => {}\n"orphan" => {}\n}\n'
                'async fn run_repository_test_group() {}\n',
                encoding="utf-8",
            )
            with patch("check_test_catalog.SCENARIO_SOURCE", source):
                with self.assertRaisesRegex(CatalogError, "没有注册场景"):
                    scenario_catalog()


if __name__ == "__main__":
    unittest.main()
