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
    allow_destructive: bool = False,
    current_username: str | None = None,
    collaborator_username: str | None = None,
    collaborator_password: str | None = None,
    refresh_runtime: Callable[[dict[str, Any]], None] | None = None,
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    state: dict[str, Any] = {}
    records: list[dict[str, Any]] = []

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
    shooting_script = call(
        "generate_chapter_shooting_script",
        {
            "input": {
                "project_id": project_id,
                "source_chapter_id": chapter_id,
                "visual_plan_id": visual_plan_id,
            }
        },
    )
    shooting_data = shooting_script.get("data")
    if not isinstance(shooting_data, dict) or not isinstance(shooting_data.get("script"), dict):
        raise RunnerError("fixture generate_chapter_shooting_script 未返回 script")
    call(
        "get_chapter_shooting_script",
        {"input": {"project_id": project_id, "source_chapter_id": chapter_id, "visual_plan_id": visual_plan_id}},
    )

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
    return state, records
