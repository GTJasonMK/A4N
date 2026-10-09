from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from run_tauri_generation_e2e import RunnerError, parse_functional_result


class TauriGenerationE2ETest(unittest.TestCase):
    def test_functional_result_requires_generated_text_after_remount(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "wdio.log"
            valid_result = {
                "remountFirstTextAt": 100,
                "remountOccurrences": 1,
                "remountCompleted": True,
            }
            path.write_text("A4N_TAURI_WRY_RESULT=" + json.dumps(valid_result))
            self.assertEqual(parse_functional_result(path), valid_result)

            for invalid_fields in (
                {"remountCompleted": False},
                {"remountOccurrences": 2},
                {"remountOccurrences": 0},
                {"remountFirstTextAt": 0},
                {"remountFirstTextAt": -1},
            ):
                with self.subTest(fields=invalid_fields):
                    result = {**valid_result, **invalid_fields}
                    path.write_text("A4N_TAURI_WRY_RESULT=" + json.dumps(result))
                    with self.assertRaisesRegex(RunnerError, "功能结果不完整"):
                        parse_functional_result(path)

    def test_functional_result_rejects_missing_or_duplicate_records(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "wdio.log"
            path.write_text("no result")
            with self.assertRaisesRegex(RunnerError, "唯一结果记录"):
                parse_functional_result(path)
            path.write_text("A4N_TAURI_WRY_RESULT={}\nA4N_TAURI_WRY_RESULT={}")
            with self.assertRaisesRegex(RunnerError, "唯一结果记录"):
                parse_functional_result(path)


if __name__ == "__main__":
    unittest.main()
