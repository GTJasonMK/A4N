#!/usr/bin/env python3
"""Keep the watcher on startup's feature graph without forcing recompilation."""

from __future__ import annotations

import json
import os
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
WATCHER = ROOT / "scripts/dev/watch-runtime-workers.sh"
BUILDER = ROOT / "scripts/dev/run-cargo-targets.sh"


class RuntimeWorkerWatcherTest(unittest.TestCase):
    def run_watcher(
        self, changed_paths: list[Path], *, args: tuple[str, ...] = (),
        managed_sidecars: str = "1", fail_build: bool = False,
    ) -> tuple[list[dict], dict, bool, str]:
        with tempfile.TemporaryDirectory(prefix="a4n-worker-watcher-") as directory:
            temp = Path(directory)
            fake_bin = temp / "bin"
            fake_bin.mkdir()
            cargo_log = temp / "cargo.log"

            for name, body in {
                "cargo": """#!/usr/bin/env python3
import json, os, sys
with open(os.environ['A4N_TEST_CARGO_LOG'], 'a') as stream:
    stream.write(json.dumps({'args': sys.argv[1:], 'incremental': os.environ.get('CARGO_INCREMENTAL'),
                             'debug': os.environ.get('CARGO_PROFILE_DEV_DEBUG')}) + '\\n')
sys.exit(1 if os.environ.get('A4N_TEST_BUILD_FAILURE') == '1' else 0)
""",
                "inotifywait": "#!/usr/bin/env bash\nprintf '%s\\n' \"$A4N_TEST_CHANGED_PATHS\"\n",
            }.items():
                executable = fake_bin / name
                executable.write_text(body, encoding="utf-8")
                executable.chmod(executable.stat().st_mode | stat.S_IXUSR)

            environment = os.environ.copy()
            environment.update(
                {
                    "PATH": f"{fake_bin}:{environment['PATH']}",
                    "CARGO_TARGET_DIR": str(temp / "target"),
                    "A4N_TEST_CARGO_LOG": str(cargo_log),
                    "A4N_TEST_CHANGED_PATHS": "\n".join(map(str, changed_paths)),
                    "A4N_RUNTIME_WORKER_RELOAD_MARKER": str(temp / "reload.ready"),
                    "A4N_DEV_ENTRY_BINARY": "a4n-server",
                    "A4N_DEV_BUILD_INCLUDE_CLI": "1",
                    "A4N_DEV_BUILD_MANAGED_SIDECARS": managed_sidecars,
                    "A4N_DEV_BUILD_FULL_HOST": "1" if "--full" in args else "0",
                    "A4N_DEV_BUILD_ONLY": "1",
                    "CARGO_INCREMENTAL": "1",
                    "CARGO_PROFILE_DEV_DEBUG": "2",
                }
            )
            startup_args = ["--release"] if "--release" in args else []
            subprocess.run(
                [str(BUILDER), *startup_args], cwd=ROOT, env=environment,
                check=True, text=True, capture_output=True, timeout=10,
            )
            startup = json.loads(cargo_log.read_text().strip())
            cargo_log.unlink()
            environment["A4N_TEST_BUILD_FAILURE"] = "1" if fail_build else "0"
            result = subprocess.run(
                [str(WATCHER), *args],
                cwd=ROOT,
                env=environment,
                check=True,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=10,
            )
            commands = [json.loads(line) for line in cargo_log.read_text().splitlines()] if cargo_log.exists() else []
            return commands, startup, (temp / "reload.ready").exists(), result.stdout + result.stderr

    def test_runtime_changes_keep_the_complete_startup_feature_graph(self) -> None:
        for name in [
            "host/a4n-control", "host/a4n-edge", "agent/a4n-agent-worker",
            "image/a4n-image-service", "agent/a4n-agent-worker-contract",
            "agent/a4n-agent-worker-client", "platform/a4n-domain",
        ]:
            with self.subTest(name=name):
                commands, startup, reloaded, _ = self.run_watcher([ROOT / f"crates/{name}/src/lib.rs"])
                self.assertEqual(commands, [startup])
                self.assertIn("a4n-control", startup["args"])
                self.assertIn("a4n-edge", startup["args"])
                self.assertIn("a4n-image-service", startup["args"])
                self.assertTrue(reloaded)

    def test_full_release_and_external_sidecar_modes_match_startup(self) -> None:
        for args, sidecars in [(('--full',), '1'), (('--release',), '1'), ((), '0')]:
            with self.subTest(args=args, sidecars=sidecars):
                commands, startup, reloaded, _ = self.run_watcher(
                    [ROOT / "crates/platform/a4n-domain/src/lib.rs"], args=args,
                    managed_sidecars=sidecars,
                )
                self.assertEqual(commands, [startup])
                self.assertTrue(reloaded)

    def test_multiple_source_events_are_coalesced_into_one_build(self) -> None:
        commands, startup, reloaded, _ = self.run_watcher([
            ROOT / "crates/host/a4n-control/src/lib.rs",
            ROOT / "crates/agent/a4n-agent-worker/src/lib.rs",
            ROOT / "Cargo.lock",
        ])
        self.assertEqual(commands, [startup])
        self.assertTrue(reloaded)

    def test_documentation_and_entry_sources_do_not_trigger_runtime_watcher(self) -> None:
        commands, _, reloaded, _ = self.run_watcher([
            ROOT / "README.md", ROOT / "crates/host/a4n-tauri/src/main.rs",
            ROOT / "crates/host/a4n-server/src/main.rs",
        ])
        self.assertEqual(commands, [])
        self.assertFalse(reloaded)

    def test_failed_build_does_not_publish_a_reload_marker(self) -> None:
        commands, startup, reloaded, output = self.run_watcher(
            [ROOT / "crates/host/a4n-control/src/lib.rs"], fail_build=True,
        )
        self.assertEqual(commands, [startup])
        self.assertFalse(reloaded)
        self.assertIn("编译失败", output)


if __name__ == "__main__":
    unittest.main()
