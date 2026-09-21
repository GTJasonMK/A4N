#!/usr/bin/env python3
"""Check domain grouping, inherited dependencies, and package lookup."""

import json
import subprocess
import tempfile
import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class CrateLayoutTest(unittest.TestCase):
    def test_workspace_members_and_dependencies(self):
        metadata = json.loads(subprocess.check_output(
            ["cargo", "metadata", "--manifest-path", str(ROOT / "Cargo.toml"),
             "--locked", "--offline", "--no-deps", "--format-version", "1"],
            text=True, timeout=60,
        ))
        packages = metadata["packages"]
        self.assertTrue(packages)
        self.assertEqual(
            {Path(package["manifest_path"]) for package in packages},
            set(ROOT.glob("crates/*/a4n-*/Cargo.toml")),
        )
        self.assertEqual(list(ROOT.glob("crates/*/Cargo.toml")), [])
        names = {package["name"] for package in packages}
        for package in packages:
            manifest = Path(package["manifest_path"])
            self.assertEqual(manifest.parent.name, package["name"])
            self.assertEqual(len(manifest.relative_to(ROOT).parts), 4)
            data = tomllib.loads(manifest.read_text())
            self.assertNotIn("workspace", data)
            for table in [data, *data.get("target", {}).values()]:
                for kind in ("dependencies", "dev-dependencies", "build-dependencies"):
                    for name, spec in table.get(kind, {}).items():
                        if name in names:
                            with self.subTest(package=package["name"], dependency=name):
                                self.assertTrue(spec.get("workspace"))
                                self.assertNotIn("path", spec)

    def package_dir(self, root, package):
        return subprocess.run(
            ["bash", "-c",
             'source "$1/scripts/lib/common.sh"; PROJECT_ROOT="$2"; cargo_package_dir "$3"',
             "crate-layout-test", str(ROOT), str(root), package],
            text=True, capture_output=True, timeout=10,
        )

    def test_package_lookup_across_domains(self):
        for group, package in [
            ("host", "a4n-runtime"),
            ("llm", "a4n-llm-client"),
            ("process", "a4n-service-protocol"),
        ]:
            result = self.package_dir(ROOT, package)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout.strip(), str(ROOT / "crates" / group / package))

    def test_package_lookup_rejects_missing_ambiguous_and_invalid_names(self):
        with tempfile.TemporaryDirectory(prefix=".crate-layout-", dir=ROOT) as directory:
            root = Path(directory)
            self.assertNotEqual(self.package_dir(root, "a4n-missing").returncode, 0)
            self.assertNotEqual(self.package_dir(root, "../a4n-runtime").returncode, 0)
            for group in ("first", "second"):
                manifest = root / "crates" / group / "a4n-duplicate" / "Cargo.toml"
                manifest.parent.mkdir(parents=True)
                manifest.write_text('[package]\nname = "a4n-duplicate"\n')
            self.assertNotEqual(self.package_dir(root, "a4n-duplicate").returncode, 0)


if __name__ == "__main__":
    unittest.main()
