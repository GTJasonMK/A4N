use a4n_core::config::app_config::llm_token_profiles;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(super) const GALGAME_PIPELINE_WORKFLOW_KIND: &str = "galgame_pipeline";
pub(super) const GALGAME_PIPELINE_MODE_FULL: &str = "full_pipeline_v1";
pub(super) const GALGAME_PIPELINE_MAX_TOKENS_PROFILE: &str = llm_token_profiles::EXTRA_LONG;
pub(super) const GALGAME_CHECKPOINT_SCENES: &str = "scenes";
pub(super) const GALGAME_CHECKPOINT_LINES_DRAFT: &str = "lines_draft";
pub(super) const GALGAME_CHECKPOINT_LINES_STAGED: &str = "lines_staged";
pub(super) const GALGAME_CHECKPOINT_VISUAL_PLAN: &str = "visual_plan";
pub(super) const GALGAME_CHECKPOINT_BRANCH_PLAN: &str = "branch_plan";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartGalgamePipelineInput {
    pub project_id: i64,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub start_from_stage: Option<String>,
    #[serde(default)]
    pub resume_from_checkpoint: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgamePipelineStatusSummary {
    pub run: Option<GalgameWorkflowRunSummary>,
    pub checkpoints: Vec<GalgameWorkflowCheckpointSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameWorkflowRunSummary {
    pub id: i64,
    pub status: String,
    pub mode: Option<String>,
    pub current_step_key: Option<String>,
    pub progress_json: Option<Value>,
    pub active_task_id: Option<String>,
    pub can_resume: bool,
    pub resume_block_reason: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalgameWorkflowCheckpointSummary {
    pub id: i64,
    pub step_key: String,
    pub step_order: i32,
    pub status: String,
    pub checkpoint_json: Option<Value>,
    pub artifacts_json: Option<Value>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}
