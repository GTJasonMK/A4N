"""Check real business requests before returning deterministic success fixtures."""
from __future__ import annotations

import functools
import json
import subprocess
from pathlib import Path

from mock_openai.fixtures.reverse import message_text

ROOT = Path(__file__).resolve().parents[3]
START = "[A4N_OUTPUT_CONTRACT]"
END = "[/A4N_OUTPUT_CONTRACT]"


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate JSON key: " + key)
        result[key] = value
    return result


@functools.lru_cache(maxsize=512)
def validate_fixture(schema_json: str, output: str) -> None:
    value = json.loads(output, object_pairs_hook=unique_object)
    records = [
        {"kind": "schema", "schema_id": "request", "schema": json.loads(schema_json)},
        {"case_id": "fixture", "schema_id": "request", "value": value},
    ]
    result = subprocess.run(
        ["node", "scripts/contracts/check-model-vectors.mjs"], cwd=ROOT,
        input="".join(json.dumps(row, ensure_ascii=False) + "\n" for row in records),
        capture_output=True, text=True, timeout=10,
    )
    if result.returncode:
        raise ValueError("fixture schema checker failed: " + result.stderr[-1500:])
    observation = json.loads(result.stdout)
    if observation.get("accepted") is not True:
        raise ValueError("normal fixture violates request contract: " + json.dumps(observation.get("errors"), ensure_ascii=False))


def validate_business_request(body: dict, messages: list[dict], normal_output: str) -> None:
    if not isinstance(body.get("model"), str) or not body["model"].strip():
        raise ValueError("model must be non-empty")
    user_input = [message_text(row.get("content", "")) for row in messages if row.get("role") == "user"]
    if not user_input or not user_input[-1].strip():
        raise ValueError("business request has no current input")
    contracts = []
    for row in messages:
        if row.get("role") != "system":
            continue
        text = message_text(row.get("content", ""))
        while START in text:
            _, text = text.split(START, 1)
            if END not in text:
                raise ValueError("unclosed output contract")
            raw, text = text.split(END, 1)
            contracts.append(json.loads(raw, object_pairs_hook=unique_object))
    native = body.get("response_format", {}).get("json_schema", {}).get("schema")
    native = body.get("text", {}).get("format", {}).get("schema", native)
    native = body.get("generationConfig", {}).get("responseJsonSchema", native)
    if not contracts:
        if native is not None or normal_output.lstrip().startswith(("{", "[")):
            raise ValueError("structured business request is missing protected output contract")
        return
    contract = contracts[0]
    if any(row != contract for row in contracts) or "schema" not in contract or not contract.get("name"):
        raise ValueError("conflicting or incomplete output contracts")
    if native is not None and native != contract["schema"]:
        raise ValueError("native schema conflicts with protected prompt contract")
    validate_fixture(json.dumps(contract["schema"], sort_keys=True), normal_output)
