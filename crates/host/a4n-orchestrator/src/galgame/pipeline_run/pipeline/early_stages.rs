use a4n_error::AppError;

use super::contracts::{GalgamePipelineEarlyOutputs, GalgamePipelineExecution};

mod branch;
pub(super) mod scene;
mod script;
mod staging;

pub(super) async fn run_galgame_pipeline_early_stages(
    execution: &GalgamePipelineExecution<'_>,
) -> Result<GalgamePipelineEarlyOutputs, AppError> {
    let (scenes_json, scenes) = scene::run_shooting_script_projection_stage(execution).await?;
    let (branch_plan_json, branch_plan) =
        branch::run_branch_design_stage(execution, &scenes_json, &scenes).await?;
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
    Ok(GalgamePipelineEarlyOutputs {
        scenes_json,
        scenes,
        branch_plan,
        lines_staged_json,
        staging,
    })
}
