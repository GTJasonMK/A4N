//! 漫画导演计划驱动的七阶段生成流水线与漫画提示词账本。

use a4n_prompt::rendering::PromptRenderer;

use crate::OrchestratorContext;

mod application;
mod layout_rules;
mod output_sync;
mod pipeline_json;
mod pipeline_persist;
mod pipeline_preview;
mod pipeline_prompt;
mod pipeline_run;
mod pipeline_state;
mod prompt_optimization;
pub mod record_errors;
mod reference_assets;
mod shooting_script_projection;
mod source;
mod workflow;

pub use application::{MangaPipelineApplicationInput, MangaPipelinePreviewApplicationInput};
pub use layout_rules::{bind_reference_asset_in_layout, resolve_manga_prompt_downstream_conflict};
pub use pipeline_preview::MangaPipelineRequestPreviewStep;
pub(in crate::manga) use pipeline_state::{MANGA_PIPELINE_TOTAL_STEPS, MangaPipelineStage};
pub use prompt_optimization::{MangaPromptDownstreamCleanupPort, MangaPromptOptimizationInput};
pub use reference_assets::{MangaCharacterPortraitRef, MangaReferenceContext};
pub use source::ensure_manga_source_can_write;
pub use workflow::{manga_pipeline_business_key, manga_pipeline_fingerprint};

pub struct MangaGeneratePagePromptsInput<'a> {
    pub chapter_id: i64,
    pub project_id: i64,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub reference_context: MangaReferenceContext,
    pub task_id: &'a str,
    pub workflow_run_id: i64,
    pub resume_from_checkpoint: bool,
}

pub struct MangaPipelineRequestPreviewInput<'a> {
    pub chapter_id: i64,
    pub project_id: i64,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub reference_context: MangaReferenceContext,
    pub workflow_run_id: Option<i64>,
    pub resume_from_checkpoint: bool,
}

#[cfg(test)]
mod tests;

pub use a4n_db::manga::{MangaPromptRecordError, MangaPromptUpdatedAt};

use crate::workflow::{
    GetWorkflowRunInput, ListWorkflowRunsInput, UpsertWorkflowRunInput, WORKFLOW_STATUS_CANCELLED,
    WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_INTERRUPTED,
    WORKFLOW_STATUS_RUNNING,
};
use a4n_core::error::AppError;
use a4n_core::models::generation::workflow_run;

impl OrchestratorContext {
    pub(crate) async fn resumable_manga_workflow_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        input_fingerprint: &serde_json::Value,
    ) -> Result<workflow_run::Model, AppError> {
        let business_key = manga_pipeline_business_key(project_id, chapter_id);
        let runs = self
            .workflow_run_models(ListWorkflowRunsInput {
                project_id: Some(project_id),
                chapter_id: Some(chapter_id),
                workflow_kind: Some("manga_pipeline".to_string()),
                status: None,
                resumable_only: Some(true),
                limit: Some(100),
            })
            .await?;
        let mut selected = None;
        for run in runs.into_iter().filter(|run| {
            matches!(
                run.status.as_str(),
                WORKFLOW_STATUS_FAILED | WORKFLOW_STATUS_CANCELLED | WORKFLOW_STATUS_INTERRUPTED
            ) && run.business_key == business_key
                && run.input_fingerprint_json.as_ref() == Some(input_fingerprint)
        }) {
            let completed_checkpoint_count = self
                .workflow_run_detail(GetWorkflowRunInput {
                    workflow_run_id: run.id,
                    include_generation_runs: Some(false),
                })
                .await?
                .checkpoints
                .into_iter()
                .filter(|checkpoint| checkpoint.status == WORKFLOW_STATUS_COMPLETED)
                .count();
            if completed_checkpoint_count == 0 {
                continue;
            }
            if selected
                .as_ref()
                .is_none_or(|(_, best_count)| completed_checkpoint_count > *best_count)
            {
                selected = Some((run, completed_checkpoint_count));
            }
        }
        selected
            .map(|(run, _)| run)
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_resumable_manga_workflow".to_string(),
                operation: "续跑漫画流水线".to_string(),
            })
    }

    /// 漫画流水线：校验后创建 workflow_run（RUNNING）。
    pub(crate) async fn begin_manga_pipeline_workflow_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        content: &str,
        reference_context: &MangaReferenceContext,
        task_id: String,
        resume_from_checkpoint: bool,
    ) -> Result<workflow_run::Model, AppError> {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument("项目 ID 必须大于 0".to_string()));
        }
        if chapter_id <= 0 {
            return Err(AppError::InvalidArgument("章节 ID 必须大于 0".to_string()));
        }
        let input_fingerprint = manga_pipeline_fingerprint(content, reference_context);
        let input = UpsertWorkflowRunInput {
            business_key: manga_pipeline_business_key(project_id, chapter_id),
            workflow_kind: "manga_pipeline".to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("chapter".to_string()),
            scope_id: Some(chapter_id),
            chapter_id: Some(chapter_id),
            mode: None,
            input_fingerprint_json: Some(input_fingerprint.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: None,
            active_task_id: Some(task_id),
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        };
        if resume_from_checkpoint {
            let run = self
                .resumable_manga_workflow_validated(project_id, chapter_id, &input_fingerprint)
                .await?;
            self.mark_workflow_running_validated(run.id).await
        } else {
            self.begin_workflow_run_validated(input).await
        }
    }
}
