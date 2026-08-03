use super::*;
use a4n_core::models::{
    chapter::{chapter, chapter_video_prompt},
    generation::{generation_run, generation_run_target},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set,
};
use serde_json::json;

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

async fn insert_project(db: &DatabaseConnection) -> i64 {
    crate::project::create_project_record(
        db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: "视频提示词事务测试".to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .unwrap()
    .id
}

async fn insert_chapter(db: &DatabaseConnection, project_id: i64) -> chapter::Model {
    let now = chrono::Utc::now().naive_utc();
    chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("章节正文".to_string()),
        word_count: Set(4),
        status: Set("draft".to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

async fn insert_prompt(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
) -> chapter_video_prompt::Model {
    let now = chrono::Utc::now().naive_utc();
    chapter_video_prompt::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(chapter_id),
        style: Set(Some("电影感写实".to_string())),
        prompt_type: Set("video_prompt".to_string()),
        workflow_mode: Set("boundary_frames".to_string()),
        shot_number: Set(1),
        title: Set(Some("镜头一".to_string())),
        scene_summary: Set(Some("雨夜对峙".to_string())),
        visual_prompt: Set("原始提示词".to_string()),
        motion_prompt: Set(Some("慢推镜头".to_string())),
        negative_prompt: Set(None),
        duration_secs: Set(6),
        aspect_ratio: Set(Some("16:9".to_string())),
        reference_image_ids_json: Set(None),
        status: Set("completed".to_string()),
        generated_video_id: Set(None),
        metadata_json: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

fn completed_run_input(
    project_id: i64,
) -> crate::generation::RecordCompletedGenerationRunWithCheckpointInput {
    crate::generation::RecordCompletedGenerationRunWithCheckpointInput {
        run: crate::generation::RecordCompletedGenerationRunInput {
            project_id,
            workflow: "video_prompt_generation".to_string(),
            step_key: "video_prompt_update_transaction_test".to_string(),
            title: "视频提示词事务测试".to_string(),
            model_kind: "llm".to_string(),
            provider: None,
            model: None,
            prompt_template_names_json: None,
            prompt_template_hashes_json: None,
            rendered_messages_json: None,
            rendered_prompts_json: None,
            input_variables_json: None,
            source_refs_json: None,
            media_refs_json: None,
            request_json: None,
            response_summary_json: None,
            targets: Vec::new(),
        },
        checkpoint: crate::generation::GenerationRunCheckpointContext {
            parsed_output_json: Some(json!({"existing": true})),
            artifact_refs_json: Some(json!({
                "targets": [{
                    "target_kind": "existing",
                    "target_table": "existing_table",
                    "target_id": 9,
                }]
            })),
            ..Default::default()
        },
    }
}

fn update_input(visual_prompt: &str) -> UpdateChapterVideoPromptInput {
    UpdateChapterVideoPromptInput {
        visual_prompt: Some(visual_prompt.to_string()),
        ..Default::default()
    }
}

fn sync_draft(
    project_id: i64,
    chapter_id: i64,
    prompt_type: &str,
    workflow_mode: &str,
    shot_number: i32,
) -> ChapterVideoPromptSyncDraft {
    ChapterVideoPromptSyncDraft {
        fields: video::ChapterVideoPromptRecordFields {
            project_id,
            chapter_id,
            style: Some("测试风格".to_string()),
            prompt_type: prompt_type.to_string(),
            workflow_mode: workflow_mode.to_string(),
            shot_number,
            title: Some(format!("镜头 {shot_number}")),
            scene_summary: None,
            visual_prompt: format!("提示词 {shot_number}"),
            motion_prompt: None,
            negative_prompt: None,
            duration_secs: 6,
            aspect_ratio: Some("16:9".to_string()),
            reference_image_ids_json: Some(json!([])),
            metadata_json: Some(json!({
                "contract_version": "test",
                "source": {"content_sha256": "same-source"}
            })),
        },
        generation_trace: None,
    }
}

#[tokio::test]
async fn segment_recipe_update_writes_only_the_locked_plan_metadata() {
    let db = setup_db().await;
    let project_id = insert_project(&db).await;
    let chapter = insert_chapter(&db, project_id).await;
    let original = insert_prompt(&db, project_id, chapter.id).await;
    let segment = json!({
        "schema_version": "video_segment_plan_v2",
        "segment_number": 1,
        "source_shot_id": "shot-1",
        "segment_index_in_shot": 1,
        "shot_segment_count": 1,
        "covered_beat_ids": [],
        "title": "片段 1",
        "scene_summary": "场景",
        "blocking_summary": "调度",
        "spatial_anchors": [],
        "emotion_action_goals": [],
        "video_intent": {"hook":"h","audience_effect":"a","rhythm":"r","visual_priority":"v"},
        "emotional_arc": {"start":"s","turn":"t","end":"e","function":"f"},
        "performance_plan": {"movement_path":"m","body_performance":"b","gaze_and_hands":"g","pause_or_reaction":"p","camera_relationship":"c","time_blocks":[],"action_script":[]},
        "material_coverage": {"required_entities":[],"missing_or_weak_references":[],"risk_note":"r"},
        "transition_to_next": null,
        "source_span_ids": ["span-1"],
        "duration_secs": 6,
        "aspect_ratio": "16:9",
        "reference_image_ids": []
    });
    let recipe = json!({
        "schema_version": "video_segment_execution_recipe_v1",
        "segment_number": 1,
        "workflow_mode": "chat_video",
        "style": "cinematic",
        "prompt_output_language": "zh_cn",
        "duration_secs": 6,
        "duration_secs_options": [6],
        "aspect_ratio": "16:9",
        "reference_image_ids": [],
        "video_library_image_ids": [],
        "guidance_mode": "none",
        "guidance_reference_image_ids": [],
        "continuity_source_video_id": null,
        "reference_media": null,
        "adopted_video_id": null
    });
    let mut active: chapter_video_prompt::ActiveModel = original.clone().into();
    active.style = Set(Some("cinematic".to_string()));
    active.prompt_type = Set("frame_segment_plan".to_string());
    active.workflow_mode = Set("shared_segment_plan".to_string());
    active.shot_number = Set(0);
    active.metadata_json = Set(Some(json!({
        "video_execution_identity": {
            "source_content_sha256": a4n_core::source_spans::source_text_hash(&chapter.content)
        },
        "segments": [segment],
        "segment_recipes": [recipe.clone()]
    })));
    let plan = active.update(&db).await.unwrap();

    let mut updated: a4n_core::video_execution::VideoSegmentExecutionRecipeV1 =
        serde_json::from_value(recipe).unwrap();
    updated.workflow_mode = a4n_core::video_execution::VideoSegmentWorkflowModeV1::Storyboard;
    updated.guidance_mode = a4n_core::video_execution::VideoSegmentGuidanceModeV1::SystemFrames;
    let recipes = update_chapter_video_segment_recipe(&db, plan.id, updated)
        .await
        .unwrap();

    assert_eq!(recipes.len(), 1);
    assert_eq!(
        recipes[0].workflow_mode,
        a4n_core::video_execution::VideoSegmentWorkflowModeV1::Storyboard
    );
    let stored = chapter_video_prompt::Entity::find_by_id(plan.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.visual_prompt, original.visual_prompt);
    assert_eq!(
        stored.metadata_json.unwrap()["segment_recipes"][0]["workflow_mode"],
        "storyboard"
    );
}

#[test]
fn prompt_sync_only_marks_shared_upstream_prompts_completed() {
    assert_eq!(generated_prompt_status("frame_segment_plan"), "completed");
    assert_eq!(generated_prompt_status("video_prompt"), "pending");
}

#[tokio::test]
async fn prompt_sync_empty_drafts_returns_before_database_access() {
    let rows = sync_chapter_video_prompts_with_generation_runs(
        &DatabaseConnection::Disconnected,
        1,
        Vec::new(),
        ChapterVideoPromptSyncPolicy::default(),
    )
    .await
    .expect("empty prompt sync should not access the database");

    assert!(rows.is_empty());
}

#[test]
fn prompt_sync_video_prompt_requires_shared_segment_plan_source() {
    let mut valid = sync_draft(1, 2, "video_prompt", "first_frame_chain", 1);
    valid.fields.metadata_json = Some(json!({
        "source_prompt_type": "frame_segment_plan",
        "source": {"content_sha256": "same-source"},
        "video_execution_identity": {
            "segment_plan_prompt_id": 10,
            "segment_number": 1,
            "segment_recipe_fingerprint": "recipe-1"
        }
    }));
    validate_prompt_sync_draft(1, 2, &valid).unwrap();

    let mut invalid = valid;
    invalid.fields.metadata_json = Some(json!({
        "source_prompt_type": "unknown_segment_plan",
        "source": {"content_sha256": "same-source"},
        "video_execution_identity": {
            "segment_plan_prompt_id": 10,
            "segment_number": 1,
            "segment_recipe_fingerprint": "recipe-1"
        }
    }));
    let err = validate_prompt_sync_draft(1, 2, &invalid)
        .expect_err("video_prompt must use shared segment source");
    assert!(
        matches!(&err, VideoDbError::InvalidArgument(message) if message.contains("source_prompt_type=frame_segment_plan")),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn prompt_sync_trace_failure_rolls_back_stale_delete_and_insert() {
    let db = setup_db().await;
    let project_id = insert_project(&db).await;
    let chapter = insert_chapter(&db, project_id).await;
    let old = video::insert_chapter_video_prompt_record(
        &db,
        sync_draft(
            project_id,
            chapter.id,
            "frame_segment_plan",
            "shared_segment_plan",
            1,
        )
        .fields,
        "completed".to_string(),
        chrono::Utc::now().naive_utc(),
    )
    .await
    .unwrap();
    let mut replacement = sync_draft(
        project_id,
        chapter.id,
        "frame_segment_plan",
        "shared_segment_plan",
        2,
    );
    replacement.generation_trace = Some(PreparedVideoPromptGenerationTrace {
        prompt_name: " ".to_string(),
        request_json: json!({"messages": []}),
        rendered_messages_json: json!([]),
        response_text: "trace failure".to_string(),
        workflow_run_id: None,
        usage_json: None,
    });

    let err = sync_chapter_video_prompts_with_generation_runs(
        &db,
        chapter.id,
        vec![replacement],
        ChapterVideoPromptSyncPolicy::default(),
    )
    .await
    .unwrap_err();

    assert!(
        matches!(&err, VideoDbError::InvalidArgument(message) if message.contains("step_key")),
        "unexpected error: {err}"
    );
    assert!(
        chapter_video_prompt::Entity::find_by_id(old.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(
        chapter_video_prompt::Entity::find()
            .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
            .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
            .filter(chapter_video_prompt::Column::ShotNumber.eq(2))
            .count(&db)
            .await
            .unwrap(),
        0
    );
    assert_eq!(generation_run::Entity::find().count(&db).await.unwrap(), 0);
}

#[tokio::test]
async fn prompt_update_checkpoint_commits_all_records_together() {
    let db = setup_db().await;
    let project_id = insert_project(&db).await;
    let chapter = insert_chapter(&db, project_id).await;
    let prompt = insert_prompt(&db, project_id, chapter.id).await;

    let updated = update_chapter_video_prompt_with_generation_run_checkpoint(
        &db,
        prompt.id,
        update_input("  更新后的提示词  "),
        None,
        false,
        completed_run_input(project_id),
    )
    .await
    .unwrap();

    assert_eq!(updated.visual_prompt, "更新后的提示词");
    let run = generation_run::Entity::find()
        .filter(generation_run::Column::ProjectId.eq(project_id))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(run.status, "completed");
    assert_eq!(
        run.parsed_output_json
            .as_ref()
            .and_then(|value| value.get("existing"))
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        run.parsed_output_json
            .as_ref()
            .and_then(|value| value.get("chapter_video_prompt"))
            .and_then(|value| value.get("chapter_video_prompt_id"))
            .and_then(Value::as_i64),
        Some(prompt.id)
    );
    assert_eq!(
        run.artifact_refs_json
            .as_ref()
            .and_then(|value| value.get("targets"))
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(2)
    );
    let target = generation_run_target::Entity::find()
        .filter(generation_run_target::Column::GenerationRunId.eq(run.id))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(target.target_kind, "chapter_video_prompt");
    assert_eq!(target.target_table, "chapter_video_prompt");
    assert_eq!(target.target_id, prompt.id);
}

#[tokio::test]
async fn prompt_update_checkpoint_project_mismatch_rolls_back_prompt() {
    let db = setup_db().await;
    let project_id = insert_project(&db).await;
    let chapter = insert_chapter(&db, project_id).await;
    let prompt = insert_prompt(&db, project_id, chapter.id).await;

    let err = update_chapter_video_prompt_with_generation_run_checkpoint(
        &db,
        prompt.id,
        update_input("不应提交的提示词"),
        None,
        false,
        completed_run_input(project_id + 999),
    )
    .await
    .unwrap_err();

    assert!(matches!(
        err,
        VideoDbError::InvalidArgument(message)
            if message == "视频提示词生成运行记录 project_id 与提示词资产不一致"
    ));
    let stored = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.visual_prompt, prompt.visual_prompt);
    assert_eq!(generation_run::Entity::find().count(&db).await.unwrap(), 0);
}

#[tokio::test]
async fn prompt_update_checkpoint_invalid_parsed_output_rolls_back_prompt() {
    let db = setup_db().await;
    let project_id = insert_project(&db).await;
    let chapter = insert_chapter(&db, project_id).await;
    let prompt = insert_prompt(&db, project_id, chapter.id).await;
    let mut generation_run = completed_run_input(project_id);
    generation_run.checkpoint.parsed_output_json = Some(json!("invalid"));

    let err = update_chapter_video_prompt_with_generation_run_checkpoint(
        &db,
        prompt.id,
        update_input("不应提交的提示词"),
        None,
        false,
        generation_run,
    )
    .await
    .unwrap_err();

    assert!(matches!(
        err,
        VideoDbError::InvalidArgument(message)
            if message == "视频提示词生成运行记录 parsed_output_json 必须是对象"
    ));
    let stored = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.visual_prompt, prompt.visual_prompt);
    assert_eq!(generation_run::Entity::find().count(&db).await.unwrap(), 0);
}

#[test]
fn frame_prompt_metadata_sync_preserves_unedited_fields() {
    let metadata = json!({
        "boundary_frame": {
            "title": "旧标题",
            "frame_prompt": "旧提示词",
            "negative_prompt": "旧负面词",
            "duration_secs": 4,
            "reference_image_ids": [1]
        }
    });

    let synced = sync_chapter_video_prompt_metadata_after_update(
        "frame_prompt",
        Some(metadata),
        " 新提示词 ",
        Some(" 新负面词 "),
        6,
        Some(&json!([2, 3])),
    )
    .unwrap()
    .unwrap();
    let frame = synced
        .get("boundary_frame")
        .and_then(Value::as_object)
        .unwrap();
    assert_eq!(frame.get("frame_prompt"), Some(&json!("新提示词")));
    assert_eq!(frame.get("negative_prompt"), Some(&json!("新负面词")));
    assert_eq!(frame.get("duration_secs"), Some(&json!(6)));
    assert_eq!(frame.get("reference_image_ids"), Some(&json!([2, 3])));
    assert_eq!(frame.get("title"), Some(&json!("旧标题")));
}
