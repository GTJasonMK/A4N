use a4n_db_models::models::media::media_asset;
use chrono::Utc;

use crate::project::duplicate::ProjectDuplicateError;

use super::rules::ensure_copyable_derived_media_asset;

fn asset(status: &str, file_path: Option<&str>) -> media_asset::Model {
    let now = Utc::now().naive_utc();
    media_asset::Model {
        id: 9,
        project_id: 7,
        root_project_id: 1,
        source_snapshot_id: Some(2),
        source_chapter_id: None,
        source_snapshot_chapter_id: None,
        media_kind: "video".to_string(),
        origin: "generated".to_string(),
        file_path: file_path.map(str::to_string),
        thumbnail_path: None,
        width: None,
        height: None,
        duration_secs: None,
        fps: None,
        file_size: None,
        checksum: None,
        status: status.to_string(),
        error_message: None,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn completed_derived_media_asset_requires_file_path() {
    let error = ensure_copyable_derived_media_asset(&asset("completed", None)).unwrap_err();

    assert!(matches!(
        error,
        ProjectDuplicateError::InvalidState { current, operation }
            if current == "已完成媒体资产 9 缺少 file_path"
                && operation == "复制派生项目媒体资产"
    ));
}

#[test]
fn unfinished_or_materialized_derived_media_asset_is_copyable() {
    assert!(ensure_copyable_derived_media_asset(&asset("pending", None)).is_ok());
    assert!(ensure_copyable_derived_media_asset(&asset("completed", Some("video.mp4"))).is_ok());
}
