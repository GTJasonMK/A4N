mod generation_specs;
mod graph;
mod media;
mod provenance;

use std::collections::HashMap;

use a4n_core::models::media::media_asset::{FILE_PATH_FIELD, THUMBNAIL_PATH_FIELD};
use a4n_core::models::{
    ai::generated_video,
    canvas::{canvas_edge, canvas_graph, canvas_node},
    generation::generation_spec,
    media::{media_asset, media_provenance},
    project::novel_project,
    video_project::assembly_timeline,
};
use a4n_core::video_workflow::WORKFLOW_MODE_CANVAS_VIDEO;
use a4n_domain::CanvasNodeRefKind;

use crate::media::MEDIA_STATUS_COMPLETED;
use crate::project::video_studio_package::{PACKAGE_MODE, VIDEO_STUDIO_PACKAGE_VERSION};
use crate::project::{
    ImportMediaPayload, MediaFileEntry, PROJECT_KIND_VIDEO, SOURCE_SYNC_CURRENT,
    VideoStudioProjectPackage, media_file_key, sha256_bytes,
};

use super::{validate_media_entry_bytes, validate_package_manifest};

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

fn video_with_file() -> generated_video::Model {
    generated_video::Model {
        id: 10,
        project_id: 1,
        chapter_id: None,
        video_type: generated_video::VIDEO_TYPE_CANVAS.to_string(),
        workflow_mode: Some(WORKFLOW_MODE_CANVAS_VIDEO.to_string()),
        file_path: Some("projects/1/videos/video.mp4".to_string()),
        thumbnail_path: Some("projects/1/videos/video.jpg".to_string()),
        prompt: "prompt".to_string(),
        negative_prompt: None,
        model: "test".to_string(),
        width: 1280,
        height: 720,
        duration_secs: 3.0,
        fps: 24,
        file_size: Some(10),
        source_video_id: None,
        source_image_id: None,
        status: MEDIA_STATUS_COMPLETED.to_string(),
        provider_job_id: None,
        error_message: None,
        timeline_item_id: None,
        metadata_json: None,
        is_favorite: false,
        created_at: now(),
        updated_at: now(),
    }
}

fn empty_payload() -> ImportMediaPayload {
    ImportMediaPayload {
        entries: HashMap::new(),
        bytes: HashMap::new(),
    }
}

#[test]
fn validate_video_studio_manifest_rejects_blank_project_title() {
    let mut package = package();
    package.project.title = " ".to_string();

    let error =
        validate_package_manifest(&package, &empty_payload()).expect_err("空项目标题必须失败");
    assert!(error.contains("项目标题"));
}

fn graph() -> canvas_graph::Model {
    canvas_graph::Model {
        id: 20,
        project_id: 1,
        name: "默认画布".to_string(),
        graph_kind: "video_studio".to_string(),
        version: 1,
        viewport_json: None,
        created_at: now(),
        updated_at: now(),
    }
}

fn node_with_ref(ref_type: &str, ref_id: i64) -> canvas_node::Model {
    canvas_node::Model {
        id: 30,
        graph_id: 20,
        node_kind: "VideoNode".to_string(),
        ref_type: Some(ref_type.to_string()),
        ref_id: Some(ref_id),
        config_json: serde_json::json!({}),
        position_json: serde_json::json!({ "x": 0, "y": 0 }),
        state: "idle".to_string(),
        created_at: now(),
        updated_at: now(),
    }
}

fn canvas_node(id: i64, node_kind: &str) -> canvas_node::Model {
    canvas_node::Model {
        id,
        graph_id: 20,
        node_kind: node_kind.to_string(),
        ref_type: Some("inline".to_string()),
        ref_id: None,
        config_json: serde_json::json!({}),
        position_json: serde_json::json!({ "x": 0, "y": 0 }),
        state: "idle".to_string(),
        created_at: now(),
        updated_at: now(),
    }
}

fn canvas_edge(
    id: i64,
    source_node_id: i64,
    target_node_id: i64,
    source_port: &str,
    target_port: &str,
    role: &str,
    config_json: serde_json::Value,
) -> canvas_edge::Model {
    canvas_edge::Model {
        id,
        graph_id: 20,
        source_node_id,
        source_port: source_port.to_string(),
        target_node_id,
        target_port: target_port.to_string(),
        role: role.to_string(),
        config_json,
    }
}

fn assembly_timeline_with_cache_key(cache_key: &str) -> assembly_timeline::Model {
    assembly_timeline::Model {
        id: 40,
        project_id: 1,
        source_chapter_id: None,
        source_snapshot_chapter_id: None,
        version: 1,
        status: MEDIA_STATUS_COMPLETED.to_string(),
        preview_media_asset_id: None,
        preview_cache_key: Some(cache_key.to_string()),
        created_at: now(),
        updated_at: now(),
    }
}

fn media_asset_with_file(bytes: &[u8]) -> media_asset::Model {
    media_asset::Model {
        id: 60,
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
        checksum: Some(sha256_bytes(bytes)),
        status: MEDIA_STATUS_COMPLETED.to_string(),
        error_message: None,
        created_at: now(),
        updated_at: now(),
    }
}

fn media_asset_without_file() -> media_asset::Model {
    let mut asset = media_asset_with_file(b"unused");
    asset.file_path = None;
    asset.file_size = None;
    asset.checksum = None;
    asset.status = "failed".to_string();
    asset.error_message = Some("导入包测试媒体资产".to_string());
    asset
}

fn media_provenance(
    source_external_path: Option<&str>,
    request_snapshot_json: Option<serde_json::Value>,
) -> media_provenance::Model {
    media_provenance::Model {
        media_asset_id: 60,
        generation_spec_id: None,
        prompt_artifact_id: None,
        provider_job_id: None,
        source_media_asset_id: None,
        source_external_path: source_external_path.map(str::to_string),
        imported_from_user: false,
        request_snapshot_json,
        created_at: now(),
    }
}

fn generation_spec_with_target(
    target_entity_kind: &str,
    target_entity_id: Option<i64>,
) -> generation_spec::Model {
    let source_material = serde_json::json!({"prompt":"prompt","source_node_ids":[]});
    let params_json = serde_json::json!({"source_material":source_material});
    generation_spec::Model {
        id: 50,
        project_id: 1,
        root_project_id: 1,
        source_context_kind: Some(generation_spec::SOURCE_CONTEXT_FREEFORM.to_string()),
        source_snapshot_id: None,
        source_chapter_id: None,
        source_snapshot_chapter_id: None,
        canvas_text_asset_id: None,
        target_kind: "video".to_string(),
        target_entity_kind: target_entity_kind.to_string(),
        target_entity_id,
        status: "completed".to_string(),
        source_version_hash: generation_spec::freeform_source_version_hash(&params_json).unwrap(),
        params_json,
        created_at: now(),
        updated_at: now(),
    }
}
