"""Small parsers shared by deterministic prompt fixtures."""

from __future__ import annotations

import json
import re
from typing import Any
def current_task_prompt_name(combined: str) -> str | None:
    matches = re.findall(
        r'"task"\s*:\s*\{[^{}]*"(?:prompt_name|name)"\s*:\s*"([^"]+)"',
        combined,
    )
    return matches[-1] if matches else None


def json_task_payload(combined: str) -> dict[str, Any]:
    marker = "\n\n输入：\n"
    marker_index = combined.rfind(marker)
    if marker_index < 0:
        raise ValueError("prompt contains no JSON task payload")
    payload = json.loads(combined[marker_index + len(marker) :])
    if not isinstance(payload, dict):
        raise ValueError("JSON task payload must be an object")
    return payload
