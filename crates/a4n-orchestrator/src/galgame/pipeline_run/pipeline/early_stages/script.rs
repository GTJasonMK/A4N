use a4n_core::error::AppError;
use serde_json::Value;

use crate::galgame::{
    GalgamePipelineStartStage, galgame_script_adaptation_user_message, script_adaptation_from_value,
};

use super::super::super::contracts::CHECKPOINT_LINES_DRAFT;
use super::super::super::llm::GalgameLlmStageInput;
use super::super::super::progress::ensure_galgame_pipeline_not_cancelled;
use super::super::contracts::GalgamePipelineExecution;

pub(super) async fn run_script_adaptation_stage(
    execution: &GalgamePipelineExecution<'_>,
    scenes_json: &Value,
    branch_plan_json: &Value,
) -> Result<Value, AppError> {
    let stage = GalgamePipelineStartStage::ScriptAdaptation;
    ensure_galgame_pipeline_not_cancelled(&execution.service.db, execution.options.workflow_run_id)
        .await?;
    execution
        .service
        .report_pipeline_progress(
            execution.options.workflow_run_id,
            execution.options.task_id.as_deref(),
            execution.options.progress_reporter.as_ref(),
            stage,
            3,
            stage.message(),
        )
        .await?;
    let user_prompt = galgame_script_adaptation_user_message(
        &execution.prompt_context.chapters,
        scenes_json,
        branch_plan_json,
    )?;
    let lines_draft_json = execution
        .service
        .run_galgame_llm_stage(
            GalgameLlmStageInput {
                prompt_renderer: execution.prompt_renderer,
                project_id: execution.project_id,
                root_project_id: execution.prompt_context.root_project_id,
                workflow_run_id: execution.options.workflow_run_id,
                stage,
                checkpoint_key: CHECKPOINT_LINES_DRAFT,
                prompt_name: "galgame_script_adaptation",
                user_prompt,
                restore_from_checkpoint: execution.should_restore(stage),
                restore_required: execution.restore_required(stage),
            },
            |value| {
                let parsed = script_adaptation_from_value(value, "galgame_script_adaptation")?;
                serde_json::to_value(parsed).map_err(AppError::Serialization)
            },
        )
        .await?;
    execution
        .service
        .report_pipeline_progress(
            execution.options.workflow_run_id,
            execution.options.task_id.as_deref(),
            execution.options.progress_reporter.as_ref(),
            stage,
            3,
            "已完成 Galgame 剧本行改编",
        )
        .await?;
    Ok(lines_draft_json)
}
