import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import Mock, patch

from run_creation_e2e import (
    BROWSER_FEATURE_SPECS, FULL_GENERATION_SUITES, ROOT, RunnerError, STAGES, acceptance_passed,
    acceptance_result, build_cache_paths, files_fingerprint, frontend_fingerprint,
    full_generation_matrix_report, read_all_workflows_report, read_suite_report,
    selected_browser_specs, snapshot_source,
    selected_suites, source_fingerprint, stop_owned_workers, stop_postgres,
)
from run_browser_full_project_e2e import secure_mkdir, stop_process, tail


class CreationAcceptanceTest(unittest.TestCase):
    def scratch(self):
        parent = ROOT / "data"
        parent.mkdir(exist_ok=True)
        return Path(self.enterContext(tempfile.TemporaryDirectory(prefix="creation-check-", dir=parent)))

    def test_unreadable_evidence_and_unprotected_report_directories_fail_explicitly(self):
        directory = self.scratch()
        with self.assertRaises(FileNotFoundError):
            tail(directory / "missing.log")
        with patch.object(Path, "chmod", side_effect=PermissionError("private directory denied")), self.assertRaises(PermissionError):
            secure_mkdir(directory)

    def test_empty_unknown_and_missing_checks_cannot_pass(self):
        for selection in (set(), {"nonexistent"}):
            with self.assertRaises(RunnerError):
                selected_suites(selection)
        result = acceptance_result(set(STAGES), {}, {})
        self.assertEqual(set(result), set(STAGES))
        self.assertTrue(all(row["status"] == "blocked" for row in result.values()))

    def test_stage_requires_both_business_assertions_and_real_browser_result(self):
        suites = {"authoring": {"success": True, "checks": [{"stage": "inspiration", "name": "flow"}], "stages": {"inspiration": {"status": "passed"}}}}
        browser = {"inspiration": {"status": "passed"}}
        self.assertEqual(acceptance_result({"inspiration"}, suites, browser)["inspiration"]["status"], "passed")
        browser["inspiration"]["status"] = "failed"
        self.assertEqual(acceptance_result({"inspiration"}, suites, browser)["inspiration"]["status"], "failed")
        browser["inspiration"]["status"] = "passed"
        suites["authoring"]["checks"] = []
        self.assertEqual(acceptance_result({"inspiration"}, suites, browser)["inspiration"]["status"], "failed")
        suites["authoring"]["stages"] = {}
        self.assertEqual(acceptance_result({"inspiration"}, suites, browser)["inspiration"]["status"], "failed")

    def test_later_stage_failure_does_not_erase_a_completed_earlier_stage(self):
        suites = {"authoring": {"success": False, "checks": [{"stage": "inspiration", "name": "flow"}], "stages": {"inspiration": {"status": "passed"}, "blueprint": {"status": "failed"}}}}
        browser = {"inspiration": {"status": "passed"}}
        self.assertEqual(acceptance_result({"inspiration"}, suites, browser)["inspiration"]["status"], "passed")
        self.assertNotIn("director", selected_suites({"inspiration"}))

    def test_full_run_selects_every_generation_suite_and_single_stage_does_not(self):
        selected = selected_suites(set(STAGES))
        self.assertTrue(set(FULL_GENERATION_SUITES).issubset(selected))
        self.assertFalse(set(FULL_GENERATION_SUITES) & set(selected_suites({"inspiration"})))
        for script in FULL_GENERATION_SUITES.values():
            self.assertTrue((ROOT / "scripts/tests" / script).is_file(), script)

    def test_browser_feature_flows_extend_the_creation_browser_path(self):
        default = selected_browser_specs(False)
        complete = selected_browser_specs(True)
        self.assertEqual(default, ["./e2e/full-project/generation-stages.e2e.mjs"])
        self.assertEqual(complete, [*default, *BROWSER_FEATURE_SPECS])
        self.assertTrue(all((ROOT / "frontend" / spec.removeprefix("./")).is_file() for spec in complete))

    def test_browser_feature_flow_sources_are_registered_and_have_setup(self):
        source = (ROOT / "scripts/tests/run_creation_e2e.py").read_text()
        self.assertIn("media-authoring-workspaces", source)
        self.assertIn("specialized-agent-workspace", source)
        self.assertIn("shot-sync-workspace", source)
        self.assertIn('A4N_MEDIA_AUTHORING_CLI_REPORT', source)
        self.assertIn('A4N_SPECIALIZED_FIXTURE', source)
        self.assertIn('A4N_SHOT_SYNC_FIXTURE', source)

    def test_full_generation_matrix_fails_if_any_suite_is_missing_or_failed(self):
        passed = {name: {"success": True} for name in FULL_GENERATION_SUITES}
        self.assertEqual(full_generation_matrix_report(passed)["status"], "passed")
        del passed["all_workflows"]
        self.assertIn("all_workflows", full_generation_matrix_report(passed)["failed_suites"])
        passed["all_workflows"] = {"success": False}
        self.assertIn("all_workflows", full_generation_matrix_report(passed)["failed_suites"])

    def test_all_workflows_report_requires_every_declared_fixture_path(self):
        from run_all_workflows_e2e import FIXTURE_PATHS

        path = self.scratch() / "all-workflows.json"
        report = {
            "success": True,
            "mode": "fixture_only",
            "fixtures": {
                "enabled": True,
                "only": True,
                "paths": {
                    "complete": True,
                    "paths": {name: {"status": "passed"} for name in FIXTURE_PATHS},
                },
            },
        }
        path.write_text(json.dumps(report))
        self.assertTrue(read_all_workflows_report(path, 0)["success"])
        paths = report["fixtures"]["paths"]["paths"]
        paths["manga"] = None
        path.write_text(json.dumps(report))
        self.assertFalse(read_all_workflows_report(path, 0)["success"])
        paths["manga"] = {"status": "passed"}
        paths.pop("manga")
        path.write_text(json.dumps(report))
        self.assertFalse(read_all_workflows_report(path, 0)["success"])
        report["fixtures"]["paths"]["paths"] = {name: {"status": "passed"} for name in FIXTURE_PATHS}
        report["fixtures"]["paths"]["paths"]["manga"]["status"] = "failed"
        path.write_text(json.dumps(report))
        self.assertFalse(read_all_workflows_report(path, 0)["success"])
        report["fixtures"]["paths"]["paths"]["manga"]["status"] = "passed"
        report["fixtures"]["paths"]["complete"] = False
        path.write_text(json.dumps(report))
        self.assertFalse(read_all_workflows_report(path, 0)["success"])
        report["fixtures"]["paths"]["complete"] = True
        report["fixtures"]["only"] = False
        path.write_text(json.dumps(report))
        self.assertFalse(read_all_workflows_report(path, 0)["success"])

    def test_suite_exit_failure_cannot_be_hidden_by_completed_stage_rows(self):
        report = {"browser": {"exit_code": 0}, "stages": {"inspiration": {"status": "passed"}}, "suites": {"authoring": {"success": False}}}
        self.assertFalse(acceptance_passed(report))
        report["suites"]["authoring"]["success"] = True
        self.assertTrue(acceptance_passed(report))
        report["browser"]["exit_code"] = 1
        self.assertFalse(acceptance_passed(report))
        self.assertFalse(acceptance_passed({}))

    def test_malformed_and_missing_suite_reports_are_explicit_failures(self):
        path = self.scratch() / "suite.json"
        self.assertFalse(read_suite_report(path, 0)["success"])
        for value in ("{", '[]', '{"success":true,"checks":"not checks"}'):
            path.write_text(value)
            result = read_suite_report(path, 0)
            self.assertFalse(result["success"])
            self.assertIn("error", result)
        path.write_text(json.dumps({"success": True, "checks": ["verified"], "stages": {"inspiration": {"status": "passed"}}}))
        self.assertTrue(read_suite_report(path, 0)["success"])
        failed = read_suite_report(path, 124)
        self.assertFalse(failed["success"])
        self.assertEqual(failed["stages"]["inspiration"]["status"], "passed")

    def test_build_resources_and_version_files_invalidate_reuse(self):
        root = self.scratch()
        for name in ("Cargo.toml", "Cargo.lock", "tools/onnxruntime/VERSION", "scripts/cargo.sh", "scripts/lib/common.sh", "crates/model/prompt.md", ".cargo/config.toml", "frontend/src/main.ts", "frontend/public/icon.svg"):
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("original")
        for name in ("tools/onnxruntime/VERSION", "crates/model/prompt.md", ".cargo/config.toml", "scripts/lib/common.sh"):
            before = source_fingerprint(root)
            (root / name).write_text("changed")
            self.assertNotEqual(source_fingerprint(root), before, name)
        before = frontend_fingerprint(root)
        (root / "frontend/public/icon.svg").write_text("changed")
        self.assertNotEqual(frontend_fingerprint(root), before)
        (root / "a").write_text("bc")
        (root / "ab").write_text("c")
        self.assertNotEqual(files_fingerprint(root, [root / "a"]), files_fingerprint(root, [root / "ab"]))

    def test_application_source_snapshot_includes_checkout_root_files_but_not_environment_files(self):
        root = self.scratch()
        application = root / "application"
        (application / "frontend").mkdir(parents=True)
        (application / "Cargo.toml").write_text("[workspace]\n")
        (application / ".env.production").write_text("secret=value\n")
        (application / "frontend/package.json").write_text("{}\n")

        snapshot = snapshot_source(root / "report", application)

        self.assertEqual((snapshot / "Cargo.toml").read_text(), "[workspace]\n")
        self.assertEqual((snapshot / "frontend/package.json").read_text(), "{}\n")
        self.assertFalse((snapshot / ".env.production").exists())
        self.assertTrue((snapshot / ".env").is_file())

    def test_ci_build_cache_stays_inside_checkout_and_uses_a_private_lock(self):
        target = ROOT / "target" / "creation-e2e"
        with patch.dict("os.environ", {"A4N_E2E_CARGO_TARGET_DIR": str(target)}):
            self.assertEqual(
                build_cache_paths(),
                (target.resolve(), ROOT / "target" / ".creation-e2e-build.lock"),
            )

    def test_build_cache_override_rejects_paths_outside_checkout(self):
        with patch.dict("os.environ", {"A4N_E2E_CARGO_TARGET_DIR": "/tmp/external-target"}):
            with self.assertRaisesRegex(RunnerError, "必须是 checkout/target 下"):
                build_cache_paths()

        with patch.dict("os.environ", {"A4N_E2E_CARGO_TARGET_DIR": str(ROOT / "target")}):
            with self.assertRaisesRegex(RunnerError, "必须是 checkout/target 下"):
                build_cache_paths()

    def test_cargo_target_dir_override_is_preserved(self):
        target = ROOT / "target" / "caller-selected"
        with patch.dict(
            "os.environ",
            {"A4N_E2E_CARGO_TARGET_DIR": "", "CARGO_TARGET_DIR": str(target)},
        ):
            self.assertEqual(build_cache_paths()[0], target.resolve())

    def test_failed_process_shutdown_is_not_swallowed_and_closes_its_log(self):
        process, log = Mock(pid=123), Mock()
        process.poll.return_value = None
        with patch("run_browser_full_project_e2e.os.killpg", side_effect=PermissionError("denied")), self.assertRaises(PermissionError):
            stop_process(process, log)
        log.close.assert_called_once()
        log.reset_mock()
        process.wait.side_effect = subprocess.TimeoutExpired("test process", 10)
        with patch("run_browser_full_project_e2e.os.killpg"), self.assertRaises(RunnerError):
            stop_process(process, log)
        log.close.assert_called_once()

    def test_worker_cleanup_tolerates_process_exit_during_environment_read(self):
        root = self.scratch()
        process = root / "proc" / "12345"
        process.mkdir(parents=True)
        (process / "cmdline").write_bytes(b"/usr/bin/chromium\0")
        binary_dir = root / "bin"
        binary_dir.mkdir()
        browser_artifacts = root / "browser"
        browser_artifacts.mkdir()
        original_iterdir = Path.iterdir
        original_read_bytes = Path.read_bytes

        def iter_processes(path):
            if path == Path("/proc"):
                return iter((process,))
            return original_iterdir(path)

        def read_process_file(path):
            if path == process / "environ":
                raise ProcessLookupError("process exited during cleanup")
            return original_read_bytes(path)

        with patch("run_creation_e2e.Path.iterdir", iter_processes), patch(
            "run_creation_e2e.Path.read_bytes", read_process_file
        ):
            stop_owned_workers(binary_dir, browser_artifacts)

    def test_postgres_is_removed_even_when_its_log_cannot_be_opened(self):
        path = self.scratch() / "missing" / "postgres.log"
        with patch("run_creation_e2e.subprocess.run") as run, self.assertRaises(FileNotFoundError):
            stop_postgres("owned-test-container", path)
        run.assert_called_once_with(["docker", "rm", "-f", "owned-test-container"], stdout=subprocess.DEVNULL, check=True, timeout=30)


if __name__ == "__main__":
    unittest.main()
