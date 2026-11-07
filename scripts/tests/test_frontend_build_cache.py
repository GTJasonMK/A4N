#!/usr/bin/env python3
"""Exercise frontend cache hits and invalidation through the real build entry."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
BUILDER = ROOT / "scripts/dev/build-frontend.mjs"


class FrontendBuildCacheTest(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory(prefix="a4n-frontend-cache-")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.builder = self.root / "scripts/dev/build-frontend.mjs"
        self.builder.parent.mkdir(parents=True)
        shutil.copy2(BUILDER, self.builder)
        self.frontend = self.root / "frontend"
        for name in ["src", "public", "resources", "node_modules"]:
            (self.frontend / name).mkdir(parents=True)
        (self.frontend / "src/main.ts").write_text("console.log('source');\n")
        (self.frontend / "package.json").write_text('{"scripts":{"build":"fixture"}}\n')
        (self.frontend / "package-lock.json").write_text('{"lockfileVersion":3}\n')
        bin_dir = self.root / "bin"
        bin_dir.mkdir()
        npm = bin_dir / "npm"
        npm.write_text("""#!/usr/bin/env python3
import os, pathlib, sys
assert sys.argv[1:] == ['run', 'build'], sys.argv
assert pathlib.Path.cwd().name == 'frontend'
log = pathlib.Path(os.environ['A4N_TEST_BUILD_LOG'])
with log.open('a') as stream: stream.write('build\\n')
dist = pathlib.Path('dist')
dist.mkdir(exist_ok=True)
(dist / 'app.js').write_text('built output')
if os.environ.get('A4N_TEST_BUILD_FAILURE') == '1': sys.exit(7)
if os.environ.get('A4N_TEST_MISSING_INDEX') != '1':
    (dist / 'index.html').write_text('<script src="app.js"></script>')
if os.environ.get('A4N_TEST_CHANGE_SOURCE') == '1':
    pathlib.Path('src/main.ts').write_text('changed during build')
""")
        npm.chmod(0o755)
        self.log = self.root / "build.log"
        self.env = {
            key: value for key, value in os.environ.items()
            if not key.startswith(("VITE_", "TAURI_")) and key != "NODE_ENV"
        }
        self.env.update(PATH=f"{bin_dir}:{self.env['PATH']}", A4N_TEST_BUILD_LOG=str(self.log))

    def build(self, *, success: bool = True) -> subprocess.CompletedProcess[str]:
        result = subprocess.run(
            ["node", str(self.builder)], cwd=self.root, env=self.env,
            text=True, capture_output=True, timeout=10,
        )
        if success:
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        else:
            self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        return result

    def build_count(self) -> int:
        return len(self.log.read_text().splitlines()) if self.log.exists() else 0

    def test_unchanged_inputs_reuse_build_even_if_source_timestamp_changes(self) -> None:
        self.build()
        source = self.frontend / "src/main.ts"
        os.utime(source, (source.stat().st_atime, source.stat().st_mtime + 10))
        self.build()
        self.assertEqual(self.build_count(), 1)

    def test_source_content_change_rebuilds_even_with_the_same_timestamp(self) -> None:
        self.build()
        source = self.frontend / "src/main.ts"
        timestamp = source.stat()
        source.write_text("console.log('update');\n")
        os.utime(source, (timestamp.st_atime, timestamp.st_mtime))
        self.build()
        self.assertEqual(self.build_count(), 2)

    def test_public_resources_config_and_env_files_invalidate_build(self) -> None:
        self.build()
        for count, name in enumerate([
            "public/icon.svg", "resources/template.json", "vite.config.ts",
            ".env.production.local", "package-lock.json", "node_modules/.package-lock.json",
        ], start=2):
            with self.subTest(name=name):
                (self.frontend / name).write_text("changed build input\n")
                self.build()
                self.assertEqual(self.build_count(), count)

    def test_deleted_source_invalidates_build(self) -> None:
        self.build()
        (self.frontend / "src/main.ts").unlink()
        self.build()
        self.assertEqual(self.build_count(), 2)

    def test_build_environment_changes_invalidate_but_runtime_settings_do_not(self) -> None:
        self.build()
        self.env["A4N_DATABASE_URL"] = "postgres://runtime-only.invalid/a4n"
        self.build()
        self.assertEqual(self.build_count(), 1)
        for count, (key, value) in enumerate([
            ("VITE_A4N_BASE_PATH", "/changed"),
            ("VITE_A4N_API_BASE_URL", "/changed/api"),
            ("TAURI_ENV_PLATFORM", "linux"),
            ("NODE_ENV", "development"),
        ], start=2):
            self.env[key] = value
            self.build()
            self.assertEqual(self.build_count(), count)

    def test_documentation_and_e2e_changes_do_not_rebuild(self) -> None:
        self.build()
        (self.frontend / "README.md").write_text("docs")
        (self.frontend / "UI_PAGE_AUDIT.md").write_text("audit")
        (self.frontend / "e2e").mkdir()
        (self.frontend / "e2e/test.mjs").write_text("test")
        self.build()
        self.assertEqual(self.build_count(), 1)

    def test_missing_or_changed_output_rebuilds(self) -> None:
        self.build()
        (self.frontend / "dist/app.js").unlink()
        self.build()
        (self.frontend / "dist/app.js").write_text("another build overwrote the output")
        self.build()
        (self.frontend / "dist/index.html").unlink()
        self.build()
        self.assertEqual(self.build_count(), 4)

    def test_failed_build_cannot_reuse_an_older_success_receipt(self) -> None:
        self.build()
        source = self.frontend / "src/main.ts"
        original = source.read_text()
        source.write_text("changed source")
        self.env["A4N_TEST_BUILD_FAILURE"] = "1"
        self.assertEqual(self.build(success=False).returncode, 7)
        source.write_text(original)
        del self.env["A4N_TEST_BUILD_FAILURE"]
        self.build()
        self.assertEqual(self.build_count(), 3)

    def test_success_without_index_is_a_failure(self) -> None:
        self.env["A4N_TEST_MISSING_INDEX"] = "1"
        self.build(success=False)
        del self.env["A4N_TEST_MISSING_INDEX"]
        self.build()
        self.assertEqual(self.build_count(), 2)

    def test_source_changed_during_build_is_not_cached_as_success(self) -> None:
        self.env["A4N_TEST_CHANGE_SOURCE"] = "1"
        self.build(success=False)
        del self.env["A4N_TEST_CHANGE_SOURCE"]
        self.build()
        self.assertEqual(self.build_count(), 2)

    def test_corrupt_receipts_rebuild_and_cannot_accept_missing_output(self) -> None:
        self.build()
        receipt = self.frontend / "node_modules/.cache/a4n/frontend-build.json"
        receipt.write_text("{invalid json")
        result = self.build()
        self.assertIn("缓存记录损坏", result.stderr)
        invalid = json.loads(receipt.read_text())
        invalid["output"] = None
        receipt.write_text(json.dumps(invalid))
        (self.frontend / "dist/index.html").unlink()
        self.build()
        self.assertEqual(self.build_count(), 3)

    def test_build_recipe_changes_invalidate_the_receipt(self) -> None:
        self.build()
        with self.builder.open("a") as stream:
            stream.write("\n// changed build recipe\n")
        self.build()
        self.assertEqual(self.build_count(), 2)


if __name__ == "__main__":
    unittest.main()
