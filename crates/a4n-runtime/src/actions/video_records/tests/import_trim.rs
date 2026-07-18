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
fn import_video_extension_normalizes_supported_extensions() {
    for (name, expected) in [
        ("clip.mp4", "mp4"),
        ("clip.WEBM", "webm"),
        ("clip.mov", "mov"),
        ("clip.M4V", "m4v"),
    ] {
        assert_eq!(
            import_video_extension(PathBuf::from(name).as_path()).expect("supported video"),
            expected
        );
    }
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
fn reference_media_import_options_reject_invalid_duration_and_aspect_ratio() {
    for duration_secs in [0, -1, 61] {
        let err = validate_reference_media_import_options(duration_secs, "16:9")
            .expect_err("reference media import duration must be bounded");

        assert_eq!(err, "参考媒体视频时长必须在 1 到 60 秒之间");
    }

    let err = validate_reference_media_import_options(6, "2:1")
        .expect_err("reference media import aspect ratio must be explicit");

    assert!(err.contains("不支持画幅 2:1"));
}

#[test]
fn reference_media_import_options_normalize_declared_aspect_ratio() {
    let options = validate_reference_media_import_options(6, " 3:4 ")
        .expect("valid reference media import options");

    assert_eq!(options.aspect_ratio, "3:4");
}

#[tokio::test]
async fn delete_video_record_then_files_removes_files_after_record_cleanup() {
    let data_dir = unique_temp_dir("a4n_video_delete_order_success");
    let video_path = data_dir.join("videos/chapter_1/video.mp4");
    std::fs::create_dir_all(video_path.parent().expect("video parent")).expect("create parent");
    std::fs::write(&video_path, b"video").expect("write video");
    let cleanup_called = Arc::new(AtomicBool::new(false));

    delete_video_record_then_files(vec![("视频文件", video_path.clone())], {
        let cleanup_called = cleanup_called.clone();
        let video_path = video_path.clone();
        || async move {
            assert!(video_path.exists());
            cleanup_called.store(true, Ordering::SeqCst);
            Ok(())
        }
    })
    .await
    .expect("file cleanup should run after record deletion succeeds");

    assert!(!video_path.exists());
    assert!(cleanup_called.load(Ordering::SeqCst));
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[tokio::test]
async fn delete_video_record_then_files_keeps_files_when_record_cleanup_fails() {
    let data_dir = unique_temp_dir("a4n_video_delete_record_failure");
    let video_path = data_dir.join("videos/chapter_1/video.mp4");
    std::fs::create_dir_all(video_path.parent().expect("video parent")).expect("create parent");
    std::fs::write(&video_path, b"video").expect("write video");

    let err = delete_video_record_then_files(vec![("视频文件", video_path.clone())], {
        || async move { Err("记录删除失败".to_string()) }
    })
    .await
    .expect_err("file cleanup must not run after record deletion fails");

    assert_eq!(err, "记录删除失败");
    assert!(video_path.exists());
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[tokio::test]
async fn delete_video_record_then_files_reports_file_delete_error_after_record_cleanup() {
    let data_dir = unique_temp_dir("a4n_video_delete_order_failure");
    let bad_target = data_dir.join("videos/chapter_1/not_a_file.mp4");
    std::fs::create_dir_all(&bad_target).expect("create directory target");
    let cleanup_called = Arc::new(AtomicBool::new(false));

    let err = delete_video_record_then_files(vec![("视频文件", bad_target.clone())], {
        let cleanup_called = cleanup_called.clone();
        || async move {
            cleanup_called.store(true, Ordering::SeqCst);
            Ok(())
        }
    })
    .await
    .expect_err("file cleanup error should be returned after record cleanup");

    assert!(err.contains("视频文件删除失败"));
    assert!(bad_target.exists());
    assert!(cleanup_called.load(Ordering::SeqCst));
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn validate_video_trim_range_rejects_non_finite_values() {
    for (start_secs, end_secs) in [
        (f32::NAN, 2.0),
        (0.0, f32::NAN),
        (f32::INFINITY, f32::INFINITY),
        (0.0, f32::INFINITY),
    ] {
        let err = validate_video_trim_range(start_secs, end_secs)
            .expect_err("non-finite trim range should be rejected");

        assert_eq!(err, "裁剪时间范围无效");
    }
}

#[test]
fn validate_video_trim_range_accepts_finite_forward_range() {
    validate_video_trim_range(0.25, 1.5).expect("finite forward range should be valid");
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

#[test]
fn trimmed_video_metadata_uses_source_video_column_as_source_id_fact() {
    let source_without_context = video_with_metadata(7, Some(json!({ "prompt_id": 3 })));
    let metadata = trimmed_video_metadata(&source_without_context, 1.25, 4.5);

    assert_eq!(
        metadata
            .get("trim")
            .and_then(|value| value.get("start_secs")),
        Some(&json!(1.25))
    );
    assert_eq!(
        metadata.get("trim").and_then(|value| value.get("end_secs")),
        Some(&json!(4.5))
    );
    assert!(
        metadata.get("source_video_id").is_none(),
        "source_video_id is recorded by the generated_video column, not duplicated in metadata"
    );
    assert!(
        metadata.get("source").is_none(),
        "missing source context should not be recorded as source=null"
    );

    let source_context = json!({ "content_sha256": "same-source" });
    let source_with_context =
        video_with_metadata(8, Some(json!({ "source": source_context.clone() })));
    let metadata = trimmed_video_metadata(&source_with_context, 0.0, 2.0);

    assert_eq!(metadata.get("source"), Some(&source_context));
    assert!(metadata.get("source_video_id").is_none());
}

#[test]
fn imported_chapter_video_metadata_omits_missing_source_context() {
    let prompt_without_context = prompt_with_metadata(9, Some(json!({ "draft": true })));
    let metadata = imported_chapter_video_metadata(
        &prompt_without_context,
        "external.mp4",
        "mp4",
        "2026-06-29T19:19:00Z".to_string(),
    );

    assert_eq!(metadata.get("prompt_id"), Some(&json!(9)));
    assert_eq!(metadata.get("shot_number"), Some(&json!(2)));
    assert_eq!(
        metadata.get("source_prompt_type"),
        Some(&json!("video_prompt"))
    );
    assert!(
        metadata.get("source").is_none(),
        "missing prompt source context should not be recorded as source=null"
    );
    assert_eq!(
        metadata
            .get("import")
            .and_then(|value| value.get("source_file_name")),
        Some(&json!("external.mp4"))
    );

    let source_context = json!({ "content_sha256": "same-source" });
    let prompt_with_context =
        prompt_with_metadata(10, Some(json!({ "source": source_context.clone() })));
    let metadata = imported_chapter_video_metadata(
        &prompt_with_context,
        "external.webm",
        "webm",
        "2026-06-29T19:20:00Z".to_string(),
    );

    assert_eq!(metadata.get("source"), Some(&source_context));
    assert_eq!(
        metadata
            .get("import")
            .and_then(|value| value.get("source_extension")),
        Some(&json!("webm"))
    );
}
