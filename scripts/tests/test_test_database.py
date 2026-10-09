#!/usr/bin/env python3
"""Unit tests for disposable test database command construction."""

from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
from unittest.mock import Mock, patch

from runner_support import ReportDirectoryError, private_file, safe_report_dir
from run_with_test_database import (
    build_database_url,
    container_command,
    main,
    remove_container_command,
)


class TestDatabaseRunnerTest(unittest.TestCase):
    def test_database_url_contains_only_the_redacted_host_path_shape(self) -> None:
        self.assertEqual(
            build_database_url(54321, "pass word"),
            "postgres://a4n_test:pass%20word@127.0.0.1:54321/a4n_test",
        )

    def test_container_command_uses_a_unique_host_port_and_fixed_test_identity(self) -> None:
        command = container_command("a4n-test", 54321, "postgres:16")
        self.assertIn("127.0.0.1:54321:5432", command)
        self.assertIn("POSTGRES_PASSWORD", command)
        self.assertIn("POSTGRES_USER=a4n_test", command)
        self.assertIn("POSTGRES_DB=a4n_test", command)
        self.assertNotIn("a4n", command[-3:])

    def test_removal_drops_the_anonymous_postgres_volume(self) -> None:
        self.assertEqual(
            remove_container_command("a4n-test"),
            ["docker", "rm", "--volumes", "--force", "a4n-test"],
        )

    def test_report_directory_is_private_and_existing_evidence_is_preserved(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "run"
            safe_report_dir(path)
            self.assertEqual(path.stat().st_mode & 0o777, 0o700)
            with private_file(path / "secret.log") as output:
                output.write(b"private")
            self.assertEqual((path / "secret.log").stat().st_mode & 0o777, 0o600)
            with self.assertRaisesRegex(ReportDirectoryError, "必须为空"):
                safe_report_dir(path)

    def test_timeout_records_failure_and_removes_container_volume(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            report_dir = Path(directory) / "run"
            child = Mock()
            child.wait.side_effect = subprocess.TimeoutExpired(["true"], 1)
            commands = []

            def docker_run(command, **_kwargs):
                commands.append(command)
                return subprocess.CompletedProcess(command, 0, stderr="")

            with (
                patch("run_with_test_database.shutil.which", return_value="/usr/bin/docker"),
                patch("run_with_test_database.free_port", return_value=54321),
                patch("run_with_test_database.subprocess.run", side_effect=docker_run),
                patch("run_with_test_database.subprocess.Popen", return_value=child),
                patch("run_with_test_database.terminate_process") as terminate,
                patch(
                    "sys.argv",
                    ["run_with_test_database.py", "--report-dir", str(report_dir), "--timeout", "1", "--", "true"],
                ),
            ):
                with redirect_stdout(StringIO()):
                    self.assertEqual(main(), 1)

            report = json.loads((report_dir / "database.json").read_text(encoding="utf-8"))
            self.assertEqual(report["status"], "timeout")
            self.assertEqual(report["exit_code"], 124)
            terminate.assert_called_once_with(child)
            self.assertIn(
                remove_container_command(report["container"]),
                commands,
            )

    def test_unverified_container_cleanup_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            report_dir = Path(directory) / "run"
            child = Mock()
            child.wait.return_value = 0
            commands = []

            def docker_run(command, **_kwargs):
                commands.append(command)
                if command[:3] == ["docker", "container", "inspect"]:
                    return subprocess.CompletedProcess(command, 1, stderr="daemon unavailable")
                return subprocess.CompletedProcess(command, 0, stderr="")

            with (
                patch("run_with_test_database.shutil.which", return_value="/usr/bin/docker"),
                patch("run_with_test_database.free_port", return_value=54321),
                patch("run_with_test_database.subprocess.run", side_effect=docker_run),
                patch("run_with_test_database.subprocess.Popen", return_value=child),
                patch(
                    "sys.argv",
                    ["run_with_test_database.py", "--report-dir", str(report_dir), "--", "true"],
                ),
            ):
                with redirect_stdout(StringIO()):
                    self.assertEqual(main(), 1)

            report = json.loads((report_dir / "database.json").read_text(encoding="utf-8"))
            self.assertEqual(report["status"], "failed")
            self.assertIn("无法确认 PostgreSQL 容器已删除", report["cleanup_error"])
            self.assertFalse(any(command[:2] == ["docker", "rm"] for command in commands))


if __name__ == "__main__":
    unittest.main()
