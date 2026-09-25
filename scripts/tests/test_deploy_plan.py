#!/usr/bin/env python3
"""Verify deploy-plan compares application inputs with the deployed source snapshot."""

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
        self.state.write_text("{}\n", encoding="utf-8")
        self.record_deployment(self.plan("--mode", "full"))

    def tearDown(self) -> None:
        self.temp_dir.cleanup()

    def run_git(self, *args: str) -> None:
        subprocess.run(["git", *args], cwd=self.root, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)

    def record_deployment(self, plan: dict) -> None:
        plan_path = self.root / "plan.json"
        plan_path.write_text(json.dumps(plan), encoding="utf-8")
        subprocess.run(
            ["ruby", str(PLAN), "record", "--plan", str(plan_path), "--output", str(self.state),
             "--release-id", "previous", "--status", "active"],
            check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        )

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
        self.assertIn("bin/a4n-agent-worker", plan["artifact_paths"])
        self.assertNotIn("bin/services/a4n-agent-worker", plan["artifact_paths"])
        self.assertNotIn("a4n-control", plan["build_packages"])

    def test_shared_worker_contract_change_is_full(self) -> None:
        contract = self.root / "crates/process/a4n-worker-contract/src/lib.rs"
        contract.write_text("pub fn fixture() { println!(\"contract\"); }\n", encoding="utf-8")

        plan = self.plan()

        self.assertEqual(plan["mode"], "full")
        self.assertIn("full-restart package or protocol", plan["fallback_reason"])

    def test_committed_worker_change_is_delta_from_the_deployed_content(self) -> None:
        source = self.root / "crates/agent/a4n-agent-worker/src/lib.rs"
        source.write_text('pub fn fixture() { println!("committed"); }\n', encoding="utf-8")
        self.run_git("add", ".")
        self.run_git("commit", "-qm", "change worker")

        plan = self.plan()

        self.assertEqual(plan["mode"], "delta")
        self.assertEqual(plan["build_packages"], ["a4n-agent-worker"])
        self.assertFalse(plan["dirty"])

    def test_deployed_dirty_content_can_be_committed_or_reverted(self) -> None:
        source = self.root / "crates/agent/a4n-agent-worker/src/lib.rs"
        original = source.read_text(encoding="utf-8")
        source.write_text('pub fn fixture() { println!("deployed dirty"); }\n', encoding="utf-8")
        deployed = self.plan()
        self.record_deployment(deployed)

        self.assertEqual(self.plan()["mode"], "noop")
        self.assertTrue(self.plan()["dirty"])
        self.run_git("add", ".")
        self.run_git("commit", "-qm", "commit already deployed content")
        self.assertEqual(self.plan()["mode"], "noop")

        source.write_text(original, encoding="utf-8")
        plan = self.plan()
        self.assertEqual(plan["mode"], "delta")
        self.assertEqual(plan["build_packages"], ["a4n-agent-worker"])

    def test_untracked_and_deleted_paths_are_compared_without_splitting_names(self) -> None:
        source = self.root / "crates/agent/a4n-agent-worker/src/extra space\nfile.rs"
        source.write_text("pub fn extra() {}\n", encoding="utf-8")
        plan = self.plan()
        self.assertEqual(plan["build_packages"], ["a4n-agent-worker"])
        self.assertIn(str(source.relative_to(self.root)), plan["changed_paths"])
        self.record_deployment(plan)
        self.assertEqual(self.plan()["mode"], "noop")

        source.unlink()
        plan = self.plan()
        self.assertEqual(plan["mode"], "delta")
        self.assertIn(str(source.relative_to(self.root)), plan["changed_paths"])

    def test_committed_frontend_change_only_rebuilds_static_payload(self) -> None:
        source = self.root / "frontend/src/main.ts"
        source.parent.mkdir(parents=True)
        source.write_text('console.log("frontend");\n', encoding="utf-8")
        self.run_git("add", ".")
        self.run_git("commit", "-qm", "add frontend")

        plan = self.plan()
        self.assertEqual(plan["mode"], "delta")
        self.assertEqual(plan["build_packages"], [])
        self.assertEqual(plan["payloads"], ["static"])

    def test_documentation_and_test_changes_do_not_force_deployment(self) -> None:
        for relative in ("README.md", "docs/guide.md", "frontend/e2e/smoke.mjs", "scripts/tests/fixture.py"):
            with self.subTest(relative=relative):
                source = self.root / relative
                source.parent.mkdir(parents=True, exist_ok=True)
                source.write_text("fixture\n", encoding="utf-8")

                plan = self.plan()

                self.assertEqual(plan["mode"], "noop")
                self.assertNotIn(relative, plan["changed_paths"])

    def test_unknown_inputs_are_recorded_even_outside_the_old_fingerprint_scope(self) -> None:
        source = self.root / "frontend/vite.config.ts"
        source.parent.mkdir(parents=True)
        source.write_text("export default {};\n", encoding="utf-8")
        plan = self.plan()
        self.assertEqual(plan["mode"], "full")
        self.record_deployment(plan)
        self.assertEqual(self.plan()["mode"], "noop")

        source.write_text('export default { base: "/changed/" };\n', encoding="utf-8")
        self.run_git("add", ".")
        self.run_git("commit", "-qm", "change build input")
        self.assertEqual(self.plan()["mode"], "full")

    def test_old_deployment_state_requires_one_full_build(self) -> None:
        self.state.write_text(json.dumps({"release_id": "previous"}), encoding="utf-8")
        plan = self.plan()
        self.assertEqual(plan["mode"], "full")
        self.assertIn("source snapshot", plan["fallback_reason"])
        self.record_deployment(plan)
        self.assertEqual(self.plan()["mode"], "noop")

    def test_invalid_source_snapshot_fails_explicitly(self) -> None:
        state = json.loads(self.state.read_text(encoding="utf-8"))
        for snapshot in ({"Cargo.toml": "invalid-digest"}, {"../outside": "0" * 64}, []):
            with self.subTest(snapshot=snapshot):
                state["source_files"] = snapshot
                self.state.write_text(json.dumps(state), encoding="utf-8")
                with self.assertRaises(subprocess.CalledProcessError) as raised:
                    self.plan()
                self.assertIn(b"source snapshot", raised.exception.stderr)

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
