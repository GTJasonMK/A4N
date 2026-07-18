use chrono::NaiveDateTime;
use serde_json::Value;

#[derive(Debug)]
pub(in crate::asset_lineage_sync) struct GenerationSpecUpsertInput {
    pub(in crate::asset_lineage_sync) canonical_key: String,
    pub(in crate::asset_lineage_sync) project_id: i64,
    pub(in crate::asset_lineage_sync) root_project_id: i64,
    pub(in crate::asset_lineage_sync) source_snapshot_id: i64,
    pub(in crate::asset_lineage_sync) source_chapter_id: Option<i64>,
    pub(in crate::asset_lineage_sync) source_snapshot_chapter_id: Option<i64>,
    pub(in crate::asset_lineage_sync) target_kind: String,
    pub(in crate::asset_lineage_sync) target_entity_kind: String,
    pub(in crate::asset_lineage_sync) target_entity_id: Option<i64>,
    pub(in crate::asset_lineage_sync) status: String,
    pub(in crate::asset_lineage_sync) source_version_hash: String,
    pub(in crate::asset_lineage_sync) params_json: Value,
    pub(in crate::asset_lineage_sync) created_at: NaiveDateTime,
    pub(in crate::asset_lineage_sync) updated_at: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub(in crate::asset_lineage_sync) struct MediaUpsertInput {
    pub(in crate::asset_lineage_sync) canonical_key: String,
    pub(in crate::asset_lineage_sync) project_id: i64,
    pub(in crate::asset_lineage_sync) root_project_id: i64,
    pub(in crate::asset_lineage_sync) source_snapshot_id: Option<i64>,
    pub(in crate::asset_lineage_sync) source_chapter_id: Option<i64>,
    pub(in crate::asset_lineage_sync) source_snapshot_chapter_id: Option<i64>,
    pub(in crate::asset_lineage_sync) media_kind: String,
    pub(in crate::asset_lineage_sync) origin: String,
    pub(in crate::asset_lineage_sync) file_path: Option<String>,
    pub(in crate::asset_lineage_sync) thumbnail_path: Option<String>,
    pub(in crate::asset_lineage_sync) width: Option<i32>,
    pub(in crate::asset_lineage_sync) height: Option<i32>,
    pub(in crate::asset_lineage_sync) duration_secs: Option<f32>,
    pub(in crate::asset_lineage_sync) fps: Option<f32>,
    pub(in crate::asset_lineage_sync) file_size: Option<i64>,
    pub(in crate::asset_lineage_sync) status: String,
    pub(in crate::asset_lineage_sync) error_message: Option<String>,
    pub(in crate::asset_lineage_sync) generation_spec_id: Option<i64>,
    pub(in crate::asset_lineage_sync) prompt_artifact_id: Option<i64>,
    pub(in crate::asset_lineage_sync) provider_job_id: Option<String>,
    pub(in crate::asset_lineage_sync) source_media_asset_id: Option<i64>,
    pub(in crate::asset_lineage_sync) source_external_path: Option<String>,
    pub(in crate::asset_lineage_sync) imported_from_user: bool,
    pub(in crate::asset_lineage_sync) request_snapshot_json: Value,
    pub(in crate::asset_lineage_sync) created_at: NaiveDateTime,
}
