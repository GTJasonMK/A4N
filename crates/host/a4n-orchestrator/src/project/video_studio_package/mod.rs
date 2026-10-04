//! 视频画布项目包：类型、纯校验与「校验后落库」application 入口。
//!
//! Orchestrator 负责包合同与导入导出编排；Host 只执行 zip 和物理文件 IO。

use a4n_db::canvas::VideoStudioProjectImportRows;
use a4n_domain::canvas::{
    CanvasEdgeRecord, CanvasGraphRecord, CanvasNodeRecord, CanvasTextAssetRecord,
};
use a4n_domain::generation::{GenerationSpecRecord, PromptArtifactRecord};
use a4n_domain::media::{
    GeneratedImageRecord, GeneratedVideoRecord, MediaAssetRecord, MediaBindingRecord,
    MediaProvenanceRecord,
};
use a4n_domain::project::ProjectRecord;
use a4n_domain::video_transfer::{AssemblyClipRecord, AssemblyTimelineRecord, VideoSegmentRecord};
use serde::{Deserialize, Serialize};

use crate::project::{ImportMediaPayload, MediaFileEntry};

mod export_assembly;
mod validation;

#[cfg(test)]
pub(crate) use validation::MediaEntryOwnerIndex;

/// 视频画布项目包版本号。
pub(crate) const VIDEO_STUDIO_PACKAGE_VERSION: &str = "canvas_project_v1";
/// 视频画布项目包 mode。
pub(crate) const PACKAGE_MODE: &str = "canvas_project";
/// zip 内清单路径。
pub const MANIFEST_PATH: &str = "a4n-canvas-project.json";

/// 视频画布导出/导入包 manifest。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoStudioProjectPackage {
    pub version: String,
    pub exported_at: String,
    pub mode: String,
    pub project: ProjectRecord,
    pub canvas_graphs: Vec<CanvasGraphRecord>,
    pub canvas_nodes: Vec<CanvasNodeRecord>,
    pub canvas_edges: Vec<CanvasEdgeRecord>,
    pub canvas_text_assets: Vec<CanvasTextAssetRecord>,
    pub generation_specs: Vec<GenerationSpecRecord>,
    pub prompt_artifacts: Vec<PromptArtifactRecord>,
    pub generated_images: Vec<GeneratedImageRecord>,
    pub generated_videos: Vec<GeneratedVideoRecord>,
    pub media_assets: Vec<MediaAssetRecord>,
    pub media_provenance: Vec<MediaProvenanceRecord>,
    pub media_bindings: Vec<MediaBindingRecord>,
    pub video_segments: Vec<VideoSegmentRecord>,
    pub assembly_timelines: Vec<AssemblyTimelineRecord>,
    pub assembly_clips: Vec<AssemblyClipRecord>,
    pub(crate) provider_slots: Vec<crate::project::ProjectBundleProviderSlot>,
    pub media_files: Vec<MediaFileEntry>,
}

impl VideoStudioProjectPackage {
    /// 转为 DB 导入行（不含 media_files 索引）。
    pub(crate) fn into_import_rows(self) -> VideoStudioProjectImportRows {
        VideoStudioProjectImportRows {
            project: self.project,
            canvas_graphs: self.canvas_graphs,
            canvas_nodes: self.canvas_nodes,
            canvas_edges: self.canvas_edges,
            canvas_text_assets: self.canvas_text_assets,
            generation_specs: self.generation_specs,
            prompt_artifacts: self.prompt_artifacts,
            generated_images: self.generated_images,
            generated_videos: self.generated_videos,
            media_assets: self.media_assets,
            media_provenance: self.media_provenance,
            media_bindings: self.media_bindings,
            video_segments: self.video_segments,
            assembly_timelines: self.assembly_timelines,
            assembly_clips: self.assembly_clips,
        }
    }
}

/// 组装视频画布导出 manifest。
pub(crate) use export_assembly::assemble_video_studio_export_package;

/// 校验包 manifest 与媒体 payload 一致性（纯规则，无 DB）。
pub(crate) fn validate_package_manifest(
    package: &VideoStudioProjectPackage,
    media_payload: &ImportMediaPayload,
) -> Result<(), String> {
    validation::validate_package_manifest(package, media_payload)
}
