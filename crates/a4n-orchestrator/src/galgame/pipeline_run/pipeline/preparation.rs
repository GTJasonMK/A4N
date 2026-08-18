use a4n_core::error::AppError;

use crate::galgame::{
    GalgamePipelinePromptContext, GalgamePipelineRunOptions, GalgamePipelineStartStage,
    OrchestratorContext,
};

use super::super::contracts::ALL_GALGAME_PIPELINE_CHECKPOINT_KEYS;
use super::super::progress::ensure_galgame_pipeline_not_cancelled;

pub(super) async fn prepare_galgame_pipeline(
    service: &OrchestratorContext,
    project_id: i64,
    options: &GalgamePipelineRunOptions,
) -> Result<GalgamePipelinePromptContext, AppError> {
    service.ensure_media_workflow_v2_ready(project_id).await?;
    let context = service.galgame_pipeline_prompt_context(project_id).await?;
    ensure_galgame_pipeline_not_cancelled(service, options.workflow_run_id).await?;

    if !options.resume_from_checkpoint && options.start_from_stage.is_none() {
        service
            .delete_workflow_checkpoints_by_keys(
                options.workflow_run_id,
                ALL_GALGAME_PIPELINE_CHECKPOINT_KEYS,
            )
            .await?;
    }
    if let Some(start_stage) = options.start_from_stage {
        service
            .delete_workflow_checkpoints_by_keys(
                options.workflow_run_id,
                start_stage.checkpoint_keys_to_clear(),
            )
            .await?;
    }

    service
        .report_pipeline_progress(
            options.workflow_run_id,
            options.task_id.as_deref(),
            options.progress_reporter.as_ref(),
            GalgamePipelineStartStage::ShootingScriptProjection,
            1,
            GalgamePipelineStartStage::ShootingScriptProjection.message(),
        )
        .await?;
    Ok(context)
}
