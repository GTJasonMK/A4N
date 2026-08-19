use std::io::{Cursor, Write};

use a4n_core::models::media::media_asset::FILE_PATH_FIELD;
use a4n_core::models::{media::media_asset, project::novel_project};
use a4n_domain::CanvasNodeRefKind;
use a4n_orchestrator::media::MEDIA_STATUS_COMPLETED;
use a4n_orchestrator::project::{
    PACKAGE_MODE, PROJECT_KIND_VIDEO, SOURCE_SYNC_CURRENT, VIDEO_STUDIO_PACKAGE_VERSION,
    VideoStudioMediaFileEntry,
};
use serde_json::json;
use sha2::Digest;
use zip::{ZipWriter, write::SimpleFileOptions};

use super::super::{MANIFEST_PATH, VideoStudioProjectPackage};
use super::reader::read_video_studio_package_reader;

fn now() -> chrono::NaiveDateTime {
    chrono::Utc::now().naive_utc()
}

fn freeform_project() -> novel_project::Model {
    novel_project::Model {
        id: 1,
        user_id: 1,
        title: "自由视频".to_string(),
        description: String::new(),
        project_type: "script".to_string(),
        project_kind: PROJECT_KIND_VIDEO.to_string(),
        root_project_id: None,
        source_snapshot_id: None,
        source_sync_status: SOURCE_SYNC_CURRENT.to_string(),
        status: "draft".to_string(),
        lifecycle_gate: "active".to_string(),
        target_chapters: 1,
        current_chapter: 1,
        total_chapters: None,
        chapters_per_part: None,
        avatar_svg: None,
        avatar_animal: None,
        raw_blueprint: None,
        import_analysis_progress: None,
        export_config: None,
        created_at: now(),
        updated_at: now(),
    }
}

fn package() -> VideoStudioProjectPackage {
    VideoStudioProjectPackage {
        version: VIDEO_STUDIO_PACKAGE_VERSION.to_string(),
        exported_at: now().to_string(),
        mode: PACKAGE_MODE.to_string(),
        project: freeform_project(),
        canvas_graphs: Vec::new(),
        canvas_nodes: Vec::new(),
        canvas_edges: Vec::new(),
        canvas_text_assets: Vec::new(),
        generation_specs: Vec::new(),
        prompt_artifacts: Vec::new(),
        generated_images: Vec::new(),
        generated_videos: Vec::new(),
        media_assets: Vec::new(),
        media_provenance: Vec::new(),
        media_bindings: Vec::new(),
        video_segments: Vec::new(),
        assembly_timelines: Vec::new(),
        assembly_clips: Vec::new(),
        media_files: Vec::new(),
    }
}

fn zip_with_entries(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in entries {
        if name.ends_with('/') {
            zip.add_directory(*name, options)
                .expect("start zip directory entry");
        } else {
            zip.start_file(*name, options).expect("start zip entry");
            zip.write_all(bytes).expect("write zip entry");
        }
    }
    zip.finish().expect("finish zip").into_inner()
}

#[test]
fn read_video_studio_package_rejects_undeclared_zip_file() {
    let manifest = serde_json::to_vec(&package()).expect("serialize package");
    let bytes = zip_with_entries(&[
        (MANIFEST_PATH, &manifest),
        ("media/generated_video/10/file.mp4", b"undeclared"),
    ]);

    let err =
        read_video_studio_package_reader(bytes).expect_err("undeclared package file must fail");

    assert!(err.contains("未在清单声明"));
    assert!(err.contains("media/generated_video/10/file.mp4"));
}

#[test]
fn read_video_studio_package_ignores_directory_entries() {
    let manifest = serde_json::to_vec(&package()).expect("serialize package");
    let bytes = zip_with_entries(&[(MANIFEST_PATH, &manifest), ("media/", b"")]);

    read_video_studio_package_reader(bytes).expect("directory entries should be ignored");
}

#[test]
fn read_video_studio_package_rejects_parent_dir_entry() {
    let manifest = serde_json::to_vec(&package()).expect("serialize package");
    let bytes = zip_with_entries(&[(MANIFEST_PATH, &manifest), ("../payload.bin", b"bad")]);

    let err = read_video_studio_package_reader(bytes)
        .expect_err("zip entries must not escape package root");

    assert!(err.contains("路径不能越界"));
}

#[test]
fn read_video_studio_package_accepts_declared_media_file() {
    let mut package = package();
    let bytes = b"media bytes";
    package.media_assets.push(media_asset::Model {
        id: 10,
        project_id: 1,
        root_project_id: 1,
        source_snapshot_id: None,
        source_chapter_id: None,
        source_snapshot_chapter_id: None,
        media_kind: "video".to_string(),
        origin: "imported".to_string(),
        file_path: Some("projects/1/videos/asset.mp4".to_string()),
        thumbnail_path: None,
        width: Some(1280),
        height: Some(720),
        duration_secs: Some(3.0),
        fps: Some(24.0),
        file_size: Some(bytes.len() as i64),
        checksum: Some({
            let mut hasher = sha2::Sha256::new();
            hasher.update(bytes);
            format!("{:x}", hasher.finalize())
        }),
        status: MEDIA_STATUS_COMPLETED.to_string(),
        error_message: None,
        created_at: now(),
        updated_at: now(),
    });
    package.media_files.push(VideoStudioMediaFileEntry {
        owner_kind: CanvasNodeRefKind::MediaAsset.as_str().to_string(),
        owner_id: 10,
        field: FILE_PATH_FIELD.to_string(),
        original_path: "projects/1/videos/asset.mp4".to_string(),
        package_path: "media/media_asset/10/file.mp4".to_string(),
    });
    let manifest = serde_json::to_vec(&package).expect("serialize package");
    let archive = zip_with_entries(&[
        (MANIFEST_PATH, &manifest),
        ("media/media_asset/10/file.mp4", bytes),
    ]);

    let (_, payload) =
        read_video_studio_package_reader(archive).expect("declared media file should read");

    assert_eq!(
        payload.bytes["media/media_asset/10/file.mp4"],
        bytes.to_vec()
    );
    assert_eq!(payload.entries.len(), 1);
    assert_eq!(json!(payload.bytes.len()), json!(1));
}
