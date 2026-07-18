use crate::actions::dependency_conflict::DownstreamConflictStrategy;
use a4n_core::manga::{MangaPageDrawingPromptV2, MangaPageV3};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MangaPromptSummary {
    pub id: i64,
    pub chapter_id: i64,
    pub project_id: i64,
    pub page_number: i32,
    pub panel_number: i32,
    pub prompt_type: String,
    pub image_prompt: String,
    pub dialogue_text: Option<String>,
    pub shooting_script_id: i64,
    pub shooting_script_revision: i32,
    pub page_plan: MangaPageV3,
    pub drawing_prompt: MangaPageDrawingPromptV2,
    pub reference_bindings: Vec<MangaReferenceBindingSummary>,
    pub source: MangaPromptSourceSummary,
    pub generated_image_id: Option<i64>,
    pub status: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MangaPromptSourceSummary {
    pub root_project_id: i64,
    pub source_chapter_id: i64,
    pub source_snapshot_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub source_revision_key: String,
    pub content_sha256: String,
    pub shooting_script_id: i64,
    pub shooting_script_revision: i32,
    pub shooting_script_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MangaReferenceBindingSummary {
    pub slot_id: String,
    pub target_type: String,
    pub target_name: Option<String>,
    pub label: String,
    pub purpose: String,
    pub required: bool,
    pub asset_kind: String,
    pub status: String,
    #[serde(default)]
    pub bound_asset: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MangaPageGenerationLayoutV2 {
    pub schema_version: String,
    pub shooting_script_id: i64,
    pub shooting_script_revision: i32,
    pub page_plan: MangaPageV3,
    pub drawing_prompt: MangaPageDrawingPromptV2,
    #[serde(rename = "reference_assets")]
    pub _reference_assets: Value,
    pub reference_bindings: Vec<MangaReferenceBindingSummary>,
    #[serde(rename = "prompt_fingerprint")]
    pub _prompt_fingerprint: Value,
    pub source: MangaPromptSourceSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStatusSummary {
    pub chapter_id: i64,
    pub pending: i64,
    pub generating: i64,
    pub completed: i64,
    pub failed: i64,
    pub cancelled: i64,
    pub pipeline_status: Option<String>,
    pub stage: Option<String>,
    pub current: Option<i32>,
    pub total: Option<i32>,
    pub message: Option<String>,
    pub task_id: Option<String>,
    pub can_cancel: bool,
    pub can_resume: bool,
    pub has_checkpoint: bool,
    pub checkpoint_valid: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BindReferenceImageInput {
    pub prompt_id: i64,
    pub slot_id: String,
    pub asset_kind: String,
    pub asset_id: i64,
    #[serde(default)]
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateMangaPromptInput {
    pub prompt_id: i64,
    pub image_prompt: String,
    #[serde(default)]
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptimizeMangaPromptInput {
    pub prompt_id: i64,
    pub instruction: String,
    #[serde(default)]
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OptimizedMangaPromptFields {
    pub image_prompt: String,
}
