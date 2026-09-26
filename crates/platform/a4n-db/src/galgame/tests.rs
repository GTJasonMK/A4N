use super::*;

#[test]
fn galgame_generation_record_validation_rejects_invalid_ids_and_text() {
    assert!(matches!(
        validate_galgame_generation_id(0, "generation_spec_id"),
        Err(GalgameGenerationRecordError::InvalidArgument(message))
            if message == "generation_spec_id 必须是正整数"
    ));
    assert!(matches!(
        require_galgame_generation_text("  ", "prompt_text"),
        Err(GalgameGenerationRecordError::InvalidArgument(message))
            if message == "prompt_text 不能为空"
    ));
}

#[test]
fn galgame_generation_database_error_keeps_db_error() {
    let error =
        GalgameGenerationRecordError::Database(DbErr::Custom("fixture failure".to_string()));
    assert!(matches!(
        &error,
        GalgameGenerationRecordError::Database(DbErr::Custom(message))
            if message == "fixture failure"
    ));
}

#[test]
fn galgame_portrait_read_validation_rejects_invalid_ids_and_blank_pose() {
    assert!(matches!(
        validate_galgame_generation_id(0, "root_project_id"),
        Err(GalgameGenerationRecordError::InvalidArgument(message))
            if message == "root_project_id 必须是正整数"
    ));
    assert!(matches!(
        require_galgame_generation_text(" \t", "pose"),
        Err(GalgameGenerationRecordError::InvalidArgument(message))
            if message == "pose 不能为空"
    ));
}

#[test]
fn galgame_portrait_spec_target_input_rejects_invalid_source_line_ids() {
    let spec = generation_spec_model();
    let input = GalgamePortraitSpecTargetInput {
        profile_id: 7,
        expression: "smile".to_string(),
        pose: "standing".to_string(),
        position_samples: Vec::new(),
        source_line_ids: vec![11, 0],
    };
    assert!(matches!(
        validate_galgame_portrait_spec_target_input(&spec, &input),
        Err(GalgameGenerationRecordError::InvalidArgument(message))
            if message == "source_line_ids 必须全部是正整数: 0"
    ));
}

#[test]
fn galgame_asset_generation_spec_query_rejects_invalid_filters() {
    assert!(matches!(
        validate_galgame_asset_generation_spec_query(0, &["background"], &[], None),
        Err(GalgameAssetPlanSpecRecordError::InvalidArgument(message))
            if message == "project_id 必须是正整数"
    ));
    assert!(matches!(
        validate_galgame_asset_generation_spec_query(1, &[], &[], None),
        Err(GalgameAssetPlanSpecRecordError::InvalidArgument(message))
            if message == "target_kinds 不能为空"
    ));
    assert!(matches!(
        validate_galgame_asset_generation_spec_query(1, &["background"], &[2, 2], None),
        Err(GalgameAssetPlanSpecRecordError::InvalidArgument(message))
            if message == "generation_spec_ids 不能包含重复值"
    ));
    assert!(matches!(
        validate_galgame_asset_generation_spec_query(1, &["background"], &[], Some(0)),
        Err(GalgameAssetPlanSpecRecordError::InvalidArgument(message))
            if message == "limit 必须大于 0"
    ));
}

#[test]
fn galgame_asset_plan_database_error_keeps_db_error() {
    let error =
        GalgameAssetPlanSpecRecordError::Database(DbErr::Custom("fixture failure".to_string()));
    assert!(matches!(
        &error,
        GalgameAssetPlanSpecRecordError::Database(DbErr::Custom(message))
            if message == "fixture failure"
    ));
}

#[test]
fn galgame_media_binding_ids_are_rejected_before_database_work() {
    assert!(matches!(
        validate_galgame_media_target_ids(0, 1),
        Err(GalgameMediaRecordError::InvalidArgument(message))
            if message == "project_id 和 target_id 必须是正整数"
    ));
    assert!(matches!(
        validate_galgame_media_target_ids(1, 0),
        Err(GalgameMediaRecordError::InvalidArgument(message))
            if message == "project_id 和 target_id 必须是正整数"
    ));
    assert!(matches!(
        validate_galgame_media_asset_id(Some(0)),
        Err(GalgameMediaRecordError::InvalidArgument(message))
            if message == "media_asset_id 必须是正整数"
    ));
    assert!(validate_galgame_media_asset_id(None).is_ok());
}

#[test]
fn galgame_media_database_error_keeps_db_error() {
    let error = GalgameMediaRecordError::Database(DbErr::Custom("fixture failure".to_string()));
    assert!(matches!(
        &error,
        GalgameMediaRecordError::Database(DbErr::Custom(message))
            if message == "fixture failure"
    ));
}

#[test]
fn completed_visual_asset_rejects_wrong_project_family() {
    let spec = generation_spec_model();
    let mut asset = media_asset_model();
    asset.project_id = spec.project_id + 1;
    let error = validate_completed_visual_asset(&spec, &asset)
        .expect_err("completed visual asset must belong to the spec project family");
    assert!(error.to_string().contains("绑定媒体项目族不一致"));
}

#[test]
fn galgame_media_asset_source_context_accepts_same_snapshot_without_chapter() {
    let mut asset = media_asset_model();
    asset.source_chapter_id = None;
    asset.source_snapshot_chapter_id = None;

    validate_galgame_media_asset_source_context(&asset, &background_target_context())
        .expect("same source snapshot can be bound without a chapter-specific asset context");
}

#[test]
fn galgame_media_asset_source_context_rejects_missing_snapshot() {
    let mut asset = media_asset_model();
    asset.source_snapshot_id = None;

    let err = validate_galgame_media_asset_source_context(&asset, &background_target_context())
        .expect_err("Galgame media bindings must keep source snapshot context");

    assert!(err.to_string().contains("缺少 source_snapshot_id"));
}

#[test]
fn galgame_media_asset_source_context_rejects_snapshot_mismatch() {
    let mut asset = media_asset_model();
    asset.source_snapshot_id = Some(21);

    let err = validate_galgame_media_asset_source_context(&asset, &background_target_context())
        .expect_err("Galgame media source snapshot must match the target script snapshot");

    assert!(err.to_string().contains("源快照与目标不一致"));
    assert!(err.to_string().contains("asset_source_snapshot_id=21"));
    assert!(err.to_string().contains("target_source_snapshot_id=20"));
}

#[test]
fn galgame_media_asset_source_context_rejects_snapshot_chapter_mismatch() {
    let mut asset = media_asset_model();
    asset.source_snapshot_chapter_id = Some(41);

    let err = validate_galgame_media_asset_source_context(&asset, &background_target_context())
        .expect_err("chapter-scoped Galgame media must match the target scene chapter");

    assert!(err.to_string().contains("源快照章节与目标不一致"));
    assert!(
        err.to_string()
            .contains("asset_source_snapshot_chapter_id=41")
    );
    assert!(
        err.to_string()
            .contains("target_source_snapshot_chapter_id=40")
    );
}

#[test]
fn galgame_media_asset_source_context_rejects_partial_chapter_context() {
    let mut asset = media_asset_model();
    asset.source_snapshot_chapter_id = None;

    let err = validate_galgame_media_asset_source_context(&asset, &background_target_context())
        .expect_err("source_chapter_id without source_snapshot_chapter_id must fail");

    assert!(err.to_string().contains("源章节上下文不完整"));
    assert!(err.to_string().contains("source_snapshot_chapter_id"));
}

#[test]
fn galgame_media_asset_source_context_rejects_snapshot_chapter_without_source_chapter() {
    let mut asset = media_asset_model();
    asset.source_chapter_id = None;

    let err = validate_galgame_media_asset_source_context(&asset, &background_target_context())
        .expect_err("source_snapshot_chapter_id without source_chapter_id must fail");

    assert!(err.to_string().contains("源章节上下文不完整"));
    assert!(err.to_string().contains("source_chapter_id"));
}

#[test]
fn galgame_portrait_spec_family_context_accepts_matching_project_context() {
    let spec = generation_spec_model();

    assert_eq!(
        validate_galgame_portrait_spec_family_context(&spec, 1).unwrap(),
        20
    );
}

#[test]
fn galgame_portrait_spec_family_context_rejects_foreign_root_and_missing_snapshot() {
    let mut spec = generation_spec_model();
    assert!(validate_galgame_portrait_spec_family_context(&spec, 2).is_err());
    spec.source_snapshot_id = None;
    assert!(validate_galgame_portrait_spec_family_context(&spec, 1).is_err());
}

#[test]
fn galgame_portrait_spec_source_chapter_rejects_chapter_context() {
    let mut spec = generation_spec_model();
    spec.source_chapter_id = Some(30);
    spec.source_snapshot_chapter_id = Some(40);

    let err = validate_galgame_portrait_spec_source_chapter(&spec)
        .expect_err("portrait expression specs must stay snapshot-scoped");

    assert!(err.to_string().contains("必须使用源快照级上下文"));
}

#[test]
fn galgame_portrait_position_samples_must_match_source_positions() {
    let samples = vec!["left".to_string()];
    let source_positions = BTreeSet::from(["right".to_string()]);

    let err = validate_galgame_portrait_position_samples(1, &samples, &source_positions)
        .expect_err("position samples must match source line positions");

    assert!(
        err.to_string()
            .contains("position_samples 与 source_line_ids 行站位不一致")
    );
}

fn generation_spec_model() -> generation_spec::Model {
    let now = Utc::now().naive_utc();
    generation_spec::Model {
        id: 100,
        project_id: 10,
        root_project_id: 1,
        source_context_kind: Some("snapshot".to_string()),
        source_snapshot_id: Some(20),
        source_chapter_id: None,
        source_snapshot_chapter_id: None,
        canvas_text_asset_id: None,
        target_kind: "galgame_portrait_expression".to_string(),
        target_entity_kind: "character_profile".to_string(),
        target_entity_id: Some(42),
        status: "draft".to_string(),
        source_version_hash: "hash".to_string(),
        params_json: serde_json::json!({}),
        created_at: now,
        updated_at: now,
    }
}

fn media_asset_model() -> media_asset::Model {
    let now = Utc::now().naive_utc();
    media_asset::Model {
        id: 11,
        project_id: 10,
        root_project_id: 1,
        source_snapshot_id: Some(20),
        source_chapter_id: Some(30),
        source_snapshot_chapter_id: Some(40),
        media_kind: MEDIA_KIND_IMAGE.to_string(),
        origin: "generated".to_string(),
        file_path: Some("projects/10/images/bg.png".to_string()),
        thumbnail_path: None,
        width: Some(1920),
        height: Some(1080),
        duration_secs: None,
        fps: None,
        file_size: None,
        checksum: None,
        status: MEDIA_STATUS_COMPLETED.to_string(),
        error_message: None,
        created_at: now,
        updated_at: now,
    }
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
