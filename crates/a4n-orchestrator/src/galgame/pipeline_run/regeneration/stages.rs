mod branch;
mod script;
mod staging;
mod visual;

use a4n_error::AppError;

use crate::galgame::{GalgameSceneRegenerationInput, OrchestratorContext};
use a4n_prompt::rendering::PromptRenderer;

use super::contracts::{GalgameSceneRegenerationStageOutputs, PreparedGalgameSceneRegeneration};

pub(super) async fn run_galgame_scene_regeneration_stages(
    service: &OrchestratorContext,
    input: &GalgameSceneRegenerationInput,
    prepared: &PreparedGalgameSceneRegeneration,
    prompt_renderer: &dyn PromptRenderer,
) -> Result<GalgameSceneRegenerationStageOutputs, AppError> {
    let (branch_plan_json, branch_plan) = branch::run_branch_design_stage()?;
    let lines_draft_json = script::run_script_adaptation_stage(
        service,
        input,
        prepared,
        &branch_plan_json,
        prompt_renderer,
    )
    .await?;
    let (lines_staged_json, staging) =
        staging::run_character_staging_stage(service, input, &lines_draft_json, prompt_renderer)
            .await?;
    let (visual_plan_json, visual_plan) = visual::run_visual_planning_stage(
        service,
        input,
        prepared,
        &lines_staged_json,
        prompt_renderer,
    )
    .await?;
    Ok(GalgameSceneRegenerationStageOutputs {
        lines_draft_json,
        lines_staged_json,
        visual_plan_json,
        branch_plan_json,
        staging,
        visual_plan,
        branch_plan,
    })
}
