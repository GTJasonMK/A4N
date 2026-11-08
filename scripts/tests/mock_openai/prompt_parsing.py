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
    text = combined[marker_index + len(marker) :].lstrip()
    decoder = json.JSONDecoder()
    payload, consumed = decoder.raw_decode(text)
    remainder = text[consumed:].strip()
    while remainder:
        feedback, consumed = decoder.raw_decode(remainder)
        if not isinstance(feedback, dict) or feedback.get("kind") != "model_output_correction":
            raise ValueError("unexpected content after JSON task payload")
        remainder = remainder[consumed:].strip()
    if not isinstance(payload, dict):
        raise ValueError("JSON task payload must be an object")
    return payload
