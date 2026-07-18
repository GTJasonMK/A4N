use super::{
    deletion::stage_image_delete_file,
    rules::{is_video_reference_image_candidate, resolve_video_reference_image_project_ids},
};
use crate::file_utils::deletable_data_file_path;
use a4n_core::models::ai::generated_image;
use serde_json::json;
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap();
    std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), stamp))
}

fn image_candidate(chapter_id: Option<i64>, image_type: &str) -> generated_image::Model {
    generated_image::Model {
        id: 1,
        project_id: 10,
        image_type: image_type.to_string(),
        file_path: "images/ref.png".to_string(),
        prompt: "prompt".to_string(),
        model: "model".to_string(),
        width: 512,
        height: 512,
        file_size: None,
        ai_score: None,
        is_favorite: false,
        chapter_id,
        panel_id: None,
        metadata_json: None,
        created_at: chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(),
    }
}

#[test]
fn image_delete_file_path_accepts_existing_relative_file() {
    let data_dir = unique_temp_dir("a4n_image_delete_ok");
    let file_path = data_dir.join("projects/1/images/ref.png");
    std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
    std::fs::write(&file_path, b"demo").unwrap();
    let resolved = deletable_data_file_path(&data_dir, "projects/1/images/ref.png", "图片文件")
        .unwrap()
        .unwrap();
    assert_eq!(resolved, file_path.canonicalize().unwrap());
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn image_delete_file_path_skips_missing_file() {
    let data_dir = unique_temp_dir("a4n_image_delete_missing");
    std::fs::create_dir_all(&data_dir).unwrap();
    assert_eq!(
        deletable_data_file_path(&data_dir, "missing.png", "图片文件").unwrap(),
        None
    );
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn image_delete_file_path_rejects_absolute_path() {
    let data_dir = unique_temp_dir("a4n_image_delete_absolute");
    std::fs::create_dir_all(&data_dir).unwrap();
    let absolute = data_dir.join("image.png");
    assert!(
        deletable_data_file_path(&data_dir, &absolute.to_string_lossy(), "图片文件")
            .unwrap_err()
            .contains("相对路径")
    );
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn image_delete_file_path_rejects_directory() {
    let data_dir = unique_temp_dir("a4n_image_delete_directory");
    std::fs::create_dir_all(data_dir.join("images/ref.png")).unwrap();
    assert_eq!(
        deletable_data_file_path(&data_dir, "images/ref.png", "图片文件").unwrap_err(),
        "图片文件不是文件"
    );
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn staged_image_delete_file_can_restore_original_file() {
    let data_dir = unique_temp_dir("a4n_image_delete_restore");
    let file_path = data_dir.join("projects/1/images/ref.png");
    std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
    std::fs::write(&file_path, b"demo").unwrap();
    let canonical = file_path.canonicalize().unwrap();
    let staged = stage_image_delete_file(&canonical).unwrap();
    assert!(!canonical.exists());
    assert!(staged.staged_path.exists());
    staged.restore().unwrap();
    assert_eq!(std::fs::read(&canonical).unwrap(), b"demo");
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn staged_image_delete_file_remove_deletes_staged_file_and_dir() {
    let data_dir = unique_temp_dir("a4n_image_delete_remove");
    let file_path = data_dir.join("projects/1/images/ref.png");
    std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
    std::fs::write(&file_path, b"demo").unwrap();
    let canonical = file_path.canonicalize().unwrap();
    let staged = stage_image_delete_file(&canonical).unwrap();
    let staging_dir = staged.staging_dir.clone();
    staged.remove().unwrap();
    assert!(!canonical.exists());
    assert!(!staging_dir.exists());
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn video_reference_candidate_keeps_original_scope_rules() {
    assert!(is_video_reference_image_candidate(
        &image_candidate(Some(7), "scene"),
        7
    ));
    assert!(is_video_reference_image_candidate(
        &image_candidate(None, "scene"),
        7
    ));
    let mut reusable = image_candidate(Some(9), "scene_material");
    reusable.metadata_json = Some(json!({"profile_kind": "scene", "profile_id": 3}));
    assert!(is_video_reference_image_candidate(&reusable, 7));
    assert!(!is_video_reference_image_candidate(
        &image_candidate(Some(9), "scene"),
        7
    ));
}

#[test]
fn video_reference_image_project_ids_default_to_allowed_scope() {
    assert_eq!(
        resolve_video_reference_image_project_ids(Vec::new(), &[20, 10]).unwrap(),
        vec![10, 20]
    );
}

#[test]
fn video_reference_image_project_ids_deduplicate_requested_scope() {
    assert_eq!(
        resolve_video_reference_image_project_ids(vec![20, 10, 20], &[20, 10]).unwrap(),
        vec![10, 20]
    );
}

#[test]
fn video_reference_image_project_ids_reject_non_positive_ids() {
    assert!(
        resolve_video_reference_image_project_ids(vec![10, 0], &[10])
            .unwrap_err()
            .contains("image_project_ids[1] 必须为正整数")
    );
}

#[test]
fn video_reference_image_project_ids_reject_projects_outside_video_scope() {
    assert!(
        resolve_video_reference_image_project_ids(vec![10, 99], &[10, 20])
            .unwrap_err()
            .contains("不属于当前视频项目允许的素材项目：99")
    );
}
