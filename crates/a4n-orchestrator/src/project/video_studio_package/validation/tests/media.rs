use super::{
    CanvasNodeRefKind, FILE_PATH_FIELD, THUMBNAIL_PATH_FIELD, VideoStudioMediaFileEntry,
    empty_payload, media_asset_with_file, media_file_key, package, validate_media_entry_bytes,
    validate_package_manifest, video_with_file,
};

#[test]
fn validate_video_studio_manifest_rejects_missing_generated_video_file_payload() {
    let mut package = package();
    package.generated_videos.push(video_with_file());

    let err = validate_package_manifest(&package, &empty_payload())
        .expect_err("generated video file path must require package payload");

    assert!(err.contains("owner_kind=generated_video"));
    assert!(err.contains("field=file_path"));
}

#[test]
fn validate_video_studio_manifest_rejects_non_terminal_generated_video() {
    let mut package = package();
    let mut video = video_with_file();
    video.file_path = None;
    video.thumbnail_path = None;
    video.status = "running".to_string();
    package.generated_videos.push(video);

    let err = validate_package_manifest(&package, &empty_payload())
        .expect_err("active provider videos must not be transferred");

    assert!(err.contains("只能传输终态生成视频"));
}

#[test]
fn validate_video_studio_manifest_rejects_generated_video_runtime_identity() {
    let mut package = package();
    let mut video = video_with_file();
    video.file_path = None;
    video.thumbnail_path = None;
    video.metadata_json = Some(serde_json::json!({"workflow_run_id":40}));
    package.generated_videos.push(video);

    let err = validate_package_manifest(&package, &empty_payload())
        .expect_err("workflow identity must not cross project transfer");

    assert!(err.contains("workflow_run_id"));
}

#[test]
fn validate_video_studio_manifest_rejects_missing_generated_video_thumbnail_payload() {
    let mut package = package();
    let mut video = video_with_file();
    video.file_path = None;
    package.generated_videos.push(video);

    let err = validate_package_manifest(&package, &empty_payload())
        .expect_err("generated video thumbnail path must require package payload");

    assert!(err.contains("owner_kind=generated_video"));
    assert!(err.contains("field=thumbnail_path"));
}

#[test]
fn validate_video_studio_manifest_accepts_generated_video_media_payloads() {
    let mut package = package();
    let video = video_with_file();
    package.generated_videos.push(video.clone());
    let mut payload = empty_payload();
    for (field, path) in [
        (FILE_PATH_FIELD, video.file_path.clone().unwrap()),
        (THUMBNAIL_PATH_FIELD, video.thumbnail_path.clone().unwrap()),
    ] {
        let entry = VideoStudioMediaFileEntry {
            owner_kind: CanvasNodeRefKind::GeneratedVideo.as_str().to_string(),
            owner_id: video.id,
            field: field.to_string(),
            original_path: path,
            package_path: format!("media/generated_video/{}/{}.bin", video.id, field),
        };
        payload.entries.insert(
            media_file_key(&entry.owner_kind, entry.owner_id, &entry.field),
            entry,
        );
    }

    validate_package_manifest(&package, &payload)
        .expect("generated video media payloads should satisfy manifest validation");
}

#[test]
fn validate_video_studio_manifest_rejects_missing_video_source_ref() {
    let mut package = package();
    let mut video = video_with_file();
    video.file_path = None;
    video.thumbnail_path = None;
    video.source_video_id = Some(999);
    package.generated_videos.push(video);

    let err = validate_package_manifest(&package, &empty_payload())
        .expect_err("missing source video ref must fail");

    assert!(err.contains("source_video_id=999"));
}

#[test]
fn validate_video_studio_manifest_rejects_negative_media_file_size() {
    let mut package = package();
    let mut video = video_with_file();
    video.file_path = None;
    video.thumbnail_path = None;
    video.file_size = Some(-1);
    package.generated_videos.push(video);

    let err = validate_package_manifest(&package, &empty_payload())
        .expect_err("negative media file size must fail");

    assert!(err.contains("file_size 不能为负数"));
    assert!(err.contains("owner_kind=generated_video"));
}

#[test]
fn validate_media_entry_bytes_rejects_generated_video_file_size_mismatch() {
    let mut package = package();
    let video = video_with_file();
    package.generated_videos.push(video.clone());
    let entry = VideoStudioMediaFileEntry {
        owner_kind: CanvasNodeRefKind::GeneratedVideo.as_str().to_string(),
        owner_id: video.id,
        field: FILE_PATH_FIELD.to_string(),
        original_path: video.file_path.unwrap(),
        package_path: "media/generated_video/10/file.mp4".to_string(),
    };

    let err = validate_media_entry_bytes(&package, &entry, b"short")
        .expect_err("payload bytes must match generated video file_size");

    assert!(err.contains("媒体文件大小与清单不一致"));
    assert!(err.contains("owner_kind=generated_video"));
}

#[test]
fn validate_media_entry_bytes_rejects_media_asset_checksum_mismatch() {
    let mut package = package();
    let asset = media_asset_with_file(b"expected bytes");
    package.media_assets.push(asset.clone());
    let entry = VideoStudioMediaFileEntry {
        owner_kind: CanvasNodeRefKind::MediaAsset.as_str().to_string(),
        owner_id: asset.id,
        field: FILE_PATH_FIELD.to_string(),
        original_path: asset.file_path.unwrap(),
        package_path: "media/media_asset/60/file.mp4".to_string(),
    };

    let err = validate_media_entry_bytes(&package, &entry, b"different data")
        .expect_err("payload bytes must match media asset checksum");

    assert!(err.contains("checksum 与清单不一致"));
    assert!(err.contains("owner_kind=media_asset"));
}
