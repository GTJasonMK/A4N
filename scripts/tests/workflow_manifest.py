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
