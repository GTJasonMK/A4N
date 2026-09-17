#!/usr/bin/env python3
"""Verify the Runtime worker watcher selects only affected packages."""

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
        commands = self.run_watcher(ROOT / "crates/a4n-control/src/lib.rs")
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertNotIn("a4n-agent-worker", commands[0])

    def test_agent_worker_change_builds_only_agent_worker(self) -> None:
        commands = self.run_watcher(ROOT / "crates/a4n-agent-worker/src/main.rs")
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-agent-worker", commands[0])
        self.assertNotIn("a4n-control", commands[0])

    def test_runtime_change_builds_only_control(self) -> None:
        commands = self.run_watcher(ROOT / "crates/a4n-runtime/src/lib.rs")
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertNotIn("a4n-agent-worker", commands[0])

    def test_agent_runtime_change_builds_only_agent_worker(self) -> None:
        commands = self.run_watcher(ROOT / "crates/a4n-agent-runtime/src/lib.rs")
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-agent-worker", commands[0])
        self.assertNotIn("a4n-control", commands[0])

    def test_edge_change_builds_only_edge(self) -> None:
        commands = self.run_watcher(ROOT / "crates/a4n-edge/src/lib.rs")
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-edge", commands[0])
        self.assertNotIn("a4n-control", commands[0])
        self.assertNotIn("a4n-agent-worker", commands[0])

    def test_novel_planning_worker_change_builds_only_novel_planning_worker(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-novel-planning-worker/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-novel-planning-worker", commands[0])
        self.assertNotIn("a4n-control", commands[0])
        self.assertNotIn("a4n-agent-worker", commands[0])
        self.assertNotIn("-p a4n-edge", commands[0])

    def test_novel_planning_contract_change_builds_control_and_worker(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-novel-planning-worker-contract/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertIn("-p a4n-novel-planning-worker", commands[0])
        self.assertNotIn("a4n-agent-worker", commands[0])

    def test_novel_planning_client_change_builds_only_control(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-novel-planning-worker-client/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertNotIn("a4n-novel-planning-worker ", commands[0])
        self.assertNotIn("a4n-agent-worker", commands[0])

    def test_artifact_media_worker_change_builds_only_artifact_media_worker(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-artifact-media-worker/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-artifact-media-worker", commands[0])
        self.assertNotIn("-p a4n-control", commands[0])
        self.assertNotIn("-p a4n-agent-worker", commands[0])

    def test_artifact_media_contract_change_builds_artifact_media_worker(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-artifact-media-contract/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-artifact-media-worker", commands[0])
        self.assertNotIn("-p a4n-control", commands[0])

    def test_shared_crate_change_builds_both_workers(self) -> None:
        commands = self.run_watcher(ROOT / "crates/a4n-api-contract/src/lib.rs")
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertIn("-p a4n-agent-worker", commands[0])


if __name__ == "__main__":
    unittest.main()
