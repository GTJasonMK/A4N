mod script;
mod staging;
mod visual;

use a4n_error::AppError;

use crate::galgame::{GalgameBranchDesign, GalgameSceneRegenerationInput, OrchestratorContext};
use a4n_prompt::rendering::PromptRenderer;

use super::contracts::{GalgameSceneRegenerationStageOutputs, PreparedGalgameSceneRegeneration};

pub(super) async fn run_galgame_scene_regeneration_stages(
    service: &OrchestratorContext,
    input: &GalgameSceneRegenerationInput,
    prepared: &PreparedGalgameSceneRegeneration,
    prompt_renderer: &dyn PromptRenderer,
) -> Result<GalgameSceneRegenerationStageOutputs, AppError> {
    let branch_plan = GalgameBranchDesign {
        choices: Vec::new(),
    };
    let branch = (serde_json::to_value(&branch_plan)?, branch_plan);
    let script =
        script::run_script_adaptation_stage(service, input, prepared, &branch, prompt_renderer)
            .await?;
    let staging =
        staging::run_character_staging_stage(service, input, prepared, &script, prompt_renderer)
            .await?;
    let (visual_plan_json, visual_plan) = visual::run_visual_planning_stage(
        service,
        input,
        prepared,
        &staging,
        &branch.1,
        prompt_renderer,
    )
    .await?;
    Ok(GalgameSceneRegenerationStageOutputs {
        lines_draft_json: script.0,
        lines_staged_json: staging.0,
        visual_plan_json,
        branch_plan_json: branch.0,
        staging: staging.1,
        visual_plan,
        branch_plan: branch.1,
    })
}
