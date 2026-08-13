use super::*;
use a4n_core::models::media::{media_asset, media_binding, media_provenance};
use a4n_core::models::task::cleanup_outbox;
use a4n_core::models::{
    ai::{generated_image, generated_video},
    chapter::{chapter, chapter_video_prompt},
    project::novel_project,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set,
};
use serde_json::{Value, json};

mod generated_images;

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

fn checkpoint_artifacts_with_flag(
    artifacts_json: Option<&Value>,
    key: &str,
) -> Result<Value, crate::workflow::WorkflowWriteError> {
    let mut artifacts = artifacts_json
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| {
            crate::workflow::WorkflowWriteError::InvalidData(
                "checkpoint artifacts_json 必须是对象".to_string(),
            )
        })?;
    artifacts.insert(key.to_string(), Value::Bool(true));
    Ok(Value::Object(artifacts))
}

async fn freeform_video_project(db: &DatabaseConnection, title: &str) -> novel_project::Model {
    crate::project::create_freeform_video_project_record(
        db,
        1,
        crate::project::CreateFreeformVideoProjectParams {
            title: title.to_string(),
            description: String::new(),
        },
    )
    .await
    .unwrap()
}

async fn video_test_chapter(db: &DatabaseConnection, project_id: i64) -> chapter::Model {
    let now = Utc::now().naive_utc();
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

async fn shared_segment_plan(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
    source_content_sha256: &str,
) -> chapter_video_prompt::Model {
    let now = Utc::now().naive_utc();
    chapter_video_prompt::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(chapter_id),
        style: Set(Some("cinematic".to_string())),
        prompt_type: Set("frame_segment_plan".to_string()),
        workflow_mode: Set("shared_segment_plan".to_string()),
        shot_number: Set(0),
        title: Set(Some("片段规划".to_string())),
        scene_summary: Set(None),
        visual_prompt: Set("{}".to_string()),
        motion_prompt: Set(None),
        negative_prompt: Set(None),
        duration_secs: Set(6),
        aspect_ratio: Set(Some("16:9".to_string())),
        reference_image_ids_json: Set(None),
        status: Set("completed".to_string()),
        generated_video_id: Set(None),
        metadata_json: Set(Some(json!({
            "video_execution_identity": {
                "source_content_sha256": source_content_sha256
            }
        }))),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

#[test]
fn video_read_ids_and_frame_numbers_reject_non_positive_values() {
    assert!(matches!(
        validate_positive_video_i64(0, "project_id"),
        Err(VideoDbError::InvalidArgument(message)) if message == "project_id 必须大于 0"
    ));
    assert!(matches!(
        validate_positive_video_i32(0, "shot_number"),
        Err(VideoDbError::InvalidArgument(message)) if message == "shot_number 必须大于 0"
    ));
    assert!(matches!(
        validate_positive_video_ids(&[1, -2, 3], "image_ids"),
        Err(VideoDbError::InvalidArgument(message))
            if message == "image_ids 必须全部大于 0: -2"
    ));
    assert!(validate_positive_video_ids(&[], "prompt_ids").is_ok());
}

#[test]
fn video_database_error_keeps_db_source() {
    let error = VideoDbError::Database(DbErr::Custom("fixture failure".to_string()));
    assert!(matches!(
        &error,
        VideoDbError::Database(DbErr::Custom(message)) if message == "fixture failure"
    ));
    assert!(error.to_string().contains("fixture failure"));
}

#[tokio::test]
async fn current_shared_segment_plan_query_filters_source_hash() {
    let db = setup_db().await;
    let project = freeform_video_project(&db, "当前片段规划查询").await;
    let chapter = video_test_chapter(&db, project.id).await;
    let current = shared_segment_plan(&db, project.id, chapter.id, "current").await;

    assert_eq!(
        completed_shared_segment_plan_id_for_source_hash(&db, project.id, chapter.id, "current")
            .await
            .unwrap(),
        Some(current.id),
    );
    assert_eq!(
        completed_shared_segment_plan_id_for_source_hash(&db, project.id, chapter.id, "stale")
            .await
            .unwrap(),
        None,
    );
}

#[tokio::test]
async fn delete_generated_image_removes_only_generated_target_binding() {
    let db = setup_db().await;
    let project = freeform_video_project(&db, "图片 target binding 删除测试").await;
    let image = create_generated_image_record(
        &db,
        CreateGeneratedImageRecordDraftInput {
            project_id: project.id,
            image_type: "canvas_image".to_string(),
            file_path: format!("projects/{}/images/delete-binding.png", project.id),
            prompt: "delete binding".to_string(),
            model: "test".to_string(),
            width: 512,
            height: 512,
            file_size: Some(1024),
            chapter_id: None,
            panel_id: None,
            metadata_json: None,
            created_at: None,
        },
    )
    .await
    .unwrap();
    let asset = crate::canvas::ensure_canvas_image_media_asset_record(
        &db,
        crate::canvas::CanvasImageMediaAssetEnsureInput {
            project_id: project.id,
            image: image.clone(),
            generation_spec_id: None,
            origin: "generated".to_string(),
        },
    )
    .await
    .unwrap();
    media_binding::ActiveModel {
        media_asset_id: Set(asset.id),
        target_kind: Set("video_segment".to_string()),
        target_id: Set(999),
        role: Set("generated_result".to_string()),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    delete_generated_image_record(&db, image.id).await.unwrap();

    assert!(
        generated_image::Entity::find_by_id(image.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        media_binding::Entity::find()
            .filter(media_binding::Column::TargetKind.eq("generated_image"))
            .filter(media_binding::Column::TargetId.eq(image.id))
            .count(&db)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        media_binding::Entity::find()
            .filter(media_binding::Column::MediaAssetId.eq(asset.id))
            .filter(media_binding::Column::TargetKind.eq("video_segment"))
            .count(&db)
            .await
            .unwrap(),
        1
    );
    assert!(
        media_asset::Entity::find_by_id(asset.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        media_provenance::Entity::find_by_id(asset.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn delete_generated_image_clears_every_segment_recipe_image_reference() {
    let db = setup_db().await;
    let root = crate::project::create_project_record(
        &db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: "片段方案图片引用删除测试".to_string(),
            description: String::new(),
            project_type: crate::project::PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .unwrap();
    let chapter = video_test_chapter(&db, root.id).await;
    let project = crate::project::create_derived_project_record(
        &db,
        1,
        crate::project::CreateDerivedProjectParams {
            root_project_id: root.id,
            derived_kind: crate::project::PROJECT_KIND_VIDEO.to_string(),
            title: "片段方案视频派生".to_string(),
            description: String::new(),
            chapter_scope: crate::project::DerivedChapterScope {
                start_chapter: 1,
                end_chapter: 1,
            },
            creation_options_json: json!({}),
        },
    )
    .await
    .unwrap();
    let image = create_generated_image_record(
        &db,
        CreateGeneratedImageRecordDraftInput {
            project_id: project.id,
            image_type: "video_reference".to_string(),
            file_path: format!("projects/{}/images/recipe-reference.png", project.id),
            prompt: "recipe reference".to_string(),
            model: "test".to_string(),
            width: 512,
            height: 512,
            file_size: Some(1024),
            chapter_id: Some(chapter.id),
            panel_id: None,
            metadata_json: None,
            created_at: None,
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let plan = chapter_video_prompt::ActiveModel {
        project_id: Set(project.id),
        chapter_id: Set(chapter.id),
        style: Set(Some("cinematic".to_string())),
        prompt_type: Set("frame_segment_plan".to_string()),
        workflow_mode: Set("shared_segment_plan".to_string()),
        shot_number: Set(1),
        title: Set(Some("片段规划".to_string())),
        scene_summary: Set(None),
        visual_prompt: Set("{}".to_string()),
        motion_prompt: Set(None),
        negative_prompt: Set(None),
        duration_secs: Set(6),
        aspect_ratio: Set(Some("16:9".to_string())),
        reference_image_ids_json: Set(None),
        status: Set("completed".to_string()),
        generated_video_id: Set(None),
        metadata_json: Set(Some(json!({
            "segment_recipes": [{
                "schema_version": "video_segment_execution_recipe_v1",
                "segment_number": 1,
                "workflow_mode": "storyboard",
                "style": "cinematic",
                "prompt_output_language": "zh_cn",
                "duration_secs": 6,
                "duration_secs_options": [6],
                "aspect_ratio": "16:9",
                "reference_image_ids": [image.id, 101],
                "video_library_image_ids": [image.id, 102],
                "guidance_mode": "keyframes",
                "guidance_reference_image_ids": [image.id, 103],
                "continuity_source_video_id": null,
                "reference_media": null,
                "adopted_video_id": null
            }]
        }))),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    delete_generated_image_record(&db, image.id).await.unwrap();

    let stored = chapter_video_prompt::Entity::find_by_id(plan.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    let recipe = &stored.metadata_json.unwrap()["segment_recipes"][0];
    assert_eq!(recipe["reference_image_ids"], json!([101]));
    assert_eq!(recipe["video_library_image_ids"], json!([102]));
    assert_eq!(recipe["guidance_reference_image_ids"], json!([103]));
}

#[tokio::test]
async fn delete_generated_image_with_cleanup_commits_record_and_file_intent_together() {
    let db = setup_db().await;
    let project = freeform_video_project(&db, "图片 cleanup intent 原子性").await;
    let image = create_generated_image_record(
        &db,
        CreateGeneratedImageRecordDraftInput {
            project_id: project.id,
            image_type: "canvas_image".to_string(),
            file_path: format!("projects/{}/images/atomic.png", project.id),
            prompt: "atomic cleanup".to_string(),
            model: "test".to_string(),
            width: 512,
            height: 512,
            file_size: Some(1),
            chapter_id: None,
            panel_id: None,
            metadata_json: None,
            created_at: None,
        },
    )
    .await
    .unwrap();

    delete_generated_image_record_with_cleanup(&db, image.id)
        .await
        .unwrap();

    let intent = cleanup_outbox::Entity::find()
        .filter(
            cleanup_outbox::Column::DedupeKey.eq(format!("generated-image-delete:{}", image.id)),
        )
        .one(&db)
        .await
        .unwrap()
        .expect("图片记录删除必须留下可重试的文件 cleanup intent");
    assert_eq!(intent.project_id, Some(project.id));
    assert_eq!(intent.payload_json["paths"][0], image.file_path);
}

#[tokio::test]
async fn delete_generated_video_with_cleanup_records_video_and_thumbnail_paths() {
    let db = setup_db().await;
    let project = freeform_video_project(&db, "视频 cleanup intent").await;
    let video = create_generated_video_record(
        &db,
        CreateGeneratedVideoRecordDraftInput {
            project_id: project.id,
            chapter_id: None,
            video_type: "canvas_video".to_string(),
            workflow_mode: Some("canvas_video".to_string()),
            prompt: "cleanup".to_string(),
            negative_prompt: None,
            model: "test".to_string(),
            width: 1280,
            height: 720,
            duration_secs: 1.0,
            fps: 24,
            source_video_id: None,
            source_image_id: None,
            provider_job_id: None,
            metadata_json: None,
        },
    )
    .await
    .unwrap();
    let thumbnail = format!("projects/{}/videos/atomic.jpg", project.id);
    let video = mark_generated_video_completed_and_link_prompt_record(
        &db,
        video.id,
        &format!("projects/{}/videos/atomic.mp4", project.id),
        Some(&thumbnail),
        1,
        None,
        None,
    )
    .await
    .unwrap();

    delete_generated_video_record_with_cleanup(&db, video.id)
        .await
        .unwrap();

    let intent = cleanup_outbox::Entity::find()
        .filter(
            cleanup_outbox::Column::DedupeKey.eq(format!("generated-video-delete:{}", video.id)),
        )
        .one(&db)
        .await
        .unwrap()
        .expect("视频记录删除必须留下文件 cleanup intent");
    assert_eq!(intent.payload_json["paths"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn missing_video_repair_uses_binding_project_and_removes_dangling_binding() {
    let db = setup_db().await;
    let project = freeform_video_project(&db, "缺失视频 binding 修复测试").await;
    let video = create_generated_video_record(
        &db,
        CreateGeneratedVideoRecordDraftInput {
            project_id: project.id,
            chapter_id: None,
            video_type: "canvas_video".to_string(),
            workflow_mode: Some("canvas_video".to_string()),
            prompt: "missing video".to_string(),
            negative_prompt: None,
            model: "test".to_string(),
            width: 1280,
            height: 720,
            duration_secs: 5.0,
            fps: 24,
            source_video_id: None,
            source_image_id: None,
            provider_job_id: None,
            metadata_json: None,
        },
    )
    .await
    .unwrap();
    let video = mark_generated_video_completed_and_link_prompt_record(
        &db,
        video.id,
        &format!("projects/{}/videos/missing-binding.mp4", project.id),
        None,
        2048,
        None,
        None,
    )
    .await
    .unwrap();
    let asset = crate::canvas::ensure_canvas_video_media_asset_record(
        &db,
        crate::canvas::CanvasVideoMediaAssetEnsureInput {
            project_id: project.id,
            video: video.clone(),
            generation_spec_id: None,
            origin: "generated".to_string(),
        },
    )
    .await
    .unwrap();
    generated_video::Entity::delete_by_id(video.id)
        .exec(&db)
        .await
        .unwrap();

    assert_eq!(
        missing_video_reference_project_ids(&db, video.id)
            .await
            .unwrap(),
        vec![project.id]
    );
    assert_eq!(
        delete_missing_generated_video_references_record(&db, video.id)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        media_binding::Entity::find()
            .filter(media_binding::Column::TargetKind.eq("generated_video"))
            .filter(media_binding::Column::TargetId.eq(video.id))
            .count(&db)
            .await
            .unwrap(),
        0
    );
    assert!(
        media_asset::Entity::find_by_id(asset.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        media_provenance::Entity::find_by_id(asset.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn segment_plan_draft_query_projects_only_restoration_fields() {
    let db = setup_db().await;
    let run = crate::workflow::upsert_workflow_run(
        &db,
        crate::workflow::UpsertWorkflowRunInput {
            business_key: format!("video-segment-plan-test:{}", uuid::Uuid::new_v4()),
            workflow_kind: "video_segment_plan".to_string(),
            project_id: None,
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: None,
            scope_id: None,
            chapter_id: None,
            mode: Some("shared_segment_plan".to_string()),
            input_fingerprint_json: None,
            status: crate::workflow::WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: None,
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        },
    )
    .await
    .unwrap();
    let large_context = "x".repeat(10_000);
    crate::workflow::upsert_workflow_checkpoint(
        &db,
        crate::workflow::UpsertWorkflowCheckpointInput {
            workflow_run_id: run.id,
            step_key: "video_segment_plan.shot.unit_1_shot_1".to_string(),
            step_order: 200,
            status: crate::workflow::WORKFLOW_STATUS_COMPLETED.to_string(),
            input_fingerprint_sha256: Some("fingerprint".to_string()),
            input_fingerprint_json: Some(json!({
                "large_context": large_context,
                "extra": { "shot_index": 1, "total_shots": 1 }
            })),
            checkpoint_json: Some(json!({
                "output": [{ "segment_number": 1 }],
                "parsed_output": [{ "segment_number": 1 }],
                "response_content": "x".repeat(10_000)
            })),
            artifacts_json: Some(json!({
                "segment_recipes": [{ "segment_number": 1 }],
                "preserved": true
            })),
            generation_run_ids_json: Some(json!([9])),
            error_message: None,
            completed: true,
        },
    )
    .await
    .unwrap();
    crate::workflow::upsert_workflow_checkpoint(
        &db,
        crate::workflow::UpsertWorkflowCheckpointInput {
            workflow_run_id: run.id,
            step_key: "video_segment_plan.shot.unit_1_shot_1:schema_repair".to_string(),
            step_order: 201,
            status: crate::workflow::WORKFLOW_STATUS_COMPLETED.to_string(),
            input_fingerprint_sha256: None,
            input_fingerprint_json: None,
            checkpoint_json: Some(json!({ "parsed_output": [] })),
            artifacts_json: None,
            generation_run_ids_json: None,
            error_message: None,
            completed: true,
        },
    )
    .await
    .unwrap();
    crate::workflow::upsert_workflow_checkpoint(
        &db,
        crate::workflow::UpsertWorkflowCheckpointInput {
            workflow_run_id: run.id,
            step_key: "videoXsegmentYplan.shot.not_a_video_checkpoint".to_string(),
            step_order: 202,
            status: crate::workflow::WORKFLOW_STATUS_COMPLETED.to_string(),
            input_fingerprint_sha256: None,
            input_fingerprint_json: Some(json!({
                "extra": { "shot_index": 2, "total_shots": 2 }
            })),
            checkpoint_json: Some(json!({
                "parsed_output": [{ "segment_number": 1 }]
            })),
            artifacts_json: Some(json!({
                "segment_recipes": [{ "segment_number": 1 }]
            })),
            generation_run_ids_json: None,
            error_message: None,
            completed: true,
        },
    )
    .await
    .unwrap();

    crate::workflow::mutate_completed_workflow_checkpoint_artifacts_unless_step_exists(
        &db,
        run.id,
        "video_segment_plan.shot.unit_1_shot_1",
        "video_segment_plan.final",
        |parsed_output_json, artifacts_json| {
            assert_eq!(parsed_output_json, Some(&json!([{ "segment_number": 1 }])));
            checkpoint_artifacts_with_flag(artifacts_json, "updated")
        },
    )
    .await
    .unwrap();

    let checkpoints = video_segment_plan_draft_checkpoints(&db, run.id, "video_segment_plan.shot.")
        .await
        .unwrap();
    assert_eq!(checkpoints.len(), 1);
    let projected = &checkpoints[0];
    assert_eq!(
        projected.extra_json,
        Some(json!({ "shot_index": 1, "total_shots": 1 }))
    );
    assert_eq!(
        projected.parsed_output_json,
        Some(json!([{ "segment_number": 1 }]))
    );
    assert_eq!(
        projected.artifacts_json,
        Some(json!({
            "segment_recipes": [{ "segment_number": 1 }],
            "preserved": true,
            "updated": true
        }))
    );
    let first_db = db.clone();
    let second_db = db.clone();
    let first = crate::workflow::mutate_completed_workflow_checkpoint_artifacts_unless_step_exists(
        &first_db,
        run.id,
        "video_segment_plan.shot.unit_1_shot_1",
        "video_segment_plan.final",
        |_, artifacts_json| checkpoint_artifacts_with_flag(artifacts_json, "parallel_first"),
    );
    let second = crate::workflow::mutate_completed_workflow_checkpoint_artifacts_unless_step_exists(
        &second_db,
        run.id,
        "video_segment_plan.shot.unit_1_shot_1",
        "video_segment_plan.final",
        |_, artifacts_json| checkpoint_artifacts_with_flag(artifacts_json, "parallel_second"),
    );
    let (first, second) = tokio::join!(first, second);
    first.unwrap();
    second.unwrap();
    let concurrent = video_segment_plan_draft_checkpoints(&db, run.id, "video_segment_plan.shot.")
        .await
        .unwrap();
    let concurrent_artifacts = concurrent[0].artifacts_json.as_ref().unwrap();
    assert_eq!(
        concurrent_artifacts.get("parallel_first"),
        Some(&Value::Bool(true))
    );
    assert_eq!(
        concurrent_artifacts.get("parallel_second"),
        Some(&Value::Bool(true))
    );

    crate::workflow::mark_workflow_status(
        &db,
        run.id,
        crate::workflow::WORKFLOW_STATUS_CANCELLED,
        true,
        None,
    )
    .await
    .unwrap();
    let resumable_artifacts = json!({
        "segment_recipes": [{ "segment_number": 1 }],
        "preserved": true,
        "updated_while_cancelled": true
    });
    crate::workflow::mutate_completed_workflow_checkpoint_artifacts_unless_step_exists(
        &db,
        run.id,
        "video_segment_plan.shot.unit_1_shot_1",
        "video_segment_plan.final",
        {
            let resumable_artifacts = resumable_artifacts.clone();
            move |_, _| Ok(resumable_artifacts)
        },
    )
    .await
    .unwrap();
    assert_eq!(
        video_segment_plan_draft_checkpoints(&db, run.id, "video_segment_plan.shot.")
            .await
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
            .artifacts_json,
        Some(resumable_artifacts.clone()),
    );
    crate::workflow::resume_workflow_run(&db, run.id)
        .await
        .unwrap();

    crate::workflow::upsert_workflow_checkpoint(
        &db,
        crate::workflow::UpsertWorkflowCheckpointInput {
            workflow_run_id: run.id,
            step_key: "video_segment_plan.final".to_string(),
            step_order: 299,
            status: crate::workflow::WORKFLOW_STATUS_COMPLETED.to_string(),
            input_fingerprint_sha256: None,
            input_fingerprint_json: None,
            checkpoint_json: Some(json!({ "segment_plan": [] })),
            artifacts_json: None,
            generation_run_ids_json: None,
            error_message: None,
            completed: true,
        },
    )
    .await
    .unwrap();
    let conflict =
        crate::workflow::mutate_completed_workflow_checkpoint_artifacts_unless_step_exists(
            &db,
            run.id,
            "video_segment_plan.shot.unit_1_shot_1",
            "video_segment_plan.final",
            |_, _| panic!("blocking checkpoint should prevent mutation"),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        conflict,
        crate::workflow::WorkflowWriteError::StatusConflict { ref current, .. }
            if current == "checkpoint_exists:video_segment_plan.final"
    ));
    let unchanged = video_segment_plan_draft_checkpoints(&db, run.id, "video_segment_plan.shot.")
        .await
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(unchanged.artifacts_json, Some(resumable_artifacts));
}
