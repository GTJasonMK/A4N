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
        self.assertIn("-p a4n-control", commands[0])

    def test_blueprint_worker_change_builds_only_blueprint_worker(self) -> None:
        commands = self.run_watcher(ROOT / "crates/a4n-blueprint-worker/src/lib.rs")
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-blueprint-worker", commands[0])
        self.assertNotIn("-p a4n-control", commands[0])
        self.assertNotIn("-p a4n-outline-worker", commands[0])

    def test_blueprint_contract_change_builds_control_and_worker(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-blueprint-worker-contract/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertIn("-p a4n-blueprint-worker", commands[0])
        self.assertNotIn("-p a4n-agent-worker", commands[0])

    def test_blueprint_client_change_builds_only_control(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-blueprint-worker-client/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertNotIn("-p a4n-blueprint-worker ", commands[0])
        self.assertNotIn("-p a4n-agent-worker", commands[0])

    def test_asset_graph_worker_change_builds_only_asset_graph_worker(self) -> None:
        commands = self.run_watcher(ROOT / "crates/a4n-asset-graph-worker/src/lib.rs")
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-asset-graph-worker", commands[0])
        self.assertNotIn("-p a4n-control", commands[0])
        self.assertNotIn("-p a4n-blueprint-worker", commands[0])

    def test_asset_graph_contract_change_builds_control_and_worker(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-asset-graph-worker-contract/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertIn("-p a4n-asset-graph-worker", commands[0])
        self.assertNotIn("-p a4n-agent-worker", commands[0])

    def test_asset_graph_client_change_builds_only_control(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-asset-graph-worker-client/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertNotIn("-p a4n-asset-graph-worker ", commands[0])
        self.assertNotIn("a4n-agent-worker", commands[0])

    def test_foreshadowing_worker_change_builds_only_foreshadowing_worker(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-foreshadowing-worker/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-foreshadowing-worker", commands[0])
        self.assertNotIn("-p a4n-control", commands[0])
        self.assertNotIn("-p a4n-asset-graph-worker", commands[0])

    def test_foreshadowing_contract_change_builds_control_and_worker(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-foreshadowing-worker-contract/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertIn("-p a4n-foreshadowing-worker", commands[0])
        self.assertNotIn("a4n-agent-worker", commands[0])

    def test_foreshadowing_client_change_builds_only_control(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-foreshadowing-worker-client/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertNotIn("-p a4n-foreshadowing-worker ", commands[0])
        self.assertNotIn("a4n-agent-worker", commands[0])

    def test_object_profile_worker_change_builds_only_object_profile_worker(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-object-profile-worker/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-object-profile-worker", commands[0])
        self.assertNotIn("-p a4n-control", commands[0])
        self.assertNotIn("-p a4n-scene-profile-worker", commands[0])

    def test_object_profile_contract_change_builds_control_and_worker(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-object-profile-worker-contract/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertIn("-p a4n-object-profile-worker", commands[0])
        self.assertNotIn("a4n-agent-worker", commands[0])

    def test_object_profile_client_change_builds_only_control(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-object-profile-worker-client/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertNotIn("-p a4n-object-profile-worker ", commands[0])

    def test_scene_profile_worker_change_builds_only_scene_profile_worker(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-scene-profile-worker/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-scene-profile-worker", commands[0])
        self.assertNotIn("-p a4n-control", commands[0])
        self.assertNotIn("-p a4n-object-profile-worker", commands[0])

    def test_scene_profile_contract_change_builds_control_and_worker(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-scene-profile-worker-contract/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertIn("-p a4n-scene-profile-worker", commands[0])
        self.assertNotIn("a4n-agent-worker", commands[0])

    def test_scene_profile_client_change_builds_only_control(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-scene-profile-worker-client/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertNotIn("-p a4n-scene-profile-worker ", commands[0])

    def test_portrait_worker_change_builds_only_portrait_worker(self) -> None:
        commands = self.run_watcher(ROOT / "crates/a4n-portrait-worker/src/lib.rs")
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-portrait-worker", commands[0])
        self.assertNotIn("-p a4n-control", commands[0])
        self.assertNotIn("-p a4n-scene-profile-worker", commands[0])

    def test_portrait_contract_change_builds_control_and_worker(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-portrait-worker-contract/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertIn("-p a4n-portrait-worker", commands[0])
        self.assertNotIn("a4n-agent-worker", commands[0])

    def test_portrait_client_change_builds_only_control(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-portrait-worker-client/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertNotIn("-p a4n-portrait-worker ", commands[0])

    def test_source_snapshot_worker_change_builds_only_source_snapshot_worker(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-source-snapshot-worker/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-source-snapshot-worker", commands[0])
        self.assertNotIn("-p a4n-control", commands[0])
        self.assertNotIn("-p a4n-portrait-worker", commands[0])

    def test_source_snapshot_contract_change_builds_control_and_worker(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-source-snapshot-worker-contract/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertIn("-p a4n-source-snapshot-worker", commands[0])
        self.assertNotIn("a4n-agent-worker", commands[0])

    def test_source_snapshot_client_change_builds_only_control(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-source-snapshot-worker-client/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertNotIn("-p a4n-source-snapshot-worker ", commands[0])

    def test_theme_read_worker_change_builds_only_theme_read_worker(self) -> None:
        commands = self.run_watcher(ROOT / "crates/a4n-theme-read-worker/src/lib.rs")
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-theme-read-worker", commands[0])
        self.assertNotIn("-p a4n-control", commands[0])

    def test_theme_read_contract_change_builds_control_and_worker(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-theme-read-worker-contract/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertIn("-p a4n-theme-read-worker", commands[0])

    def test_theme_read_client_change_builds_only_control(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-theme-read-worker-client/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertNotIn("-p a4n-theme-read-worker ", commands[0])

    def test_project_read_worker_change_builds_only_project_read_worker(self) -> None:
        commands = self.run_watcher(ROOT / "crates/a4n-project-read-worker/src/lib.rs")
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-project-read-worker", commands[0])
        self.assertNotIn("-p a4n-control", commands[0])

    def test_project_read_contract_change_builds_control_and_worker(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-project-read-worker-contract/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertIn("-p a4n-project-read-worker", commands[0])

    def test_project_read_client_change_builds_only_control(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-project-read-worker-client/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertNotIn("-p a4n-project-read-worker ", commands[0])

    def test_import_review_worker_change_builds_only_import_review_worker(self) -> None:
        commands = self.run_watcher(ROOT / "crates/a4n-import-review-worker/src/lib.rs")
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-import-review-worker", commands[0])
        self.assertNotIn("-p a4n-control", commands[0])

    def test_import_review_contract_change_builds_control_and_worker(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-import-review-worker-contract/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertIn("-p a4n-import-review-worker", commands[0])

    def test_import_review_client_change_builds_only_control(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-import-review-worker-client/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertNotIn("-p a4n-import-review-worker ", commands[0])

    def test_inspiration_worker_change_builds_only_inspiration_worker(self) -> None:
        commands = self.run_watcher(ROOT / "crates/a4n-inspiration-worker/src/lib.rs")
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-inspiration-worker", commands[0])
        self.assertNotIn("-p a4n-control", commands[0])

    def test_inspiration_contract_change_builds_control_and_worker(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-inspiration-worker-contract/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertIn("-p a4n-inspiration-worker", commands[0])

    def test_inspiration_client_change_builds_only_control(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-inspiration-worker-client/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertNotIn("-p a4n-inspiration-worker ", commands[0])

    def test_project_access_worker_change_builds_only_project_access_worker(self) -> None:
        commands = self.run_watcher(ROOT / "crates/a4n-project-access-worker/src/lib.rs")
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-project-access-worker", commands[0])
        self.assertNotIn("-p a4n-control", commands[0])

    def test_project_access_contract_change_builds_control_and_worker(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-project-access-worker-contract/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertIn("-p a4n-project-access-worker", commands[0])

    def test_project_access_client_change_builds_only_control(self) -> None:
        commands = self.run_watcher(
            ROOT / "crates/a4n-project-access-worker-client/src/lib.rs"
        )
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        self.assertNotIn("-p a4n-project-access-worker ", commands[0])

    def test_shared_crate_change_builds_all_runtime_workers(self) -> None:
        commands = self.run_watcher(ROOT / "crates/a4n-api-contract/src/lib.rs")
        self.assertEqual(len(commands), 1)
        self.assertIn("-p a4n-control", commands[0])
        for package in (
            "a4n-agent-worker",
            "a4n-novel-planning-worker",
            "a4n-artifact-media-worker",
            "a4n-outline-worker",
            "a4n-blueprint-worker",
            "a4n-asset-graph-worker",
            "a4n-foreshadowing-worker",
            "a4n-object-profile-worker",
            "a4n-scene-profile-worker",
            "a4n-portrait-worker",
            "a4n-source-snapshot-worker",
            "a4n-theme-read-worker",
            "a4n-project-read-worker",
            "a4n-import-review-worker",
            "a4n-inspiration-worker",
            "a4n-project-access-worker",
        ):
            self.assertIn(f"-p {package}", commands[0])


if __name__ == "__main__":
    unittest.main()
