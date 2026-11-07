use super::bindings::normalized_generated_target_ids;
use super::creation::validate_media_asset_source_context_shape;
use super::paths::{media_path_is_blank, normalized_media_paths};
use super::*;
use a4n_db_models::models::{
    ai::{generated_image, generated_video},
    character::character_portrait,
    media::{media_asset, media_provenance},
};
use a4n_domain::character::CharacterPortraitCreateInput;
use a4n_domain::image::IMAGE_TYPE_CANVAS;
use a4n_domain::media::{CreateMediaAssetRecordInput, MediaProvenanceRecordInput};
use a4n_domain::video_workflow::WORKFLOW_MODE_CANVAS_VIDEO;
use sea_orm::{
    ColumnTrait, ConnectOptions, Database, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter,
};
use serde_json::json;
use std::time::Duration;

mod provenance_context;

async fn setup_db() -> DatabaseConnection {
    let db = a4n_migration::test_support::connect_test_database().await;
    a4n_test_support::database::ensure_test_user(&db, 1).await;
    db
}

#[test]
fn media_asset_source_chapter_context_must_be_paired() {
    let mut input = media_asset_input();
    input.source_chapter_id = Some(10);
    let err = validate_media_asset_source_context_shape(&input)
        .expect_err("source chapter ids must be paired");
    assert!(matches!(
        err,
        MediaDbError::InvalidArgument(message)
            if message == "source_chapter_id 和 source_snapshot_chapter_id 必须同时提供或同时为空"
    ));

    input.source_snapshot_chapter_id = Some(20);
    validate_media_asset_source_context_shape(&input)
        .expect("paired source chapter ids should pass shape validation");
}

#[test]
fn generated_target_ids_are_positive_sorted_and_deduplicated() {
    assert_eq!(
        normalized_generated_target_ids(&[5, 2, 5, 0, -3, 2, 9]),
        vec![2, 5, 9]
    );
}

#[test]
fn media_paths_are_non_blank_sorted_and_deduplicated() {
    assert!(media_path_is_blank(""));
    assert!(media_path_is_blank("   "));
    assert!(!media_path_is_blank(" a.png "));
    assert_eq!(
        normalized_media_paths(&[
            "b.mp4".to_string(),
            String::new(),
            "   ".to_string(),
            "a.png".to_string(),
            "b.mp4".to_string(),
        ])
        .into_iter()
        .collect::<Vec<_>>(),
        vec!["a.png".to_string(), "b.mp4".to_string()]
    );
}

#[tokio::test]
async fn media_cleanup_does_not_fall_back_when_reference_query_fails() {
    let mut options = ConnectOptions::new("postgres://a4n@127.0.0.1:1/a4n");
    options
        .connect_lazy(true)
        .connect_timeout(Duration::from_millis(100));
    let db = Database::connect(options)
        .await
        .expect("lazy database connection should be constructible");
    let error = super::schedule_unreferenced_media_cleanup(
        &db,
        1,
        &["projects/1/unresolved.png".to_string()],
        "media-reference-query-failure".to_string(),
    )
    .await
    .expect_err("引用复核失败时不得继续写入 cleanup intent");

    assert!(error.to_string().contains("Connection"));
}

#[tokio::test]
async fn media_path_filter_covers_all_persisted_path_columns() {
    let db = setup_db().await;
    let project = crate::project::create_canvas_project_record(
        &db,
        1,
        crate::project::CreateCanvasProjectParams {
            title: "视频路径引用测试".to_string(),
            description: String::new(),
        },
    )
    .await
    .unwrap();
    let image_path = format!("projects/{}/images/ref.png", project.id);
    let image = crate::video::create_generated_image_record(
        &db,
        a4n_domain::media::CreateGeneratedImageRecordDraftInput {
            project_id: project.id,
            image_type: IMAGE_TYPE_CANVAS.to_string(),
            file_path: image_path.clone(),
            prompt: "path reference".to_string(),
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
    let video = crate::video::create_generated_video_record(
        &db,
        a4n_domain::media::CreateGeneratedVideoRecordDraftInput {
            project_id: project.id,
            chapter_id: None,
            video_type: a4n_domain::video_workflow::VIDEO_TYPE_CANVAS.to_string(),
            workflow_mode: Some(WORKFLOW_MODE_CANVAS_VIDEO.to_string()),
            prompt: "path reference".to_string(),
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
    let video_path = format!("projects/{}/videos/ref.mp4", project.id);
    let video_thumbnail_path = format!("projects/{}/videos/ref.jpg", project.id);
    let video = crate::video::mark_generated_video_completed_and_link_prompt_record(
        &db,
        video.id,
        &video_path,
        Some(&video_thumbnail_path),
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
    let profile = crate::character::create_profile(
        &db,
        project.id,
        "路径测试角色",
        json!({
            "explicit_attributes": {},
            "implicit_attributes": {},
            "social_attributes": {},
        }),
    )
    .await
    .unwrap();
    let portrait_path = format!("projects/{}/portraits/ref.png", project.id);
    let portrait = crate::character::create_character_portrait_for_profile(
        &db,
        CharacterPortraitCreateInput {
            profile_id: profile.id,
            project_id: project.id,
            portrait_type: "full_body".to_string(),
            image_path: portrait_path.clone(),
            prompt: "path reference".to_string(),
            expression: None,
            pose: None,
        },
    )
    .await
    .unwrap();
    let asset_path = format!("projects/{}/assets/ref.png", project.id);
    let asset_thumbnail_path = format!("projects/{}/assets/ref-thumb.png", project.id);
    let mut asset_input = media_asset_input();
    asset_input.project_id = project.id;
    asset_input.root_project_id = project.id;
    asset_input.file_path = Some(asset_path.clone());
    asset_input.thumbnail_path = Some(asset_thumbnail_path.clone());
    asset_input.status = "ready".to_string();
    let asset = create_media_asset_with_provenance_record(&db, asset_input)
        .await
        .unwrap();
    let candidates = vec![
        image_path.clone(),
        video_path.clone(),
        video_thumbnail_path.clone(),
        portrait_path.clone(),
        asset_path.clone(),
        asset_thumbnail_path.clone(),
        "   ".to_string(),
    ];

    assert!(
        filter_unreferenced_media_paths(&db, &candidates)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        media_path_prefix_has_persisted_reference(&db, &format!("projects/{}/", project.id))
            .await
            .unwrap()
    );
    assert!(
        !media_path_prefix_has_persisted_reference(&db, &format!("projects/{}0/", project.id))
            .await
            .unwrap()
    );

    let mut batched_candidates = (0..=500)
        .map(|index| format!("batch/unreferenced-{index:03}.bin"))
        .collect::<Vec<_>>();
    batched_candidates.push(asset_thumbnail_path.clone());
    assert_eq!(
        filter_unreferenced_media_paths(&db, &batched_candidates)
            .await
            .unwrap(),
        batched_candidates[..=500]
    );

    generated_image::Entity::delete_by_id(image.id)
        .exec(&db)
        .await
        .unwrap();
    generated_video::Entity::delete_by_id(video.id)
        .exec(&db)
        .await
        .unwrap();
    character_portrait::Entity::delete_by_id(portrait.id)
        .exec(&db)
        .await
        .unwrap();
    media_provenance::Entity::delete_many()
        .filter(media_provenance::Column::MediaAssetId.eq(asset.id))
        .exec(&db)
        .await
        .unwrap();
    media_asset::Entity::delete_by_id(asset.id)
        .exec(&db)
        .await
        .unwrap();
    let mut expected = candidates
        .iter()
        .filter(|path| !media_path_is_blank(path))
        .cloned()
        .collect::<Vec<_>>();
    expected.sort();
    assert_eq!(
        filter_unreferenced_media_paths(&db, &candidates)
            .await
            .unwrap(),
        expected
    );
    assert!(
        !media_path_prefix_has_persisted_reference(&db, &format!("projects/{}/", project.id))
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn media_asset_provenance_failure_rolls_back_asset_insert() {
    let db = setup_db().await;
    let project = crate::project::create_canvas_project_record(
        &db,
        1,
        crate::project::CreateCanvasProjectParams {
            title: "媒体事务回滚测试".to_string(),
            description: String::new(),
        },
    )
    .await
    .unwrap();
    let mut input = media_asset_input();
    input.project_id = project.id;
    input.root_project_id = project.id;
    input.provenance.generation_spec_id = Some(i64::MAX);

    let err = create_media_asset_with_provenance_record(&db, input)
        .await
        .expect_err("invalid provenance must roll back the inserted asset");

    assert!(matches!(err, MediaDbError::NotFound(message) if message.contains("生成意图不存在")));
    assert_eq!(
        media_asset::Entity::find()
            .filter(media_asset::Column::ProjectId.eq(project.id))
            .count(&db)
            .await
            .unwrap(),
        0
    );
}

fn media_asset_input() -> CreateMediaAssetRecordInput {
    CreateMediaAssetRecordInput {
        project_id: 1,
        root_project_id: 1,
        source_snapshot_id: None,
        source_chapter_id: None,
        source_snapshot_chapter_id: None,
        media_kind: "image".to_string(),
        origin: "imported".to_string(),
        file_path: None,
        thumbnail_path: None,
        width: None,
        height: None,
        duration_secs: None,
        fps: None,
        file_size: None,
        checksum: None,
        status: "pending".to_string(),
        error_message: None,
        provenance: MediaProvenanceRecordInput {
            generation_spec_id: None,
            prompt_artifact_id: None,
            provider_job_id: None,
            source_media_asset_id: None,
            source_external_path: None,
            imported_from_user: false,
            request_snapshot_json: None,
        },
    }
}
