#!/usr/bin/env python3
"""回归：运行前预检失败时不得进入前端或 Cargo 构建。"""

from __future__ import annotations

import os
import socket
import subprocess
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


class RunPreflightTest(unittest.TestCase):
    def test_occupied_vite_port_blocks_debug_build(self) -> None:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
            listener.bind(("127.0.0.1", 0))
            listener.listen()
            env = os.environ.copy()
            env.update(
                {
                    "A4N_DATABASE_URL": "postgres://a4n:a4n@127.0.0.1:1/a4n",
                    "A4N_JWT_SECRET": "preflight-test",
                    "A4N_SIDECAR_AUTH_TOKEN": "preflight-test",
                    "VITE_PORT": str(listener.getsockname()[1]),
                }
            )
            result = subprocess.run(
                [str(ROOT / "scripts/run.sh"), "--mode", "native", "--no-sidecars"],
                cwd=ROOT,
                env=env,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                timeout=10,
                check=False,
            )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Vite 开发端口", result.stdout)
        self.assertIn("已被占用", result.stdout)
        self.assertNotIn("PostgreSQL 预检失败", result.stdout)
        self.assertNotIn("Finished `dev` profile", result.stdout)

    def test_unreachable_postgres_blocks_debug_build(self) -> None:
        env = os.environ.copy()
        env.update(
            {
                "A4N_DATABASE_URL": "postgres://a4n:a4n@127.0.0.1:1/a4n",
                "A4N_JWT_SECRET": "preflight-test",
                "A4N_SIDECAR_AUTH_TOKEN": "preflight-test",
            }
        )
        result = subprocess.run(
            [
                str(ROOT / "scripts/run.sh"),
                "--mode",
                "native",
                "--no-sidecars",
                "--no-hot",
            ],
            cwd=ROOT,
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=10,
            check=False,
        )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("PostgreSQL 预检失败", result.stdout)
        self.assertNotIn("vite", result.stdout)
        self.assertNotIn("Finished `dev` profile", result.stdout)


if __name__ == "__main__":
    unittest.main()
