"""Shared process and report helpers for test runners."""

from __future__ import annotations

import os
import signal
import subprocess
import time
from pathlib import Path
from typing import BinaryIO


class ReportDirectoryError(RuntimeError):
    pass


def validate_report_dir(path: Path) -> Path:
    resolved = path.expanduser().resolve()
    if resolved == Path("/"):
        raise ReportDirectoryError("报告目录不能是根目录")
    if resolved.exists():
        if not resolved.is_dir():
            raise ReportDirectoryError(f"报告路径必须是目录：{resolved}")
        if any(resolved.iterdir()):
            raise ReportDirectoryError(f"报告目录必须为空，避免覆盖旧证据：{resolved}")
    return resolved


def safe_report_dir(path: Path) -> Path:
    resolved = validate_report_dir(path)
    resolved.mkdir(parents=True, exist_ok=True, mode=0o700)
    resolved.chmod(0o700)
    return resolved


def private_file(path: Path, *, append: bool = False) -> BinaryIO:
    path.parent.mkdir(parents=True, exist_ok=True)
    flags = os.O_WRONLY | os.O_CREAT | (os.O_APPEND if append else os.O_TRUNC)
    descriptor = os.open(path, flags, 0o600)
    return os.fdopen(descriptor, "ab")


def _process_group_exists(process_group: int) -> bool:
    try:
        os.killpg(process_group, 0)
    except ProcessLookupError:
        return False
    return True


def terminate_process(
    process: subprocess.Popen[bytes], *, grace_seconds: float = 5
) -> None:
    process_group = process.pid
    try:
        os.killpg(process_group, signal.SIGTERM)
    except ProcessLookupError:
        if process.poll() is None:
            process.wait()
        return

    deadline = time.monotonic() + grace_seconds
    while _process_group_exists(process_group):
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            break
        if process.poll() is None:
            try:
                process.wait(timeout=min(0.1, remaining))
            except subprocess.TimeoutExpired:
                pass
        else:
            time.sleep(min(0.1, remaining))

    if _process_group_exists(process_group):
        try:
            os.killpg(process_group, signal.SIGKILL)
        except ProcessLookupError:
            pass
    if process.poll() is None:
        process.wait()


def raise_interrupted(_signum: int, _frame: object) -> None:
    raise KeyboardInterrupt
