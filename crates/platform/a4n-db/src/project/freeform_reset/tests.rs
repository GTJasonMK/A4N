use super::FreeformGeneratedResetDbError;
use crate::project::cleanup::ProjectCleanupDbError;
use a4n_db_models::models::{
    ai::{generated_image, generated_video},
    media::media_asset,
};
use a4n_domain::image::IMAGE_TYPE_CANVAS;
use a4n_domain::video_workflow::WORKFLOW_MODE_CANVAS_VIDEO;
use sea_orm::DbErr;
use serde_json::json;
use std::error::Error;

fn test_time(offset_seconds: i64) -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
        .expect("valid test date")
        .and_hms_opt(0, 0, 0)
        .expect("valid test time")
        + chrono::Duration::seconds(offset_seconds)
}

fn canvas_image_row(
    model: &str,
    metadata_json: Option<serde_json::Value>,
) -> generated_image::Model {
    generated_image::Model {
        id: 41,
        project_id: 9,
        image_type: IMAGE_TYPE_CANVAS.to_string(),
        file_path: "projects/9/images/imported.png".to_string(),
        prompt: "导入画布图片".to_string(),
        model: model.to_string(),
        width: 1024,
        height: 1024,
        file_size: Some(4),
        ai_score: None,
        is_favorite: false,
        chapter_id: None,
        panel_id: None,
        metadata_json,
        created_at: test_time(0),
    }
}

fn canvas_video_row(
    model: &str,
    metadata_json: Option<serde_json::Value>,
) -> generated_video::Model {
    generated_video::Model {
        id: 42,
        project_id: 9,
        chapter_id: None,
        video_type: a4n_domain::video_workflow::VIDEO_TYPE_CANVAS.to_string(),
        workflow_mode: Some(WORKFLOW_MODE_CANVAS_VIDEO.to_string()),
        file_path: Some("projects/9/videos/canvas.mp4".to_string()),
        thumbnail_path: Some("projects/9/videos/canvas.jpg".to_string()),
        prompt: "画布视频".to_string(),
        negative_prompt: None,
        model: model.to_string(),
        width: 1280,
        height: 720,
        duration_secs: 6.0,
        fps: 24,
        file_size: Some(4),
        source_video_id: None,
        source_image_id: None,
        status: "completed".to_string(),
        provider_job_id: None,
        error_message: None,
        timeline_item_id: None,
        metadata_json,
        is_favorite: false,
        created_at: test_time(0),
        updated_at: test_time(0),
    }
}

fn media_asset_row(origin: &str) -> media_asset::Model {
    media_asset::Model {
        id: 43,
        project_id: 9,
        root_project_id: 9,
        source_snapshot_id: None,
        source_chapter_id: None,
        source_snapshot_chapter_id: None,
        media_kind: "video".to_string(),
        origin: origin.to_string(),
        file_path: Some("projects/9/media/asset.mp4".to_string()),
        thumbnail_path: None,
        width: Some(1280),
        height: Some(720),
        duration_secs: Some(6.0),
        fps: Some(24.0),
        file_size: Some(4),
        checksum: Some("checksum".to_string()),
        status: "completed".to_string(),
        error_message: None,
        created_at: test_time(0),
        updated_at: test_time(0),
    }
}

#[test]
fn freeform_generated_reset_error_classifies_and_preserves_sources() {
    let missing = FreeformGeneratedResetDbError::ProjectNotFound { project_id: 17 };
    assert_eq!(missing.to_string(), "项目不存在: 17");
    assert!(missing.source().is_none());

    let unsupported = FreeformGeneratedResetDbError::UnsupportedProject;
    assert_eq!(
        unsupported.to_string(),
        "仅自由画布视频项目支持该生成数据重置入口"
    );

    let cleanup = FreeformGeneratedResetDbError::from(ProjectCleanupDbError::Database {
        source: DbErr::Custom("cleanup failed".to_string()),
    });
    assert!(matches!(
        cleanup,
        FreeformGeneratedResetDbError::ProjectCleanup { .. }
    ));
    assert!(cleanup.source().is_some());

    let canvas = FreeformGeneratedResetDbError::from(
        crate::canvas::CanvasDbError::InvalidArgument("invalid canvas".to_string()),
    );
    assert!(matches!(
        canvas,
        FreeformGeneratedResetDbError::Canvas { .. }
    ));
    assert!(canvas.source().is_some());

    let database = FreeformGeneratedResetDbError::from(DbErr::Custom("begin failed".into()));
    assert!(matches!(
        database,
        FreeformGeneratedResetDbError::Database {
            source: DbErr::Custom(ref message),
        } if message == "begin failed"
    ));
    assert!(database.source().is_some());
}

#[test]
fn freeform_reset_preserves_imported_canvas_image_and_video_records() {
    let imported_image = canvas_image_row(
        "external-import",
        Some(json!({
            "canvas_import": true,
            "source": "video_canvas"
        })),
    );
    let imported_video = canvas_video_row(
        "external-import",
        Some(json!({
            "canvas_import": {
                "external": true,
                "source_file_name": "clip.mp4"
            }
        })),
    );

    assert!(super::classification::is_freeform_imported_canvas_image(
        &imported_image
    ));
    assert!(super::classification::is_freeform_imported_canvas_video(
        &imported_video
    ));
}

#[test]
fn freeform_reset_deletes_generated_canvas_media_and_preserves_imported_assets() {
    let generated_image = canvas_image_row("image-provider", Some(json!({"canvas_import": true})));
    let generated_video =
        canvas_video_row("video-provider", Some(json!({"canvas": {"node_id": 1}})));
    let imported_asset = media_asset_row("imported");
    let generated_asset = media_asset_row("generated");
    let preview_asset = media_asset_row("assembled_preview");

    assert!(!super::classification::is_freeform_imported_canvas_image(
        &generated_image
    ));
    assert!(!super::classification::is_freeform_imported_canvas_video(
        &generated_video
    ));
    assert!(!super::classification::is_freeform_generated_media_asset(
        &imported_asset
    ));
    assert!(super::classification::is_freeform_generated_media_asset(
        &generated_asset
    ));
    assert!(super::classification::is_freeform_generated_media_asset(
        &preview_asset
    ));
}
