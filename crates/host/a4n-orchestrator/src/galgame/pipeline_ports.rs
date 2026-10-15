use a4n_api_contract::galgame::{
    GalgamePipelineCancellationApplicationPort, GalgamePipelineExecutionApplicationPort,
    GalgamePipelinePreviewApplicationPort, GalgamePipelinePreviewStep,
    GalgamePipelineReadApplicationPort, GalgamePipelineStatusSummary, GalgameProgressReporter,
    PreviewGalgamePipelineInput, StartGalgamePipelineInput,
};
use a4n_error::AppError;

use crate::{OrchestratorContext, prompt_records::PromptRendererAdapter};

use super::{GalgameScriptDetail, application::GalgamePipelineApplicationInput};

#[async_trait::async_trait]
impl GalgamePipelineExecutionApplicationPort for OrchestratorContext {
    async fn start(
        &self,
        user_id: i64,
        input: StartGalgamePipelineInput,
        progress_reporter: Option<GalgameProgressReporter>,
        prompt_renderer: &dyn a4n_api_contract::prompt::PromptRenderPort,
    ) -> Result<Option<GalgameScriptDetail>, AppError> {
        let prompt_renderer = PromptRendererAdapter(prompt_renderer);
        self.run_galgame_pipeline_for_user_validated(
            user_id,
            GalgamePipelineApplicationInput {
                project_id: input.project_id,
                task_id: input.task_id,
                start_from_stage: input.start_from_stage,
                resume_from_checkpoint: input.resume_from_checkpoint,
                stop_after: input.stop_after,
                progress_reporter,
            },
            &prompt_renderer,
        )
        .await
    }
}

#[async_trait::async_trait]
impl GalgamePipelinePreviewApplicationPort for OrchestratorContext {
    async fn preview(
        &self,
        user_id: i64,
        input: PreviewGalgamePipelineInput,
        prompt_renderer: &dyn a4n_api_contract::prompt::PromptRenderPort,
    ) -> Result<Vec<GalgamePipelinePreviewStep>, AppError> {
        let prompt_renderer = PromptRendererAdapter(prompt_renderer);
        let mut steps = self
            .prepare_galgame_pipeline_preview_for_user_validated(
                user_id,
                input.project_id,
                input.start_from_stage,
                input.resume_from_checkpoint,
                input.stop_after,
                &prompt_renderer,
            )
            .await?;
        if input.stop_after
            == Some(a4n_api_contract::galgame::GalgamePipelineStopAfter::BranchDesign)
            && let Some(index) = steps.iter().position(|step| step.key == "branch_design")
        {
            steps.truncate(index + 1);
        }
        steps
            .into_iter()
            .map(galgame_pipeline_preview_step)
            .collect()
    }
}

#[async_trait::async_trait]
impl GalgamePipelineReadApplicationPort for OrchestratorContext {
    async fn status(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<GalgamePipelineStatusSummary, AppError> {
        self.get_galgame_pipeline_status_for_user_validated(user_id, project_id)
            .await
    }
}

#[async_trait::async_trait]
impl GalgamePipelineCancellationApplicationPort for OrchestratorContext {
    async fn cancel(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<GalgamePipelineStatusSummary, AppError> {
        self.cancel_galgame_pipeline_for_user_validated(user_id, project_id)
            .await?;
        self.get_galgame_pipeline_status_for_user_validated(user_id, project_id)
            .await
    }
}

fn galgame_pipeline_preview_step(
    step: super::GalgamePipelinePreviewRequest,
) -> Result<GalgamePipelinePreviewStep, AppError> {
    Ok(GalgamePipelinePreviewStep {
        key: step.key,
        title: step.title,
        prompt_name: step.prompt_name,
        request: Some(serde_json::to_value(step.request)?),
    })
}
