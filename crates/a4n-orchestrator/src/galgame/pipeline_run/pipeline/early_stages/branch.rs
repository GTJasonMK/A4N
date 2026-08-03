use a4n_core::error::AppError;
use serde_json::Value;

use crate::galgame::{
    GalgameBranchDesign, GalgamePipelineStartStage, branch_design_from_value,
    galgame_branch_design_session_user_message, galgame_branch_design_user_message,
};

use super::super::super::contracts::CHECKPOINT_BRANCH_PLAN;
use super::super::super::llm::GalgameLlmStageInput;
use super::super::super::progress::ensure_galgame_pipeline_not_cancelled;
use super::super::contracts::GalgamePipelineExecution;

pub(super) async fn run_branch_design_stage(
    execution: &GalgamePipelineExecution<'_>,
    scenes_json: &Value,
) -> Result<(Value, GalgameBranchDesign, usize), AppError> {
    let stage = GalgamePipelineStartStage::BranchDesign;
    ensure_galgame_pipeline_not_cancelled(
        &execution.service.database,
        execution.options.workflow_run_id,
    )
    .await?;
    execution
        .service
        .report_pipeline_progress(
            execution.options.workflow_run_id,
            execution.options.task_id.as_deref(),
            execution.options.progress_reporter.as_ref(),
            stage,
            2,
            stage.message(),
        )
        .await?;
    let user_prompt = galgame_branch_design_user_message(scenes_json)?;
    let request_user_prompt = galgame_branch_design_session_user_message()?;
    let branch_plan_json = execution
        .service
        .run_galgame_llm_stage(
            GalgameLlmStageInput {
                prompt_renderer: execution.prompt_renderer,
                project_id: execution.project_id,
                root_project_id: execution.prompt_context.root_project_id,
                workflow_run_id: execution.options.workflow_run_id,
                stage,
                checkpoint_key: CHECKPOINT_BRANCH_PLAN,
                prompt_name: "galgame_branch_design",
                user_prompt,
                request_user_prompt,
                shared_shooting_script_projection: scenes_json,
                restore_from_checkpoint: execution.should_restore(stage),
                restore_required: execution.restore_required(stage),
            },
            |value| {
                let parsed = branch_design_from_value(value, "galgame_branch_design")?;
                serde_json::to_value(parsed).map_err(AppError::Serialization)
            },
        )
        .await?;
    let branch_choice_count = branch_plan_json
        .get("choices")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let branch_plan = branch_design_from_value(branch_plan_json.clone(), CHECKPOINT_BRANCH_PLAN)?;
    execution
        .service
        .report_pipeline_progress(
            execution.options.workflow_run_id,
            execution.options.task_id.as_deref(),
            execution.options.progress_reporter.as_ref(),
            stage,
            2,
            "已完成 Galgame 分支设计",
        )
        .await?;
    Ok((branch_plan_json, branch_plan, branch_choice_count))
}
