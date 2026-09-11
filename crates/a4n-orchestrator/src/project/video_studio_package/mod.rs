//! 视频画布项目包：类型、纯校验与「校验后落库」application 入口。
//!
//! Orchestrator 负责包合同与导入导出编排；Host 只执行 zip 和物理文件 IO。

use a4n_db::canvas::VideoStudioProjectImportRows;
use a4n_db_models::models::{
    ai::{generated_image, generated_video},
    canvas::{canvas_edge, canvas_graph, canvas_node, canvas_text_asset},
    generation::{generation_spec, prompt_artifact},
    media::{media_asset, media_binding, media_provenance},
    project::novel_project,
    video_project::{assembly_clip, assembly_timeline, video_segment},
};
use serde::{Deserialize, Serialize};

use crate::project::{ImportMediaPayload, MediaFileEntry};

mod export_assembly;
mod validation;

pub use validation::MediaEntryOwnerIndex;

/// 视频画布项目包版本号。
pub(crate) const VIDEO_STUDIO_PACKAGE_VERSION: &str = "video_studio_project_v2";
/// 视频画布项目包 mode。
pub(crate) const PACKAGE_MODE: &str = "video_studio_project";
/// zip 内清单路径。
pub const MANIFEST_PATH: &str = "a4n-video-studio-project.json";

/// 视频画布导出/导入包 manifest。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoStudioProjectPackage {
    pub version: String,
    pub exported_at: String,
    pub mode: String,
    pub project: novel_project::Model,
    pub canvas_graphs: Vec<canvas_graph::Model>,
    pub canvas_nodes: Vec<canvas_node::Model>,
    pub canvas_edges: Vec<canvas_edge::Model>,
    pub canvas_text_assets: Vec<canvas_text_asset::Model>,
    pub generation_specs: Vec<generation_spec::Model>,
    pub prompt_artifacts: Vec<prompt_artifact::Model>,
    pub generated_images: Vec<generated_image::Model>,
    pub generated_videos: Vec<generated_video::Model>,
    pub media_assets: Vec<media_asset::Model>,
    pub media_provenance: Vec<media_provenance::Model>,
    pub media_bindings: Vec<media_binding::Model>,
    pub video_segments: Vec<video_segment::Model>,
    pub assembly_timelines: Vec<assembly_timeline::Model>,
    pub assembly_clips: Vec<assembly_clip::Model>,
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

/// 校验媒体清单项 owner 引用（包读取时用）。
pub fn validate_media_entry_owner(
    package: &VideoStudioProjectPackage,
    entry: &MediaFileEntry,
) -> Result<(), String> {
    validation::validate_media_entry_owner(package, entry)
}

/// 校验媒体字节与记录字段一致性（包读取时用）。
pub fn validate_media_entry_bytes(
    package: &VideoStudioProjectPackage,
    entry: &MediaFileEntry,
    bytes: &[u8],
) -> Result<(), String> {
    validation::validate_media_entry_bytes(package, entry, bytes)
}
