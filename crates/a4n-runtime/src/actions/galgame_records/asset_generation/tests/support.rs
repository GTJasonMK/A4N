fn portrait_expression_spec(
    id: i64,
    target_entity_id: Option<i64>,
    params_json: serde_json::Value,
) -> generation_spec::Model {
    let now = fixed_time();
    generation_spec::Model {
        id,
        project_id: 10,
        root_project_id: 1,
        source_context_kind: Some("snapshot_backed".to_string()),
        source_snapshot_id: Some(20),
        source_chapter_id: None,
        source_snapshot_chapter_id: None,
        canvas_text_asset_id: None,
        target_kind: GALGAME_TARGET_PORTRAIT_EXPRESSION.to_string(),
        target_entity_kind: GALGAME_ENTITY_CHARACTER_PROFILE.to_string(),
        target_entity_id,
        status: "draft".to_string(),
        source_version_hash: "hash".to_string(),
        params_json,
        created_at: now,
        updated_at: now,
    }
}

fn background_visual_spec(
    id: i64,
    target_entity_id: Option<i64>,
    source_chapter_id: Option<i64>,
    source_snapshot_chapter_id: Option<i64>,
    params_json: serde_json::Value,
) -> generation_spec::Model {
    visual_spec(
        id,
        GALGAME_TARGET_BACKGROUND,
        GALGAME_ENTITY_SCENE,
        target_entity_id,
        source_chapter_id,
        source_snapshot_chapter_id,
        params_json,
    )
}

fn cg_visual_spec(
    id: i64,
    target_entity_id: Option<i64>,
    source_chapter_id: Option<i64>,
    source_snapshot_chapter_id: Option<i64>,
    params_json: serde_json::Value,
) -> generation_spec::Model {
    visual_spec(
        id,
        GALGAME_TARGET_CG,
        GALGAME_ENTITY_LINE,
        target_entity_id,
        source_chapter_id,
        source_snapshot_chapter_id,
        params_json,
    )
}

fn visual_spec(
    id: i64,
    target_kind: &str,
    target_entity_kind: &str,
    target_entity_id: Option<i64>,
    source_chapter_id: Option<i64>,
    source_snapshot_chapter_id: Option<i64>,
    params_json: serde_json::Value,
) -> generation_spec::Model {
    let now = fixed_time();
    generation_spec::Model {
        id,
        project_id: 10,
        root_project_id: 1,
        source_context_kind: Some("snapshot_backed".to_string()),
        source_snapshot_id: Some(20),
        source_chapter_id,
        source_snapshot_chapter_id,
        canvas_text_asset_id: None,
        target_kind: target_kind.to_string(),
        target_entity_kind: target_entity_kind.to_string(),
        target_entity_id,
        status: "draft".to_string(),
        source_version_hash: "hash".to_string(),
        params_json,
        created_at: now,
        updated_at: now,
    }
}

fn prompt_engine() -> TeraPromptEngine {
    let prompts_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../prompts");
    TeraPromptEngine::from_directory(&prompts_dir).expect("prompts should load")
}

fn background_target_context() -> GalgameTargetContext {
    GalgameTargetContext {
        project_id: 10,
        root_project_id: 1,
        source_snapshot_id: 20,
        source_chapter_id: 30,
        source_snapshot_chapter_id: 40,
        scene_id: 100,
        line_id: None,
    }
}

fn cg_target_context() -> GalgameTargetContext {
    GalgameTargetContext {
        line_id: Some(1000),
        ..background_target_context()
    }
}

fn fixed_time() -> NaiveDateTime {
    chrono::DateTime::from_timestamp(1_700_000_000, 0)
        .expect("valid timestamp")
        .naive_utc()
}
