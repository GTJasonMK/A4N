use a4n_api_contract::galgame::GalgameProgressReporter;
use a4n_domain::llm::llm_token_profiles;
use serde_json::Value;

use super::super::{
    GalgamePersistScene, GalgamePipelinePromptSourceSpan, GalgamePipelineStartStage,
    GalgameSpeakerProfile,
};

pub(super) const GALGAME_PIPELINE_WORKFLOW: &str = "galgame_pipeline";
pub(super) const GALGAME_PIPELINE_MAX_TOKENS_PROFILE: &str = llm_token_profiles::EXTRA_LONG;
pub(super) const CHECKPOINT_SCENES: &str = "scenes";
pub(super) const CHECKPOINT_LINES_DRAFT: &str = "lines_draft";
pub(super) const CHECKPOINT_LINES_STAGED: &str = "lines_staged";
pub(super) const CHECKPOINT_VISUAL_PLAN: &str = "visual_plan";
pub(super) const CHECKPOINT_BRANCH_PLAN: &str = "branch_plan";
pub(super) const CHECKPOINT_PERSIST_SCRIPT: &str = "persist_script";
pub(super) const CHECKPOINT_ASSET_GENERATION: &str = "asset_generation";
pub(super) const SOURCE_SYNC_STAGE_SHOOTING_SCRIPT: &str = "shooting_script_projection";
pub(super) const SOURCE_SYNC_STAGE_SCRIPT_ADAPTATION: &str = "script_adaptation";
pub(super) const SOURCE_SYNC_STAGE_CHARACTER_STAGING: &str = "character_staging";
pub(super) const SOURCE_SYNC_STAGE_VISUAL_PLANNING: &str = "visual_planning";
pub(super) const SOURCE_SYNC_STAGE_BRANCH_DESIGN: &str = "branch_design";
pub(super) const SOURCE_SYNC_REGENERATION_STAGE_OUTPUT_KEYS: &[&str] = &[
    SOURCE_SYNC_STAGE_SHOOTING_SCRIPT,
    SOURCE_SYNC_STAGE_SCRIPT_ADAPTATION,
    SOURCE_SYNC_STAGE_CHARACTER_STAGING,
    SOURCE_SYNC_STAGE_VISUAL_PLANNING,
    SOURCE_SYNC_STAGE_BRANCH_DESIGN,
];
pub(super) const ALL_GALGAME_PIPELINE_CHECKPOINT_KEYS: &[&str] = &[
    CHECKPOINT_SCENES,
    CHECKPOINT_LINES_DRAFT,
    CHECKPOINT_LINES_STAGED,
    CHECKPOINT_VISUAL_PLAN,
    CHECKPOINT_BRANCH_PLAN,
    CHECKPOINT_PERSIST_SCRIPT,
    CHECKPOINT_ASSET_GENERATION,
];

#[derive(Clone)]
pub struct GalgamePipelineRunOptions {
    pub workflow_run_id: i64,
    pub task_id: Option<String>,
    pub progress_reporter: Option<GalgameProgressReporter>,
    pub resume_from_checkpoint: bool,
    pub start_from_stage: Option<GalgamePipelineStartStage>,
}

#[derive(Debug, Clone)]
pub struct GalgameSceneRegenerationInput {
    pub project_id: i64,
    pub root_project_id: i64,
    pub workflow_run_id: i64,
    pub source_snapshot_id: i64,
    pub source_snapshot_chapter_id: i64,
    pub source_chapter_id: i64,
    pub chapter_number: i32,
    pub chapter_title: Option<String>,
    pub chapter_summary: Option<String>,
    pub chapter_content_hash: String,
    pub chapter_content: String,
    pub scene_id: i64,
    pub scene_index: i32,
    pub scene_title: String,
    pub location: Option<String>,
    pub time_of_day: Option<String>,
    pub background_note: Option<String>,
    pub directing_scene: super::super::GalgamePipelineScene,
    pub source_spans: Vec<GalgamePipelinePromptSourceSpan>,
    pub speaker_profiles: Vec<GalgameSpeakerProfile>,
}

#[derive(Debug, Clone)]
pub struct GalgameSceneRegenerationPlan {
    pub scene: GalgamePersistScene,
    pub stage_outputs_json: Value,
}
