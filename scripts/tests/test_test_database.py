#!/usr/bin/env python3
"""Unit tests for disposable test database command construction."""

from __future__ import annotations

import unittest

from run_with_test_database import build_database_url, container_command


class TestDatabaseRunnerTest(unittest.TestCase):
    def test_database_url_contains_only_the_redacted_host_path_shape(self) -> None:
        self.assertEqual(
            build_database_url(54321, "pass word"),
            "postgres://a4n_test:pass%20word@127.0.0.1:54321/a4n_test",
        )

    def test_container_command_uses_a_unique_host_port_and_fixed_test_identity(self) -> None:
        command = container_command("a4n-test", 54321, "postgres:16", "secret")
        self.assertIn("127.0.0.1:54321:5432", command)
        self.assertIn("POSTGRES_PASSWORD", command)
        self.assertNotIn("secret", command)
        self.assertIn("POSTGRES_USER=a4n_test", command)
        self.assertIn("POSTGRES_DB=a4n_test", command)
        self.assertNotIn("a4n", command[-3:])


if __name__ == "__main__":
    unittest.main()
