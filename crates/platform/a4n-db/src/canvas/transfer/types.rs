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

#[derive(Debug, Clone, Default)]
pub struct VideoStudioProjectExportRows {
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
}

#[derive(Debug, Clone)]
pub struct VideoStudioProjectImportRows {
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
pub(crate) struct VideoStudioProjectImportIdMaps {
    pub(crate) image_ids: HashMap<i64, i64>,
    pub(crate) image_file_paths: HashMap<i64, String>,
    pub(crate) video_ids: HashMap<i64, i64>,
    pub(crate) video_file_paths: HashMap<i64, Option<String>>,
    pub(crate) video_thumbnail_paths: HashMap<i64, Option<String>>,
    pub(crate) text_asset_ids: HashMap<i64, i64>,
    pub(crate) graph_ids: HashMap<i64, i64>,
    pub(crate) node_ids: HashMap<i64, i64>,
    pub(crate) generation_spec_ids: HashMap<i64, i64>,
    pub(crate) prompt_artifact_ids: HashMap<i64, i64>,
    pub(crate) media_asset_ids: HashMap<i64, i64>,
    pub(crate) video_segment_ids: HashMap<i64, i64>,
    pub(crate) assembly_timeline_ids: HashMap<i64, i64>,
    pub(crate) assembly_clip_ids: HashMap<i64, i64>,
}

pub(crate) struct DeferredVideoStudioImportGenerationSpecTarget {
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
