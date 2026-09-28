#[test]
fn video_probe_error_mapping_preserves_invalid_json_label() {
    let error = a4n_media_processing_core::parse_video_probe_properties(b"{not-json")
        .expect_err("invalid ffprobe JSON should fail");

    let message = video_probe_error_message("导入参考媒体视频", error);

    assert!(message.contains("导入参考媒体视频元数据不是有效 JSON"));
}

#[test]
fn video_probe_error_mapping_preserves_missing_stream_message() {
    let error = a4n_media_processing_core::parse_video_probe_properties(
        br#"{
            "streams": [],
            "format": {
                "duration": "8.0"
            }
        }"#,
    )
    .expect_err("missing video stream should fail");

    assert_eq!(
        video_probe_error_message("导入视频", error),
        "导入视频缺少视频流"
    );
}

#[test]
fn video_probe_error_mapping_preserves_invalid_duration_message() {
    let error = a4n_media_processing_core::parse_video_probe_properties(
        br#"{
            "streams": [
                {
                    "width": 1280,
                    "height": 720,
                    "r_frame_rate": "24"
                }
            ],
            "format": {
                "duration": "0"
            }
        }"#,
    )
    .expect_err("missing positive duration should fail");

    assert_eq!(
        video_probe_error_message("插入视频", error),
        "插入视频缺少有效时长"
    );
}

#[test]
fn assembly_cache_matches_same_manifest_and_output() {
    let data_dir = unique_temp_dir("a4n_assembly_cache_hit");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let clip_path = data_dir.join("clip_1.mp4");
    let output_path = data_dir.join("assembled.mp4");
    let manifest_path = data_dir.join("assembled.mp4.cache.json");
    std::fs::write(&clip_path, b"clip").expect("write clip");
    std::fs::write(&output_path, b"assembled").expect("write output");
    let mut clip = assembly_clip(1, 1280, 720, 24);
    clip.path = clip_path;
    clip.file_path = "videos/chapter_1/clip_1.mp4".to_string();
    let manifest = build_test_assembly_cache_manifest(&[clip], &[]);
    write_assembly_cache_manifest(&manifest_path, &manifest).expect("write manifest");

    assert!(assembly_cache_matches(&output_path, &manifest_path, &manifest).expect("match cache"));
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn assembly_cache_misses_after_clip_file_changes() {
    let data_dir = unique_temp_dir("a4n_assembly_cache_file_change");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let clip_path = data_dir.join("clip_1.mp4");
    let output_path = data_dir.join("assembled.mp4");
    let manifest_path = data_dir.join("assembled.mp4.cache.json");
    std::fs::write(&clip_path, b"clip").expect("write clip");
    std::fs::write(&output_path, b"assembled").expect("write output");
    let mut clip = assembly_clip(1, 1280, 720, 24);
    clip.path = clip_path.clone();
    clip.file_path = "videos/chapter_1/clip_1.mp4".to_string();
    let original_manifest = build_test_assembly_cache_manifest(&[clip.clone()], &[]);
    write_assembly_cache_manifest(&manifest_path, &original_manifest).expect("write manifest");

    std::fs::write(&clip_path, b"clip changed").expect("change clip");
    let changed_manifest = build_test_assembly_cache_manifest(&[clip], &[]);

    assert!(
        !assembly_cache_matches(&output_path, &manifest_path, &changed_manifest)
            .expect("changed clip should miss")
    );
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn assembly_cache_misses_after_clip_order_changes() {
    let data_dir = unique_temp_dir("a4n_assembly_cache_order_change");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let first_path = data_dir.join("clip_1.mp4");
    let second_path = data_dir.join("clip_2.mp4");
    let output_path = data_dir.join("assembled.mp4");
    let manifest_path = data_dir.join("assembled.mp4.cache.json");
    std::fs::write(&first_path, b"clip one").expect("write first clip");
    std::fs::write(&second_path, b"clip two").expect("write second clip");
    std::fs::write(&output_path, b"assembled").expect("write output");
    let mut first = assembly_clip(1, 1280, 720, 24);
    first.path = first_path;
    first.file_path = "videos/chapter_1/clip_1.mp4".to_string();
    let mut second = assembly_clip(2, 1280, 720, 24);
    second.path = second_path;
    second.file_path = "videos/chapter_1/clip_2.mp4".to_string();
    let original_manifest =
        build_test_assembly_cache_manifest(&[first.clone(), second.clone()], &[]);
    write_assembly_cache_manifest(&manifest_path, &original_manifest).expect("write manifest");
    let reordered_manifest = build_test_assembly_cache_manifest(&[second, first], &[]);

    assert!(
        !assembly_cache_matches(&output_path, &manifest_path, &reordered_manifest)
            .expect("reordered clips should miss")
    );
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn assembly_cache_rejects_invalid_manifest_shape() {
    let data_dir = unique_temp_dir("a4n_assembly_cache_invalid_manifest");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let clip_path = data_dir.join("clip_1.mp4");
    let output_path = data_dir.join("assembled.mp4");
    let manifest_path = data_dir.join("assembled.mp4.cache.json");
    std::fs::write(&clip_path, b"clip").expect("write clip");
    std::fs::write(&output_path, b"assembled").expect("write output");
    std::fs::write(&manifest_path, br#"{"version":1}"#).expect("write invalid manifest");
    let mut clip = assembly_clip(1, 1280, 720, 24);
    clip.path = clip_path;
    clip.file_path = "videos/chapter_1/clip_1.mp4".to_string();
    let manifest = build_test_assembly_cache_manifest(&[clip], &[]);

    let err = assembly_cache_matches(&output_path, &manifest_path, &manifest)
        .expect_err("invalid manifest shape should be explicit");
    assert!(err.contains("格式无效"));
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn assembly_preview_cache_failure_removes_output_and_manifest() {
    let data_dir = unique_temp_dir("a4n_assembly_cache_failure_cleanup");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let output_path = data_dir.join("assembled.mp4");
    let manifest_path = data_dir.join("assembled.mp4.cache.json");

    let error =
        super::assembly::persist_assembly_preview_cache(&output_path, &manifest_path, || {
            std::fs::write(&output_path, b"partial output").expect("write output");
            std::fs::write(&manifest_path, b"partial manifest").expect("write manifest");
            Err::<(), _>("preview failed".to_string())
        })
        .expect_err("failed preview should invalidate cache pair");

    assert_eq!(error, "preview failed");
    assert!(!output_path.exists());
    assert!(!manifest_path.exists());
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn assembly_preview_cache_success_preserves_output_and_manifest() {
    let data_dir = unique_temp_dir("a4n_assembly_cache_success");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let output_path = data_dir.join("assembled.mp4");
    let manifest_path = data_dir.join("assembled.mp4.cache.json");

    let value =
        super::assembly::persist_assembly_preview_cache(&output_path, &manifest_path, || {
            std::fs::write(&output_path, b"output").expect("write output");
            std::fs::write(&manifest_path, b"manifest").expect("write manifest");
            Ok(42)
        })
        .expect("successful cache operation");

    assert_eq!(value, 42);
    assert!(output_path.is_file());
    assert!(manifest_path.is_file());
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn assembly_preview_cache_cleanup_error_keeps_primary_failure() {
    let data_dir = unique_temp_dir("a4n_assembly_cache_cleanup_error");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let output_path = data_dir.join("assembled.mp4");
    let manifest_path = data_dir.join("assembled.mp4.cache.json");
    std::fs::create_dir_all(&output_path).expect("create output directory");
    std::fs::create_dir_all(&manifest_path).expect("create manifest directory");

    let error =
        super::assembly::persist_assembly_preview_cache(&output_path, &manifest_path, || {
            Err::<(), _>("preview failed".to_string())
        })
        .expect_err("cleanup failure should be explicit");

    assert!(error.starts_with("preview failed；"));
    assert!(error.contains("成片预览文件删除失败"));
    assert!(error.contains("成片预览缓存清单删除失败"));
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn assembly_cache_manifest_replaces_existing_file_without_tmp_residue() {
    let data_dir = unique_temp_dir("a4n_assembly_manifest_replace");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let clip_path = data_dir.join("clip_1.mp4");
    let manifest_path = data_dir.join("assembled.mp4.cache.json");
    let tmp_path = manifest_path.with_extension("tmp");
    std::fs::write(&clip_path, b"clip").expect("write clip");
    std::fs::write(&manifest_path, b"old manifest").expect("write old manifest");
    let mut clip = assembly_clip(1, 1280, 720, 24);
    clip.path = clip_path;
    let manifest = build_test_assembly_cache_manifest(&[clip], &[]);

    write_assembly_cache_manifest(&manifest_path, &manifest).expect("replace manifest");

    let persisted: AssemblyCacheManifest =
        serde_json::from_slice(&std::fs::read(&manifest_path).expect("read replaced manifest"))
            .expect("parse replaced manifest");
    assert_eq!(persisted, manifest);
    assert!(!tmp_path.exists());
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn assembly_cache_manifest_target_error_removes_tmp_file() {
    let data_dir = unique_temp_dir("a4n_assembly_manifest_target_error");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let clip_path = data_dir.join("clip_1.mp4");
    let manifest_path = data_dir.join("assembled.mp4.cache.json");
    let tmp_path = manifest_path.with_extension("tmp");
    std::fs::write(&clip_path, b"clip").expect("write clip");
    std::fs::create_dir_all(&manifest_path).expect("create invalid manifest target");
    let mut clip = assembly_clip(1, 1280, 720, 24);
    clip.path = clip_path;
    let manifest = build_test_assembly_cache_manifest(&[clip], &[]);

    let error = write_assembly_cache_manifest(&manifest_path, &manifest)
        .expect_err("directory target should fail explicitly");

    assert!(error.contains("旧成片预览缓存清单删除失败"));
    assert!(!tmp_path.exists());
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn assembly_cache_manifest_is_fresh_after_write() {
    let data_dir = unique_temp_dir("a4n_assembly_cache_fresh");
    std::fs::create_dir_all(&data_dir).expect("create cache root");
    let manifest_path = data_dir.join("assembled.mp4.cache.json");
    std::fs::write(&manifest_path, b"{}").expect("write manifest");

    assert!(assembly_cache_is_fresh(&manifest_path).expect("check cache freshness"));

    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn expired_assembly_cache_scan_returns_manifest_and_video() {
    let data_dir = unique_temp_dir("a4n_assembly_cache_expiry_scan");
    let chapter_dir = data_dir.join("projects/7/videos/chapter_3");
    std::fs::create_dir_all(&chapter_dir).expect("create chapter video dir");
    let output = chapter_dir.join("assembled_storyboard.mp4");
    let manifest = chapter_dir.join("assembled_storyboard.mp4.cache.json");
    std::fs::write(&output, b"video").expect("write output");
    std::fs::write(&manifest, b"{}").expect("write manifest");

    let caches = expired_assembly_cache_files(
        &data_dir,
        SystemTime::now() + std::time::Duration::from_secs(1),
    )
    .expect("scan expired caches");

    assert_eq!(caches.len(), 1);
    assert_eq!(caches[0].project_id, 7);
    assert_eq!(
        caches[0].paths,
        [
            "projects/7/videos/chapter_3/assembled_storyboard.mp4.cache.json",
            "projects/7/videos/chapter_3/assembled_storyboard.mp4",
        ]
    );

    let _ = std::fs::remove_dir_all(&data_dir);
}
