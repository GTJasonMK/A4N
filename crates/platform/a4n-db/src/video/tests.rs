use super::*;
use a4n_db_models::models::media::{media_asset, media_binding, media_provenance};
use a4n_db_models::models::task::cleanup_outbox;
use a4n_db_models::models::{
    ai::{generated_image, generated_video},
    chapter::{chapter, chapter_video_prompt},
    project::novel_project,
};
use a4n_domain::image::IMAGE_TYPE_CANVAS;
use a4n_domain::media::{
    CreateGeneratedImageRecordDraftInput, CreateGeneratedVideoRecordDraftInput,
    GeneratedVideoErrorStatusBatchInput,
};
use a4n_domain::video_workflow::WORKFLOW_MODE_CANVAS_VIDEO;
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

#[tokio::test]
async fn timeline_prompt_query_returns_the_timeline_projection() {
    let db = setup_db().await;
    let project = freeform_video_project(&db, "时间线提示词窄读").await;
    let chapter = video_test_chapter(&db, project.id).await;
    let now = Utc::now().naive_utc();
    chapter_video_prompt::ActiveModel {
        project_id: Set(project.id),
        chapter_id: Set(chapter.id),
        style: Set(Some("cinematic".to_string())),
        prompt_type: Set(chapter_video_prompt::PROMPT_TYPE_VIDEO.to_string()),
        workflow_mode: Set(chapter_video_prompt::WORKFLOW_MODE_STORYBOARD.to_string()),
        shot_number: Set(1),
        title: Set(Some("时间线镜头".to_string())),
        scene_summary: Set(Some("场景摘要".to_string())),
        visual_prompt: Set("视觉提示词".to_string()),
        motion_prompt: Set(Some("运动提示词".to_string())),
        negative_prompt: Set(Some("无关字段".to_string())),
        duration_secs: Set(5),
        aspect_ratio: Set(Some("16:9".to_string())),
        reference_image_ids_json: Set(Some(json!([1]))),
        status: Set(chapter_video_prompt::STATUS_COMPLETED.to_string()),
        generated_video_id: Set(Some(99)),
        metadata_json: Set(Some(json!({
            "ignored": "timeline projection",
            "prompt_output_language": "zh_cn"
        }))),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let rows = chapter_video_prompt_timeline_records_for_chapter(&db, project.id, chapter.id)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0],
        ChapterVideoPromptTimelineRecord {
            id: rows[0].id,
            generated_video_id: Some(99),
            title: Some("时间线镜头".to_string()),
            scene_summary: Some("场景摘要".to_string()),
            visual_prompt: "视觉提示词".to_string(),
            motion_prompt: Some("运动提示词".to_string()),
        }
    );
    assert_eq!(
        chapter_video_prompt_video_binding_records_for_chapter(&db, project.id, chapter.id)
            .await
            .unwrap(),
        vec![(rows[0].id, Some(99))]
    );
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
            image_type: IMAGE_TYPE_CANVAS.to_string(),
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
            image: crate::video::read_models::generated_image_by_id(&db, image.id)
                .await
                .unwrap()
                .unwrap(),
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

    delete_generated_image_record_with_cleanup(&db, image.id)
        .await
        .unwrap();

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
            project_type: "novel_long".to_string(),
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
        },
    )
    .await
    .unwrap();
    let image = create_generated_image_record(
        &db,
        CreateGeneratedImageRecordDraftInput {
            // A derived video prompt is allowed to reference a root-project image.
            project_id: root.id,
            image_type: "video_reference".to_string(),
            file_path: format!("projects/{}/images/recipe-reference.png", root.id),
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

    delete_generated_image_record_with_cleanup(&db, image.id)
        .await
        .unwrap();

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
            image_type: IMAGE_TYPE_CANVAS.to_string(),
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
            video_type: a4n_domain::video_workflow::VIDEO_TYPE_CANVAS.to_string(),
            workflow_mode: Some(WORKFLOW_MODE_CANVAS_VIDEO.to_string()),
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
        &a4n_domain::media::VideoFileProperties {
            file_size: 1,
            width: video.width,
            height: video.height,
            duration_secs: video.duration_secs,
            fps: video.fps,
        },
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
async fn delete_generated_video_rebinds_prompt_to_completed_candidate() {
    let db = setup_db().await;
    let project = crate::project::create_project_record(
        &db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: "视频引用批量候选".to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .unwrap();
    let chapter = video_test_chapter(&db, project.id).await;
    let prompt = shared_segment_plan(&db, project.id, chapter.id, "replacement").await;
    let video_input = |metadata| CreateGeneratedVideoRecordDraftInput {
        project_id: project.id,
        chapter_id: Some(chapter.id),
        video_type: a4n_domain::video_workflow::VIDEO_TYPE_CHAPTER.to_string(),
        workflow_mode: Some("boundary_frames".to_string()),
        prompt: "replacement candidate".to_string(),
        negative_prompt: None,
        model: "test-model".to_string(),
        width: 1280,
        height: 720,
        duration_secs: 5.0,
        fps: 24,
        source_video_id: None,
        source_image_id: None,
        provider_job_id: None,
        metadata_json: Some(metadata),
    };
    let deleted = create_generated_video_record(&db, video_input(json!({"prompt_id": prompt.id})))
        .await
        .unwrap();
    let candidate =
        create_generated_video_record(&db, video_input(json!({"prompt_id": prompt.id})))
            .await
            .unwrap();
    mark_generated_video_completed_and_link_prompt_record(
        &db,
        deleted.id,
        &format!("projects/{}/videos/deleted.mp4", project.id),
        None,
        &a4n_domain::media::VideoFileProperties {
            file_size: 1024,
            width: deleted.width,
            height: deleted.height,
            duration_secs: deleted.duration_secs,
            fps: deleted.fps,
        },
        None,
        Some(prompt.id),
    )
    .await
    .unwrap();
    mark_generated_video_completed_and_link_prompt_record(
        &db,
        candidate.id,
        &format!("projects/{}/videos/candidate.mp4", project.id),
        None,
        &a4n_domain::media::VideoFileProperties {
            file_size: 1024,
            width: candidate.width,
            height: candidate.height,
            duration_secs: candidate.duration_secs,
            fps: candidate.fps,
        },
        None,
        None,
    )
    .await
    .unwrap();

    delete_generated_video_record_with_cleanup(&db, deleted.id)
        .await
        .unwrap();

    let prompt = chapter_video_prompt::Entity::find_by_id(prompt.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(prompt.generated_video_id, Some(candidate.id));
    assert_eq!(prompt.status, chapter_video_prompt::STATUS_COMPLETED);
}

#[tokio::test]
async fn batch_error_status_updates_many_videos_in_one_contract() {
    let db = setup_db().await;
    let project = freeform_video_project(&db, "批量终止视频状态").await;
    let mut videos = Vec::new();
    for index in 0..20 {
        videos.push(
            create_generated_video_record(
                &db,
                CreateGeneratedVideoRecordDraftInput {
                    project_id: project.id,
                    chapter_id: None,
                    video_type: a4n_domain::video_workflow::VIDEO_TYPE_CANVAS.to_string(),
                    workflow_mode: Some(WORKFLOW_MODE_CANVAS_VIDEO.to_string()),
                    prompt: format!("batch-{index}"),
                    negative_prompt: None,
                    model: "test".to_string(),
                    width: 1280,
                    height: 720,
                    duration_secs: 1.0,
                    fps: 24,
                    source_video_id: None,
                    source_image_id: None,
                    provider_job_id: (index % 2 == 0).then(|| format!("job-{index}")),
                    metadata_json: None,
                },
            )
            .await
            .unwrap(),
        );
    }
    let inputs = videos
        .iter()
        .map(|video| GeneratedVideoErrorStatusBatchInput {
            video_id: video.id,
            prompt_id: None,
            status: a4n_domain::video_workflow::VIDEO_STATUS_INTERRUPTED.to_string(),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        mark_generated_videos_error_status_and_prompt_records(&db, &inputs, "批量取消测试",)
            .await
            .unwrap(),
        Vec::<i64>::new()
    );
    let stored = generated_videos_by_ids(
        &db,
        &videos.iter().map(|video| video.id).collect::<Vec<_>>(),
    )
    .await
    .unwrap();
    assert_eq!(stored.len(), 20);
    assert!(
        stored
            .iter()
            .all(|video| video.status == a4n_domain::video_workflow::VIDEO_STATUS_INTERRUPTED)
    );
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
            video_type: a4n_domain::video_workflow::VIDEO_TYPE_CANVAS.to_string(),
            workflow_mode: Some(WORKFLOW_MODE_CANVAS_VIDEO.to_string()),
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
        &a4n_domain::media::VideoFileProperties {
            file_size: 2048,
            width: video.width,
            height: video.height,
            duration_secs: video.duration_secs,
            fps: video.fps,
        },
        None,
        None,
    )
    .await
    .unwrap();
    let asset = crate::canvas::ensure_canvas_video_media_asset_record(
        &db,
        crate::canvas::CanvasVideoMediaAssetEnsureInput {
            project_id: project.id,
            video: crate::video::read_models::generated_video_by_id(&db, video.id)
                .await
                .unwrap()
                .unwrap(),
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
        Some("测试取消后恢复"),
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
    let duplicate_resume = crate::workflow::resume_workflow_run(&db, run.id)
        .await
        .expect_err("running workflow must reject duplicate resume");
    assert!(matches!(
        duplicate_resume,
        crate::workflow::WorkflowWriteError::StatusConflict { current, operation }
            if current == crate::workflow::WORKFLOW_STATUS_RUNNING
                && operation == "重复恢复工作流"
    ));

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
