//! 视频画布导出：freeform 校验后组装 manifest（不含磁盘媒体读取）。

use a4n_core::models::project::novel_project;
use a4n_db::canvas::VideoStudioProjectExportRows;
use chrono::Utc;

use super::{
    PACKAGE_MODE, VIDEO_STUDIO_PACKAGE_VERSION, VideoStudioMediaFileEntry,
    VideoStudioProjectPackage,
};
use crate::project::validate_freeform_video_project;

/// 组装视频画布导出 manifest。
pub fn assemble_video_studio_export_package(
    project: novel_project::Model,
    rows: VideoStudioProjectExportRows,
    media_files: Vec<VideoStudioMediaFileEntry>,
) -> Result<VideoStudioProjectPackage, String> {
    validate_freeform_video_project(&project, "视频画布项目导出")?;
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
