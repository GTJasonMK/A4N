use super::*;

#[derive(Debug, thiserror::Error)]
pub enum AssetLineageSyncError {
    #[error("参数无效：{0}")]
    InvalidArgument(String),
    #[error("未找到：{0}")]
    NotFound(String),
    #[error("数据库错误：{0}")]
    Database(#[from] sea_orm::DbErr),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssetLineageBackfillSummary {
    pub scanned_projects: u64,
    pub scanned_manga_prompts: u64,
    pub scanned_video_prompts: u64,
    pub scanned_generated_images: u64,
    pub scanned_generated_videos: u64,
    pub created_generation_specs: u64,
    pub updated_generation_specs: u64,
    pub created_prompt_artifacts: u64,
    pub updated_prompt_artifacts: u64,
    pub created_media_assets: u64,
    pub updated_media_assets: u64,
    pub created_media_provenance: u64,
    pub updated_media_provenance: u64,
    pub created_media_bindings: u64,
    pub created_generation_media_refs: u64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub(super) struct ProjectLineageContext {
    pub(super) project_id: i64,
    pub(super) root_project_id: i64,
    pub(super) source_mode: a4n_domain::project::ProjectSourceMode,
    pub(super) snapshot: Option<project_source_snapshot::Model>,
    pub(super) snapshot_chapters_by_source: BTreeMap<i64, source_snapshot_chapter::Model>,
}

#[derive(Debug, Clone)]
pub(super) struct FamilyLineageContext {
    pub(super) project_ids: Vec<i64>,
    pub(super) contexts_by_project_id: BTreeMap<i64, ProjectLineageContext>,
}

#[derive(Debug, Clone)]
pub(super) struct SourceLineageContext {
    pub(super) project_id: i64,
    pub(super) root_project_id: i64,
    pub(super) source_snapshot_id: i64,
    pub(super) source_version_hash: String,
    pub(super) source_chapter_id: Option<i64>,
    pub(super) source_snapshot_chapter_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub(super) struct MediaSourceLineageContext {
    pub(super) source_snapshot_id: Option<i64>,
    pub(super) source_chapter_id: Option<i64>,
    pub(super) source_snapshot_chapter_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub(super) struct SyncedPromptRef {
    pub(super) generation_spec_id: i64,
    pub(super) prompt_artifact_id: i64,
}
