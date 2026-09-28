use a4n_error::AppError;

use super::contracts::{GalgamePipelineEarlyOutputs, GalgamePipelineExecution};

mod branch;
pub(super) mod scene;
mod script;
mod staging;

pub(super) async fn run_galgame_pipeline_early_stages(
    execution: &GalgamePipelineExecution<'_>,
) -> Result<Option<GalgamePipelineEarlyOutputs>, AppError> {
    let (scenes_json, scenes) = scene::run_adaptation_projection_stage(execution).await?;
    let (branch_plan_json, branch_plan, branch_plan_ref) =
        branch::run_branch_design_stage(execution, &scenes_json, &scenes).await?;
    if execution.options.stop_after
        == Some(a4n_api_contract::galgame::GalgamePipelineStopAfter::BranchDesign)
    {
        return Ok(None);
    }
    let (lines_draft_json, script) = script::run_script_adaptation_stage(
        execution,
        &scenes_json,
        &scenes,
        &branch_plan_json,
        &branch_plan,
    )
    .await?;
    let (lines_staged_json, staging) =
        staging::run_character_staging_stage(execution, &scenes_json, &lines_draft_json, &script)
            .await?;
    Ok(Some(GalgamePipelineEarlyOutputs {
        scenes_json,
        scenes,
        branch_plan,
        branch_plan_ref,
        lines_staged_json,
        staging,
    }))
}
