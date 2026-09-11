use super::files::{
    persist_reserved_generated_image_file, prepare_generated_image_file, required_filename_segment,
};
use a4n_orchestrator::media::{PersistGeneratedImageFileInput, PrepareGeneratedImageFileInput};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after UNIX_EPOCH")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), stamp))
}

#[test]
fn generated_image_file_persistence_keeps_output_and_reports_actual_size() {
    let data_dir = unique_temp_dir("a4n_generated_image_persist");

    let prepared = prepare_generated_image_file(
        &data_dir,
        PrepareGeneratedImageFileInput {
            project_id: 7,
            chapter_id: Some(3),
            image_type: "scene",
            file_extra: Some("panel-1"),
            format: "png",
            publication_id: "publication-test",
            file_context: "Provider 图片文件",
        },
    )
    .expect("prepare generated image file");
    let persisted = persist_reserved_generated_image_file(
        &data_dir,
        PersistGeneratedImageFileInput {
            project_id: 7,
            chapter_id: Some(3),
            image_type: "scene",
            file_extra: Some("panel-1"),
            bytes: b"generated-image",
            format: "png",
            file_context: "Provider 图片文件",
        },
        &prepared,
    )
    .expect("persist generated image file");
    let path = data_dir.join(&persisted.relative_path);

    assert!(path.is_file());
    assert_eq!(
        persisted.relative_path,
        "projects/7/images/chapter_3/scene_panel-1_publication-test.png"
    );
    assert_eq!(persisted.file_size, 15);
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn filename_segment_rejects_empty_or_path_like_values() {
    let empty =
        required_filename_segment("  ", "kind").expect_err("empty filename segment should fail");
    assert!(empty.contains("不能为空"));

    let slash = required_filename_segment("../scene", "kind")
        .expect_err("path-like filename segment should fail");
    assert!(slash.contains("只能包含"));
}

#[test]
fn filename_segment_accepts_safe_ascii_values() {
    let segment = required_filename_segment("manga_page-01", "kind")
        .expect("safe filename segment should pass");

    assert_eq!(segment, "manga_page-01");
}
