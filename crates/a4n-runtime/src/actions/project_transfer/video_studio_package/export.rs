mod media;

use a4n_core::models::project::novel_project;
use chrono::Utc;

use crate::{actions::app_error_message, state::AppState};

use super::package_io::package_zip_bytes;
use super::validation::validate_freeform_video_project;
use super::{PACKAGE_MODE, VIDEO_STUDIO_PACKAGE_VERSION, VideoStudioProjectPackage};
use media::collect_media_files;

pub(in super::super) async fn export_video_studio_project_package_bytes(
    state: &AppState,
    project_id: i64,
) -> Result<Vec<u8>, String> {
    let project = state
        .orchestrator
        .project_by_id(project_id)
        .await
        .map_err(app_error_message)?
        .ok_or_else(|| format!("视频画布项目不存在: {project_id}"))?;
    validate_freeform_video_project(&project, "视频画布项目导出")?;

    let package = build_export_package(state, project).await?;
    package_zip_bytes(state, &package).await
}

async fn build_export_package(
    state: &AppState,
    project: novel_project::Model,
) -> Result<VideoStudioProjectPackage, String> {
    let project_id = project.id;
    let rows = state
        .orchestrator
        .video_studio_project_export_rows(project_id)
        .await
        .map_err(app_error_message)?;
    let media_files = collect_media_files(
        state,
        &rows.generated_images,
        &rows.generated_videos,
        &rows.media_assets,
    )?;

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
