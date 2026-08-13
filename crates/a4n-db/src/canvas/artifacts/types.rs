use a4n_core::models::ai::{generated_image, generated_video};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasMediaSourceContext {
    pub root_project_id: i64,
    pub source_snapshot_id: Option<i64>,
    pub source_chapter_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct FreeformCanvasGenerationSpecUpsertInput {
    pub project_id: i64,
    pub target_entity_id: i64,
    pub canvas_text_asset_id: Option<i64>,
    pub target_kind: String,
    pub target_entity_kind: String,
    pub status: String,
    pub params_json: Value,
}

#[derive(Debug, Clone)]
pub struct CanvasPromptArtifactUpsertInput {
    pub generation_spec_id: i64,
    pub prompt_kind: String,
    pub prompt_text: String,
}

#[derive(Debug, Clone)]
pub struct CanvasImageMediaAssetEnsureInput {
    pub project_id: i64,
    pub image: generated_image::Model,
    pub generation_spec_id: Option<i64>,
    pub origin: String,
}

#[derive(Debug, Clone)]
pub struct CanvasVideoMediaAssetEnsureInput {
    pub project_id: i64,
    pub video: generated_video::Model,
    pub generation_spec_id: Option<i64>,
    pub origin: String,
}

#[derive(Debug, Clone)]
pub struct CanvasAssemblySourceVideoInput {
    pub video_id: i64,
}

#[derive(Debug, Clone)]
pub struct CanvasAssemblyContractSyncInput {
    pub project_id: i64,
    pub graph_id: i64,
    pub node_id: i64,
    pub assembly_video_id: i64,
    pub source_videos: Vec<CanvasAssemblySourceVideoInput>,
    pub generation_spec_id: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasAssemblyContractSyncSummary {
    pub preview_media_asset_id: i64,
}

#[derive(Debug, Clone)]
pub struct CanvasNodeArtifactUpdateInput {
    pub node_id: i64,
    pub ref_type: String,
    pub ref_id: i64,
    pub config_patch: Value,
}

#[derive(Debug, Clone)]
pub struct CanvasNodeObservationUpdateInput {
    pub node_id: i64,
    pub config_patch: Value,
}
