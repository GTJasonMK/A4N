#!/usr/bin/env python3
"""Migration and server startup must retain the same Cargo dependency graph."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


class RunCargoTargetsTest(unittest.TestCase):
    def test_tauri_runner_preserves_feature_and_target_arguments(self) -> None:
        with tempfile.TemporaryDirectory(prefix="a4n-tauri-build-graph-") as directory:
            temp = Path(directory)
            binary = temp / "cargo"
            binary.write_text("""#!/usr/bin/env python3
import json, os, subprocess, sys
if sys.argv[1] == 'metadata':
    sys.exit(subprocess.call([os.environ['A4N_TEST_REAL_CARGO'], *sys.argv[1:]]))
with open(os.environ['A4N_TEST_BUILD_LOG'], 'w') as stream:
    json.dump({'args': sys.argv[1:], 'target': os.environ['CARGO_TARGET_DIR']}, stream)
""")
            binary.chmod(0o755)
            log = temp / "command.json"
            target = temp / "explicit target"
            env = os.environ.copy()
            env.update(
                A4N_TEST_REAL_CARGO=shutil.which("cargo"), PATH=f"{temp}:{env['PATH']}",
                A4N_TEST_BUILD_LOG=str(log), CARGO_TARGET_DIR=str(temp / "unused-target"),
                A4N_DEV_ENTRY_BINARY="a4n-tauri", A4N_DEV_BUILD_MODE="local-server",
                A4N_DEV_BUILD_INCLUDE_CLI="0", A4N_DEV_BUILD_MANAGED_SIDECARS="1",
                A4N_DEV_BUILD_FULL_HOST="0", A4N_DEV_BUILD_ONLY="1",
            )
            result = subprocess.run(
                [str(ROOT / "scripts/dev/run-cargo-targets.sh"), "run",
                 "--no-default-features", "--features", "migration", "--color", "always",
                 "--target-dir", str(target), "--", "application-argument"],
                cwd=ROOT / "frontend", env=env, text=True, capture_output=True, timeout=10,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            command = json.loads(log.read_text())
            self.assertEqual(command["target"], str(target))
            self.assertIn(str(ROOT / "Cargo.toml"), command["args"])
            self.assertIn("a4n-tauri/migration", command["args"])
            self.assertIn("a4n-server/default", command["args"])
            self.assertIn("--no-default-features", command["args"])
            self.assertNotIn("application-argument", command["args"])

    def test_migration_server_and_cli_use_one_build_command(self) -> None:
        for arguments in [[], ["--release"], ["--features", "a4n-server/embedded-capabilities"]]:
            with self.subTest(arguments=arguments), tempfile.TemporaryDirectory(prefix="a4n-startup-graph-") as directory:
                temp = Path(directory)
                binary = temp / "cargo"
                binary.write_text("""#!/usr/bin/env python3
import json, os, sys
with open(os.environ['A4N_TEST_BUILD_LOG'], 'a') as stream:
    stream.write(json.dumps({'args': sys.argv[1:], 'target': os.environ['CARGO_TARGET_DIR']}) + '\\n')
""")
                binary.chmod(0o755)
                log = temp / "commands.jsonl"
                env = os.environ.copy()
                env.update(
                    PATH=f"{temp}:{env['PATH']}", CARGO_TARGET_DIR=str(temp / "target"),
                    A4N_TEST_BUILD_LOG=str(log), A4N_DEV_BUILD_ONLY="1",
                    A4N_DEV_BUILD_MODE="local-server", A4N_DEV_BUILD_INCLUDE_CLI="1",
                    A4N_DEV_BUILD_MANAGED_SIDECARS="1", A4N_DEV_BUILD_FULL_HOST="0",
                )
                for entry in ["a4n-migration", "a4n-server", "a4n"]:
                    env["A4N_DEV_ENTRY_BINARY"] = entry
                    result = subprocess.run(
                        [str(ROOT / "scripts/dev/run-cargo-targets.sh"), *arguments],
                        cwd=ROOT, env=env, text=True, capture_output=True, timeout=10,
                    )
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                commands = [json.loads(line) for line in log.read_text().splitlines()]
                self.assertEqual(len(commands), 3)
                self.assertEqual(commands[0], commands[1])
                self.assertEqual(commands[1], commands[2])
                self.assertIn(str(ROOT / "Cargo.toml"), commands[0]["args"])
                self.assertEqual(commands[0]["target"], str(temp / "target"))


if __name__ == "__main__":
    unittest.main()
