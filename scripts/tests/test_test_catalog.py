#!/usr/bin/env python3
"""Tests for the functional catalog maintenance gate."""

from __future__ import annotations

import unittest

from check_test_catalog import CatalogError, build_report, resolve_profile


class TestCatalog(unittest.TestCase):
    def test_repository_catalog_has_handlers_and_matrix_profiles(self) -> None:
        report = build_report()
        self.assertGreaterEqual(report["scenario_count"], 1)
        self.assertIn("fast", report["matrix"]["profiles"])

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


if __name__ == "__main__":
    unittest.main()
