#[test]
fn import_video_source_path_accepts_supported_video_file() {
    let data_dir = unique_temp_dir("a4n_video_import_source");
    let video_path = data_dir.join("external.MP4");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    std::fs::write(&video_path, b"video").expect("write video");

    let resolved =
        import_video_source_path(&video_path.to_string_lossy()).expect("mp4 should import");

    assert_eq!(
        resolved,
        video_path.canonicalize().expect("canonical import source")
    );
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn import_video_source_path_rejects_unsupported_extension() {
    let data_dir = unique_temp_dir("a4n_video_import_bad_ext");
    let video_path = data_dir.join("external.avi");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    std::fs::write(&video_path, b"video").expect("write video");

    let err = import_video_source_path(&video_path.to_string_lossy())
        .expect_err("unsupported extension should fail");

    assert!(err.contains("导入视频格式不支持"));
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn staged_imported_video_source_is_owned_by_temp_directory_until_drop() {
    let root = unique_temp_dir("a4n_staged_video_source");
    let temp_dir = root.join("temp");
    let source_path = root.join("external.mp4");
    std::fs::create_dir_all(&temp_dir).expect("create temp dir");
    std::fs::write(&source_path, b"video-bytes").expect("write source video");
    let source = super::imports::import_video_source(
        &source_path.to_string_lossy(),
        "导入视频",
    )
    .expect("load external video source");

    let staged = super::imports::stage_imported_video_source(&temp_dir, source)
        .expect("stage external video source");
    let staged_path = staged.path.clone();
    assert!(staged_path.starts_with(&temp_dir));
    assert_eq!(std::fs::read(&staged_path).expect("read staged video"), b"video-bytes");
    assert!(source_path.is_file());

    drop(staged);
    assert!(!staged_path.exists());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn uploaded_video_source_keeps_temp_file_until_drop_and_writes_all_bytes() {
    let temp_dir = unique_temp_dir("a4n_uploaded_video_source");

    let source = super::imports::persist_uploaded_video_source(
        &temp_dir,
        "source.mp4".to_string(),
        "mp4",
        b"video-bytes",
        "上传视频",
    )
    .expect("persist uploaded video source");
    let temp_path = source.path.clone();

    assert!(temp_path.is_file());
    assert_eq!(source.file_name, "source.mp4");
    assert_eq!(
        std::fs::read(&temp_path).expect("read temp video"),
        b"video-bytes"
    );
    assert_eq!(
        std::fs::metadata(&temp_path)
            .expect("read temp video metadata")
            .len(),
        11
    );

    drop(source);
    assert!(!temp_path.exists());
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn uploaded_video_recovery_only_selects_owned_regular_files_before_cutoff() {
    let temp_dir = unique_temp_dir("a4n_uploaded_video_recovery");
    let upload_dir = temp_dir.join("video_uploads");
    std::fs::create_dir_all(&upload_dir).expect("create upload dir");
    let owned = upload_dir.join("video_upload_1_2.mp4");
    let unrelated = upload_dir.join("keep.mp4");
    let directory = upload_dir.join("video_upload_directory.mp4");
    std::fs::write(&owned, b"owned").expect("write owned temp file");
    std::fs::write(&unrelated, b"unrelated").expect("write unrelated file");
    std::fs::create_dir(&directory).expect("create similarly named directory");

    let expired = super::imports::expired_uploaded_video_sources(
        &temp_dir,
        std::time::SystemTime::now() + std::time::Duration::from_secs(1),
    )
    .expect("scan uploaded video temp files");

    assert_eq!(expired, vec![owned]);
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[cfg(unix)]
#[test]
fn uploaded_video_source_removes_owned_temp_file_when_source_loading_fails() {
    use std::os::unix::ffi::OsStringExt;

    let mut invalid_component = format!(
        "a4n_uploaded_video_invalid_utf8_{}_{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    )
    .into_bytes();
    invalid_component.push(0xff);
    let temp_dir = std::env::temp_dir().join(std::ffi::OsString::from_vec(invalid_component));

    let error = match super::imports::persist_uploaded_video_source(
        &temp_dir,
        "source.mp4".to_string(),
        "mp4",
        b"video-bytes",
        "上传视频",
    ) {
        Ok(source) => {
            drop(source);
            panic!("invalid UTF-8 temp path must fail source loading");
        }
        Err(error) => error,
    };

    assert_eq!(error, "视频上传临时路径不是有效 UTF-8");
    let upload_dir = temp_dir.join("video_uploads");
    assert_eq!(
        std::fs::read_dir(&upload_dir)
            .expect("read upload temp dir")
            .count(),
        0,
        "failed source loading must remove the owned temp file"
    );
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn copy_imported_video_file_keeps_successful_output_and_returns_relative_path() {
    let root = unique_temp_dir("a4n_video_import_copy_success");
    let data_dir = root.join("data");
    let source = root.join("source.mp4");
    let output = data_dir.join("projects/1/videos/imported.mp4");
    std::fs::create_dir_all(output.parent().expect("output parent")).expect("create output dir");
    std::fs::write(&source, b"video").expect("write source video");

    let relative_path = super::imports::copy_imported_video_file(
        &data_dir,
        &source,
        &output,
        "复制导入视频失败",
        "导入视频",
        "导入视频文件",
    )
    .expect("imported video copy should succeed");

    assert_eq!(relative_path, "projects/1/videos/imported.mp4");
    assert_eq!(std::fs::read(&output).expect("read output"), b"video");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn copy_imported_video_file_removes_output_on_relative_path_error() {
    let root = unique_temp_dir("a4n_video_import_copy_relative_error");
    let data_dir = root.join("data");
    let source = root.join("source.mp4");
    let output = root.join("outside/imported.mp4");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    std::fs::create_dir_all(output.parent().expect("output parent")).expect("create output dir");
    std::fs::write(&source, b"video").expect("write source video");

    let error = super::imports::copy_imported_video_file(
        &data_dir,
        &source,
        &output,
        "复制导入视频失败",
        "导入视频",
        "导入视频文件",
    )
    .expect_err("outside output must fail relative path normalization");

    assert!(error.contains("导入视频不在应用数据目录内"));
    assert!(!output.exists());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn copy_imported_video_file_reports_copy_error_without_leaving_output() {
    let root = unique_temp_dir("a4n_video_import_copy_failure");
    let data_dir = root.join("data");
    let source = root.join("missing.mp4");
    let output = data_dir.join("projects/1/videos/imported.mp4");
    std::fs::create_dir_all(output.parent().expect("output parent")).expect("create output dir");

    let error = super::imports::copy_imported_video_file(
        &data_dir,
        &source,
        &output,
        "复制导入视频失败",
        "导入视频",
        "导入视频文件",
    )
    .expect_err("missing source must fail copy");

    assert!(error.starts_with("复制导入视频失败："));
    assert!(!output.exists());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn persist_trimmed_video_output_removes_partial_file_on_trim_error() {
    let root = unique_temp_dir("a4n_trim_output_partial_error");
    let data_dir = root.join("data");
    let output = data_dir.join("projects/1/videos/trimmed.mp4");
    std::fs::create_dir_all(output.parent().expect("output parent")).expect("create output dir");

    let error = super::imports::persist_trimmed_video_output(&data_dir, &output, || {
        std::fs::write(&output, b"partial").expect("write partial trim output");
        Err("ffmpeg trim failed".to_string())
    })
    .expect_err("trim failure must remove partial output");

    assert_eq!(error, "ffmpeg trim failed");
    assert!(!output.exists());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn persist_trimmed_video_output_removes_file_on_relative_path_error() {
    let root = unique_temp_dir("a4n_trim_output_relative_error");
    let data_dir = root.join("data");
    let output = root.join("outside/trimmed.mp4");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    std::fs::create_dir_all(output.parent().expect("output parent")).expect("create output dir");

    let error = super::imports::persist_trimmed_video_output(&data_dir, &output, || {
        std::fs::write(&output, b"trimmed").expect("write trim output");
        Ok(())
    })
    .expect_err("outside trim output must fail relative path normalization");

    assert!(error.contains("裁剪视频不在应用数据目录内"));
    assert!(!output.exists());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn persist_trimmed_video_output_returns_size_and_keeps_successful_file() {
    let root = unique_temp_dir("a4n_trim_output_success");
    let data_dir = root.join("data");
    let output = data_dir.join("projects/1/videos/trimmed.mp4");
    std::fs::create_dir_all(output.parent().expect("output parent")).expect("create output dir");

    let (size, relative_path) =
        super::imports::persist_trimmed_video_output(&data_dir, &output, || {
            std::fs::write(&output, b"trimmed").expect("write trim output");
            Ok(())
        })
        .expect("successful trim output should be retained");

    assert_eq!(size, 7);
    assert_eq!(relative_path, "projects/1/videos/trimmed.mp4");
    assert!(output.exists());
    let _ = std::fs::remove_dir_all(&root);
}
