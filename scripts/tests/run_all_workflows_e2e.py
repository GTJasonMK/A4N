#!/usr/bin/env python3
"""Invoke every exported A4N Host API descriptor through the real CLI.

This is a manifest probe, not a claim that every command has a valid business
fixture. Every descriptor is invoked and the report separates successful
business calls, expected contract/state rejections, and real failures.
"""

from __future__ import annotations

import argparse
import base64
import json
import os
import re
import shutil
import sys
import tempfile
import time
import uuid
from pathlib import Path
from typing import Any, Callable

from run_cli_contract_e2e import (
    ROOT,
    Cli,
    RunnerError,
    check_provider_contract,
    checked,
    command_error,
    runtime_snapshot,
    secure_json,
    start_provider,
)
from mock_openai.fixtures.story import chapter_body_response
from mock_openai.fixtures.media import chapter_visual_plan_fixture


PROBE_ID = 999_999_999
PROBE_TASK_ID = "00000000-0000-4000-8000-000000000001"
PROBE_WORKFLOW_ID = 999_999_999
AUTH_COMMANDS = {
    "change_user_password",
    "login_user",
    "oauth2_callback",
    "refresh_token",
    "register_user",
    "revoke_session",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--a4n", default=os.environ.get("A4N_CLI", str(ROOT / "target/debug/a4n")))
    parser.add_argument("--server", required=True)
    parser.add_argument("--username", help="管理员或其他已有账号；密码只从 --password-env 读取")
    parser.add_argument("--password-env", default="A4N_PASSWORD")
    parser.add_argument("--collaborator-username", help="可选的第二真实账号；用于接受项目邀请")
    parser.add_argument(
        "--collaborator-password-env",
        default="A4N_COLLABORATOR_PASSWORD",
        help="第二真实账号密码所在环境变量",
    )
    parser.add_argument("--config", type=Path, help="CLI config；未提供时使用临时配置")
    parser.add_argument("--context", type=Path, help="CLI context；未提供时使用临时 context")
    parser.add_argument("--manifest", type=Path, help="已导出的 manifest；省略时通过 CLI 动态导出")
    parser.add_argument("--fixture-map", type=Path, help="命令名到参数对象的 JSON 映射")
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--timeout", type=float, default=30)
    parser.add_argument("--provider-url", help="已有 mock provider 的 /v1 URL")
    parser.add_argument(
        "--provider-script",
        type=Path,
        default=Path(__file__).with_name("mock_openai_provider.py"),
    )
    parser.add_argument("--provider-host", default="127.0.0.1")
    parser.add_argument("--provider-port", type=int, default=0)
    parser.add_argument("--provider-start-timeout", type=float, default=10)
    parser.add_argument(
        "--allow-destructive",
        action="store_true",
        help="允许 descriptor 标记为 destructive 的调用；只对隔离 fixture 使用",
    )
    parser.add_argument(
        "--execute-writes",
        action="store_true",
        help="在隔离环境中执行 schema-valid write 参数并准备真实业务 fixture",
    )
    parser.add_argument(
        "--fixture-only",
        action="store_true",
        help="只执行并报告真实业务 fixture；必须同时使用 --execute-writes --allow-destructive",
    )
    return parser.parse_args()


def enum_value(schema: dict[str, Any]) -> Any:
    values = schema.get("enum")
    return values[0] if isinstance(values, list) and values else None


def scalar(name: str, command: str) -> Any:
    key = name.lower()
    if key in {
        "projectid",
        "project_id",
        "root_project_id",
        "collaboration_root_project_id",
        "chapterid",
        "chapter_id",
        "source_chapter_id",
        "userid",
        "user_id",
        "owner_user_id",
        "memberuserid",
        "member_user_id",
        "promptid",
        "prompt_id",
        "videoid",
        "video_id",
        "profileid",
        "profile_id",
        "character_profile_id",
        "sceneid",
        "scene_id",
        "themeid",
        "theme_id",
        "inviteid",
        "invite_id",
        "nodeid",
        "pushvideoid",
        "push_video_id",
    }:
        return PROBE_ID
    if key in {"configid", "config_id"}:
        if "image" in command:
            return 900002
        if "video" in command:
            return 900003
        if "embedding" in command or "rag" in command:
            return 900004
        return 900001
    if key in {"workflow_run_id", "workflowrunid"}:
        return PROBE_WORKFLOW_ID
    if key in {"taskid", "task_id"}:
        return PROBE_TASK_ID
    if key in {"client_request_id", "start_request_id"}:
        return f"manifest-probe-{command}"
    if key in {"password", "old_password", "new_password"}:
        return "manifest-probe-password"
    if key == "email":
        return "manifest-probe@example.invalid"
    if key == "username":
        return "manifest-probe-user"
    if key in {"name", "character_name", "template_name", "title", "description", "query", "message", "text"}:
        return "manifest probe"
    if key in {"project_type"}:
        return "novel_long"
    if key in {"provider"}:
        return "openai"
    if key == "backend":
        if "embedding" in command or "rag" in command:
            return "api"
        if "video" in command:
            return "openai-compatible-video"
        return "openai-compatible-image-edit" if "image" in command else "api"
    if key in {"workflow_mode"}:
        return "boundary_frames"
    if key in {"content_hash", "base_content_hash", "query_fingerprint", "client_rag_query_fingerprint"}:
        return "0" * 64
    if key in {"file_name", "filename"}:
        if "family" in command:
            return "manifest-probe.a4n-family.zip"
        if "video_studio" in command:
            return "manifest-probe.a4n-video-studio.zip"
        return "manifest-probe.txt"
    if key in {"mime_type", "mimetype"}:
        return "text/plain"
    if key in {"data_base64", "source_data_base64"}:
        return base64.b64encode(b"A4N manifest probe\n").decode("ascii")
    if key in {"image_type"}:
        return "illustration"
    if key in {"media_kind"}:
        return "image"
    if key in {"role"}:
        return "manager"
    if key in {"status"}:
        return "pending"
    if key in {"mode"}:
        return "light"
    if key in {"target_audience", "genre", "style", "tone", "importance", "object_type", "scene_type"}:
        return "manifest probe"
    if key in {"expected_updated_at"}:
        return "2026-01-01T00:00:00Z"
    if key in {"model"}:
        if "embedding" in command or "rag" in command:
            return "a4n-virtual-embedding"
        if "video" in command:
            return "a4n-virtual-video"
        if "image" in command:
            return "a4n-virtual-image"
        return "a4n-virtual-text"
    if key in {"base_url"}:
        return "http://127.0.0.1:1/v1"
    if key in {"api_key_encrypted", "api_key"}:
        return "a4n-e2e-provider-key"
    if key in {"default_width", "default_height"}:
        return 1024
    if key in {"supported_sizes"}:
        return [{"width": 1024, "height": 1024}]
    if key in {"default_fps"}:
        return 24
    if key in {"default_duration_secs", "min_duration_secs", "max_duration_secs", "duration_secs"}:
        return 1
    if key in {"dimension"}:
        return 8
    if key in {"batch_size"}:
        return 32
    if key in {"max_concurrent"}:
        return 2
    if key in {"timeout_secs"}:
        return 60
    if key in {"max_context_tokens"}:
        return 128000
    if key in {"max_output_tokens"}:
        return 8192
    if key in {"min_word_count"}:
        return 800
    if key in {"max_word_count"}:
        return 1600
    if key in {"days", "recent_limit", "evidence_limit", "window_chapters", "image_limit_per_profile"}:
        return 1
    if key in {"limit", "top_k", "page_size", "pagesize"}:
        return 20
    if key in {"page", "offset", "start_chapter", "end_chapter", "chapter_number", "part_id", "partid"}:
        return 1
    if key in {"include_audio", "includeaudio", "rebuild", "resumable_only", "use_saved_api_key"}:
        return False
    if key.endswith("_secs") or key.endswith("secs"):
        return 1
    if key.endswith("count") or key.startswith("min_") or key.startswith("max_"):
        return 1
    return "probe"


def resolve_ref(schema: Any, root: dict[str, Any]) -> dict[str, Any] | None:
    if not isinstance(schema, dict):
        return None
    reference = schema.get("$ref")
    if not isinstance(reference, str) or not reference.startswith("#/$defs/"):
        return None
    current: Any = root
    for part in reference.removeprefix("#/").split("/"):
        if not isinstance(current, dict):
            return None
        current = current.get(part)
    return current if isinstance(current, dict) else None


def example(
    schema: Any,
    command: str,
    field: str = "",
    root: dict[str, Any] | None = None,
) -> Any:
    if schema is True:
        if field.lower() in {
            "analysis_result",
            "base_attributes",
            "config_json",
            "entry_points_json",
            "extra_body_json",
            "metadata_json",
            "position_json",
            "payload",
            "rules_json",
            "supported_durations_json",
            "value",
            "variables",
            "viewport_json",
        }:
            return {}
        return scalar(field, command)
    if schema is False:
        return None
    if not isinstance(schema, dict):
        return scalar(field, command)
    root = root or schema
    resolved = resolve_ref(schema, root)
    if resolved is not None:
        return example(resolved, command, field, root)
    if "const" in schema:
        return schema["const"]
    if "enum" in schema:
        return enum_value(schema)
    if "default" in schema and schema["default"] is not None:
        return schema["default"]
    for union_name in ("anyOf", "oneOf"):
        options = schema.get(union_name)
        if isinstance(options, list):
            for option in options:
                if isinstance(option, dict) and option.get("type") != "null":
                    return example(option, command, field, root)
            return None
    schema_type = schema.get("type")
    if isinstance(schema_type, list):
        schema_type = next((item for item in schema_type if item != "null"), "string")
    if schema_type == "object" or "properties" in schema:
        result: dict[str, Any] = {}
        properties = schema.get("properties", {})
        child_root = schema if "$defs" in schema else root
        for name in schema.get("required", []):
            result[name] = example(properties.get(name, {}), command, name, child_root)
        return result
    if schema_type == "array":
        minimum = int(schema.get("minItems", 0))
        item_schema = schema.get("items", {})
        return [example(item_schema, command, field, root) for _ in range(minimum)]
    if schema_type in {"integer", "number"}:
        candidate = scalar(field, command)
        if isinstance(candidate, bool) or not isinstance(candidate, (int, float)):
            candidate = 1 if schema_type == "integer" else 1.0
        minimum = schema.get("minimum")
        maximum = schema.get("maximum")
        if isinstance(minimum, (int, float)) and candidate < minimum:
            candidate = minimum
        if isinstance(maximum, (int, float)) and candidate > maximum:
            candidate = maximum
        return int(candidate) if schema_type == "integer" else float(candidate)
    if schema_type == "boolean":
        candidate = scalar(field, command)
        return candidate if isinstance(candidate, bool) else False
    if schema_type == "string":
        candidate = scalar(field, command)
        if not isinstance(candidate, str):
            candidate = "probe"
        minimum = schema.get("minLength")
        maximum = schema.get("maxLength")
        if isinstance(minimum, int) and len(candidate) < minimum:
            candidate += "-probe" * ((minimum - len(candidate) + 6) // 7)
        if isinstance(maximum, int):
            candidate = candidate[:maximum]
        pattern = schema.get("pattern")
        if isinstance(pattern, str) and "A-Za-z0-9" in pattern:
            candidate = re.sub(r"[^A-Za-z0-9_.:-]", "-", candidate)
        return candidate
    if "minimum" in schema:
        minimum = schema.get("minimum")
        if isinstance(minimum, (int, float)):
            return minimum
    return scalar(field, command)


def default_args(command: dict[str, Any]) -> dict[str, Any]:
    schema = command.get("args_schema")
    if not isinstance(schema, dict):
        return {}
    result = {
        name: example(schema.get("properties", {}).get(name, {}), command["name"], name, schema)
        for name in schema.get("required", [])
    }
    for field in schema.get("x-a4n-context-fields", []):
        result.setdefault(field, scalar(field, command["name"]))
    return result


def _schema_type(schema: dict[str, Any]) -> str | None:
    schema_type = schema.get("type")
    if isinstance(schema_type, list):
        schema_type = next((item for item in schema_type if item != "null"), None)
    return schema_type if isinstance(schema_type, str) else None


def _schema_error(path: str, message: str) -> ValueError:
    return ValueError(f"{path or '/data'}: {message}")


def validate_result_schema(
    schema: Any,
    value: Any,
    root: dict[str, Any],
    path: str = "/data",
) -> None:
    if schema is True or schema is None:
        return
    if schema is False:
        raise _schema_error(path, "schema 禁止该值")
    if not isinstance(schema, dict):
        return
    resolved = resolve_ref(schema, root)
    if resolved is not None:
        validate_result_schema(resolved, value, root, path)
        return
    if "const" in schema and value != schema["const"]:
        raise _schema_error(path, f"应为 const {schema['const']!r}")
    values = schema.get("enum")
    if isinstance(values, list) and values and value not in values:
        raise _schema_error(path, f"不在 enum {values!r} 中")
    for union_name in ("anyOf", "oneOf"):
        options = schema.get(union_name)
        if isinstance(options, list):
            errors = []
            for option in options:
                try:
                    validate_result_schema(option, value, root, path)
                    return
                except ValueError as error:
                    errors.append(str(error))
            raise _schema_error(path, "不匹配任一联合 schema: " + "; ".join(errors[:2]))
    schema_type = _schema_type(schema)
    if value is None:
        if schema_type in {None, "null"} or "null" in (
            schema.get("type") if isinstance(schema.get("type"), list) else []
        ):
            return
        raise _schema_error(path, "不允许 null")
    if schema_type == "null":
        raise _schema_error(path, "应为 null")
    if schema_type == "object" or "properties" in schema:
        if not isinstance(value, dict):
            raise _schema_error(path, "应为 object")
        properties = schema.get("properties", {})
        for name in schema.get("required", []):
            if name not in value:
                raise _schema_error(f"{path}/{name}", "缺少必填字段")
        if isinstance(properties, dict):
            for name, child in properties.items():
                if name in value:
                    validate_result_schema(child, value[name], root, f"{path}/{name}")
        additional_properties = schema.get("additionalProperties", True)
        unknown = [
            name for name in value
            if not isinstance(properties, dict) or name not in properties
        ]
        if additional_properties is False and unknown:
            raise _schema_error(f"{path}/{unknown[0]}", "不允许未声明字段")
        if isinstance(additional_properties, dict):
            for name in unknown:
                validate_result_schema(additional_properties, value[name], root, f"{path}/{name}")
        return
    if schema_type == "array":
        if not isinstance(value, list):
            raise _schema_error(path, "应为 array")
        minimum = schema.get("minItems")
        maximum = schema.get("maxItems")
        if isinstance(minimum, int) and len(value) < minimum:
            raise _schema_error(path, "array 少于 minItems")
        if isinstance(maximum, int) and len(value) > maximum:
            raise _schema_error(path, "array 超过 maxItems")
        if schema.get("uniqueItems") is True:
            seen = set()
            for index, item in enumerate(value):
                marker = json.dumps(
                    item,
                    ensure_ascii=False,
                    sort_keys=True,
                    separators=(",", ":"),
                )
                if marker in seen:
                    raise _schema_error(f"{path}/{index}", "array 包含重复项")
                seen.add(marker)
        for index, item in enumerate(value):
            validate_result_schema(schema.get("items", True), item, root, f"{path}/{index}")
        return
    if schema_type == "boolean" and not isinstance(value, bool):
        raise _schema_error(path, "应为 boolean")
    if schema_type == "integer" and (isinstance(value, bool) or not isinstance(value, int)):
        raise _schema_error(path, "应为 integer")
    if schema_type == "number" and (isinstance(value, bool) or not isinstance(value, (int, float))):
        raise _schema_error(path, "应为 number")
    if schema_type in {"integer", "number"}:
        minimum = schema.get("minimum")
        maximum = schema.get("maximum")
        if isinstance(minimum, (int, float)) and not isinstance(minimum, bool) and value < minimum:
            raise _schema_error(path, f"number 小于 minimum {minimum}")
        if isinstance(maximum, (int, float)) and not isinstance(maximum, bool) and value > maximum:
            raise _schema_error(path, f"number 超过 maximum {maximum}")
        return
    if schema_type == "string":
        if not isinstance(value, str):
            raise _schema_error(path, "应为 string")
        minimum = schema.get("minLength")
        maximum = schema.get("maxLength")
        if isinstance(minimum, int) and len(value) < minimum:
            raise _schema_error(path, "string 少于 minLength")
        if isinstance(maximum, int) and len(value) > maximum:
            raise _schema_error(path, "string 超过 maxLength")
        pattern = schema.get("pattern")
        if isinstance(pattern, str) and re.search(pattern, value) is None:
            raise _schema_error(path, f"string 不匹配 pattern {pattern!r}")


def validate_result_references(descriptor: dict[str, Any], envelope: dict[str, Any]) -> None:
    if not descriptor.get("result_references"):
        return
    meta = envelope.get("meta")
    if not isinstance(meta, dict):
        raise ValueError("/meta: 引用投影要求 object")
    references = meta.get("references")
    if not isinstance(references, dict):
        raise ValueError("/meta/references: 缺少引用投影 object")
    for kind in descriptor["result_references"]:
        values = references.get(kind)
        if values is None:
            continue
        if not isinstance(values, list):
            raise ValueError(f"/meta/references/{kind}: 应为 array")
        for value in values:
            if kind.endswith("_ids") and kind not in {"task_ids", "workflow_run_ids"}:
                if isinstance(value, bool) or not isinstance(value, int) or value <= 0:
                    raise ValueError(f"/meta/references/{kind}: 含无效整数 ID")
            elif kind == "task_ids":
                if not isinstance(value, str) or not re.fullmatch(r"[0-9a-fA-F-]{36}", value):
                    raise ValueError(f"/meta/references/{kind}: 含无效 UUID")
            elif kind == "workflow_run_ids":
                if isinstance(value, bool) or not isinstance(value, int) or value <= 0:
                    raise ValueError(f"/meta/references/{kind}: 含无效 workflow ID")


def load_map(path: Path | None) -> dict[str, Any]:
    if path is None or not path.is_file():
        return {}
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise RunnerError("fixture map 必须是 object")
    return value


PNG_1X1_BASE64 = (
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk"
    "YAAAAAYAAjCB0C8AAAAASUVORK5CYII="
)


def _first_value(value: Any, keys: tuple[str, ...]) -> Any:
    if isinstance(value, dict):
        for key in keys:
            candidate = value.get(key)
            if candidate not in (None, ""):
                return candidate
        for child in value.values():
            candidate = _first_value(child, keys)
            if candidate not in (None, ""):
                return candidate
    elif isinstance(value, list):
        for child in value:
            candidate = _first_value(child, keys)
            if candidate not in (None, ""):
                return candidate
    return None


def _first_video_segment_target(prompt: Any) -> dict[str, Any]:
    if not isinstance(prompt, dict):
        raise RunnerError("fixture 视频 segment plan 记录格式无效")
    for candidate in (prompt.get("metadata_json"), prompt.get("visual_prompt")):
        if isinstance(candidate, str):
            try:
                candidate = json.loads(candidate)
            except json.JSONDecodeError:
                continue
        if not isinstance(candidate, dict) or not isinstance(candidate.get("segments"), list):
            continue
        recipes = {
            recipe.get("segment_number"): recipe
            for recipe in candidate.get("segment_recipes", [])
            if isinstance(recipe, dict) and isinstance(recipe.get("segment_number"), int)
        }
        segments = sorted(
            (segment for segment in candidate["segments"] if isinstance(segment, dict)),
            key=lambda segment: (
                segment.get("segment_number", 0),
                segment.get("source_shot_id", ""),
            ),
        )
        for segment in segments:
            source_shot_id = segment.get("source_shot_id")
            segment_index = segment.get("segment_index_in_shot")
            recipe = recipes.get(segment.get("segment_number"))
            workflow_mode = recipe.get("workflow_mode") if isinstance(recipe, dict) else None
            prompt_output_language = (
                recipe.get("prompt_output_language") if isinstance(recipe, dict) else None
            )
            if (
                isinstance(source_shot_id, str)
                and source_shot_id.strip()
                and isinstance(segment_index, int)
                and not isinstance(segment_index, bool)
                and segment_index > 0
                and isinstance(workflow_mode, str)
                and workflow_mode.strip()
                and isinstance(prompt_output_language, str)
                and prompt_output_language.strip()
            ):
                return {
                    "source_shot_id": source_shot_id,
                    "segment_index_in_shot": segment_index,
                    "workflow_mode": workflow_mode,
                    "prompt_output_language": prompt_output_language,
                    "recipe": dict(recipe),
                }
    raise RunnerError("fixture 视频 segment plan 未返回可执行的目标片段")


def _replace_fixture_ids(value: Any, key: str, state: dict[str, Any], command: str) -> Any:
    normalized = key.lower().replace("-", "_")
    mapping: dict[str, str] = {
        "projectid": "project_id",
        "project_id": "project_id",
        "root_project_id": "project_id",
        "collaboration_root_project_id": "project_id",
        "derivedprojectid": "derived_project_id",
        "derived_project_id": "derived_project_id",
        "source_chapter_id": "chapter_id",
        "chapterid": "chapter_id",
        "chapter_id": "chapter_id",
        "profileid": "profile_id",
        "profile_id": "profile_id",
        "character_profile_id": "profile_id",
        "object_profile_id": "object_profile_id",
        "scene_profile_id": "scene_profile_id",
        "foreshadowing_id": "foreshadowing_id",
        "themeid": "theme_id",
        "theme_id": "theme_id",
        "record_id": "visual_plan_id",
        "visual_plan_id": "visual_plan_id",
        "expected_revision": "visual_plan_revision",
        "sync_runid": "sync_run_id",
        "sync_run_id": "sync_run_id",
        "inviteid": "invite_id",
        "invite_id": "invite_id",
        "configid": "config_id",
        "config_id": "config_id",
        "graph_id": "graph_id",
        "text_asset_id": "text_asset_id",
        "segment_plan_prompt_id": "segment_plan_prompt_id",
        "video_prompt_id": "video_prompt_id",
        "prompt_id": "prompt_id",
        "videoid": "video_id",
        "video_id": "video_id",
        "workflow_run_id": "workflow_run_id",
        "workflowrunid": "workflow_run_id",
        "taskid": "task_id",
        "task_id": "task_id",
    }
    state_key = mapping.get(normalized)
    if state_key == "config_id":
        if "image" in command or "galgame" in command:
            state_key = "image_config_id"
        elif "video" in command:
            state_key = "video_config_id"
        elif "embedding" in command or "rag" in command:
            state_key = "embedding_config_id"
        else:
            state_key = "llm_config_id"
    if state_key == "project_id":
        if "canvas" in command:
            state_key = "canvas_project_id"
        elif "galgame" in command:
            state_key = "galgame_project_id"
    if state_key == "prompt_id":
        state_key = "manga_prompt_id" if "manga" in command else "video_prompt_id"
    candidate = state.get(state_key) if state_key else None
    if candidate is None:
        return value
    if value in {PROBE_ID, PROBE_TASK_ID, PROBE_WORKFLOW_ID, "probe", "manifest probe"}:
        return candidate
    return value


def _replace_fixture_values(value: Any, state: dict[str, Any], command: str, key: str = "") -> Any:
    if isinstance(value, dict):
        return {
            name: _replace_fixture_values(item, state, command, name)
            for name, item in value.items()
        }
    if isinstance(value, list):
        return [_replace_fixture_values(item, state, command, key) for item in value]
    return _replace_fixture_ids(value, key, state, command)


def fixture_payload(
    command: str,
    payload: dict[str, Any],
    state: dict[str, Any],
    run_id: str,
    provider_url: str,
    customize: bool,
) -> dict[str, Any]:
    result = _replace_fixture_values(payload, state, command)
    if not customize:
        return result
    input_value = result.get("input")
    if not isinstance(input_value, dict):
        input_value = None

    if command == "update_chapter_video_segment_recipe":
        prompt_id = state.get("segment_plan_prompt_id")
        recipe = state.get("video_segment_recipe")
        if not isinstance(prompt_id, int) or prompt_id <= 0 or not isinstance(recipe, dict):
            raise RunnerError("fixture 视频 segment recipe 缺少已发布规划状态")
        return {"promptId": prompt_id, "recipe": dict(recipe)}

    if command in {
        "preview_import_project_analysis_request",
        "run_import_project_analysis",
        "get_project_import_analysis_progress",
    }:
        result["projectId"] = state.get("project_id", result.get("projectId"))

    if command == "create_project" and input_value is not None:
        input_value.update(
            title=f"A4N manifest {run_id}",
            description="Disposable project created by the manifest CLI E2E suite.",
            project_type="novel_long",
        )
    elif command == "create_profile" and input_value is not None:
        input_value.update(
            name="周衡",
            base_attributes={
                "explicit_attributes": {},
                "implicit_attributes": {},
                "social_attributes": {},
            },
        )
    elif command == "create_object_profile" and input_value is not None:
        profile = input_value.setdefault("profile", {})
        profile.update(
            name=f"齿轮暗记-{run_id[-8:]}",
            object_type="token",
            importance="primary",
            visual_description="未来来信信封角落的齿轮形暗记。",
            material_and_state="墨迹清晰，年代和来源未明。",
            story_usage="连接未来来信与十年前实验秘密的线索。",
            visual_usage="固定信封角落的视觉线索位置。",
            generation_usage="保持齿轮暗记位于未来来信信封角落。",
            first_seen_chapter=1,
            last_seen_chapter=1,
            image_prompt_seed="未来来信信封角落的齿轮暗记",
        )
    elif command == "create_scene_profile" and input_value is not None:
        profile = input_value.setdefault("profile", {})
        profile.update(
            name=f"Manifest scene {run_id}",
            scene_type="room",
            importance="primary",
            base_description="A quiet railway waiting room at night.",
            visual_description="Rain on tall windows and a single clock.",
            spatial_layout="One room, benches along the walls, clock above the door.",
            atmosphere="Tense and wet after a storm.",
            first_seen_chapter=1,
            last_seen_chapter=1,
            image_prompt_seed="rainy railway waiting room at night",
        )
    elif command == "create_foreshadowing" and input_value is not None:
        input_value.update(
            project_id=state.get("project_id", input_value.get("project_id")),
            record={
                "description": "The station clock loses seven minutes every night.",
                "setup_chapter": 1,
                "status": "pending",
                "importance": 3,
            },
        )
    elif command in {"create_theme", "import_theme"} and input_value is not None:
        input_value.update(
            name=f"Manifest theme {run_id}-{command}",
            mode="light",
            document={
                "version": 1,
                "tokens": {"--color-accent": "#334155"},
                "backgrounds": {},
            },
        )
    elif command == "create_llm_config" and input_value is not None:
        input_value.update(
            name=f"Manifest LLM {run_id}",
            provider="openai",
            base_url=provider_url,
            model="a4n-virtual-text",
            max_concurrent=2,
            timeout_secs=60,
            max_context_tokens=128000,
            max_output_tokens=8192,
            api_key_encrypted="a4n-e2e-provider-key",
        )
    elif command == "create_embedding_config" and input_value is not None:
        input_value.update(
            name=f"Manifest embedding {run_id}",
            backend="api",
            base_url=provider_url,
            model="a4n-virtual-embedding",
            dimension=8,
            batch_size=32,
            api_key_encrypted="a4n-e2e-provider-key",
        )
    elif command == "add_image_config" and input_value is not None:
        input_value.update(
            name=f"Manifest image {run_id}",
            backend="openai-compatible-image-edit",
            base_url=provider_url,
            model="a4n-virtual-image",
            max_concurrent=2,
            supported_sizes=[{"width": 1024, "height": 1024}],
            api_key_encrypted="a4n-e2e-provider-key",
        )
    elif command == "add_video_config" and input_value is not None:
        input_value.update(
            name=f"Manifest video {run_id}",
            backend="openai-compatible-video",
            base_url=provider_url,
            model="a4n-virtual-video",
            max_concurrent=2,
            default_width=1280,
            default_height=720,
            default_duration_secs=1,
            min_duration_secs=1,
            max_duration_secs=10,
            duration_mode="range",
            default_fps=24,
            api_key_encrypted="a4n-e2e-provider-key",
        )
    elif command == "create_derived_project" and input_value is not None:
        input_value.update(
            root_project_id=state.get("project_id", input_value.get("root_project_id")),
            derived_kind="galgame",
            title=f"Manifest Galgame {run_id}",
            description="Disposable Galgame derived project for CLI coverage.",
            start_chapter=1,
            end_chapter=1,
        )
    elif command == "duplicate_project" and input_value is not None:
        input_value.update(
            project_id=state.get("project_id", input_value.get("project_id")),
            new_title=f"Manifest duplicate {run_id}",
        )
    elif command == "import_project_text_upload" and input_value is not None:
        input_value.update(
            file_name=f"manifest-{run_id}.txt",
            data_base64=base64.b64encode(
                f"第一章\n{chapter_body_response()}\n".encode("utf-8")
            ).decode("ascii"),
        )
    elif command in {"import_chapter", "update_chapter_content"} and input_value is not None:
        input_value["content"] = chapter_body_response()
    elif command == "upload_image" and input_value is not None:
        input_value.update(
            file_name=f"manifest-{run_id}.png",
            mime_type="image/png",
            data_base64=PNG_1X1_BASE64,
            image_type="illustration",
            prompt="A tiny deterministic test image.",
            metadata_json={},
        )
    elif command == "upload_portrait" and input_value is not None:
        input_value.update(
            file_name=f"manifest-{run_id}.png",
            mime_type="image/png",
            data_base64=PNG_1X1_BASE64,
            portrait_type="face",
            prompt="A tiny deterministic portrait fixture.",
            set_primary=False,
        )
    elif command == "upload_profile_material_image" and input_value is not None:
        input_value.update(
            file_name=f"manifest-{run_id}.png",
            mime_type="image/png",
            data_base64=PNG_1X1_BASE64,
            profile_kind="object",
            prompt_snapshot="A tiny deterministic material fixture.",
            set_primary=False,
        )
    elif command == "save_blueprint" and input_value is not None:
        input_value.update(
            project_id=state.get("project_id", input_value.get("project_id")),
            genre="都市时间悬疑",
            total_chapters=1,
            one_sentence_summary="一座车站的钟每天少走七分钟。",
            full_synopsis="管理员 CLI E2E 使用的最小蓝图。",
        )
    elif command == "save_canvas_graph" and input_value is not None:
        input_value.update(
            project_id=state.get("canvas_project_id", input_value.get("project_id")),
            graph={"name": f"Manifest graph {run_id}", "graph_kind": "video_studio", "nodes": [], "edges": []},
        )
    elif command == "save_canvas_text_asset" and input_value is not None:
        input_value.update(
            project_id=state.get("canvas_project_id", input_value.get("project_id")),
            text_asset={"title": "Manifest text", "text_kind": "freeform", "content": "A4N CLI fixture."},
        )
    elif command == "import_canvas_media" and input_value is not None:
        input_value.update(
            project_id=state.get("canvas_project_id", input_value.get("project_id")),
            media_kind="image",
            file_name=f"manifest-{run_id}.png",
            mime_type="image/png",
            data_base64=PNG_1X1_BASE64,
            file_path=None,
        )
    elif command == "confirm_chapter_visual_plan" and input_value is not None:
        input_value.update(
            project_id=state.get("project_id", input_value.get("project_id")),
            source_chapter_id=state.get("chapter_id", input_value.get("source_chapter_id")),
            record_id=state.get("visual_plan_id", input_value.get("record_id")),
            expected_revision=state.get("visual_plan_revision", input_value.get("expected_revision")),
        )
    elif command == "start_chapter_shooting_script_generation" and input_value is not None:
        input_value.update(
            project_id=state.get("project_id", input_value.get("project_id")),
            source_chapter_id=state.get("chapter_id", input_value.get("source_chapter_id")),
            visual_plan_id=state.get("visual_plan_id", input_value.get("visual_plan_id")),
            mode="full",
        )
    elif command == "start_manga_pipeline" and input_value is not None:
        input_value.update(
            project_id=state.get("project_id", input_value.get("project_id")),
            chapter_number=state.get("chapter_number", 1),
            image_size=input_value.get("image_size") or {"width": 1024, "height": 1024},
        )
    elif command == "start_galgame_pipeline" and input_value is not None:
        input_value.update(
            project_id=state.get("galgame_project_id", input_value.get("project_id")),
            task_id=f"manifest-galgame-{run_id}",
            resume_from_checkpoint=False,
            start_from_stage=None,
        )
    elif command == "plan_galgame_assets" and input_value is not None:
        input_value["project_id"] = state.get("galgame_project_id", input_value.get("project_id"))
    elif command == "run_galgame_asset_generation" and input_value is not None:
        input_value.update(
            project_id=state.get("galgame_project_id", input_value.get("project_id")),
            config_id=state.get("image_config_id"),
            generation_spec_ids=state.get("generation_spec_ids", []),
            limit=1,
        )
    elif command in {"get_galgame_pipeline_status", "get_galgame_script"} and input_value is not None:
        input_value["project_id"] = state.get("galgame_project_id", input_value.get("project_id"))
    elif command in {"get_manga_pipeline_status", "list_manga_page_prompts"} and input_value is not None:
        input_value.update(
            project_id=state.get("project_id", input_value.get("project_id")),
            chapter_id=state.get("chapter_id", input_value.get("chapter_id")),
        )
    elif command == "start_video_generation" and input_value is not None:
        if state.get("video_prompt_id"):
            operation = {
                "kind": "chapter",
                "chapter_id": state.get("chapter_id"),
                "prompt_id": state["video_prompt_id"],
                "config_id": state.get("video_config_id"),
            }
        elif state.get("segment_plan_prompt_id"):
            target_segment = state.get("video_target_segment")
            workflow_mode = state.get("video_workflow_mode")
            prompt_output_language = state.get("video_prompt_output_language")
            if (
                not isinstance(target_segment, dict)
                or not isinstance(workflow_mode, str)
                or not isinstance(prompt_output_language, str)
            ):
                raise RunnerError("fixture 视频 segment plan 缺少目标片段")
            operation = {
                "kind": "prompt_videos",
                "chapter_id": state.get("chapter_id"),
                "options": {
                    "workflow_mode": workflow_mode,
                    "style": "电影感",
                    "prompt_output_language": prompt_output_language,
                    "prompt_batch_size": 1,
                    "duration_secs": 6,
                    "duration_secs_options": [6],
                    "aspect_ratio": "16:9",
                    "reference_image_ids": [],
                    "video_library_image_ids": [],
                    "regenerate_existing": False,
                    "segment_plan_prompt_id": state["segment_plan_prompt_id"],
                    "target_segment": target_segment,
                },
            }
        else:
            operation = {
                "kind": "prompt_segment_plan",
                "chapter_id": state.get("chapter_id"),
                "options": {
                    "workflow_mode": "shared_segment_plan",
                    "style": "电影感",
                    "prompt_output_language": "zh_cn",
                    "prompt_batch_size": 1,
                    "duration_secs": 6,
                    "duration_secs_options": [6],
                    "aspect_ratio": "16:9",
                    "reference_image_ids": [],
                    "video_library_image_ids": [],
                    "regenerate_existing": False,
                },
            }
        input_value.update(
            meta={"start_request_id": f"manifest-video-{run_id}"},
            project_id=state.get("project_id", input_value.get("project_id")),
            operation=operation,
        )
    elif command == "list_import_queue" and input_value is not None:
        input_value["project_id"] = state.get("project_id", input_value.get("project_id"))
    elif command == "sync_character_from_chapters" and input_value is not None:
        input_value.update(
            project_id=state.get("project_id", input_value.get("project_id")),
            name="林岚",
            chapter_number=state.get("chapter_number", 1),
        )
        result["taskId"] = f"manifest-character-{run_id}"
    elif command == "sync_all_character_profiles" and input_value is not None:
        input_value.update(
            project_id=state.get("project_id", input_value.get("project_id")),
            chapter_number=state.get("chapter_number", 1),
        )
        result["taskId"] = f"manifest-character-batch-{run_id}"
    elif command == "sync_scene_profiles" and input_value is not None:
        input_value.update(
            project_id=state.get("project_id", input_value.get("project_id")),
            chapter_number=state.get("chapter_number", 1),
        )
        result["taskId"] = f"manifest-scene-{run_id}"
    elif command == "invite_project_member" and input_value is not None:
        input_value.update(
            username_or_email=state.get("collaborator_identity", input_value.get("username_or_email")),
            role="viewer",
        )
    elif command == "start_inspiration_generation" and input_value is not None:
        input_value.update(
            project_id=state.get("project_id", input_value.get("project_id")),
            message="写一段关于暴雨夜车站和慢七分钟时钟的悬疑灵感。",
            meta={"start_request_id": str(uuid.uuid4())},
        )
    elif command == "start_blueprint_generation" and input_value is not None:
        input_value.update(
            project_id=state.get("project_id", input_value.get("project_id")),
            title="逆行时钟",
            genre="都市时间悬疑",
            target_length=5,
            meta={"start_request_id": str(uuid.uuid4())},
        )
    elif command == "start_outline_generation" and input_value is not None:
        input_value.update(
            project_id=state.get("project_id", input_value.get("project_id")),
            meta={"start_request_id": str(uuid.uuid4())},
            operation={"kind": "generate_parts", "part_count": 1, "chapters_per_part": 1},
        )
    elif command == "start_chapter_generation" and input_value is not None:
        input_value.update(
            project_id=state.get("project_id", input_value.get("project_id")),
            chapter_number=1,
            meta={"start_request_id": str(uuid.uuid4())},
            min_word_count=100,
            max_word_count=300,
        )
    return result


def redact(value: Any, key: str = "") -> Any:
    normalized = key.lower().replace("-", "_")
    if (
        normalized in {
            "password",
            "old_password",
            "new_password",
            "api_key",
            "api_key_encrypted",
            "access_token",
            "refresh_token",
            "agent_token",
            "session_token",
            "authorization",
            "oauth_code",
            "client_secret",
        }
        or normalized.endswith("_secret")
        or normalized.endswith("_token")
    ):
        return "[REDACTED]"
    if isinstance(value, dict):
        return {name: redact(item, name) for name, item in value.items()}
    if isinstance(value, list):
        return [redact(item, key) for item in value]
    return value


def safe_probe_payload(
    payload: dict[str, Any],
    effect: str,
    execute_writes: bool,
) -> dict[str, Any]:
    if effect == "write" and not execute_writes:
        # Keep the shared-database smoke read-only. This intentionally checks
        # the CLI/server validation path; business write coverage belongs to
        # the isolated fixture run.
        return {}
    return payload


def auth_invocation(
    command: str,
    payload: dict[str, Any],
    allow_destructive: bool,
) -> tuple[list[str], str | None]:
    input_value = payload.get("input", {})
    if not isinstance(input_value, dict):
        input_value = {}
    if command == "login_user":
        return ["auth", "login", "--username", str(input_value.get("username", "")), "--password-stdin"], f"{input_value.get('password', '')}\n"
    if command == "register_user":
        return [
            "auth", "register", "--username", str(input_value.get("username", "")),
            "--email", str(input_value.get("email", "")),
            "--invite-code", str(input_value.get("invite_code", "")),
            "--password-stdin",
        ], f"{input_value.get('password', '')}\n"
    if command == "refresh_token":
        return ["auth", "refresh"], None
    if command == "change_user_password":
        return ["auth", "change-password", "--passwords-stdin"], f"{input_value.get('old_password', '')}\n{input_value.get('new_password', '')}\n"
    if command == "oauth2_callback":
        return ["auth", "oauth-callback", str(input_value.get("provider", "")), "--code-stdin"], f"{input_value.get('code', '')}\n"
    if command == "revoke_session":
        parts = ["auth", "revoke-session", str(input_value.get("session_id", ""))]
        if allow_destructive:
            parts.append("--yes")
        return parts, None
    raise RunnerError(f"不支持的 auth command: {command}")


AUTH_CLI_REQUIRED_FIELDS: dict[str, tuple[str, ...]] = {
    "login_user": ("server", "user", "session"),
    "register_user": ("server", "user", "session"),
    "refresh_token": ("refreshed", "session_id", "token_type", "expires_in"),
    "change_user_password": ("changed",),
    "oauth2_callback": ("server", "user", "session"),
    "revoke_session": ("revoked",),
}


def validate_auth_cli_result(command: str, value: Any) -> None:
    required = AUTH_CLI_REQUIRED_FIELDS.get(command)
    if required is None:
        return
    if not isinstance(value, dict):
        raise ValueError("/data: auth CLI 输出应为 object")
    for field in required:
        if field not in value:
            raise ValueError(f"/data/{field}: CLI auth 输出缺少字段")


def invoke(
    cli: Cli,
    command: str,
    payload: dict[str, Any],
    allow_destructive: bool,
    descriptor: dict[str, Any] | None = None,
    extra_parts: list[str] | None = None,
) -> dict[str, Any]:
    temp_config: Path | None = None
    invoke_cli = cli
    stdin: str | None = None
    if command in AUTH_COMMANDS:
        fd, temp_name = tempfile.mkstemp(prefix="a4n-auth-probe-", suffix=".json")
        os.close(fd)
        temp_config = Path(temp_name)
        shutil.copy2(cli.config, temp_config)
        invoke_cli = Cli(cli.executable, cli.server, temp_config, cli.context, cli.timeout, cli.env)
        parts, stdin = auth_invocation(command, payload, allow_destructive)
    else:
        parts = ["call", command, "--args", json.dumps(payload, ensure_ascii=False)]
        parts += extra_parts or []
        if allow_destructive:
            parts.append("--allow-destructive")
    try:
        raw = invoke_cli.run(parts, stdin)
    finally:
        if temp_config:
            temp_config.unlink(missing_ok=True)
            temp_config.with_name(temp_config.name + ".lock").unlink(missing_ok=True)
    envelope = raw["stdout_documents"][-1] if raw["stdout_documents"] else None
    ok = raw["exit_code"] == 0 and isinstance(envelope, dict) and envelope.get("ok") is True
    result: dict[str, Any] = {
        "exit_code": raw["exit_code"],
        "duration_ms": raw["duration_ms"],
        "ok": ok,
        "data": envelope.get("data") if isinstance(envelope, dict) else None,
        "meta": envelope.get("meta") if isinstance(envelope, dict) else None,
    }
    if not ok:
        result["error"] = command_error(raw)
    elif command in AUTH_CLI_REQUIRED_FIELDS:
        try:
            validate_auth_cli_result(command, result["data"])
        except ValueError as error:
            result["ok"] = False
            result["error"] = {
                "code": "invalid_cli_result",
                "kind": "client",
                "message": str(error),
            }
    elif descriptor:
        try:
            validate_result_schema(descriptor.get("result_schema"), result["data"], descriptor.get("result_schema", {}))
            validate_result_references(descriptor, envelope)
        except ValueError as error:
            result["ok"] = False
            result["error"] = {
                "code": "invalid_result_schema",
                "kind": "client",
                "message": str(error),
            }
    result["classification"] = classify(result, descriptor)
    return result


def classify(result: dict[str, Any], descriptor: dict[str, Any] | None = None) -> str:
    if result.get("ok") is True:
        if descriptor and descriptor.get("effect") == "destructive":
            return "destructive_success"
        return "business_success"
    error = result.get("error") or {}
    code = error.get("code") if isinstance(error, dict) else None
    status = error.get("http_status") if isinstance(error, dict) else None
    message = error.get("message", "") if isinstance(error, dict) else str(error)
    if result.get("exit_code") == 124 or code in {None, "invalid_cli_output"}:
        return "real_failure"
    if code == "invalid_result_schema":
        return "real_failure"
    if any(marker in message for marker in ("数据库错误", "No deterministic fixture matched", "LLM API 返回错误", "provider_job_failed", "内部错误")):
        return "real_failure"
    if status in {401, 403} or code in {"forbidden", "unauthorized", "permission_denied"}:
        return "permission_rejection"
    if isinstance(status, int) and 400 <= status < 500:
        return "expected_state_rejection"
    if code in {
        "destructive_confirmation_required",
        "invalid_input",
        "invalid_request",
        "invalid_argument",
        "not_found",
        "invalid_state",
        "conflict",
        "forbidden",
        "unauthorized",
        "unsupported",
        "capability_unavailable",
    } or result.get("exit_code") in {2, 3, 4, 5, 6, 10}:
        return "expected_state_rejection"
    return "real_failure"


def fixture_result_classification(result: dict[str, Any]) -> str:
    """Fixture setup errors fail the run; only successful setup calls pass."""
    return result["classification"] if result.get("ok") is True else "real_failure"


def manifest_data(value: Any) -> dict[str, Any]:
    data = value.get("data", value) if isinstance(value, dict) else None
    if not isinstance(data, dict) or not isinstance(data.get("commands"), list):
        raise RunnerError("manifest 缺少 data.commands")
    for descriptor in data["commands"]:
        if not isinstance(descriptor, dict) or not isinstance(descriptor.get("name"), str):
            raise RunnerError("manifest 含有无效 command descriptor")
    return data


def _provider_entries(path: Path | None) -> list[dict[str, Any]]:
    if path is None or not path.is_file():
        return []
    entries: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        try:
            value = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(value, dict):
            entries.append(value)
    return entries


def _provider_summary(
    path: Path | None,
    preflight_boundary: int | None,
    fixture_boundary: int | None,
) -> dict[str, Any]:
    entries = _provider_entries(path)
    preflight_boundary = (
        len(entries)
        if preflight_boundary is None
        else min(max(preflight_boundary, 0), len(entries))
    )
    fixture_boundary = (
        len(entries)
        if fixture_boundary is None
        else min(max(fixture_boundary, preflight_boundary), len(entries))
    )

    def count(items: list[dict[str, Any]]) -> dict[str, Any]:
        paths: dict[str, int] = {}
        for item in items:
            name = str(item.get("path", "<missing>"))
            paths[name] = paths.get(name, 0) + 1
        return {"requests": len(items), "paths": dict(sorted(paths.items()))}

    return {
        "total": count(entries),
        "preflight": count(entries[:preflight_boundary]),
        "fixture": count(entries[preflight_boundary:fixture_boundary]),
        "manifest": count(entries[fixture_boundary:]),
    }


def _provider_delta(path: Path | None, start: int) -> dict[str, Any]:
    entries = _provider_entries(path)
    paths: dict[str, int] = {}
    for item in entries[max(start, 0):]:
        name = str(item.get("path", "<missing>"))
        paths[name] = paths.get(name, 0) + 1
    return {"requests": sum(paths.values()), "paths": dict(sorted(paths.items()))}


REQUIRED_FIXTURE_PROVIDER_PATHS = frozenset(
    {
        "/v1/chat/completions",
        "/v1/images/generations",
        "/v1/videos/generations",
    }
)


FIXTURE_PATHS: dict[str, tuple[str, ...]] = {
    "novel_authoring": (
        "create_project",
        "import_project_text_upload",
        "accept_import_review",
        "start_inspiration_generation",
        "start_blueprint_generation",
        "start_outline_generation",
        "list_parts",
        "list_chapter_outlines",
        "start_chapter_generation",
        "list_chapters",
    ),
    "chapter_analysis_and_visual": (
        "generate_chapter_analysis",
        "save_chapter_visual_plan_draft",
        "confirm_chapter_visual_plan",
        "get_chapter_visual_plan",
        "start_chapter_shooting_script_generation",
        "get_chapter_shooting_script",
    ),
    "profiles_and_materials": (
        "create_profile",
        "create_object_profile",
        "create_scene_profile",
        "create_foreshadowing",
        "upload_portrait",
        "upload_profile_material_image",
        "sync_character_from_chapters",
        "sync_all_character_profiles",
        "sync_scene_profiles",
    ),
    "rag": (
        "rag_prepare_ingest_all",
        "rag_ingest_all",
        "rag_prepare_enhanced_search",
        "rag_enhanced_search",
    ),
    "provider_and_image_assets": (
        "create_llm_config",
        "create_embedding_config",
        "add_image_config",
        "add_video_config",
        "upload_image",
    ),
    "canvas": (
        "create_freeform_video_project",
        "save_canvas_graph",
        "save_canvas_text_asset",
        "import_canvas_media",
    ),
    "derived_project_and_sync": (
        "create_derived_project",
        "preview_source_sync",
        "apply_source_sync_decisions",
        "duplicate_project",
    ),
    "galgame": (
        "start_galgame_pipeline",
        "get_galgame_pipeline_status",
        "get_galgame_script",
        "plan_galgame_assets",
        "run_galgame_asset_generation",
    ),
    "manga": (
        "start_manga_pipeline",
        "get_manga_pipeline_status",
        "list_manga_page_prompts",
    ),
    "video": (
        "start_video_generation",
        "list_chapter_video_prompts",
        "update_chapter_video_segment_recipe",
        "list_videos_by_chapter",
        "poll_video_job",
    ),
    "collaboration": (
        "list_all_users",
        "invite_project_member",
        "list_project_invites",
        "list_my_project_invites",
    ),
}


def fixture_path_coverage(
    records: list[dict[str, Any]],
    collaborator_enabled: bool = False,
) -> dict[str, Any]:
    """Summarize successful business paths without counting manifest probes."""
    successful_commands = {
        str(record.get("command"))
        for record in records
        if record.get("ok") is True and record.get("command") != "task watch"
    }
    paths: dict[str, dict[str, Any]] = {}
    for name, required in FIXTURE_PATHS.items():
        required_set = set(required)
        missing = sorted(required_set - successful_commands)
        paths[name] = {
            "status": "passed" if not missing else "failed",
            "required_commands": list(required),
            "observed_commands": sorted(required_set & successful_commands),
            "missing_commands": missing,
        }
    if collaborator_enabled:
        command = "accept_project_invite"
        paths["collaboration_accept"] = {
            "status": "passed" if command in successful_commands else "failed",
            "required_commands": [command],
            "observed_commands": [command] if command in successful_commands else [],
            "missing_commands": [] if command in successful_commands else [command],
        }
    return {
        "successful_commands": sorted(successful_commands),
        "task_watch_count": sum(record.get("command") == "task watch" for record in records),
        "paths": paths,
        "complete": all(path["status"] == "passed" for path in paths.values()),
    }


def missing_fixture_provider_paths(path: Path | None, fixture_start: int) -> list[str]:
    """Return provider routes not exercised by the isolated business fixture."""
    if path is None:
        return []
    observed = {
        str(entry.get("path"))
        for entry in _provider_entries(path)[max(fixture_start, 0) :]
    }
    return sorted(REQUIRED_FIXTURE_PROVIDER_PATHS - observed)


def runtime_snapshot_for_fixture(base_url: str, state: dict[str, Any]) -> dict[str, Any]:
    snapshot = runtime_snapshot(base_url)
    for section, state_key in (
        ("llm", "llm_config_id"),
        ("image", "image_config_id"),
        ("video", "video_config_id"),
        ("embedding", "embedding_config_id"),
    ):
        config_id = state.get(state_key)
        if not isinstance(config_id, int) or config_id <= 0:
            continue
        snapshot[section]["id"] = config_id
        configs = snapshot.get(f"{section}_configs")
        if isinstance(configs, list) and configs and isinstance(configs[0], dict):
            configs[0]["id"] = config_id
    return snapshot


def ordered_descriptors(commands: list[dict[str, Any]]) -> list[dict[str, Any]]:
    def phase(descriptor: dict[str, Any]) -> int:
        if descriptor.get("effect") == "destructive":
            return 4
        if descriptor.get("execution") in {"task", "workflow"}:
            return 2
        if descriptor.get("effect") == "write":
            return 1
        return 3

    return sorted(commands, key=lambda descriptor: (phase(descriptor), descriptor["name"]))


def output_parts(
    command: str,
    descriptor: dict[str, Any],
    work_dir: Path,
) -> tuple[list[str], Path | None]:
    if not descriptor.get("file_outputs"):
        return [], None
    binary = command in {
        "export_epub_data",
        "export_pdf_data",
        "export_project_family_data",
        "export_video_studio_project_data",
        "export_galgame_webgal",
    }
    suffix = ".bin" if binary else ".txt"
    path = work_dir / "outputs" / f"{command}{suffix}"
    path.parent.mkdir(parents=True, exist_ok=True)
    option = "--save-base64" if binary else "--save-text"
    return [option, f"/data={path}"], path


def _watch_seed_task(cli: Cli, task_id: str, records: list[dict[str, Any]], label: str) -> None:
    raw = cli.run(["task", "watch", task_id, "--attempt", "1"])
    documents = raw["stdout_documents"]
    passed = (
        raw["exit_code"] == 0
        and bool(documents)
        and all(isinstance(document, dict) and document.get("ok") is True for document in documents)
    )
    records.append(
        {
            "command": "task watch",
            "label": label,
            "task_id": task_id,
            "ok": passed,
            "exit_code": raw["exit_code"],
            "duration_ms": raw["duration_ms"],
            "events": len(documents),
        }
    )
    if not passed:
        raise RunnerError(f"fixture {label} task watch 失败: {command_error(raw)}")


SYNCHRONOUS_TASK_COMMANDS = frozenset(
    {
        "generate_chapter_analysis",
        "sync_character_from_chapters",
        "sync_all_character_profiles",
        "sync_scene_profiles",
    }
)


def seed_fixtures(
    cli: Cli,
    descriptors: dict[str, dict[str, Any]],
    run_id: str,
    provider_url: str,
    state: dict[str, Any],
    records: list[dict[str, Any]],
    allow_destructive: bool = False,
    current_username: str | None = None,
    collaborator_username: str | None = None,
    collaborator_password: str | None = None,
    refresh_runtime: Callable[[dict[str, Any]], None] | None = None,
) -> None:
    def call(
        command: str,
        payload: dict[str, Any],
        destructive: bool = False,
        client: Cli = cli,
    ) -> dict[str, Any]:
        descriptor = descriptors.get(command)
        if descriptor is None:
            raise RunnerError(f"manifest 缺少 fixture command: {command}")
        result = invoke(client, command, payload, destructive and allow_destructive, descriptor)
        records.append(
            {
                "command": command,
                "ok": result["ok"],
                # A fixture failure is never an expected probe rejection. The
                # manifest probe deliberately sends stale IDs; fixture setup
                # must instead fail the run even when the Server returns the
                # same state/error code.
                "classification": fixture_result_classification(result),
                "duration_ms": result["duration_ms"],
                "payload": redact(payload),
                "error": redact(result.get("error")) if result.get("error") else None,
            }
        )
        if not result["ok"]:
            raise RunnerError(f"fixture {command} 失败: {result.get('error')}")
        return result

    def task(command: str, payload: dict[str, Any]) -> dict[str, Any]:
        result = call(command, payload)
        if command in SYNCHRONOUS_TASK_COMMANDS:
            return result
        task_id = _first_value(result.get("data"), ("task_id",))
        if not isinstance(task_id, str):
            task_id = _first_value(result.get("meta"), ("task_ids",))
        if not isinstance(task_id, str):
            task_id = payload.get("taskId")
        if not isinstance(task_id, str) or not task_id:
            raise RunnerError(f"fixture {command} 未返回有效 task_id")
        state["task_id"] = task_id
        _watch_seed_task(cli, task_id, records, command)
        return result

    def wait_for_manga() -> None:
        for _ in range(60):
            status = call(
                "get_manga_pipeline_status",
                {"input": {"project_id": project_id, "chapter_id": chapter_id}},
            )
            value = status.get("data")
            if isinstance(value, dict):
                pipeline_status = value.get("pipeline_status")
                if pipeline_status in {"failed", "cancelled"}:
                    raise RunnerError(f"fixture manga pipeline 失败: {value}")
                if pipeline_status == "completed" or (
                    isinstance(value.get("total"), int)
                    and value["total"] > 0
                    and value.get("completed") == value["total"]
                ):
                    return
            time.sleep(0.25)
        raise RunnerError("fixture manga pipeline 轮询超时")

    def poll_video(video_id: int) -> dict[str, Any]:
        for _ in range(8):
            result = call("poll_video_job", {"videoId": video_id})
            value = result.get("data")
            if isinstance(value, dict):
                if value.get("status") == "completed":
                    if not value.get("file_path") or not isinstance(value.get("duration_secs"), (int, float)) or value["duration_secs"] <= 0:
                        raise RunnerError(f"fixture video completed without a deliverable file/duration: {value}")
                    return result
                if value.get("status") in {"failed", "cancelled"}:
                    raise RunnerError(f"fixture video job 失败: {value}")
            time.sleep(0.25)
        raise RunnerError("fixture video job 轮询超时")

    project = call(
        "create_project",
        {
            "input": {
                "title": f"A4N fixture {run_id}",
                "description": "Disposable project for manifest-wide CLI coverage.",
                "project_type": "novel_long",
            }
        },
    )
    project_id = _first_value(project.get("data"), ("id",))
    if not isinstance(project_id, int) or project_id <= 0:
        raise RunnerError("fixture create_project 未返回有效 project id")
    state["project_id"] = project_id

    canvas_project = call(
        "create_freeform_video_project",
        {
            "input": {
                "title": f"A4N canvas fixture {run_id}",
                "description": "Disposable video project for canvas CLI coverage.",
            }
        },
    )
    canvas_project_id = _first_value(canvas_project.get("data"), ("id",))
    if not isinstance(canvas_project_id, int) or canvas_project_id <= 0:
        raise RunnerError("fixture create_freeform_video_project 未返回有效 project id")
    state["canvas_project_id"] = canvas_project_id

    uploaded_reviews = call(
        "import_project_text_upload",
        {
            "input": {
                "project_id": project_id,
                "file_name": f"fixture-{run_id}.txt",
                "data_base64": base64.b64encode(
                    f"第一章\n{chapter_body_response()}\n".encode("utf-8")
                ).decode("ascii"),
            }
        },
    )
    import_review_id = _first_value(uploaded_reviews.get("data"), ("id",))
    if not isinstance(import_review_id, int) or import_review_id <= 0:
        raise RunnerError("fixture import_project_text_upload 未返回有效 review id")
    state["import_review_id"] = import_review_id
    call(
        "accept_import_review",
        {"input": {"review_id": import_review_id, "analysis_result": None}},
    )
    call("preview_import_project_analysis_request", {"projectId": project_id})
    call("run_import_project_analysis", {"projectId": project_id})
    call("get_project_import_analysis_progress", {"projectId": project_id})
    call("list_import_queue", {"input": {"project_id": project_id}})
    task(
        "start_inspiration_generation",
        {
            "input": {
                "meta": {"start_request_id": str(uuid.uuid4())},
                "project_id": project_id,
                "message": "写一段关于暴雨夜车站和慢七分钟时钟的悬疑灵感。",
            }
        },
    )
    task(
        "start_blueprint_generation",
        {
            "input": {
                "meta": {"start_request_id": str(uuid.uuid4())},
                "project_id": project_id,
                "title": "逆行时钟",
                "genre": "都市时间悬疑",
                "target_length": 5,
            }
        },
    )
    task(
        "start_outline_generation",
        {
            "input": {
                "meta": {"start_request_id": str(uuid.uuid4())},
                "project_id": project_id,
                "operation": {"kind": "generate_parts", "part_count": 1, "chapters_per_part": 1},
            }
        },
    )
    parts = call("list_parts", {"projectId": project_id})
    part_id = _first_value(parts.get("data"), ("id",))
    if not isinstance(part_id, int) or part_id <= 0:
        raise RunnerError("fixture list_parts 未返回有效 part id")
    state["part_id"] = part_id
    existing_outlines = call("list_chapter_outlines", {"projectId": project_id}).get("data")
    if not isinstance(existing_outlines, list):
        raise RunnerError("fixture list_chapter_outlines 未返回数组")
    delete_targets = []
    for outline in existing_outlines:
        if not isinstance(outline, dict):
            raise RunnerError("fixture list_chapter_outlines 返回了无效记录")
        outline_id = outline.get("id")
        updated_at = outline.get("updated_at")
        if not isinstance(outline_id, int) or outline_id <= 0 or not isinstance(updated_at, str):
            raise RunnerError("fixture list_chapter_outlines 缺少删除所需版本字段")
        delete_targets.append({"id": outline_id, "expected_updated_at": updated_at})
    if delete_targets:
        call(
            "delete_latest_chapter_outlines",
            {"input": {"project_id": project_id, "targets": delete_targets}},
            destructive=True,
        )
    task(
        "start_outline_generation",
        {
            "input": {
                "meta": {"start_request_id": str(uuid.uuid4())},
                "project_id": project_id,
                "operation": {"kind": "generate_part_chapter_outlines", "part_id": part_id},
            }
        },
    )
    call("list_chapter_outlines", {"projectId": project_id})
    task(
        "start_chapter_generation",
        {
            "input": {
                "meta": {"start_request_id": str(uuid.uuid4())},
                "project_id": project_id,
                "chapter_number": 1,
                "min_word_count": 100,
                "max_word_count": 300,
            }
        },
    )
    chapters = call("list_chapters", {"projectId": project_id})
    chapter_id = _first_value(chapters.get("data"), ("id",))
    if not isinstance(chapter_id, int) or chapter_id <= 0:
        raise RunnerError("fixture list_chapters 未返回有效 chapter id")
    state["chapter_id"] = chapter_id
    state["chapter_number"] = 1

    task(
        "generate_chapter_analysis",
        {
            "input": {"chapter_id": chapter_id},
            "taskId": str(uuid.uuid4()),
        },
    )
    rag_prepare = call(
        "rag_prepare_ingest_all",
        {"projectId": project_id, "input": {"rebuild": False}},
    )
    prepare_data = rag_prepare.get("data")
    rag_records = prepare_data.get("records") if isinstance(prepare_data, dict) else None
    if (
        not isinstance(prepare_data, dict)
        or not isinstance(prepare_data.get("operation_trace_id"), str)
        or not prepare_data["operation_trace_id"]
        or not isinstance(prepare_data.get("embedding_identity"), dict)
        or not isinstance(rag_records, list)
        or not rag_records
    ):
        raise RunnerError("fixture rag_prepare_ingest_all 未返回有效分块记录")
    chunk_ids = [record.get("chunk_id") for record in rag_records if isinstance(record, dict)]
    content_hashes = [record.get("content_hash") for record in rag_records if isinstance(record, dict)]
    if (
        len(chunk_ids) != len(rag_records)
        or any(not isinstance(chunk_id, str) or not chunk_id for chunk_id in chunk_ids)
        or len(set(chunk_ids)) != len(chunk_ids)
        or any(not isinstance(content_hash, str) or not content_hash for content_hash in content_hashes)
        or any(not isinstance(record.get("text"), str) or not record["text"].strip() for record in rag_records if isinstance(record, dict))
    ):
        raise RunnerError("fixture rag_prepare_ingest_all 的分块 ID、正文或 hash 无效")
    rag_ingest = call(
        "rag_ingest_all",
        {"projectId": project_id, "input": {"rebuild": False}},
    )
    ingest_data = rag_ingest.get("data")
    if (
        not isinstance(ingest_data, dict)
        or not isinstance(ingest_data.get("added_count"), int)
        or not isinstance(ingest_data.get("updated_count"), int)
        or not isinstance(ingest_data.get("skipped_count"), int)
        or not isinstance(ingest_data.get("deleted_count"), int)
        or sum(ingest_data[field] for field in ("added_count", "updated_count", "skipped_count")) <= 0
    ):
        raise RunnerError("fixture rag_ingest_all 未返回有效摄取统计")
    rag_prepared = call(
        "rag_prepare_enhanced_search",
        {
            "projectId": project_id,
            "input": {
                "query": "车站的钟每天少走七分钟",
                "top_k": 5,
                "chapter_number": 1,
            },
        },
    )
    prepared_data = rag_prepared.get("data")
    if (
        not isinstance(prepared_data, dict)
        or not isinstance(prepared_data.get("query_texts"), list)
        or not prepared_data["query_texts"]
        or not isinstance(prepared_data.get("embedding_identity"), dict)
    ):
        raise RunnerError("fixture rag_prepare_enhanced_search 未返回可执行查询和 embedding identity")
    rag_search = call(
        "rag_enhanced_search",
        {
            "projectId": project_id,
            "input": {
                "query": "车站的钟每天少走七分钟",
                "top_k": 5,
                "chapter_number": 1,
            },
        },
    )
    search_data = rag_search.get("data")
    if not isinstance(search_data, dict):
        raise RunnerError("fixture rag_enhanced_search 未返回 object")
    results = search_data.get("results")
    queries = search_data.get("queries")
    candidate_count = search_data.get("candidate_count")
    if not isinstance(results, list) or not results:
        raise RunnerError("fixture rag_enhanced_search 未返回任何召回片段")
    if not isinstance(queries, list) or not any("七分钟" in str(item.get("query", "")) for item in queries if isinstance(item, dict)):
        raise RunnerError("fixture rag_enhanced_search 未保留原始查询及增强查询")
    if not isinstance(candidate_count, int) or candidate_count < len(results):
        raise RunnerError("fixture rag_enhanced_search candidate_count 与结果数量不一致")
    for result in results:
        if not isinstance(result, dict) or not result.get("id") or not str(result.get("content", "")).strip() or not isinstance(result.get("metadata"), dict):
            raise RunnerError("fixture rag_enhanced_search 返回了缺少来源元数据的片段")

    director_scene = call("create_scene_profile", {"input": {"project_id": project_id, "profile": {
        "name": f"Director station {run_id}", "scene_type": "room", "importance": "primary",
        "base_description": "旧车站通道", "visual_description": "门口与墙上挂钟",
        "spatial_layout": "通道通向门口", "atmosphere": "安静", "first_seen_chapter": 1,
        "last_seen_chapter": 1, "image_prompt_seed": "夜间车站通道",
    }}})
    director_scene_id = _first_value(director_scene.get("data"), ("scene_profile_id", "id"))
    if not isinstance(director_scene_id, int) or director_scene_id <= 0:
        raise RunnerError("director fixture requires a real scene profile")
    visual_plan = chapter_visual_plan_fixture(director_scene_id)
    # This path adapts novel evidence into Galgame lines. Quote actual source
    # text so the shooting script retains real source anchors.
    visual_plan["segments"][0]["prompt"] = chapter_body_response().split("\n\n")[0]
    visual_plan["frame"] = {"width": 1920, "height": 1080}
    visual_plan["points"][-1]["at_ms"] = 6000
    visual_draft = call(
        "save_chapter_visual_plan_draft",
        {
            "input": {
                "project_id": project_id,
                "source_chapter_id": chapter_id,
                "authoring_mode": "manual",
                "record_id": None,
                "expected_revision": None,
                "plan": visual_plan,
            }
        },
    )
    visual_draft_data = visual_draft.get("data")
    visual_record = (
        visual_draft_data.get("record")
        if isinstance(visual_draft_data, dict)
        else None
    )
    visual_plan_id = visual_record.get("id") if isinstance(visual_record, dict) else None
    visual_revision = visual_record.get("revision") if isinstance(visual_record, dict) else None
    if not isinstance(visual_plan_id, int) or visual_plan_id <= 0 or not isinstance(visual_revision, int):
        raise RunnerError("fixture save_chapter_visual_plan_draft 未返回有效 record")
    state.update(
        visual_plan_id=visual_plan_id,
        visual_plan_revision=visual_revision,
    )
    confirmed = call(
        "confirm_chapter_visual_plan",
        {
            "input": {
                "project_id": project_id,
                "source_chapter_id": chapter_id,
                "record_id": visual_plan_id,
                "expected_revision": state["visual_plan_revision"],
            }
        },
        destructive=True,
    )
    confirmed_data = confirmed.get("data")
    if (
        not isinstance(confirmed_data, dict)
        or not isinstance(confirmed_data.get("visual_plan"), dict)
        or not isinstance(confirmed_data["visual_plan"].get("plan"), dict)
    ):
        raise RunnerError("fixture confirm_chapter_visual_plan 未返回有效 visual_plan")
    call(
        "get_chapter_visual_plan",
        {"input": {"project_id": project_id, "source_chapter_id": chapter_id}},
    )
    task(
        "start_chapter_shooting_script_generation",
        {
            "input": {
                "project_id": project_id,
                "source_chapter_id": chapter_id,
                "visual_plan_id": visual_plan_id,
                "mode": "full",
                "meta": {"start_request_id": str(uuid.uuid4())},
            }
        },
    )
    shooting_script = call(
        "get_chapter_shooting_script",
        {"input": {"project_id": project_id, "source_chapter_id": chapter_id, "visual_plan_id": visual_plan_id}},
    )
    shooting_data = shooting_script.get("data")
    if not isinstance(shooting_data, dict) or not isinstance(shooting_data.get("script"), dict):
        raise RunnerError("fixture get_chapter_shooting_script 未返回 script")

    profile_name = f"CLI fixture character {run_id[-8:]}"
    co_profile_name = f"CLI fixture supporting character {run_id[-8:]}"
    profile_payload = {
        "input": {
            "project_id": project_id,
            "name": profile_name,
            "base_attributes": {
                "explicit_attributes": {},
                "implicit_attributes": {},
                "social_attributes": {},
            },
        }
    }
    profile = call("create_profile", profile_payload)
    profile_id = _first_value(profile.get("data"), ("profile_id", "id"))
    if not isinstance(profile_id, int) or profile_id <= 0:
        raise RunnerError("fixture create_profile 未返回有效 profile id")
    state["profile_id"] = profile_id
    co_profile = call(
        "create_profile",
        {
            "input": {
                "project_id": project_id,
                "name": co_profile_name,
                "base_attributes": {
                    "explicit_attributes": {},
                    "implicit_attributes": {},
                    "social_attributes": {},
                },
            }
        },
    )
    co_profile_id = _first_value(co_profile.get("data"), ("profile_id", "id"))
    if not isinstance(co_profile_id, int) or co_profile_id <= 0:
        raise RunnerError("fixture create_profile 未返回有效第二角色 id")
    state["co_profile_id"] = co_profile_id
    object_profile = call(
        "create_object_profile",
        {
            "input": {
                "project_id": project_id,
                "profile": {
                    "name": f"旧怀表-{run_id[-8:]}",
                    "object_type": "token",
                    "importance": "primary",
                    "visual_description": "林岚随身携带、指针逆行的旧式怀表。",
                    "material_and_state": "旧式表壳，秒针处于逆行状态。",
                    "story_usage": "证明时间异常真实发生的关键线索。",
                    "visual_usage": "固定林岚手部动作与时间异常的连续性。",
                    "generation_usage": "保持旧式怀表外观，并表现指针逆行。",
                    "first_seen_chapter": 1,
                    "last_seen_chapter": 1,
                    "image_prompt_seed": "林岚掌心指针逆行的旧怀表",
                },
            }
        },
    )
    object_profile_id = _first_value(object_profile.get("data"), ("object_profile_id", "id"))
    if not isinstance(object_profile_id, int) or object_profile_id <= 0:
        raise RunnerError("fixture create_object_profile 未返回有效 object profile id")
    state["object_profile_id"] = object_profile_id
    scene_profile = call(
        "create_scene_profile",
        {
            "input": {
                "project_id": project_id,
                "profile": {
                    "name": f"Station {run_id}",
                    "scene_type": "room",
                    "importance": "primary",
                    "base_description": "A railway waiting room at night.",
                    "visual_description": "Rain on tall windows and a single clock.",
                    "spatial_layout": "Benches along the walls, clock above the door.",
                    "atmosphere": "Tense and wet after a storm.",
                    "first_seen_chapter": 1,
                    "last_seen_chapter": 1,
                    "image_prompt_seed": "rainy railway waiting room at night",
                },
            }
        },
    )
    scene_profile_id = _first_value(scene_profile.get("data"), ("scene_profile_id", "id"))
    if not isinstance(scene_profile_id, int) or scene_profile_id <= 0:
        raise RunnerError("fixture create_scene_profile 未返回有效 scene profile id")
    state["scene_profile_id"] = scene_profile_id
    foreshadowing = call(
        "create_foreshadowing",
        {
            "input": {
                "project_id": project_id,
                "record": {
                    "description": "The station clock loses seven minutes every night.",
                    "setup_chapter": 1,
                    "status": "pending",
                    "importance": 3,
                },
            }
        },
    )
    foreshadowing_id = _first_value(foreshadowing.get("data"), ("foreshadowing_id", "id"))
    if isinstance(foreshadowing_id, int) and foreshadowing_id > 0:
        state["foreshadowing_id"] = foreshadowing_id

    theme = call(
        "create_theme",
        {
            "input": {
                "name": f"Manifest fixture theme {run_id}",
                "mode": "light",
                "document": {
                    "version": 1,
                    "tokens": {"--color-accent": "#334155"},
                    "backgrounds": {},
                },
            }
        },
    )
    theme_id = _first_value(theme.get("data"), ("id",))
    if isinstance(theme_id, int) and theme_id > 0:
        state["theme_id"] = theme_id

    for command, field in (
        ("create_llm_config", "llm_config_id"),
        ("create_embedding_config", "embedding_config_id"),
        ("add_image_config", "image_config_id"),
        ("add_video_config", "video_config_id"),
    ):
        payload = fixture_payload(command, default_args(descriptors[command]), state, run_id, provider_url, True)
        config = call(command, payload)
        config_id = _first_value(config.get("data"), ("id",))
        if isinstance(config_id, int) and config_id > 0:
            state[field] = config_id
    if refresh_runtime:
        refresh_runtime(state)

    image = call(
        "upload_image",
        {
            "input": {
                "project_id": project_id,
                "file_name": f"fixture-{run_id}.png",
                "mime_type": "image/png",
                "data_base64": PNG_1X1_BASE64,
                "image_type": "illustration",
                "prompt": "A deterministic image fixture.",
                "metadata_json": {},
            }
        },
    )
    image_id = _first_value(image.get("data"), ("image_id", "id"))
    if isinstance(image_id, int) and image_id > 0:
        state["image_id"] = image_id
    portrait = call(
        "upload_portrait",
        {
            "input": {
                "project_id": project_id,
                "profile_id": profile_id,
                "file_name": f"fixture-{run_id}.png",
                "mime_type": "image/png",
                "data_base64": PNG_1X1_BASE64,
                "portrait_type": "face",
                "prompt": "A deterministic portrait fixture.",
                "set_primary": False,
            }
        },
    )
    portrait_id = _first_value(portrait.get("data"), ("portrait_id", "id"))
    if isinstance(portrait_id, int) and portrait_id > 0:
        state["portrait_id"] = portrait_id
    material = call(
        "upload_profile_material_image",
        {
            "input": {
                "project_id": project_id,
                "profile_kind": "object",
                "profile_id": object_profile_id,
                "file_name": f"fixture-{run_id}.png",
                "mime_type": "image/png",
                "data_base64": PNG_1X1_BASE64,
                "set_primary": False,
                "prompt_snapshot": "A deterministic material fixture.",
            }
        },
    )
    material_id = _first_value(material.get("data"), ("image_id", "id"))
    if isinstance(material_id, int) and material_id > 0:
        state["material_id"] = material_id

    blueprint = call(
        "save_blueprint",
        {
            "input": {
                "project_id": project_id,
                "genre": "都市时间悬疑",
                "total_chapters": 1,
                "one_sentence_summary": "一座车站的钟每天少走七分钟。",
                "full_synopsis": "管理员 CLI E2E 使用的最小蓝图。",
            }
        },
    )
    del blueprint
    canvas = call(
        "save_canvas_graph",
        {
            "input": {
                "project_id": canvas_project_id,
                "graph": {"name": f"Fixture graph {run_id}", "graph_kind": "video_studio", "nodes": [], "edges": []},
            }
        },
    )
    graph_id = _first_value(canvas.get("data"), ("graph_id", "id"))
    if isinstance(graph_id, int) and graph_id > 0:
        state["graph_id"] = graph_id

    text_asset = call(
        "save_canvas_text_asset",
        {
            "input": {
                "project_id": canvas_project_id,
                "text_asset": {
                    "title": "Fixture text asset",
                    "text_kind": "freeform",
                    "content": "A4N CLI canvas text fixture.",
                },
            }
        },
    )
    text_asset_id = _first_value(text_asset.get("data"), ("id",))
    if isinstance(text_asset_id, int) and text_asset_id > 0:
        state["text_asset_id"] = text_asset_id
    call(
        "import_canvas_media",
        {
            "input": {
                "project_id": canvas_project_id,
                "media_kind": "image",
                "file_name": f"fixture-{run_id}.png",
                "mime_type": "image/png",
                "data_base64": PNG_1X1_BASE64,
            }
        },
    )

    derived = call(
        "create_derived_project",
        {
            "input": {
                "root_project_id": project_id,
                "derived_kind": "galgame",
                "title": f"Fixture Galgame {run_id}",
                "description": "Disposable Galgame derived project for CLI coverage.",
                "start_chapter": 1,
                "end_chapter": 1,
            }
        },
    )
    derived_project_id = _first_value(derived.get("data"), ("id",))
    if not isinstance(derived_project_id, int) or derived_project_id <= 0:
        raise RunnerError("fixture create_derived_project 未返回有效 project id")
    state["derived_project_id"] = derived_project_id
    state["galgame_project_id"] = derived_project_id
    source_sync = call(
        "preview_source_sync",
        {"input": {"derived_project_id": derived_project_id}},
    )
    source_run = source_sync.get("data", {}).get("run") if isinstance(source_sync.get("data"), dict) else None
    sync_run_id = source_run.get("id") if isinstance(source_run, dict) else None
    if not isinstance(sync_run_id, int) or sync_run_id <= 0:
        raise RunnerError("fixture preview_source_sync 未返回有效 sync_run_id")
    state["sync_run_id"] = sync_run_id
    call(
        "apply_source_sync_decisions",
        {"input": {"sync_run_id": sync_run_id, "decisions": []}},
        destructive=True,
    )
    duplicate = call(
        "duplicate_project",
        {"input": {"project_id": project_id, "new_title": f"Fixture duplicate {run_id}"}},
    )
    duplicate_project_id = _first_value(duplicate.get("data"), ("id",))
    if isinstance(duplicate_project_id, int) and duplicate_project_id > 0:
        state["duplicate_project_id"] = duplicate_project_id

    galgame = call(
        "start_galgame_pipeline",
        {
            "input": {
                "project_id": derived_project_id,
                "task_id": f"fixture-galgame-{run_id}",
                "resume_from_checkpoint": False,
                "start_from_stage": None,
            }
        },
    )
    if not isinstance(galgame.get("data"), dict):
        raise RunnerError("fixture start_galgame_pipeline 未返回 script")
    call("get_galgame_pipeline_status", {"input": {"project_id": derived_project_id}})
    call("get_galgame_script", {"input": {"project_id": derived_project_id}})
    asset_plan = call("plan_galgame_assets", {"input": {"project_id": derived_project_id}})
    asset_plan_data = asset_plan.get("data")
    generation_spec_ids: list[int] = []
    if isinstance(asset_plan_data, dict):
        for group in ("backgrounds", "portrait_expressions", "event_cgs"):
            items = asset_plan_data.get(group, [])
            if isinstance(items, list):
                for item in items:
                    spec_id = _first_value(item, ("generation_spec_id",))
                    if isinstance(spec_id, int) and spec_id > 0:
                        generation_spec_ids.append(spec_id)
    if not generation_spec_ids:
        raise RunnerError("fixture plan_galgame_assets 未返回 generation_spec_id")
    state["generation_spec_ids"] = generation_spec_ids
    call(
        "run_galgame_asset_generation",
        {
            "input": {
                "project_id": derived_project_id,
                "config_id": state.get("image_config_id"),
                "generation_spec_ids": generation_spec_ids,
                "limit": 1,
                "width": 1024,
                "height": 1024,
            }
        },
    )

    manga_task = call(
        "start_manga_pipeline",
        {"input": {"project_id": project_id, "chapter_number": 1}},
    )
    state["manga_task_id"] = manga_task.get("data")
    wait_for_manga()
    manga_prompts = call(
        "list_manga_page_prompts",
        {"input": {"project_id": project_id, "chapter_id": chapter_id}},
    )
    manga_prompt_id = _first_value(manga_prompts.get("data"), ("id",))
    if isinstance(manga_prompt_id, int) and manga_prompt_id > 0:
        state["manga_prompt_id"] = manga_prompt_id

    video_options = {
        "workflow_mode": "shared_segment_plan",
        "style": "电影感",
        "prompt_output_language": "zh_cn",
        "prompt_batch_size": 1,
        "duration_secs": 6,
        "duration_secs_options": [6],
        "aspect_ratio": "16:9",
        "reference_image_ids": [],
        "video_library_image_ids": [],
        "regenerate_existing": False,
    }
    task(
        "start_video_generation",
        {
            "input": {
                "meta": {"start_request_id": str(uuid.uuid4())},
                "project_id": project_id,
                "operation": {
                    "kind": "prompt_segment_plan",
                    "chapter_id": chapter_id,
                    "options": video_options,
                },
            }
        },
    )
    prompts = call("list_chapter_video_prompts", {"projectId": project_id, "chapterId": chapter_id})
    prompt_rows = prompts.get("data")
    shared_prompt = next(
        (
            item
            for item in prompt_rows
            if isinstance(item, dict) and item.get("workflow_mode") == "shared_segment_plan"
        ),
        None,
    ) if isinstance(prompt_rows, list) else None
    segment_plan_prompt_id = shared_prompt.get("id") if isinstance(shared_prompt, dict) else None
    if not isinstance(segment_plan_prompt_id, int) or segment_plan_prompt_id <= 0:
        raise RunnerError("fixture 视频 segment plan 未返回共享 prompt")
    state["segment_plan_prompt_id"] = segment_plan_prompt_id
    segment_target = _first_video_segment_target(shared_prompt)
    recipe = dict(segment_target["recipe"])
    recipe["workflow_mode"] = "first_frame_chain"
    recipe["guidance_mode"] = "none"
    state["video_segment_recipe"] = recipe
    call(
        "update_chapter_video_segment_recipe",
        {"promptId": segment_plan_prompt_id, "recipe": recipe},
    )
    target_segment = {
        "source_shot_id": segment_target["source_shot_id"],
        "segment_index_in_shot": segment_target["segment_index_in_shot"],
    }
    state["video_target_segment"] = target_segment
    state["video_workflow_mode"] = recipe["workflow_mode"]
    state["video_prompt_output_language"] = recipe["prompt_output_language"]
    video_options = {
        **video_options,
        "workflow_mode": recipe["workflow_mode"],
        "prompt_output_language": recipe["prompt_output_language"],
        "segment_plan_prompt_id": segment_plan_prompt_id,
        "target_segment": target_segment,
    }
    task(
        "start_video_generation",
        {
            "input": {
                "meta": {"start_request_id": str(uuid.uuid4())},
                "project_id": project_id,
                "operation": {
                    "kind": "prompt_videos",
                    "chapter_id": chapter_id,
                    "options": video_options,
                },
            }
        },
    )
    prompts = call("list_chapter_video_prompts", {"projectId": project_id, "chapterId": chapter_id})
    prompt_rows = prompts.get("data")
    video_prompt = next(
        (
            item
            for item in prompt_rows
            if isinstance(item, dict)
            and item.get("prompt_type") == "video_prompt"
            and item.get("workflow_mode") == state.get("video_workflow_mode")
        ),
        None,
    ) if isinstance(prompt_rows, list) else None
    video_prompt_id = video_prompt.get("id") if isinstance(video_prompt, dict) else None
    if not isinstance(video_prompt_id, int) or video_prompt_id <= 0:
        raise RunnerError("fixture 视频 prompt list 未返回最终 video prompt")
    state["video_prompt_id"] = video_prompt_id
    task(
        "start_video_generation",
        {
            "input": {
                "meta": {"start_request_id": str(uuid.uuid4())},
                "project_id": project_id,
                "operation": {
                    "kind": "chapter",
                    "chapter_id": chapter_id,
                    "prompt_id": video_prompt_id,
                    "config_id": state.get("video_config_id"),
                },
            }
        },
    )
    videos = call("list_videos_by_chapter", {"projectId": project_id, "chapterId": chapter_id})
    video_id = _first_value(videos.get("data"), ("id",))
    if not isinstance(video_id, int) or video_id <= 0:
        raise RunnerError("fixture list_videos_by_chapter 未返回有效 video id")
    state["video_id"] = video_id
    poll_video(video_id)

    task(
        "sync_character_from_chapters",
        {
            "input": {"project_id": project_id, "name": "林岚", "chapter_number": 1},
            "taskId": f"fixture-character-{run_id}",
        },
    )
    task(
        "sync_all_character_profiles",
        {"input": {"project_id": project_id, "chapter_number": 1}, "taskId": f"fixture-character-batch-{run_id}"},
    )
    task(
        "sync_scene_profiles",
        {"input": {"project_id": project_id, "chapter_number": 1}, "taskId": f"fixture-scene-{run_id}"},
    )

    users = call("list_all_users", {})
    user_rows = users.get("data")
    collaborator = next(
        (
            item
            for item in user_rows
            if isinstance(item, dict)
            and (item.get("username") or item.get("email"))
            and (
                not collaborator_username
                or collaborator_username in {item.get("username"), item.get("email")}
            )
            and (item.get("username") or item.get("email")) != current_username
        ),
        None,
    ) if isinstance(user_rows, list) else None
    collaborator_identity = (collaborator or {}).get("username") or (collaborator or {}).get("email")
    if not isinstance(collaborator_identity, str) or not collaborator_identity:
        raise RunnerError("fixture list_all_users 未找到不同于管理员的真实账号")
    state["collaborator_identity"] = collaborator_identity
    call(
        "invite_project_member",
        {
            "projectId": project_id,
            "input": {"username_or_email": collaborator_identity, "role": "viewer"},
        },
    )
    invites = call("list_project_invites", {"projectId": project_id})
    invite_id = _first_value(invites.get("data"), ("id", "invite_id"))
    if not isinstance(invite_id, int) or invite_id <= 0:
        raise RunnerError("fixture list_project_invites 未返回有效 invite id")
    state["invite_id"] = invite_id
    call("list_my_project_invites", {})
    if collaborator_username and collaborator_password:
        fd, collaborator_config_name = tempfile.mkstemp(prefix="a4n-collaborator-", suffix=".json")
        os.close(fd)
        collaborator_config = Path(collaborator_config_name)
        try:
            shutil.copy2(cli.config, collaborator_config)
            collaborator_cli = Cli(cli.executable, cli.server, collaborator_config, cli.context, cli.timeout, cli.env)
            checked(
                collaborator_cli,
                ["auth", "login", "--username", collaborator_username, "--password-stdin"],
                "collaborator auth login",
                f"{collaborator_password}\n",
            )
            call("accept_project_invite", {"inviteId": invite_id}, client=collaborator_cli)
        finally:
            collaborator_config.unlink(missing_ok=True)
            collaborator_config.with_name(collaborator_config.name + ".lock").unlink(missing_ok=True)


def main() -> int:
    args = parse_args()
    if args.timeout <= 0 or args.provider_start_timeout <= 0:
        raise RunnerError("timeout 必须为正数")
    if args.fixture_only and not args.execute_writes:
        raise RunnerError("--fixture-only 必须同时使用 --execute-writes")
    if args.fixture_only and not args.allow_destructive:
        raise RunnerError("--fixture-only 必须同时使用 --allow-destructive")
    run_id = f"manifest-{os.getpid()}-{uuid.uuid4().hex[:8]}"
    report_path = args.report.expanduser().resolve()
    report_path.parent.mkdir(parents=True, exist_ok=True)
    environment = os.environ.copy()
    password: str | None = None
    collaborator_password: str | None = None
    if args.username:
        password = environment.get(args.password_env)
        if not password:
            raise RunnerError(f"--username 已提供，但环境变量 {args.password_env} 为空")
        environment.pop("A4N_ACCESS_TOKEN", None)
        environment.pop(args.password_env, None)
    if args.collaborator_username:
        collaborator_password = environment.get(args.collaborator_password_env)
        if not collaborator_password:
            raise RunnerError(
                f"--collaborator-username 已提供，但环境变量 {args.collaborator_password_env} 为空"
            )
        environment.pop(args.collaborator_password_env, None)

    provider = None
    provider_log = None
    provider_preflight_boundary = None
    provider_fixture_boundary = None
    provider_contract = None
    manifest: dict[str, Any] | None = None
    manifest_result: dict[str, Any] | None = None
    results: list[dict[str, Any]] = []
    fixture_state: dict[str, Any] = {}
    fixture_results: list[dict[str, Any]] = []
    fixture_paths: dict[str, Any] = {}
    fatal_error: str | None = None
    with tempfile.TemporaryDirectory(prefix="a4n-cli-manifest-e2e-") as temporary:
        work_dir = Path(temporary)
        config = args.config.expanduser().resolve() if args.config else work_dir / "cli-config.json"
        context = args.context.expanduser().resolve() if args.context else work_dir / "cli-context.json"
        cli = Cli(str(Path(args.a4n).expanduser()), args.server, config, context, args.timeout, environment)
        try:
            checked(cli, ["health"], "health")
            checked(cli, ["capabilities", "check"], "capabilities check")
            if args.username:
                checked(cli, ["auth", "login", "--username", args.username, "--password-stdin"], "auth login", f"{password}\n")
            checked(cli, ["auth", "status"], "auth status")

            if args.manifest:
                manifest = manifest_data(json.loads(args.manifest.read_text(encoding="utf-8")))
            else:
                manifest_envelope, manifest_result = checked(cli, ["capabilities", "export"], "capabilities export")
                manifest = manifest_data(manifest_envelope)

            provider_args = argparse.Namespace(
                provider_url=args.provider_url,
                provider_host=args.provider_host,
                provider_port=args.provider_port,
                provider_script=args.provider_script,
                provider_start_timeout=args.provider_start_timeout,
                command_timeout=args.timeout,
            )
            provider, provider_log = start_provider(provider_args, report_path.parent, run_id)
            provider_contract = check_provider_contract(provider.base_url, args.timeout)
            runtime_path = work_dir / "runtime-config.json"
            secure_json(runtime_path, runtime_snapshot(provider.base_url))
            checked(cli, ["config", "runtime", "set", "--input", f"@{runtime_path}"], "config runtime set")
            provider_preflight_boundary = len(_provider_entries(provider_log))

            fixture_map = load_map(args.fixture_map)
            descriptors = {descriptor["name"]: descriptor for descriptor in manifest["commands"]}
            if args.execute_writes:
                seed_fixtures(
                    cli,
                    descriptors,
                    run_id,
                    provider.base_url,
                    fixture_state,
                    fixture_results,
                    allow_destructive=args.allow_destructive,
                    current_username=args.username,
                    collaborator_username=args.collaborator_username,
                    collaborator_password=collaborator_password,
                    refresh_runtime=lambda state: (
                        secure_json(runtime_path, runtime_snapshot_for_fixture(provider.base_url, state)),
                        checked(
                            cli,
                            ["config", "runtime", "set", "--input", f"@{runtime_path}"],
                            "config runtime refresh",
                        ),
                    ),
                )
                fixture_paths = fixture_path_coverage(
                    fixture_results,
                    collaborator_enabled=bool(args.collaborator_username and collaborator_password),
                )
                if not fixture_paths["complete"]:
                    incomplete = [
                        name
                        for name, path in fixture_paths["paths"].items()
                        if path["status"] != "passed"
                    ]
                    raise RunnerError("fixture 用户路径未完整执行: " + ", ".join(incomplete))
            provider_fixture_boundary = len(_provider_entries(provider_log))
            if args.execute_writes:
                missing_provider_paths = missing_fixture_provider_paths(
                    provider_log,
                    provider_preflight_boundary or 0,
                )
                if missing_provider_paths:
                    raise RunnerError(
                        "fixture 未覆盖 virtual provider 路径: "
                        + ", ".join(missing_provider_paths)
                    )
            manifest_descriptors = (
                () if args.fixture_only else ordered_descriptors(manifest["commands"])
            )
            for descriptor in manifest_descriptors:
                command = descriptor["name"]
                payload = fixture_map.get(command, default_args(descriptor))
                if not isinstance(payload, dict):
                    raise RunnerError(f"fixture `{command}` 必须是 object")
                payload = fixture_payload(
                    command,
                    payload,
                    fixture_state,
                    run_id,
                    provider.base_url,
                    command not in fixture_map,
                ) if args.execute_writes else payload
                payload = safe_probe_payload(payload, descriptor.get("effect", "read"), args.execute_writes)
                destructive = descriptor.get("effect") == "destructive"
                before_provider = len(_provider_entries(provider_log))
                extra_parts, output_path = output_parts(command, descriptor, work_dir)
                result = invoke(
                    cli,
                    command,
                    payload,
                    destructive and args.allow_destructive,
                    descriptor,
                    extra_parts,
                )
                if result["ok"] and output_path:
                    if not output_path.is_file():
                        result.update(
                            ok=False,
                            classification="real_failure",
                            error={
                                "code": "missing_file_output",
                                "message": f"CLI 未生成声明的输出文件: {output_path.name}",
                            },
                        )
                    else:
                        result["files"] = {"path": output_path.name, "bytes": output_path.stat().st_size}
                result.update(
                    command=command,
                    domain=descriptor.get("domain"),
                    effect=descriptor.get("effect"),
                    execution=descriptor.get("execution"),
                    route=(
                        "cli_destructive_gate"
                        if descriptor.get("effect") == "destructive" and not args.allow_destructive
                        else "server"
                    ),
                    payload=None if command in AUTH_COMMANDS else redact(payload),
                    provider_requests=_provider_delta(provider_log, before_provider),
                )
                result["data"] = redact(result.get("data"))
                result["meta"] = redact(result.get("meta"))
                if "error" in result:
                    result["error"] = redact(result["error"])
                results.append(result)
                print(json.dumps({"command": command, "classification": result["classification"], "exit_code": result["exit_code"]}, ensure_ascii=False), flush=True)
        except (RunnerError, OSError, json.JSONDecodeError) as error:
            fatal_error = str(error)
        finally:
            if provider:
                provider.stop()

    manifest_count = len(manifest["commands"]) if manifest else 0
    manifest_probe_enabled = not args.fixture_only
    invoked_names = {item["command"] for item in results}
    missing = (
        [descriptor["name"] for descriptor in manifest["commands"] if descriptor["name"] not in invoked_names]
        if manifest and manifest_probe_enabled
        else []
    )
    real_failures = [item for item in results if item["classification"] == "real_failure"]
    manifest_probe_success = (
        not manifest_probe_enabled
        or (len(results) == manifest_count and not missing and not real_failures)
    )
    if args.execute_writes:
        fixture_paths = fixture_path_coverage(
            fixture_results,
            collaborator_enabled=bool(args.collaborator_username and collaborator_password),
        )
    fixture_success = not args.execute_writes or fixture_paths.get("complete") is True
    classification_names = (
        "business_success",
        "expected_state_rejection",
        "permission_rejection",
        "destructive_success",
        "real_failure",
    )
    report = {
        "schema_version": 1,
        "run_id": run_id,
        "success": fatal_error is None and fixture_success and manifest_probe_success,
        "mode": "fixture_only" if args.fixture_only else "manifest",
        "manifest_commands": manifest_count,
        "invoked": len(results),
        "missing": missing,
        "manifest_probe": {
            "enabled": manifest_probe_enabled,
            "manifest_commands": manifest_count,
            "invoked": len(results),
            "missing": missing,
        },
        "business_success": sum(item["classification"] == "business_success" for item in results),
        "expected_state_rejection": sum(item["classification"] == "expected_state_rejection" for item in results),
        "permission_rejection": sum(item["classification"] == "permission_rejection" for item in results),
        "destructive_success": sum(item["classification"] == "destructive_success" for item in results),
        "real_failure": len(real_failures),
        "classifications": {
            key: sum(item["classification"] == key for item in results)
            for key in classification_names
        },
        "provider": {
            "base_url": provider.base_url if provider else None,
            "contract": provider_contract,
            "requests": _provider_summary(
                provider_log,
                provider_preflight_boundary,
                provider_fixture_boundary,
            ),
        },
        "fixtures": {
            "enabled": args.execute_writes,
            "only": args.fixture_only,
            "paths": fixture_paths,
            "state": redact(fixture_state),
            "results": fixture_results,
        },
        "manifest": {
            "version": manifest.get("manifest_version") if manifest else None,
            "export_duration_ms": manifest_result.get("duration_ms") if manifest_result else None,
        },
        "results": results,
        "fatal_error": fatal_error,
    }
    report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    summary = {
        "success": report["success"],
        "manifest_commands": report["manifest_commands"],
        "invoked": report["invoked"],
        "business_success": report["business_success"],
        "expected_state_rejection": report["expected_state_rejection"],
        "permission_rejection": report["permission_rejection"],
        "destructive_success": report["destructive_success"],
        "real_failure": report["real_failure"],
        "missing": report["missing"],
        "fixture_only": args.fixture_only,
        "fixture_paths_complete": fixture_paths.get("complete") if args.execute_writes else None,
        "report": str(report_path),
    }
    print(json.dumps(summary, ensure_ascii=False))
    return 0 if report["success"] else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (RunnerError, OSError) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2)
