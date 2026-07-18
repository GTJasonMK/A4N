use a4n_core::models::{
    ai::{generated_image, generated_video},
    canvas::{canvas_edge, canvas_graph, canvas_node, canvas_text_asset},
    generation::{generation_spec, prompt_artifact},
    media::{media_asset, media_binding, media_provenance},
    project::novel_project,
    video_project::{assembly_clip, assembly_timeline, video_segment},
};
use sea_orm::DbErr;
use std::collections::HashMap;

#[derive(Debug, thiserror::Error)]
pub enum VideoStudioProjectImportMediaError {
    #[error("{0}")]
    Write(String),
}

#[derive(Debug, thiserror::Error)]
pub enum VideoStudioProjectImportError {
    #[error("{0}")]
    InvalidPackage(String),
    #[error(transparent)]
    MediaWrite(#[from] VideoStudioProjectImportMediaError),
    #[error("导入视频画布项目失败: {0}")]
    ProjectInsert(#[source] DbErr),
    #[error("提交视频画布项目导入事务失败: {0}")]
    TransactionCommit(#[source] DbErr),
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, Clone)]
pub struct VideoStudioProjectExportRows {
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
}

#[derive(Debug, Clone)]
pub struct VideoStudioProjectImportRows {
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
}

pub trait VideoStudioProjectImportMediaFileWriter {
    fn write_imported_media_file(
        &mut self,
        project_id: i64,
        owner_kind: &str,
        owner_id: i64,
        field: &str,
        folder: &str,
        stem: &str,
    ) -> Result<Option<String>, VideoStudioProjectImportMediaError>;
}

#[derive(Default)]
pub(super) struct VideoStudioProjectImportIdMaps {
    pub(super) image_ids: HashMap<i64, i64>,
    pub(super) image_file_paths: HashMap<i64, String>,
    pub(super) video_ids: HashMap<i64, i64>,
    pub(super) video_file_paths: HashMap<i64, Option<String>>,
    pub(super) video_thumbnail_paths: HashMap<i64, Option<String>>,
    pub(super) text_asset_ids: HashMap<i64, i64>,
    pub(super) graph_ids: HashMap<i64, i64>,
    pub(super) node_ids: HashMap<i64, i64>,
    pub(super) generation_spec_ids: HashMap<i64, i64>,
    pub(super) prompt_artifact_ids: HashMap<i64, i64>,
    pub(super) media_asset_ids: HashMap<i64, i64>,
    pub(super) media_binding_ids: HashMap<i64, i64>,
    pub(super) video_segment_ids: HashMap<i64, i64>,
    pub(super) assembly_timeline_ids: HashMap<i64, i64>,
    pub(super) assembly_clip_ids: HashMap<i64, i64>,
}

pub(super) struct DeferredVideoStudioImportGenerationSpecTarget {
    pub(super) new_spec_id: i64,
    pub(super) target_entity_kind: String,
    pub(super) old_target_entity_id: i64,
}

pub(super) struct InitialVideoStudioGenerationSpecTarget {
    pub(super) target_entity_id: Option<i64>,
    pub(super) deferred: Option<DeferredVideoStudioGenerationSpecTarget>,
}

pub(super) struct DeferredVideoStudioGenerationSpecTarget {
    pub(super) target_entity_kind: String,
    pub(super) old_target_entity_id: i64,
}
