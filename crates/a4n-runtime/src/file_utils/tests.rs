use super::{
    CanonicalDataPathError, canonicalize_data_file_path, canonicalize_path_in_data_dir,
    deletable_data_dir_path, error_with_rollback_file_cleanup, persist_created_media_file,
    remove_created_file_on_error, remove_created_file_on_error_sync, remove_data_dir_in_data_dir,
    remove_data_file_in_data_dir, write_new_data_file,
};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .expect("system clock must be after UNIX_EPOCH");
    std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), stamp))
}

#[tokio::test]
async fn created_file_is_kept_when_followup_operation_succeeds() {
    let data_dir = unique_temp_dir("a4n_file_utils_created_commit");
    let file_path = data_dir.join("image.png");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    std::fs::write(&file_path, b"image").expect("write created file");

    let result = remove_created_file_on_error(&file_path, "已写出的图片文件", async {
        Ok::<_, String>(17)
    })
    .await
    .expect("successful followup should keep file");

    assert_eq!(result, 17);
    assert!(file_path.is_file());
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[tokio::test]
async fn created_file_is_removed_when_followup_operation_fails() {
    let data_dir = unique_temp_dir("a4n_file_utils_created_rollback");
    let file_path = data_dir.join("image.png");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    std::fs::write(&file_path, b"image").expect("write created file");

    let error = remove_created_file_on_error(&file_path, "已写出的图片文件", async {
        Err::<(), _>("图片记录写入失败".to_string())
    })
    .await
    .expect_err("failed followup should remove created file");

    assert_eq!(error, "图片记录写入失败");
    assert!(!file_path.exists());
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[tokio::test]
async fn created_file_cleanup_failure_is_appended_to_primary_error() {
    let data_dir = unique_temp_dir("a4n_file_utils_created_cleanup_error");
    let directory_at_file_path = data_dir.join("image.png");
    std::fs::create_dir_all(&directory_at_file_path).expect("create cleanup failure fixture");

    let error = remove_created_file_on_error(&directory_at_file_path, "已写出的图片文件", async {
        Err::<(), _>("图片记录写入失败".to_string())
    })
    .await
    .expect_err("cleanup failure should stay visible");

    assert!(error.starts_with("图片记录写入失败；同时删除已写出的图片文件失败："));
    assert!(directory_at_file_path.is_dir());
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn sync_created_file_is_kept_when_followup_operation_succeeds() {
    let data_dir = unique_temp_dir("a4n_file_utils_sync_created_commit");
    let file_path = data_dir.join("thumbnail.jpg");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    std::fs::write(&file_path, b"thumbnail").expect("write created file");

    let result = remove_created_file_on_error_sync(&file_path, "已写出的缩略图", || {
        Ok::<_, String>(23)
    })
    .expect("successful followup should keep file");

    assert_eq!(result, 23);
    assert!(file_path.is_file());
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn sync_created_file_is_removed_when_followup_operation_fails() {
    let data_dir = unique_temp_dir("a4n_file_utils_sync_created_rollback");
    let file_path = data_dir.join("thumbnail.jpg");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    std::fs::write(&file_path, b"thumbnail").expect("write created file");

    let error = remove_created_file_on_error_sync(&file_path, "已写出的缩略图", || {
        Err::<(), _>("缩略图路径规范化失败".to_string())
    })
    .expect_err("failed followup should remove created file");

    assert_eq!(error, "缩略图路径规范化失败");
    assert!(!file_path.exists());
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn sync_created_file_cleanup_failure_is_appended_to_primary_error() {
    let data_dir = unique_temp_dir("a4n_file_utils_sync_created_cleanup_error");
    let directory_at_file_path = data_dir.join("thumbnail.jpg");
    std::fs::create_dir_all(&directory_at_file_path).expect("create cleanup failure fixture");

    let error =
        remove_created_file_on_error_sync(&directory_at_file_path, "已写出的缩略图", || {
            Err::<(), _>("缩略图路径规范化失败".to_string())
        })
        .expect_err("cleanup failure should stay visible");

    assert!(error.starts_with("缩略图路径规范化失败；同时删除已写出的缩略图失败："));
    assert!(directory_at_file_path.is_dir());
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn rollback_file_cleanup_attempts_all_paths_and_keeps_cleanup_failures() {
    let data_dir = unique_temp_dir("a4n_file_utils_multi_cleanup");
    let removable_file = data_dir.join("created.bin");
    let directory_at_file_path = data_dir.join("not-a-file.bin");
    std::fs::create_dir_all(&directory_at_file_path).expect("create cleanup failure fixture");
    std::fs::write(&removable_file, b"created").expect("write removable file");

    let error = error_with_rollback_file_cleanup(
        "数据库写入失败".to_string(),
        &[directory_at_file_path.clone(), removable_file.clone()],
        "已创建文件",
    );

    assert!(!removable_file.exists(), "其他路径仍必须继续清理");
    assert!(error.starts_with("数据库写入失败；删除已创建文件失败："));
    assert!(error.contains(&directory_at_file_path.display().to_string()));
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn created_media_file_removes_partial_output_on_operation_error() {
    let root = unique_temp_dir("a4n_created_media_partial_error");
    let data_dir = root.join("data");
    let output = data_dir.join("projects/1/media/output.bin");
    std::fs::create_dir_all(output.parent().expect("output parent")).expect("create output dir");

    let error = persist_created_media_file(
        &data_dir,
        &output,
        |output| {
            std::fs::write(output, b"partial").expect("write partial output");
            Err("media write failed".to_string())
        },
        "媒体文件",
        "媒体文件",
        "媒体文件",
    )
    .expect_err("operation failure must remove partial output");

    assert_eq!(error, "media write failed");
    assert!(!output.exists());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn created_media_file_removes_output_on_relative_path_error() {
    let root = unique_temp_dir("a4n_created_media_relative_error");
    let data_dir = root.join("data");
    let output = root.join("outside/output.bin");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    std::fs::create_dir_all(output.parent().expect("output parent")).expect("create output dir");

    let error = persist_created_media_file(
        &data_dir,
        &output,
        |output| {
            std::fs::write(output, b"complete").expect("write complete output");
            Ok(())
        },
        "媒体文件",
        "媒体文件",
        "媒体文件",
    )
    .expect_err("outside output must fail relative path normalization");

    assert!(error.contains("媒体文件不在应用数据目录内"));
    assert!(!output.exists());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn created_media_file_keeps_successful_output_and_reports_size() {
    let root = unique_temp_dir("a4n_created_media_success");
    let data_dir = root.join("data");
    let output = data_dir.join("projects/1/media/output.bin");
    std::fs::create_dir_all(output.parent().expect("output parent")).expect("create output dir");

    let (relative_path, file_size) = persist_created_media_file(
        &data_dir,
        &output,
        |output| {
            std::fs::write(output, b"complete").expect("write complete output");
            Ok(())
        },
        "媒体文件",
        "媒体文件",
        "媒体文件",
    )
    .expect("successful output should be retained");

    assert_eq!(relative_path, "projects/1/media/output.bin");
    assert_eq!(file_size, 8);
    assert!(output.is_file());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn new_data_file_is_scoped_and_never_overwrites_existing_content() {
    let data_dir = unique_temp_dir("a4n_write_new_data_file");
    std::fs::create_dir_all(&data_dir).expect("create data dir");

    write_new_data_file(
        &data_dir,
        "projects/7/media/imported.bin",
        b"first",
        "导入媒体文件",
    )
    .expect("write new data file");
    let target = data_dir.join("projects/7/media/imported.bin");
    assert_eq!(std::fs::read(&target).unwrap(), b"first");

    assert!(
        write_new_data_file(
            &data_dir,
            "projects/7/media/imported.bin",
            b"second",
            "导入媒体文件",
        )
        .is_err()
    );
    assert_eq!(std::fs::read(&target).unwrap(), b"first");
    let outside_name = format!(
        "{}_outside.bin",
        data_dir.file_name().unwrap().to_string_lossy()
    );
    assert!(
        write_new_data_file(
            &data_dir,
            &format!("../{outside_name}"),
            b"outside",
            "导入媒体文件",
        )
        .is_err()
    );
    assert!(!data_dir.parent().unwrap().join(outside_name).exists());
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn canonicalize_path_in_data_dir_accepts_nested_file() {
    let data_dir = unique_temp_dir("a4n_file_utils_ok");
    let file_path = data_dir.join("projects/1/videos/chapter_1/video_1.mp4");
    std::fs::create_dir_all(file_path.parent().expect("nested parent")).expect("create dir");
    std::fs::write(&file_path, b"demo").expect("write file");

    let resolved = canonicalize_path_in_data_dir(&data_dir, &file_path, "视频文件")
        .expect("resolve nested file");
    assert_eq!(
        resolved,
        file_path.canonicalize().expect("canonicalize file")
    );

    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn canonicalize_data_file_path_accepts_data_dir_relative_file() {
    let data_dir = unique_temp_dir("a4n_file_utils_relative_ok");
    let file_path = data_dir.join("projects/1/images/ref.png");
    std::fs::create_dir_all(file_path.parent().expect("nested parent")).expect("create dir");
    std::fs::write(&file_path, b"demo").expect("write file");

    let resolved = canonicalize_data_file_path(&data_dir, "projects/1/images/ref.png", "图片文件")
        .expect("resolve relative data file");
    assert_eq!(
        resolved,
        file_path.canonicalize().expect("canonicalize file")
    );

    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn canonicalize_data_file_path_rejects_blank_path() {
    let data_dir = unique_temp_dir("a4n_file_utils_blank");
    std::fs::create_dir_all(&data_dir).expect("create data dir");

    let err = canonicalize_data_file_path(&data_dir, " \n\t ", "图片文件")
        .expect_err("blank path should fail before filesystem lookup");
    assert_eq!(err, "图片文件路径不能为空");

    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn canonicalize_data_file_path_rejects_directory() {
    let data_dir = unique_temp_dir("a4n_file_utils_directory");
    let dir_path = data_dir.join("projects/1/images/not_a_file");
    std::fs::create_dir_all(&dir_path).expect("create nested dir");

    let err = canonicalize_data_file_path(&data_dir, "projects/1/images/not_a_file", "图片文件")
        .expect_err("directory path must not be accepted as file");

    assert_eq!(err, "图片文件不是文件");

    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn canonicalize_data_file_path_rejects_absolute_path() {
    let data_dir = unique_temp_dir("a4n_file_utils_absolute_data");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let file_path = data_dir.join("projects/1/images/ref.png");

    let err = canonicalize_data_file_path(&data_dir, &file_path.to_string_lossy(), "图片文件")
        .expect_err("absolute data file path should fail");
    assert!(err.contains("相对路径"));

    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn canonicalize_path_in_data_dir_rejects_outside_file() {
    let data_dir = unique_temp_dir("a4n_file_utils_bad");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let outside = std::env::temp_dir().join(format!(
        "outside_{}_{}.mp4",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .expect("system clock must be after UNIX_EPOCH")
    ));
    std::fs::write(&outside, b"demo").expect("write outside file");

    let err = canonicalize_path_in_data_dir(&data_dir, &outside, "视频文件")
        .expect_err("reject outside file");
    assert!(matches!(
        err,
        CanonicalDataPathError::OutsideDataDirectory { ref label } if label == "视频文件"
    ));

    let _ = std::fs::remove_file(&outside);
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn canonicalize_data_file_path_rejects_relative_traversal() {
    let root = unique_temp_dir("a4n_file_utils_traversal");
    let data_dir = root.join("data");
    let outside = root.join("secret.png");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    std::fs::write(&outside, b"demo").expect("write outside file");

    let err = canonicalize_data_file_path(&data_dir, "../secret.png", "图片文件")
        .expect_err("reject traversal outside data dir");
    assert!(err.contains("不在应用数据目录内"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn remove_data_file_in_data_dir_removes_existing_file() {
    let data_dir = unique_temp_dir("a4n_file_utils_remove_ok");
    let file_path = data_dir.join("projects/1/images/ref.png");
    std::fs::create_dir_all(file_path.parent().expect("nested parent")).expect("create dir");
    std::fs::write(&file_path, b"demo").expect("write file");

    remove_data_file_in_data_dir(&data_dir, "projects/1/images/ref.png", "图片文件")
        .expect("remove existing data file");

    assert!(!file_path.exists());
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn remove_data_file_in_data_dir_skips_missing_file() {
    let data_dir = unique_temp_dir("a4n_file_utils_remove_missing");
    std::fs::create_dir_all(&data_dir).expect("create data dir");

    remove_data_file_in_data_dir(&data_dir, "projects/1/images/missing.png", "图片文件")
        .expect("missing delete target should be treated as already removed");

    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn remove_data_file_in_data_dir_rejects_missing_traversal_path() {
    let root = unique_temp_dir("a4n_file_utils_remove_traversal");
    let data_dir = root.join("data");
    std::fs::create_dir_all(&data_dir).expect("create data dir");

    let err = remove_data_file_in_data_dir(&data_dir, "../missing.png", "图片文件")
        .expect_err("missing traversal path must still be rejected");
    assert!(err.contains("不在应用数据目录内"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn remove_data_dir_in_data_dir_removes_existing_directory() {
    let data_dir = unique_temp_dir("a4n_file_utils_remove_dir_ok");
    let project_dir = data_dir.join("projects/1");
    let nested_file = project_dir.join("images/ref.png");
    std::fs::create_dir_all(nested_file.parent().expect("nested parent"))
        .expect("create nested dir");
    std::fs::write(&nested_file, b"demo").expect("write nested file");

    remove_data_dir_in_data_dir(&data_dir, &project_dir, "项目数据目录")
        .expect("remove existing data directory");

    assert!(!project_dir.exists());
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn remove_data_dir_in_data_dir_skips_missing_directory() {
    let data_dir = unique_temp_dir("a4n_file_utils_remove_dir_missing");
    std::fs::create_dir_all(&data_dir).expect("create data dir");

    remove_data_dir_in_data_dir(
        &data_dir,
        PathBuf::from("projects/1").as_path(),
        "项目数据目录",
    )
    .expect("missing directory should be treated as already removed");

    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn remove_data_dir_in_data_dir_rejects_file() {
    let data_dir = unique_temp_dir("a4n_file_utils_remove_dir_file");
    let file_path = data_dir.join("projects/1/project.toml");
    std::fs::create_dir_all(file_path.parent().expect("nested parent")).expect("create nested dir");
    std::fs::write(&file_path, b"demo").expect("write file");

    let err = remove_data_dir_in_data_dir(&data_dir, &file_path, "项目数据目录")
        .expect_err("file path must not be removed as directory");
    assert_eq!(err, "项目数据目录不是目录");

    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn remove_data_dir_in_data_dir_rejects_data_dir_itself() {
    let data_dir = unique_temp_dir("a4n_file_utils_remove_data_dir");
    std::fs::create_dir_all(&data_dir).expect("create data dir");

    let err = remove_data_dir_in_data_dir(&data_dir, &data_dir, "项目数据目录")
        .expect_err("data dir itself must not be removed");
    assert_eq!(err, "项目数据目录不能是应用数据目录");
    assert!(data_dir.exists());

    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn remove_data_dir_in_data_dir_rejects_outside_directory() {
    let root = unique_temp_dir("a4n_file_utils_remove_dir_outside");
    let data_dir = root.join("data");
    let outside_dir = root.join("outside");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    std::fs::create_dir_all(&outside_dir).expect("create outside dir");

    let err = remove_data_dir_in_data_dir(&data_dir, &outside_dir, "项目数据目录")
        .expect_err("outside directory must not be removed");
    assert!(err.contains("不在应用数据目录内"));
    assert!(outside_dir.exists());

    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(unix)]
#[test]
fn remove_data_dir_in_data_dir_rejects_symlink_directory() {
    let root = unique_temp_dir("a4n_file_utils_remove_dir_symlink");
    let data_dir = root.join("data");
    let target_dir = data_dir.join("targets/project");
    let project_link = data_dir.join("projects/1");
    std::fs::create_dir_all(&target_dir).expect("create target dir");
    std::fs::create_dir_all(project_link.parent().expect("project parent"))
        .expect("create project parent");
    std::os::unix::fs::symlink(&target_dir, &project_link).expect("create directory symlink");

    let err = remove_data_dir_in_data_dir(&data_dir, &project_link, "项目数据目录")
        .expect_err("directory symlink must not be removed as project dir");
    assert_eq!(err, "项目数据目录不能是符号链接");
    assert!(target_dir.exists());
    assert!(project_link.exists());

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn remove_data_dir_in_data_dir_rejects_missing_traversal_path() {
    let root = unique_temp_dir("a4n_file_utils_remove_dir_traversal");
    let data_dir = root.join("data");
    std::fs::create_dir_all(&data_dir).expect("create data dir");

    let err = remove_data_dir_in_data_dir(
        &data_dir,
        PathBuf::from("../missing_project").as_path(),
        "项目数据目录",
    )
    .expect_err("missing traversal path must still be rejected");
    assert!(err.contains("不在应用数据目录内"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn deletable_data_dir_path_accepts_relative_project_directory() {
    let data_dir = unique_temp_dir("a4n_file_utils_dir_relative_ok");
    let project_dir = data_dir.join("projects/1");
    std::fs::create_dir_all(&project_dir).expect("create project dir");

    let resolved = deletable_data_dir_path(
        &data_dir,
        PathBuf::from("projects/1").as_path(),
        "项目数据目录",
    )
    .expect("relative data dir should resolve")
    .expect("existing data dir should be deletable");
    assert_eq!(
        resolved,
        project_dir
            .canonicalize()
            .expect("canonicalize project dir")
    );

    let _ = std::fs::remove_dir_all(&data_dir);
}
