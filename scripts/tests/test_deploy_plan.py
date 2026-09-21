#!/usr/bin/env python3
"""Verify deploy-plan selects a safe release scope from a dirty worktree."""

from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PLAN = ROOT / "scripts/deploy/deploy-plan.rb"


class DeployPlanTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory(prefix="a4n-deploy-plan-")
        self.root = Path(self.temp_dir.name)
        package_dirs = {
            "a4n-agent-worker": self.root / "crates/agent/a4n-agent-worker",
            "a4n-control": self.root / "crates/host/a4n-control",
            "a4n-worker-contract": self.root / "crates/process/a4n-worker-contract",
        }
        for path in [
            *(directory / "src" for directory in package_dirs.values()),
            self.root / "crates/process/a4n-process-catalog",
        ]:
            path.mkdir(parents=True)

        (self.root / "Cargo.toml").write_text(
            '[workspace]\nmembers = ["crates/*/a4n-*"]\n'
            '[workspace.dependencies]\n'
            'a4n-worker-contract = { path = "crates/process/a4n-worker-contract" }\n',
            encoding="utf-8",
        )
        (self.root / "Cargo.lock").write_text("# fixture\n", encoding="utf-8")
        (self.root / ".gitignore").write_text("metadata.json\nstate.json\nplan.json\n", encoding="utf-8")
        for package, package_dir in package_dirs.items():
            dependencies = (
                "\n[dependencies]\na4n-worker-contract = { workspace = true }\n"
                if package in {"a4n-agent-worker", "a4n-control"}
                else ""
            )
            package_dir.joinpath("Cargo.toml").write_text(
                f'[package]\nname = "{package}"\nversion = "0.1.0"\nedition = "2024"\n{dependencies}',
                encoding="utf-8",
            )
            package_dir.joinpath("src/lib.rs").write_text("pub fn fixture() {}\n", encoding="utf-8")

        (self.root / "crates/process/a4n-process-catalog/worker-catalog.json").write_text(
            json.dumps(
                [
                    {
                        "name": "agent-worker",
                        "binary": "a4n-agent-worker",
                        "kind": "stateful_worker",
                        "contract_version": "worker-v1",
                        "capability_version": "agent-v1",
                        "addr_env": "A4N_AGENT_WORKER_ADDR",
                        "default_addr": "127.0.0.1:47650",
                    }
                ]
            ),
            encoding="utf-8",
        )
        (self.root / "crates/process/a4n-process-catalog/service-catalog.json").write_text("[]\n", encoding="utf-8")
        (self.root / "crates/process/a4n-process-catalog/release-tools.json").write_text(
            json.dumps(
                [
                    {"package": "a4n-agent-worker", "binary": "a4n-agent-worker"},
                    {"package": "a4n-control", "binary": "a4n-control"},
                ]
            ),
            encoding="utf-8",
        )

        self.run_git("init", "-q")
        self.run_git("config", "user.email", "test@example.invalid")
        self.run_git("config", "user.name", "Deploy Plan Test")
        self.run_git("add", ".")
        self.run_git("commit", "-qm", "fixture")

        self.metadata = self.root / "metadata.json"
        packages = []
        for package in ["a4n-agent-worker", "a4n-control", "a4n-worker-contract"]:
            dependencies = []
            if package in {"a4n-agent-worker", "a4n-control"}:
                dependencies.append({"name": "a4n-worker-contract"})
            packages.append(
                {
                    "name": package,
                    "manifest_path": str(package_dirs[package] / "Cargo.toml"),
                    "dependencies": dependencies,
                }
            )
        self.metadata.write_text(json.dumps({"packages": packages}), encoding="utf-8")
        self.state = self.root / "state.json"
        self.state.write_text(json.dumps({"release_id": "previous"}), encoding="utf-8")

    def tearDown(self) -> None:
        self.temp_dir.cleanup()

    def run_git(self, *args: str) -> None:
        subprocess.run(["git", *args], cwd=self.root, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)

    def plan(self, *extra: str) -> dict:
        output = self.root / "plan.json"
        subprocess.run(
            [
                "ruby",
                str(PLAN),
                "plan",
                "--root",
                str(self.root),
                "--metadata",
                str(self.metadata),
                "--state",
                str(self.state),
                "--output",
                str(output),
                "--mode",
                "auto",
                "--target",
                "linux-x86_64",
                "--builder",
                "fixture",
                "--cargo-offline",
                "true",
                *extra,
            ],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        return json.loads(output.read_text(encoding="utf-8"))

    def test_worker_change_is_delta_and_does_not_build_control(self) -> None:
        worker_source = self.root / "crates/agent/a4n-agent-worker/src/lib.rs"
        worker_source.write_text("pub fn fixture() { println!(\"changed\"); }\n", encoding="utf-8")

        plan = self.plan()

        self.assertEqual(plan["mode"], "delta")
        self.assertEqual(plan["build_packages"], ["a4n-agent-worker"])
        self.assertIn("bin/services/a4n-agent-worker", plan["artifact_paths"])
        self.assertNotIn("a4n-control", plan["build_packages"])

    def test_shared_worker_contract_change_is_full(self) -> None:
        contract = self.root / "crates/process/a4n-worker-contract/src/lib.rs"
        contract.write_text("pub fn fixture() { println!(\"contract\"); }\n", encoding="utf-8")

        plan = self.plan()

        self.assertEqual(plan["mode"], "full")
        self.assertIn("full-restart package or protocol", plan["fallback_reason"])

    def test_missing_remote_state_seeds_full_release(self) -> None:
        self.state.write_text("{}\n", encoding="utf-8")

        plan = self.plan()

        self.assertEqual(plan["mode"], "full")
        self.assertEqual(plan["fallback_reason"], "remote deployment state is missing")

    def test_public_origin_change_is_full(self) -> None:
        self.state.write_text(
            json.dumps({"release_id": "previous", "public_app_url": "https://old.test/a4n"}),
            encoding="utf-8",
        )

        plan = self.plan("--public-app-url", "https://new.test/a4n")

        self.assertEqual(plan["mode"], "full")
        self.assertEqual(plan["fallback_reason"], "public app URL changed")


if __name__ == "__main__":
    unittest.main()
