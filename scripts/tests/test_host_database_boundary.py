"""Exercise the Host database boundary gate without compiling or network access."""

import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/checks/check-host-no-business-db.sh"
PACKAGES = (
    "a4n-control",
    "a4n-host-assembly",
    "a4n-server",
    "a4n-edge",
    "a4n-tauri",
    "a4n-runtime",
)


class HostDatabaseBoundaryTest(unittest.TestCase):
    def run_gate(self, trees=None, sources=None):
        with tempfile.TemporaryDirectory(prefix="host-db-boundary-") as temporary:
            root = Path(temporary)
            (root / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")
            script = root / "scripts/checks/check-host-no-business-db.sh"
            script.parent.mkdir(parents=True)
            shutil.copy2(SCRIPT, script)

            tree_dir = root / "trees"
            tree_dir.mkdir()
            for package in PACKAGES:
                crate = root / "crates/host" / package
                (crate / "src").mkdir(parents=True)
                (crate / "Cargo.toml").write_text(
                    f'[package]\nname = "{package}"\nversion = "0.1.0"\n'
                    "[dev-dependencies]\na4n-db = \"0.1\"\n",
                    encoding="utf-8",
                )
                (crate / "src/lib.rs").write_text("", encoding="utf-8")
                (tree_dir / f"{package}.txt").write_text(
                    (trees or {}).get(package, ""), encoding="utf-8"
                )

            for relative_path, content in (sources or {}).items():
                path = root / "crates/host" / relative_path
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content, encoding="utf-8")

            bin_dir = root / "bin"
            bin_dir.mkdir()
            cargo = bin_dir / "cargo"
            cargo.write_text(
                "#!/usr/bin/env bash\n"
                "set -euo pipefail\n"
                "[[ \"$*\" == *\"--locked\"* && \"$*\" == *\"--offline\"* "
                "&& \"$*\" == *\"--edges normal\"* && \"$*\" == *\"--depth 1\"* ]] || exit 90\n"
                "package=\"\"\n"
                "while (($#)); do\n"
                "  if [[ \"$1\" == -p ]]; then package=\"$2\"; shift 2; "
                "else shift; fi\n"
                "done\n"
                "[[ -n \"$package\" ]] || exit 91\n"
                "tree_file=\"$A4N_GATE_TREE_DIR/$package.txt\"\n"
                "[[ ! -f \"$tree_file\" ]] || cat \"$tree_file\"\n",
                encoding="utf-8",
            )
            cargo.chmod(0o755)

            environment = os.environ.copy()
            environment["PATH"] = f"{bin_dir}{os.pathsep}{environment['PATH']}"
            environment["A4N_GATE_TREE_DIR"] = str(tree_dir)
            return subprocess.run(
                [str(script)],
                cwd=root,
                env=environment,
                text=True,
                capture_output=True,
                timeout=30,
                check=False,
            )

    def test_allows_runtime_infrastructure_and_test_only_database_use(self):
        runtime_tree = (
            "a4n-db-models v0.1.0\n"
            "a4n-migration v0.1.0\n"
            "sea-orm v1.1.20\n"
        )
        result = self.run_gate(
            trees={
                "a4n-runtime": runtime_tree,
                "a4n-tauri": "a4n-runtime v0.1.0\n" + runtime_tree,
            },
            sources={
                "a4n-control/src/lib.rs": "#[cfg(test)]\nuse a4n_db::project;\n",
                "a4n-control/src/db_tests.rs": "use a4n_db_models::models::project;\n",
                "a4n-runtime/src/state/database.rs": (
                    "use sea_orm::DatabaseConnection;\n"
                    "use a4n_migration::migrator;\n"
                    "const REQUIRED: &str = a4n_db_models::REQUIRED_SCHEMA_MIGRATION;\n"
                ),
                "a4n-runtime/src/state/app.rs": "use sea_orm::DatabaseConnection;\n",
                "a4n-runtime/src/state/app/bootstrap.rs": "use sea_orm::Database;\n",
            },
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("passed for all six Host crates", result.stdout)

    def test_rejects_direct_business_database_dependency(self):
        result = self.run_gate(
            trees={
                "a4n-control": "a4n-db v0.1.0\n",
                "a4n-server": "a4n-db-models v0.1.0\n",
            },
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("a4n-control direct normal dependencies", result.stderr)
        self.assertIn("a4n-db", result.stderr)
        self.assertIn("a4n-server direct normal dependencies contain ORM model", result.stderr)

    def test_rejects_business_model_and_non_runtime_orm_source(self):
        result = self.run_gate(
            sources={
                "a4n-control/src/lib.rs": "use a4n_db_models::models::chapter;\n",
                "a4n-runtime/src/actions/records.rs": "use sea_orm::EntityTrait;\n",
            },
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("a4n-control source contains forbidden business database/ORM", result.stderr)
        self.assertIn("a4n-runtime source contains forbidden non-infrastructure ORM/migration", result.stderr)


if __name__ == "__main__":
    unittest.main()
