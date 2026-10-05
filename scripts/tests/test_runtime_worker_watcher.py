#!/usr/bin/env python3
"""Verify the Runtime watcher rebuilds the shared launcher for role changes."""

from __future__ import annotations

import os
import stat
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
WATCHER = ROOT / "scripts/dev/watch-runtime-workers.sh"


class RuntimeWorkerWatcherTest(unittest.TestCase):
    def run_watcher(self, changed_path: Path) -> list[str]:
        with tempfile.TemporaryDirectory(prefix="a4n-worker-watcher-") as directory:
            temp = Path(directory)
            fake_bin = temp / "bin"
            fake_bin.mkdir()
            cargo_log = temp / "cargo.log"

            for name, body in {
                "cargo": "#!/usr/bin/env bash\nprintf '%s\\n' \"$*\" >> \"$A4N_TEST_CARGO_LOG\"\n",
                "inotifywait": "#!/usr/bin/env bash\nprintf '%s\\n' \"$A4N_TEST_CHANGED_PATH\"\n",
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
                    "A4N_TEST_CHANGED_PATH": str(changed_path),
                    "A4N_RUNTIME_WORKER_RELOAD_MARKER": "",
                }
            )
            subprocess.run(
                [str(WATCHER)],
                cwd=ROOT,
                env=environment,
                check=True,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=10,
            )
            return cargo_log.read_text(encoding="utf-8").splitlines()

    def test_control_change_builds_only_control(self) -> None:
        commands = self.run_watcher(ROOT / "crates/host/a4n-control/src/lib.rs")
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertNotIn("a4n-component-host", commands[0])
        self.assertNotIn("-p a4n-edge", commands[0])

    def test_edge_change_builds_only_edge(self) -> None:
        commands = self.run_watcher(ROOT / "crates/host/a4n-edge/src/lib.rs")
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-edge", commands[0])
        self.assertNotIn("a4n-component-host", commands[0])
        self.assertNotIn("-p a4n-control", commands[0])

    def test_worker_change_builds_the_shared_component_host(self) -> None:
        commands = self.run_watcher(ROOT / "crates/agent/a4n-agent-worker/src/lib.rs")
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-component-host", commands[0])
        self.assertNotIn("-p a4n-agent-worker", commands[0])

    def test_service_change_builds_the_shared_component_host(self) -> None:
        commands = self.run_watcher(ROOT / "crates/image/a4n-image-service/src/lib.rs")
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-component-host", commands[0])
        self.assertNotIn("-p a4n-image-service", commands[0])

    def test_worker_contract_change_builds_control_and_shared_host(self) -> None:
        commands = self.run_watcher(ROOT / "crates/agent/a4n-agent-worker-contract/src/lib.rs")
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertIn("-p a4n-component-host", commands[0])

    def test_worker_client_change_builds_only_control(self) -> None:
        commands = self.run_watcher(ROOT / "crates/agent/a4n-agent-worker-client/src/lib.rs")
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertNotIn("a4n-component-host", commands[0])

    def test_shared_domain_change_builds_control_and_component_host(self) -> None:
        commands = self.run_watcher(ROOT / "crates/platform/a4n-domain/src/lib.rs")
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertIn("-p a4n-component-host", commands[0])


if __name__ == "__main__":
    unittest.main()
