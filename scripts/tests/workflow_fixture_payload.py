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
    elif command == "generate_chapter_shooting_script" and input_value is not None:
        input_value.update(
            project_id=state.get("project_id", input_value.get("project_id")),
            source_chapter_id=state.get("chapter_id", input_value.get("source_chapter_id")),
            visual_plan_id=state.get("visual_plan_id", input_value.get("visual_plan_id")),
        )
    elif command == "start_manga_pipeline" and input_value is not None:
        input_value.update(
            project_id=state.get("project_id", input_value.get("project_id")),
            chapter_number=state.get("chapter_number", 1),
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
