#!/usr/bin/env python3
"""Unit tests for Rust package impact selection."""

from __future__ import annotations

import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch

from run_affected_rust_tests import (
    AffectedRustError,
    ROOT,
    needs_full_workspace,
    reverse_dependency_closure,
    select_packages,
    uses_onnx_runtime,
)


def package(name: str, directory: str, dependencies: tuple[str, ...] = ()) -> dict:
    return {
        "name": name,
        "manifest_path": str(ROOT / directory / "Cargo.toml"),
        "dependencies": [{"name": dependency} for dependency in dependencies],
    }


class AffectedRustTests(unittest.TestCase):
    def setUp(self) -> None:
        self.packages = {
            "a4n-migration": package("a4n-migration", "crates/platform/a4n-migration"),
            "a4n-db": package("a4n-db", "crates/platform/a4n-db", ("a4n-migration",)),
            "a4n-orchestrator": package("a4n-orchestrator", "crates/host/a4n-orchestrator", ("a4n-db",)),
            "a4n-runtime": package("a4n-runtime", "crates/host/a4n-runtime", ("a4n-orchestrator",)),
            "a4n-domain": package("a4n-domain", "crates/platform/a4n-domain"),
            "a4n-feature": package("a4n-feature", "crates/chapter/a4n-feature", ("a4n-domain",)),
            "a4n-server": package("a4n-server", "crates/host/a4n-server", ("a4n-runtime", "a4n-feature")),
            "a4n-project-contract": package("a4n-project-contract", "crates/project/a4n-project-contract"),
            "a4n-embedding-fastembed": package("a4n-embedding-fastembed", "crates/embedding/a4n-embedding-fastembed"),
            "a4n-embedding-service": package("a4n-embedding-service", "crates/embedding/a4n-embedding-service", ("a4n-embedding-fastembed",)),
        }
        self.package_by_dir = {
            "crates/platform/a4n-migration": "a4n-migration",
            "crates/platform/a4n-db": "a4n-db",
            "crates/host/a4n-orchestrator": "a4n-orchestrator",
            "crates/host/a4n-runtime": "a4n-runtime",
            "crates/platform/a4n-domain": "a4n-domain",
            "crates/chapter/a4n-feature": "a4n-feature",
            "crates/host/a4n-server": "a4n-server",
            "crates/project/a4n-project-contract": "a4n-project-contract",
            "crates/embedding/a4n-embedding-fastembed": "a4n-embedding-fastembed",
            "crates/embedding/a4n-embedding-service": "a4n-embedding-service",
        }

    def test_feature_change_selects_core_and_transitive_reverse_dependants(self) -> None:
        full, selected = select_packages(
            {"crates/chapter/a4n-feature/src/lib.rs"}, self.packages, self.package_by_dir
        )
        self.assertFalse(full)
        self.assertEqual(
            set(selected),
            {
                "a4n-migration",
                "a4n-db",
                "a4n-orchestrator",
                "a4n-runtime",
                "a4n-feature",
                "a4n-server",
            },
        )

    def test_contract_migration_and_root_dependency_changes_force_workspace(self) -> None:
        for path in (
            "Cargo.lock",
            "crates/platform/a4n-migration/src/lib.rs",
            "crates/project/a4n-project-contract/src/lib.rs",
            "crates/process/a4n-service-protocol/src/lib.rs",
        ):
            with self.subTest(path=path):
                self.assertTrue(needs_full_workspace({path}, self.packages))
                full, _ = select_packages({path}, self.packages, self.package_by_dir)
                self.assertTrue(full)

    def test_leaf_package_manifest_change_stays_scoped(self) -> None:
        full, selected = select_packages(
            {"crates/chapter/a4n-feature/Cargo.toml"}, self.packages, self.package_by_dir
        )
        self.assertFalse(full)
        self.assertIn("a4n-feature", selected)
        self.assertIn("a4n-server", selected)

    def test_untracked_package_file_is_mapped_and_docs_do_not_select_package(self) -> None:
        full, selected = select_packages(
            {
                "crates/host/a4n-server/src/new_module.rs",
                "docs/test-plan.md",
            },
            self.packages,
            self.package_by_dir,
        )
        self.assertFalse(full)
        self.assertIn("a4n-server", selected)

    def test_unowned_rust_source_fails_instead_of_silently_skipping(self) -> None:
        with self.assertRaisesRegex(AffectedRustError, "无法映射"):
            select_packages({"crates/chapter/README.rs"}, self.packages, self.package_by_dir)

    def test_onnx_runtime_follows_the_active_cargo_feature_graph(self) -> None:
        with patch(
            "run_affected_rust_tests.subprocess.run",
            return_value=subprocess.CompletedProcess(
                args=[], returncode=0, stdout="a4n-runtime v0.1.0\n", stderr=""
            ),
        ) as cargo_tree:
            self.assertFalse(uses_onnx_runtime(["a4n-runtime"]))
        self.assertIn("--edges", cargo_tree.call_args.args[0])

        with patch(
            "run_affected_rust_tests.subprocess.run",
            return_value=subprocess.CompletedProcess(
                args=[],
                returncode=0,
                stdout="a4n-embedding-service v0.1.0\na4n-embedding-fastembed v0.1.0\n",
                stderr="",
            ),
        ):
            self.assertTrue(uses_onnx_runtime(["a4n-embedding-service"]))


if __name__ == "__main__":
    unittest.main()
