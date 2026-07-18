use super::*;
use a4n_core::models::media::{media_asset, media_binding, media_provenance};
use a4n_core::models::{
    ai::{generated_image, generated_video},
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
