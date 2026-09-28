#!/usr/bin/env python3
"""Inventory actual release files, not Cargo caches or uncompressed estimates.

Example: python3 scripts/checks/package_size.py --installed /tmp/unpacked-deb \
    --archive target/release/bundle/deb/A4N.deb > size.json
"""

import argparse
import hashlib
import json
from pathlib import Path


def inventory(root):
    root = Path(root)
    if not root.is_dir():
        raise ValueError(f"installation directory does not exist: {root}")
    files = []
    for path in sorted(root.rglob("*")):
        relative = path.relative_to(root).as_posix()
        if path.is_symlink():
            files.append({"path": relative, "symlink": str(path.readlink())})
        elif path.is_file():
            with path.open("rb") as stream:
                digest = hashlib.file_digest(stream, "sha256").hexdigest()
            files.append({"path": relative, "bytes": path.stat().st_size, "sha256": digest})
    if not files:
        raise ValueError(f"installation directory is empty: {root}")
    return {"files": files, "file_bytes": sum(f.get("bytes", 0) for f in files)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--installed", type=Path, required=True)
    parser.add_argument("--archive", type=Path, required=True)
    args = parser.parse_args()
    try:
        report = inventory(args.installed)
        with args.archive.open("rb") as stream:
            report["archive"] = {
                "name": args.archive.name,
                "bytes": args.archive.stat().st_size,
                "sha256": hashlib.file_digest(stream, "sha256").hexdigest(),
            }
        print(json.dumps(report, ensure_ascii=False, indent=2))
    except (OSError, ValueError) as error:
        parser.exit(1, f"package size inventory failed: {error}\n")


if __name__ == "__main__":
    main()
