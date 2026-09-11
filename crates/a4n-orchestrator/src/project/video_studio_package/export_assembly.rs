//! 视频画布导出：freeform 校验后组装 manifest（不含磁盘媒体读取）。

use a4n_db::canvas::VideoStudioProjectExportRows;
use a4n_db_models::models::{generation::generation_spec, project::novel_project};
use a4n_domain::clear_canvas_transfer_runtime_identity;
use chrono::Utc;

use super::{PACKAGE_MODE, VIDEO_STUDIO_PACKAGE_VERSION, VideoStudioProjectPackage};
use crate::project::{MediaFileEntry, validate_freeform_video_project};

/// 组装视频画布导出 manifest。
pub(crate) fn assemble_video_studio_export_package(
    project: novel_project::Model,
    mut rows: VideoStudioProjectExportRows,
    media_files: Vec<MediaFileEntry>,
) -> Result<VideoStudioProjectPackage, String> {
    validate_freeform_video_project(&project, "视频画布项目导出")?;
    for image in &mut rows.generated_images {
        if let Some(metadata) = image.metadata_json.as_mut() {
            clear_canvas_transfer_runtime_identity(metadata);
        }
    }
    for video in &mut rows.generated_videos {
        if !a4n_domain::video_workflow::is_terminal_video_status(&video.status) {
            return Err(format!(
                "视频画布项目不能导出非终态生成视频: generated_video_id={}, status={}",
                video.id, video.status
            ));
        }
        video.provider_job_id = None;
        if let Some(metadata) = video.metadata_json.as_mut() {
            clear_canvas_transfer_runtime_identity(metadata);
        }
    }
    for spec in &mut rows.generation_specs {
        clear_canvas_transfer_runtime_identity(&mut spec.params_json);
        spec.source_version_hash = generation_spec::freeform_source_version_hash(&spec.params_json)
            .ok_or_else(|| {
                format!(
                    "生成意图 params_json.source_material 必须是 JSON 对象: generation_spec_id={}",
                    spec.id
                )
            })?;
    }
    for provenance in &mut rows.media_provenance {
        provenance.provider_job_id = None;
        if let Some(snapshot) = provenance.request_snapshot_json.as_mut() {
            clear_canvas_transfer_runtime_identity(snapshot);
        }
    }
    Ok(VideoStudioProjectPackage {
        version: VIDEO_STUDIO_PACKAGE_VERSION.to_string(),
        exported_at: Utc::now().naive_utc().to_string(),
        mode: PACKAGE_MODE.to_string(),
        project,
        canvas_graphs: rows.canvas_graphs,
        canvas_nodes: rows.canvas_nodes,
        canvas_edges: rows.canvas_edges,
        canvas_text_assets: rows.canvas_text_assets,
        generation_specs: rows.generation_specs,
        prompt_artifacts: rows.prompt_artifacts,
        generated_images: rows.generated_images,
        generated_videos: rows.generated_videos,
        media_assets: rows.media_assets,
        media_provenance: rows.media_provenance,
        media_bindings: rows.media_bindings,
        video_segments: rows.video_segments,
        assembly_timelines: rows.assembly_timelines,
        assembly_clips: rows.assembly_clips,
        media_files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_db::{
        canvas::VideoStudioProjectExportRows,
        generation::{
            GENERATION_SPEC_TARGET_ENTITY_KIND_CANVAS_NODE,
            GENERATION_SPEC_TARGET_KIND_CANVAS_VIDEO,
        },
    };
    use a4n_db_models::models::generation::generation_spec;
    use serde_json::json;

    fn now() -> chrono::NaiveDateTime {
        chrono::Utc::now().naive_utc()
    }

    fn project() -> novel_project::Model {
        novel_project::Model {
            id: 1,
            user_id: 1,
            title: "自由视频".to_string(),
            description: String::new(),
            project_type: "script".to_string(),
            project_kind: "video".to_string(),
            root_project_id: None,
            source_snapshot_id: None,
            source_sync_status: "current".to_string(),
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

    fn rows(spec: generation_spec::Model) -> VideoStudioProjectExportRows {
        VideoStudioProjectExportRows {
            canvas_graphs: Vec::new(),
            canvas_nodes: Vec::new(),
            canvas_edges: Vec::new(),
            canvas_text_assets: Vec::new(),
            generation_specs: vec![spec],
            prompt_artifacts: Vec::new(),
            generated_images: Vec::new(),
            generated_videos: Vec::new(),
            media_assets: Vec::new(),
            media_provenance: Vec::new(),
            media_bindings: Vec::new(),
            video_segments: Vec::new(),
            assembly_timelines: Vec::new(),
            assembly_clips: Vec::new(),
        }
    }

    #[test]
    fn export_clears_runtime_identity_and_rehashes_source_material() {
        let spec = generation_spec::Model {
            id: 10,
            project_id: 1,
            root_project_id: 1,
            source_context_kind: Some(generation_spec::SOURCE_CONTEXT_FREEFORM.to_string()),
            source_snapshot_id: None,
            source_chapter_id: None,
            source_snapshot_chapter_id: None,
            canvas_text_asset_id: None,
            target_kind: GENERATION_SPEC_TARGET_KIND_CANVAS_VIDEO.to_string(),
            target_entity_kind: GENERATION_SPEC_TARGET_ENTITY_KIND_CANVAS_NODE.to_string(),
            target_entity_id: Some(20),
            status: "completed".to_string(),
            source_version_hash: "stale".to_string(),
            params_json: json!({
                "source_material": {"prompt": "prompt", "config_id": 7},
                "workflow_run_id": 8
            }),
            created_at: now(),
            updated_at: now(),
        };

        let package = assemble_video_studio_export_package(project(), rows(spec), Vec::new())
            .expect("export should sanitize runtime identity");
        let exported = &package.generation_specs[0];
        let source_material = &exported.params_json["source_material"];

        assert!(source_material.get("config_id").is_none());
        assert!(exported.params_json.get("workflow_run_id").is_none());
        assert_eq!(
            Some(exported.source_version_hash.clone()),
            generation_spec::freeform_source_version_hash(&exported.params_json)
        );
    }
}
