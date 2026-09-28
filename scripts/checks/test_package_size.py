import tempfile
import unittest
from pathlib import Path

from package_size import inventory


class PackageSizeTest(unittest.TestCase):
    def test_counts_each_shipped_copy_but_not_symlink_targets_twice(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaises(ValueError):
                inventory(root)
            (root / "lib.so.1").write_bytes(b"library")
            (root / "lib.so").symlink_to("lib.so.1")
            (root / "duplicate.so").write_bytes(b"library")
            result = inventory(root)
            self.assertEqual(result["file_bytes"], 14)
            files = {entry["path"]: entry for entry in result["files"]}
            self.assertEqual(files["lib.so"]["symlink"], "lib.so.1")
            self.assertEqual(files["lib.so.1"]["sha256"], files["duplicate.so"]["sha256"])


if __name__ == "__main__":
    unittest.main()
