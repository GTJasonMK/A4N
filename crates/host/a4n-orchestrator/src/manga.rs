//! 漫画导演计划驱动的七阶段生成流水线与漫画提示词账本。

mod pipeline;

use a4n_db::project::MangaCharacterPortraitRefSummary;
use a4n_domain::media::GeneratedImageRecord;
use a4n_prompt::rendering::PromptRenderer;
use a4n_service_protocol::ProgressEvent;
use tokio_util::sync::CancellationToken;

use crate::OrchestratorContext;

mod adaptation_projection;
mod application;
mod layout_rules;
mod output_sync;
mod prompt_optimization;
mod record_errors;
mod reference_assets;
mod source;
mod stage_artifacts;
mod workflow;

pub(crate) use pipeline::json::parse_manga_page_plan;
pub(in crate::manga) use pipeline::preview::MangaPipelineRequestPreviewStep;
pub(in crate::manga) use pipeline::state::{MANGA_PIPELINE_TOTAL_STEPS, MangaPipelineStage};
pub(in crate::manga) use prompt_optimization::MangaPromptOptimizationInput;
pub(crate) use record_errors::manga_prompt_record_error;
pub use source::ensure_manga_source_can_write;
pub(crate) use workflow::{manga_pipeline_business_key, manga_pipeline_run_fingerprint};

pub struct MangaGeneratePagePromptsInput<'a> {
    pub user_id: i64,
    pub chapter_id: i64,
    pub chapter_number: i32,
    pub page_number: Option<u32>,
    pub project_id: i64,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub character_portraits: Vec<MangaCharacterPortraitRefSummary>,
    pub available_reference_images: Vec<GeneratedImageRecord>,
    pub task_id: &'a str,
    pub workflow_run_id: i64,
    pub resume_from_checkpoint: bool,
    pub stop_after: Option<a4n_api_contract::manga::MangaPipelineStopAfter>,
    pub run_stage: Option<a4n_domain::manga::MangaPipelineArtifactStage>,
    pub image_size: Option<a4n_domain::ImageSize>,
    pub image_generator: Option<&'a dyn a4n_api_contract::manga::MangaPipelinePageImageGenerator>,
    pub cancel_token: Option<&'a CancellationToken>,
    pub progress_reporter: Option<&'a (dyn Fn(ProgressEvent) + Send + Sync)>,
    pub stream_reporter: Option<&'a (dyn Fn(&str, &str) + Send + Sync)>,
}

pub struct MangaPipelineRequestPreviewInput<'a> {
    pub chapter_id: i64,
    pub project_id: i64,
    pub page_number: Option<u32>,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub character_portraits: Vec<MangaCharacterPortraitRefSummary>,
    pub available_reference_images: Vec<GeneratedImageRecord>,
    pub workflow_run_id: Option<i64>,
    pub resume_from_checkpoint: bool,
    pub stop_after: Option<a4n_api_contract::manga::MangaPipelineStopAfter>,
    pub run_stage: Option<a4n_domain::manga::MangaPipelineArtifactStage>,
}

pub(crate) fn validate_manga_page_scope(
    page_number: Option<u32>,
    stop_after: Option<a4n_api_contract::manga::MangaPipelineStopAfter>,
) -> Result<(), a4n_error::AppError> {
    if page_number == Some(0) {
        return Err(a4n_error::AppError::InvalidArgument(
            "漫画页码必须大于 0".into(),
        ));
    }
    if page_number.is_some()
        && matches!(
            stop_after,
            Some(a4n_api_contract::manga::MangaPipelineStopAfter::PageSlots)
        )
    {
        return Err(a4n_error::AppError::InvalidArgument(
            "分页规划是整章步骤，不能限定单页".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;

use crate::workflow::{
    ListWorkflowRunsInput, UpsertWorkflowRunInput, WORKFLOW_STATUS_CANCELLED,
    WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_INTERRUPTED,
    WORKFLOW_STATUS_RUNNING,
};
use a4n_domain::workflow::WorkflowRunSummary;
use a4n_error::AppError;

impl OrchestratorContext {
    pub(crate) async fn ensure_manga_pipeline_not_running(
        &self,
        project_id: i64,
        chapter_id: i64,
        operation: &str,
    ) -> Result<(), AppError> {
        let business_key = manga_pipeline_business_key(project_id, chapter_id);
        if self
            .latest_workflow_run_by_business_key(&business_key)
            .await?
            .is_some_and(|run| run.status == a4n_domain::workflow::WORKFLOW_STATUS_RUNNING)
        {
            return Err(AppError::InvalidState {
                current: "manga_pipeline_running".to_string(),
                operation: format!("{operation}：漫画流水线正在运行"),
            });
        }
        Ok(())
    }

    async fn completed_manga_pipeline_stage_workflow(
        &self,
        project_id: i64,
        chapter_id: i64,
        fingerprint: &serde_json::Value,
        stop_after: Option<a4n_api_contract::manga::MangaPipelineStopAfter>,
        page_number: Option<u32>,
    ) -> Result<Option<WorkflowRunSummary>, AppError> {
        let next_stage_order = manga_stop_after_order(stop_after);
        Ok(self
            .latest_workflow_run_by_business_key(&manga_pipeline_business_key(
                project_id, chapter_id,
            ))
            .await?
            .filter(|run| {
                run.status == WORKFLOW_STATUS_COMPLETED
                    && run.input_fingerprint_json.as_ref() == Some(fingerprint)
                    && run
                        .current_step_key
                        .as_deref()
                        .and_then(manga_completed_stage_order)
                        .is_some_and(|stage_order| {
                            manga_completed_stage_can_continue(
                                stage_order,
                                next_stage_order,
                                page_number.is_some(),
                            )
                        })
            }))
    }
    pub(crate) async fn resumable_manga_workflow_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        input_fingerprint: &serde_json::Value,
    ) -> Result<WorkflowRunSummary, AppError> {
        let business_key = manga_pipeline_business_key(project_id, chapter_id);
        let runs = self
            .workflow_run_models(ListWorkflowRunsInput {
                project_id: Some(project_id),
                chapter_id: Some(chapter_id),
                chapter_ids: None,
                workflow_kind: Some("manga_pipeline".to_string()),
                status: None,
                resumable_only: Some(true),
                limit: Some(100),
            })
            .await?;
        let candidates = runs
            .into_iter()
            .filter(|run| {
                matches!(
                    run.status.as_str(),
                    WORKFLOW_STATUS_FAILED
                        | WORKFLOW_STATUS_CANCELLED
                        | WORKFLOW_STATUS_INTERRUPTED
                ) && run.business_key == business_key
                    && run.input_fingerprint_json.as_ref() == Some(input_fingerprint)
            })
            .collect::<Vec<_>>();
        let checkpoint_rows = self
            .workflow_checkpoint_statuses_for_runs(
                &candidates.iter().map(|run| run.id).collect::<Vec<_>>(),
            )
            .await?;
        let mut completed_checkpoint_counts = std::collections::HashMap::<i64, usize>::new();
        for (workflow_run_id, status) in checkpoint_rows {
            if status == WORKFLOW_STATUS_COMPLETED {
                *completed_checkpoint_counts
                    .entry(workflow_run_id)
                    .or_default() += 1;
            }
        }
        let mut selected = None;
        for run in candidates {
            let completed_checkpoint_count = completed_checkpoint_counts
                .get(&run.id)
                .copied()
                .unwrap_or_default();
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
        character_portraits: &[MangaCharacterPortraitRefSummary],
        available_reference_images: &[GeneratedImageRecord],
        task_id: String,
        resume_from_checkpoint: bool,
        stop_after: Option<a4n_api_contract::manga::MangaPipelineStopAfter>,
        run_stage: Option<a4n_domain::manga::MangaPipelineArtifactStage>,
        page_number: Option<u32>,
    ) -> Result<(WorkflowRunSummary, bool), AppError> {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument("项目 ID 必须大于 0".to_string()));
        }
        if chapter_id <= 0 {
            return Err(AppError::InvalidArgument("章节 ID 必须大于 0".to_string()));
        }
        let input_fingerprint = manga_pipeline_run_fingerprint(
            content,
            character_portraits,
            available_reference_images,
            run_stage,
        );
        let business_key = manga_pipeline_business_key(project_id, chapter_id);
        let input = UpsertWorkflowRunInput {
            business_key: business_key.clone(),
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
            active_task_id: Some(task_id.clone()),
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        };
        if resume_from_checkpoint {
            let run = self
                .resumable_manga_workflow_validated(project_id, chapter_id, &input_fingerprint)
                .await?;
            return self
                .resume_workflow_run_with_active_task_validated(run.id, &task_id)
                .await
                .map(|run| (run, true));
        }
        if let Some(previous) = self
            .latest_workflow_run_by_business_key(&business_key)
            .await?
            .filter(|run| {
                run.workflow_kind == "manga_pipeline"
                    && run.status == WORKFLOW_STATUS_INTERRUPTED
                    && run.input_fingerprint_json.as_ref() == Some(&input_fingerprint)
            })
        {
            return self
                .restart_interrupted_workflow_run_validated(previous.id, input)
                .await
                .map(|run| (run, false));
        }
        if let Some(previous) = self
            .completed_manga_pipeline_stage_workflow(
                project_id,
                chapter_id,
                &input_fingerprint,
                stop_after,
                page_number,
            )
            .await?
        {
            return self
                .continue_completed_workflow_run_validated(previous.id, input)
                .await
                .map(|run| (run, true));
        }
        self.begin_workflow_run_validated(input)
            .await
            .map(|run| (run, false))
    }
}

fn manga_stop_after_order(
    stop_after: Option<a4n_api_contract::manga::MangaPipelineStopAfter>,
) -> u8 {
    use a4n_api_contract::manga::MangaPipelineStopAfter as StopAfter;
    match stop_after {
        Some(StopAfter::PageSlots) => 0,
        Some(StopAfter::PageAdaptation) => 1,
        Some(StopAfter::Storyboard) => 2,
        Some(StopAfter::ReferenceDiscovery) => 3,
        Some(StopAfter::DrawingPrompt) => 4,
        Some(StopAfter::Image) | None => 5,
    }
}

fn manga_completed_stage_order(stage: &str) -> Option<u8> {
    match stage {
        "page_content" => Some(0),
        "adaptation" => Some(1),
        "storyboard" => Some(2),
        "reference_discovery" => Some(3),
        "drawing_prompt" => Some(4),
        "completed" => Some(5),
        _ => None,
    }
}

pub(crate) fn manga_completed_stage_can_continue(
    completed_stage_order: u8,
    next_stage_order: u8,
    page_scoped: bool,
) -> bool {
    completed_stage_order < next_stage_order
        || (page_scoped && completed_stage_order == next_stage_order)
}
