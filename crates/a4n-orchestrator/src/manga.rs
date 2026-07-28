//! 漫画导演计划驱动的七阶段生成流水线与漫画提示词账本。

use std::collections::HashMap;

use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;

use crate::OrchestratorContext;

mod application;
mod checkpoint;
mod layout_rules;
mod output_sync;
mod pipeline_json;
mod pipeline_persist;
mod pipeline_preview;
mod pipeline_prompt;
mod pipeline_run;
mod pipeline_state;
mod pipeline_status;
mod prompt_optimization;
mod prompt_optimization_workflow_rules;
pub mod record_errors;
mod reference_assets;
mod shooting_script_projection;
mod source;
mod workflow;

pub use application::{
    BindReferenceImageInput, MangaPipelineApplicationInput, MangaPipelinePreviewApplicationInput,
    MangaPipelineStatusSummary, OptimizeMangaPromptRecordInput, UpdateMangaPromptInput,
};
pub use layout_rules::{
    bind_reference_asset_in_layout, optional_layout_string, required_layout_bool,
    required_layout_i32, required_layout_string, resolve_manga_prompt_downstream_conflict,
};
pub use pipeline_preview::MangaPipelineRequestPreviewStep;
pub use pipeline_state::MangaPipelineStartStage;
pub use prompt_optimization::{MangaPromptDownstreamCleanupPort, MangaPromptOptimizationInput};
pub use reference_assets::{MangaCharacterPortraitRef, MangaReferenceContext};
pub use source::{ensure_manga_source_can_write, manga_checkpoint_matches_source};
pub use workflow::{
    manga_pipeline_business_key, manga_pipeline_fingerprint, parse_manga_pipeline_start_stage,
};

pub const PIPELINE_STATUS_PROMPT_TYPE: &str = "pipeline_status";
pub const MANGA_PAGE_PROMPT_TYPE: &str = "page_generation";
pub const MANGA_PANEL_PLAN_TYPE: &str = "panel_plan";
pub const PIPELINE_TOTAL_STEPS: i32 = 7;

pub struct MangaGeneratePanelsInput<'a> {
    pub chapter_id: i64,
    pub project_id: i64,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, Value>,
    pub reference_context: MangaReferenceContext,
    pub task_id: Option<&'a str>,
    pub workflow_run_id: Option<i64>,
    pub resume_from_checkpoint: bool,
    pub start_from_stage: Option<MangaPipelineStartStage>,
}

pub struct MangaPipelineRequestPreviewInput<'a> {
    pub chapter_id: i64,
    pub project_id: i64,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, Value>,
    pub reference_context: MangaReferenceContext,
    pub workflow_run_id: Option<i64>,
    pub resume_from_checkpoint: bool,
    pub start_from_stage: Option<MangaPipelineStartStage>,
}

#[cfg(test)]
mod tests;

pub use a4n_db::manga::{MangaPipelineStatusRows, MangaPromptRecordError, MangaPromptUpdatedAt};

pub(crate) use pipeline_status::contracts::{
    MangaPipelineCheckpointInput, MangaPipelineStatusInput,
};

use crate::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};
use a4n_core::error::AppError;
use a4n_core::models::generation::workflow_run;

impl OrchestratorContext {
    /// 漫画流水线：校验后创建 workflow_run（RUNNING）。
    pub(crate) async fn begin_manga_pipeline_workflow_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        content: &str,
        reference_context: &MangaReferenceContext,
        start_from_stage: Option<MangaPipelineStartStage>,
        task_id: Option<String>,
    ) -> Result<workflow_run::Model, AppError> {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument("项目 ID 必须大于 0".to_string()));
        }
        if chapter_id <= 0 {
            return Err(AppError::InvalidArgument("章节 ID 必须大于 0".to_string()));
        }
        let mode = start_from_stage.map(|stage| stage.key().to_string());
        let step = start_from_stage.map(|stage| stage.key().to_string());
        self.begin_workflow_run_validated(UpsertWorkflowRunInput {
            business_key: manga_pipeline_business_key(project_id, chapter_id),
            workflow_kind: "manga_pipeline".to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("chapter".to_string()),
            scope_id: Some(chapter_id),
            chapter_id: Some(chapter_id),
            mode,
            input_fingerprint_json: Some(manga_pipeline_fingerprint(content, reference_context)),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: step,
            active_task_id: task_id,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        })
        .await
    }
}
