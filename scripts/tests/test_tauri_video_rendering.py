#!/usr/bin/env python3
"""检查真实桌面入口在启动 Tauri 前配置视频渲染，并保留显式环境设置。"""

import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
VARIABLE = "WEBKIT_GST_DISABLE_GL_SINK"


@unittest.skipUnless(sys.platform == "linux", "WebKitGTK configuration is Linux-only")
class TauriVideoRenderingTest(unittest.TestCase):
    def test_video_sink_is_configured_before_tauri_starts(self):
        (ROOT / "data").mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(
            prefix="video-startup-test-", dir=ROOT / "data"
        ) as directory:
            scratch = Path(directory)
            library = scratch / "probe.rs"
            library.write_text(
                'pub fn run() { println!("{}", '
                'std::env::var("WEBKIT_GST_DISABLE_GL_SINK").unwrap_or_default()); }\n'
            )
            rlib = scratch / "liba4n_tauri_lib.rlib"
            binary = scratch / "desktop-startup"
            # 仅替换启动后的库入口；编译和执行生产 main.rs，避免启动数据库或窗口。
            for command in [
                [
                    "rustc", "--edition=2024", "--crate-name=a4n_tauri_lib",
                    "--crate-type=rlib", str(library), "-o", str(rlib),
                ],
                [
                    "rustc", "--edition=2024",
                    str(ROOT / "crates/host/a4n-tauri/src/main.rs"),
                    "--extern", f"a4n_tauri_lib={rlib}", "-o", str(binary),
                ],
            ]:
                subprocess.run(
                    command, check=True, timeout=30, capture_output=True, text=True
                )
            for configured, expected in [(None, "1"), ("0", "0"), ("1", "1")]:
                with self.subTest(configured=configured):
                    env = os.environ.copy()
                    env.pop(VARIABLE, None)
                    if configured is not None:
                        env[VARIABLE] = configured
                    result = subprocess.run(
                        [str(binary)], env=env, check=True, timeout=5,
                        capture_output=True, text=True,
                    )
                    self.assertEqual(result.stdout.strip(), expected)


if __name__ == "__main__":
    unittest.main()
