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

mod generated_images;

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
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

#[tokio::test]
async fn selecting_segment_candidate_is_unique_and_marks_old_continuity_stale() {
    let db = setup_db().await;
    let root = crate::project::create_project_record(
        &db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: "片段候选采用测试根文本".to_string(),
            description: String::new(),
            project_type: crate::project::PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let chapter = chapter::ActiveModel {
        project_id: Set(root.id),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("正文".to_string()),
        word_count: Set(2),
        status: Set("completed".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let project = crate::project::create_derived_project_record(
        &db,
        1,
        crate::project::CreateDerivedProjectParams {
            root_project_id: root.id,
            derived_kind: crate::project::PROJECT_KIND_VIDEO.to_string(),
            title: "片段候选采用测试".to_string(),
            description: String::new(),
            chapter_scope: crate::project::DerivedChapterScope {
                start_chapter: 1,
                end_chapter: 1,
            },
            creation_options_json: serde_json::json!({}),
        },
    )
    .await
    .unwrap();
    let mut candidate_ids = Vec::new();
    for index in 0..2 {
        let video = create_generated_video_record(
            &db,
            CreateGeneratedVideoRecordDraftInput {
                project_id: project.id,
                chapter_id: Some(chapter.id),
                video_type: "chapter_video".to_string(),
                workflow_mode: Some("boundary_frames".to_string()),
                prompt: format!("candidate {index}"),
                negative_prompt: None,
                model: "test".to_string(),
                width: 1280,
                height: 720,
                duration_secs: 4.0,
                fps: 24,
                source_video_id: None,
                source_image_id: None,
                provider_job_id: None,
                metadata_json: Some(serde_json::json!({"shot_number": 1})),
            },
        )
        .await
        .unwrap();
        let mut active: generated_video::ActiveModel = video.into();
        active.status = Set("completed".to_string());
        active.file_path = Set(Some(format!("videos/candidate-{index}.mp4")));
        candidate_ids.push(active.update(&db).await.unwrap().id);
    }
    chapter_video_prompt::ActiveModel {
        project_id: Set(project.id),
        chapter_id: Set(chapter.id),
        prompt_type: Set("video_prompt".to_string()),
        workflow_mode: Set("first_frame_chain".to_string()),
        shot_number: Set(2),
        visual_prompt: Set("续接".to_string()),
        duration_secs: Set(4),
        status: Set("pending".to_string()),
        metadata_json: Set(Some(serde_json::json!({
            "guidance_mode": "continuity",
            "continuity_source": {"video_id": candidate_ids[0], "shot_number": 1}
        }))),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    select_segment_video_record(&db, project.id, chapter.id, 1, candidate_ids[0])
        .await
        .unwrap();
    select_segment_video_record(&db, project.id, chapter.id, 1, candidate_ids[1])
        .await
        .unwrap();

    let videos = generated_videos_for_chapter(&db, project.id, chapter.id)
        .await
        .unwrap();
    let selected = videos
        .iter()
        .filter(|video| {
            video
                .metadata_json
                .as_ref()
                .and_then(|metadata| metadata.get("selected_for_segment"))
                .and_then(serde_json::Value::as_bool)
                == Some(true)
        })
        .map(|video| video.id)
        .collect::<Vec<_>>();
    assert_eq!(selected, vec![candidate_ids[1]]);
    let prompt = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter.id))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        prompt
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("continuity_stale"))
            .and_then(serde_json::Value::as_bool),
        Some(true)
    );
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
