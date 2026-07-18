use sea_orm::DbErr;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct CreateMediaAssetRecordInput {
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_snapshot_id: Option<i64>,
    pub source_chapter_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub media_kind: String,
    pub origin: String,
    pub file_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration_secs: Option<f32>,
    pub fps: Option<f32>,
    pub file_size: Option<i64>,
    pub checksum: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub provenance: MediaProvenanceRecordInput,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MediaProvenanceRecordInput {
    pub generation_spec_id: Option<i64>,
    pub prompt_artifact_id: Option<i64>,
    pub provider_job_id: Option<String>,
    pub source_media_asset_id: Option<i64>,
    pub source_external_path: Option<String>,
    pub imported_from_user: bool,
    pub request_snapshot_json: Option<Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum MediaDbError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    InvalidArgument(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}
