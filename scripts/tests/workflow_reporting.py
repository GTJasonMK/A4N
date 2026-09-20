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
        "generate_chapter_shooting_script",
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

