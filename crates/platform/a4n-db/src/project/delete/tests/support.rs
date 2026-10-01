use a4n_db_models::models::{ai::generated_image, chapter::chapter, project::novel_project};
use a4n_domain::image::IMAGE_TYPE_CANVAS;
use a4n_domain::video_workflow::WORKFLOW_MODE_CANVAS_VIDEO;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};

pub(super) async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

pub(super) async fn set_project_gate(
    db: &DatabaseConnection,
    project: &a4n_domain::project::ProjectRecord,
    operation: a4n_domain::generation::ProjectLifecycleOperation,
) {
    let active = novel_project::ActiveModel {
        id: Set(project.id),
        lifecycle_gate: Set(operation.gate().to_string()),
        ..Default::default()
    };
    active.update(db).await.unwrap();
}

pub(super) async fn freeform_video_project(
    db: &DatabaseConnection,
    title: &str,
) -> a4n_domain::project::ProjectRecord {
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

pub(super) async fn text_project(
    db: &DatabaseConnection,
    title: &str,
) -> a4n_domain::project::ProjectRecord {
    crate::project::create_project_record(
        db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: title.to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .unwrap()
}

pub(super) async fn root_with_video_derivative(
    db: &DatabaseConnection,
) -> (
    a4n_domain::project::ProjectRecord,
    a4n_domain::project::ProjectRecord,
) {
    let root = text_project(db, "根文本").await;
    let now = Utc::now().naive_utc();
    chapter::ActiveModel {
        project_id: Set(root.id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("他推开门，走进雨夜。".to_string()),
        word_count: Set(10),
        status: Set(chapter::STATUS_IMPORTED.to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    let derived = crate::project::create_derived_project_record(
        db,
        1,
        crate::project::CreateDerivedProjectParams {
            root_project_id: root.id,
            derived_kind: crate::project::PROJECT_KIND_VIDEO.to_string(),
            title: "视频项目".to_string(),
            description: "视频派生".to_string(),
            chapter_scope: crate::project::DerivedChapterScope {
                start_chapter: 1,
                end_chapter: 1,
            },
        },
    )
    .await
    .unwrap();
    (root, derived)
}

pub(super) async fn generated_image_with_asset(
    db: &DatabaseConnection,
    project_id: i64,
    file_name: &str,
) -> (generated_image::Model, a4n_domain::media::MediaAssetRecord) {
    let image_record = crate::video::create_generated_image_record(
        db,
        a4n_domain::media::CreateGeneratedImageRecordDraftInput {
            project_id,
            image_type: IMAGE_TYPE_CANVAS.to_string(),
            file_path: format!("projects/{project_id}/images/{file_name}"),
            prompt: "project cleanup image".to_string(),
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
        db,
        crate::canvas::CanvasImageMediaAssetEnsureInput {
            project_id,
            image: image_record.clone(),
            generation_spec_id: None,
            origin: "generated".to_string(),
        },
    )
    .await
    .unwrap();
    let image = generated_image::Entity::find_by_id(image_record.id)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    (image, asset)
}

pub(super) async fn generated_video_with_asset(
    db: &DatabaseConnection,
    project_id: i64,
    file_name: &str,
) -> (
    a4n_domain::media::GeneratedVideoRecord,
    a4n_domain::media::MediaAssetRecord,
) {
    let video = crate::video::create_generated_video_record(
        db,
        a4n_domain::media::CreateGeneratedVideoRecordDraftInput {
            project_id,
            chapter_id: None,
            video_type: a4n_domain::video_workflow::VIDEO_TYPE_CANVAS.to_string(),
            workflow_mode: Some(WORKFLOW_MODE_CANVAS_VIDEO.to_string()),
            prompt: "project cleanup video".to_string(),
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
    let video = crate::video::mark_generated_video_completed_and_link_prompt_record(
        db,
        video.id,
        &format!("projects/{project_id}/videos/{file_name}"),
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
        db,
        crate::canvas::CanvasVideoMediaAssetEnsureInput {
            project_id,
            video: video.clone(),
            generation_spec_id: None,
            origin: "generated".to_string(),
        },
    )
    .await
    .unwrap();
    (video, asset)
}
