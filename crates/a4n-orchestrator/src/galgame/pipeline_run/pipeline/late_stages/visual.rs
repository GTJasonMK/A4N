use a4n_core::error::AppError;

use crate::galgame::{
    GalgamePipelineStartStage, GalgameVisualPlan, galgame_visual_planning_session_user_message,
    galgame_visual_planning_user_message, visual_plan_from_value,
};

use super::super::super::contracts::CHECKPOINT_VISUAL_PLAN;
use super::super::super::llm::GalgameLlmStageInput;
use super::super::super::progress::ensure_galgame_pipeline_not_cancelled;
use super::super::contracts::{GalgamePipelineEarlyOutputs, GalgamePipelineExecution};

pub(super) async fn run_visual_planning_stage(
    execution: &GalgamePipelineExecution<'_>,
    early: &GalgamePipelineEarlyOutputs,
) -> Result<GalgameVisualPlan, AppError> {
    let stage = GalgamePipelineStartStage::VisualPlanning;
    ensure_galgame_pipeline_not_cancelled(execution.service, execution.options.workflow_run_id)
        .await?;
    execution
        .service
        .report_pipeline_progress(
            execution.options.workflow_run_id,
            execution.options.task_id.as_deref(),
            execution.options.progress_reporter.as_ref(),
            stage,
            5,
            stage.message(),
        )
        .await?;
    let user_prompt =
        galgame_visual_planning_user_message(&early.scenes_json, &early.lines_staged_json)?;
    let request_user_prompt =
        galgame_visual_planning_session_user_message(&early.lines_staged_json)?;
    let visual_plan_json = execution
        .service
        .run_galgame_llm_stage(
            GalgameLlmStageInput {
                prompt_renderer: execution.prompt_renderer,
                project_id: execution.project_id,
                root_project_id: execution.prompt_context.root_project_id,
                workflow_run_id: execution.options.workflow_run_id,
                stage,
                checkpoint_key: CHECKPOINT_VISUAL_PLAN,
                prompt_name: "galgame_visual_planning",
                user_prompt,
                request_user_prompt,
                shared_shooting_script_projection: &early.scenes_json,
                restore_from_checkpoint: execution.should_restore(stage),
                restore_required: execution.restore_required(stage),
            },
            |value| {
                let parsed = visual_plan_from_value(value, "galgame_visual_planning")?;
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
            5,
            "已完成 Galgame 视觉资产规划",
        )
        .await?;
    visual_plan_from_value(visual_plan_json, CHECKPOINT_VISUAL_PLAN)
}
