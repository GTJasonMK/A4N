use a4n_db_models::models::ai::{generated_image, generated_video};
use sea_orm::FromQueryResult;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CanvasMediaSourceContext {
    pub root_project_id: i64,
    pub source_snapshot_id: Option<i64>,
    pub source_chapter_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct CanvasMediaAssetProbeRecord {
    pub file_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub duration_secs: Option<f32>,
}

#[derive(Debug, Clone, FromQueryResult)]
pub(super) struct CanvasVideoMediaSource {
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: Option<i64>,
    pub video_type: String,
    pub workflow_mode: Option<String>,
    pub file_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub model: String,
    pub width: i32,
    pub height: i32,
    pub duration_secs: f32,
    pub fps: i32,
    pub file_size: Option<i64>,
    pub status: String,
    pub provider_job_id: Option<String>,
    pub metadata_json: Option<Value>,
}

impl From<generated_video::Model> for CanvasVideoMediaSource {
    fn from(video: generated_video::Model) -> Self {
        Self {
            id: video.id,
            project_id: video.project_id,
            chapter_id: video.chapter_id,
            video_type: video.video_type,
            workflow_mode: video.workflow_mode,
            file_path: video.file_path,
            thumbnail_path: video.thumbnail_path,
            model: video.model,
            width: video.width,
            height: video.height,
            duration_secs: video.duration_secs,
            fps: video.fps,
            file_size: video.file_size,
            status: video.status,
            provider_job_id: video.provider_job_id,
            metadata_json: video.metadata_json,
        }
    }
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
pub(super) struct CanvasVideoMediaSourceEnsureInput {
    pub project_id: i64,
    pub video: CanvasVideoMediaSource,
    pub generation_spec_id: Option<i64>,
    pub origin: String,
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
