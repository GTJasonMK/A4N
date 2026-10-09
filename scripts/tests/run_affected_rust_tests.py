#!/usr/bin/env python3
"""Apply migrations and test changed Rust packages and workspace dependants."""

from __future__ import annotations

import argparse
import json
import os
import shlex
import subprocess
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
CORE_PACKAGES = {"a4n-migration", "a4n-db", "a4n-orchestrator", "a4n-runtime"}
FULL_WORKSPACE_FILES = {
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain",
    "rust-toolchain.toml",
    ".cargo/config",
    ".cargo/config.toml",
    "scripts/cargo.sh",
    "scripts/build.sh",
    "scripts/lib/common.sh",
}
FULL_WORKSPACE_PREFIXES = (
    "crates/platform/a4n-migration/",
    "crates/process/a4n-service-protocol/",
    "scripts/contracts/",
)
class AffectedRustError(RuntimeError):
    pass


def git_output(*args: str) -> str:
    result = subprocess.run(
        ["git", *args],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode:
        raise AffectedRustError(result.stderr.strip() or f"git {' '.join(args)} failed")
    return result.stdout


def base_ref(explicit: str | None) -> str:
    requested = explicit or os.environ.get("A4N_TEST_BASE_REF")
    if not requested:
        github_base = os.environ.get("GITHUB_BASE_REF")
        if github_base:
            requested = f"refs/remotes/origin/{github_base}"
    if requested:
        if subprocess.run(
            ["git", "rev-parse", "--verify", f"{requested}^{{commit}}"],
            cwd=ROOT,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        ).returncode:
            raise AffectedRustError(f"base ref 不存在：{requested}；请在 GitHub Actions fetch-depth: 0")
        return requested
    for candidate in ("origin/debug", "debug"):
        if subprocess.run(
            ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"],
            cwd=ROOT,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        ).returncode == 0:
            return candidate
    raise AffectedRustError(
        "无法确定 Rust 测试基线；请通过 --base-ref 或 A4N_TEST_BASE_REF 指定"
    )


def changed_files(reference: str) -> set[str]:
    files: set[str] = set()
    files.update(git_output("diff", "--name-only", f"{reference}...HEAD", "--").splitlines())
    files.update(git_output("diff", "--name-only", "HEAD", "--").splitlines())
    files.update(git_output("ls-files", "--others", "--exclude-standard").splitlines())
    return {name.replace("\\", "/") for name in files if name}


def workspace_package_dir(manifest_path: str) -> str:
    return Path(manifest_path).parent.relative_to(ROOT).as_posix()


def load_workspace_packages() -> tuple[dict[str, dict[str, Any]], dict[str, str]]:
    result = subprocess.run(
        ["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode:
        raise AffectedRustError(result.stderr.strip() or "cargo metadata failed")
    try:
        metadata = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise AffectedRustError(f"cargo metadata 返回无效 JSON：{error}") from error
    packages = {
        package["name"]: package
        for package in metadata.get("packages", [])
        if package.get("source") is None
    }
    package_by_dir = {
        workspace_package_dir(package["manifest_path"]): package["name"]
        for package in packages.values()
    }
    if not packages:
        raise AffectedRustError("cargo metadata 未返回 workspace packages")
    return packages, package_by_dir


def needs_full_workspace(files: set[str], packages: dict[str, dict[str, Any]]) -> bool:
    if files & FULL_WORKSPACE_FILES:
        return True
    if any(name.startswith(FULL_WORKSPACE_PREFIXES) for name in files):
        return True
    contract_dirs = {
        workspace_package_dir(package["manifest_path"])
        for package_name, package in packages.items()
        if package_name.endswith("-contract")
    }
    for name in files:
        if not name.startswith("crates/"):
            continue
        if any(name.startswith(directory + "/") for directory in contract_dirs):
            return True
    return False


def changed_packages(
    files: set[str], package_by_dir: dict[str, str]
) -> set[str]:
    changed: set[str] = set()
    for name in files:
        if not name.startswith("crates/"):
            continue
        matches = [directory for directory in package_by_dir if name == directory or name.startswith(directory + "/")]
        if matches:
            changed.add(package_by_dir[max(matches, key=len)])
        elif name.endswith((".rs", ".toml")):
            raise AffectedRustError(f"Rust 文件无法映射到 workspace package：{name}")
    return changed


def reverse_dependency_closure(
    changed: set[str], packages: dict[str, dict[str, Any]]
) -> set[str]:
    dependants: dict[str, set[str]] = {name: set() for name in packages}
    for package_name, package in packages.items():
        for dependency in package.get("dependencies", []):
            dependency_name = dependency.get("name")
            if dependency_name in dependants:
                dependants[dependency_name].add(package_name)
    selected = set(changed)
    pending = list(changed)
    while pending:
        for dependant in dependants.get(pending.pop(), ()):
            if dependant not in selected:
                selected.add(dependant)
                pending.append(dependant)
    return selected


def uses_onnx_runtime(selected: list[str]) -> bool:
    command = [
        "cargo",
        "tree",
        "--locked",
        "--edges",
        "all",
        "--prefix",
        "none",
        "--color",
        "never",
    ]
    for package in selected:
        command.extend(("--package", package))
    result = subprocess.run(
        command,
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode:
        raise AffectedRustError(result.stderr.strip() or "cargo tree failed")
    # Cargo's resolved graph excludes optional dependencies not enabled by the selected packages.
    return any(line.startswith("a4n-embedding-fastembed v") for line in result.stdout.splitlines())


def select_packages(
    files: set[str], packages: dict[str, dict[str, Any]], package_by_dir: dict[str, str]
) -> tuple[bool, list[str]]:
    if needs_full_workspace(files, packages):
        return True, []
    changed = changed_packages(files, package_by_dir)
    missing_core = sorted(CORE_PACKAGES - packages.keys())
    if missing_core:
        raise AffectedRustError(f"Cargo workspace 缺少核心数据库测试 package：{missing_core}")
    selected = CORE_PACKAGES | reverse_dependency_closure(changed, packages)
    return False, sorted(selected & packages.keys())


def build_plan(args: argparse.Namespace) -> dict[str, Any]:
    reference = base_ref(args.base_ref)
    files = changed_files(reference)
    packages, package_by_dir = load_workspace_packages()
    full_workspace, selected = select_packages(files, packages, package_by_dir)
    if full_workspace:
        cargo_args = ["cargo", "test", "--locked", "--workspace", "--all-targets", "--no-fail-fast"]
        selection = {"mode": "full_workspace", "packages": sorted(packages)}
    else:
        cargo_args = ["cargo", "test", "--locked", "--all-targets", "--no-fail-fast"]
        for package in selected:
            cargo_args.extend(("-p", package))
        selection = {"mode": "affected_packages", "packages": selected}
    cargo_args.extend(("--", "--test-threads=1"))
    selection["base_ref"] = reference
    selection["changed_files"] = sorted(files)
    selection["migration_command"] = ["cargo", "run", "--locked", "-p", "a4n-migration"]
    selection["uses_onnx_runtime"] = uses_onnx_runtime(selection["packages"])
    selection["cargo_command"] = cargo_args
    return selection


def run_test(args: argparse.Namespace) -> int:
    selection = build_plan(args)
    print(json.dumps(selection, ensure_ascii=False), flush=True)
    if args.plan:
        return 0

    if selection["uses_onnx_runtime"]:
        subprocess.run([str(ROOT / "scripts/dev/setup-onnxruntime.sh")], cwd=ROOT, check=True)
    test_command = [
        sys.executable,
        str(ROOT / "scripts/tests/run_with_test_database.py"),
        "--timeout",
        str(args.timeout),
        "--",
        "bash",
        "-c",
        f'set -euo pipefail; {shlex.join(selection["migration_command"])}; exec "$@"',
        "a4n-affected-rust-tests",
        *selection["cargo_command"],
    ]
    if selection["uses_onnx_runtime"]:
        command = [
            "bash",
            "-lc",
            'source "$1/scripts/lib/common.sh"; require_onnxruntime; shift; exec "$@"',
            "a4n-affected-tests",
            str(ROOT),
            *test_command,
        ]
    else:
        command = test_command
    return subprocess.run(command, cwd=ROOT, check=False).returncode


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-ref")
    parser.add_argument("--plan", action="store_true", help="print the selected package scope without building")
    parser.add_argument("--timeout", type=int, default=7200)
    args = parser.parse_args()
    if args.timeout <= 0:
        parser.error("--timeout 必须为正数")
    return args


def main() -> int:
    try:
        return run_test(parse_args())
    except (AffectedRustError, OSError, subprocess.CalledProcessError) as error:
        print(f"Rust 受影响测试选择失败：{error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
